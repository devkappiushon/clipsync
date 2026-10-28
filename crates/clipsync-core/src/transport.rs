//! QUIC mesh (contract §5 rules 4 to 10). Owned by Rivet.
//!
//! One endpoint per agent, server and client at once. Every trusted peer gets a dialer with
//! backoff. A connection only counts after the application Hello went both ways; until then
//! nothing reaches the modules. Module frames ride one uni stream each, input rides datagrams,
//! and every inbound message passes `authorize_inbound*` here, before any module sees it.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clipsync_proto::{
    ALPN_MESH, BlockRecord, ClipMsg, CloseCode, Datagram, EventId, Frame, Hello, InputDatagram,
    MAX_CONTROL_FRAME_BYTES, MAX_FRAME_BYTES, MAX_RECENT_BLOCKS, MAX_RECENT_REMOVALS,
    MIN_PROTOCOL_VERSION, ModuleHost, ModuleInput, ModuleKind, ModuleMsg, ModuleSet,
    PROTOCOL_VERSION, PeerId, PeerInfo, PeerPerms, PeerState, PeerStatus, RemovalRecord, SendError,
    WireError, authorize_inbound, authorize_inbound_datagram, authorize_outbound,
    authorize_outbound_datagram, decode_datagram, encode_datagram, encode_frame, min_version,
    read_frame, write_frame,
};
use quinn::{Connection, Endpoint, RecvStream, SendStream, VarInt};
use tokio::sync::{Notify, mpsc, watch};
use tokio::time::timeout;

use crate::tls::{self, PinSet, PinVerifier, VerifyMode};
use crate::{CoreError, Identity, PeerEntry, Peers, msg};

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);
/// The larger PeerId waits this long before dialing, so the smaller one dials first.
const DEFER: Duration = Duration::from_secs(2);
/// Time the control tasks get to put Bye on the wire before the connections close.
const BYE_GRACE: Duration = Duration::from_millis(100);
/// `Bye{Revoked}` only goes to a peer that decodes it (v1.16): a 0.1.0 would close with Protocol.
const REVOKED_MIN_VERSION: u16 = 2;

pub fn varint(code: CloseCode) -> VarInt {
    VarInt::from_u32(code.code())
}

/// Closes with the code and its fixed token only (rule 9).
pub fn close(conn: &Connection, code: CloseCode) {
    conn.close(varint(code), code.reason());
}

/// Reads one control frame. Above MAX_CONTROL_FRAME_BYTES the stream is stopped and reset with
/// CloseCode::Protocol before anything is allocated (rule 10).
pub async fn read_control(
    send: &mut SendStream,
    recv: &mut RecvStream,
) -> Result<Frame, WireError> {
    let res = read_frame(recv, MAX_CONTROL_FRAME_BYTES).await;
    if let Err(WireError::TooLarge { .. }) = res {
        let _ = recv.stop(varint(CloseCode::Protocol));
        let _ = send.reset(varint(CloseCode::Protocol));
    }
    res
}

pub fn hello_compatible(h: &Hello) -> bool {
    h.version >= MIN_PROTOCOL_VERSION && PROTOCOL_VERSION >= h.min_version
}

fn offer_id(msg: &ModuleMsg) -> Option<EventId> {
    match msg {
        ModuleMsg::Clip(ClipMsg::Offer(o)) => Some(o.id),
        _ => None,
    }
}

fn jitter(d: Duration) -> Duration {
    // ±20%
    d.mul_f64(0.8 + 0.4 * rand::random::<f64>())
}

pub struct MeshOptions {
    pub identity: Identity,
    pub device_name: String,
    pub listen: SocketAddr,
    /// One channel per module that is ON. A kind missing here is OFF: not announced, refused.
    pub modules: HashMap<ModuleKind, mpsc::Sender<ModuleInput>>,
    pub peers: Peers,
    pub paused: bool,
}

/// Running mesh. Dropping it does not stop the tasks: call `shutdown`.
pub struct Mesh {
    inner: Arc<Inner>,
    /// Taken once by the agent (`revocations`).
    revoked: Mutex<Option<mpsc::UnboundedReceiver<PeerId>>>,
}

struct Inner {
    id: PeerId,
    identity: Identity,
    hello: Hello,
    local: ModuleSet,
    endpoint: Endpoint,
    pins: Arc<PinSet>,
    server_verifier: Arc<PinVerifier>,
    modules: HashMap<ModuleKind, mpsc::Sender<ModuleInput>>,
    paused: AtomicBool,
    peers: Mutex<HashMap<PeerId, Slot>>,
    /// Newest first, at most MAX_RECENT_BLOCKS. Locked on its own, never inside `peers`.
    blocks: Mutex<VecDeque<BlockRecord>>,
    /// Peers that removed this device, newest first, at most MAX_RECENT_REMOVALS. Locked on its
    /// own, never inside `peers`.
    removals: Mutex<VecDeque<RemovalRecord>>,
    /// To the agent: this peer sent `Bye{Revoked}`, take it out of peers.toml.
    revoked: mpsc::UnboundedSender<PeerId>,
    next_gen: AtomicU64,
    stop: watch::Sender<bool>,
}

struct Slot {
    entry: PeerEntry,
    state: PeerState,
    active: Option<Active>,
    /// Wakes the dialer: connection lost, peers reloaded.
    wake: Arc<Notify>,
}

struct Active {
    conn: Connection,
    /// Tells the control task to send `Bye{Revoked}` (the peer left peers.toml).
    revoke: Arc<Notify>,
    remote: ModuleSet,
    /// `Hello::version` of the peer on this connection.
    version: u16,
    dialed_by: PeerId,
    generation: u64,
}

impl Slot {
    fn new(entry: PeerEntry) -> Self {
        Self {
            entry,
            state: PeerState::Disconnected,
            active: None,
            wake: Arc::new(Notify::new()),
        }
    }

    fn info(&self) -> PeerInfo {
        PeerInfo {
            id: self.entry.id,
            name: self.entry.name.clone(),
            perms: self.entry.perms,
            remote_modules: self.active.as_ref().map(|a| a.remote).unwrap_or_default(),
            connected: self.active.is_some(),
            remote_version: self.active.as_ref().map_or(0, |a| a.version),
        }
    }
}

enum DialFail {
    KeyChanged(PeerId),
    Other(String),
}

enum HelloFail {
    /// We close with this code.
    Close(CloseCode),
    /// The connection already failed.
    Lost(String),
}

impl From<WireError> for HelloFail {
    fn from(e: WireError) -> Self {
        match e {
            WireError::Io(e) => HelloFail::Lost(e.to_string()),
            _ => HelloFail::Close(CloseCode::Protocol),
        }
    }
}

enum Installed {
    Lost,
    Won {
        old: Option<Connection>,
        fresh: bool,
        info: PeerInfo,
    },
}

impl Mesh {
    pub fn start(opts: MeshOptions) -> Result<Self, CoreError> {
        let id = opts.identity.id();
        let local = ModuleSet {
            clipboard: opts.modules.contains_key(&ModuleKind::Clipboard),
            input: opts.modules.contains_key(&ModuleKind::Input),
        };
        let pins = Arc::new(PinSet::new(opts.peers.pins()));
        let server_verifier = PinVerifier::new(VerifyMode::Mesh(pins.clone()));
        let server = tls::server_config(&opts.identity, server_verifier.clone(), ALPN_MESH)?;
        let endpoint = Endpoint::server(server, opts.listen)
            .map_err(|e| CoreError::Transport(format!("bind {}: {e}", opts.listen)))?;
        let listen_port = endpoint
            .local_addr()
            .map(|a| a.port())
            .unwrap_or(opts.listen.port());
        let hello = Hello {
            version: PROTOCOL_VERSION,
            min_version: MIN_PROTOCOL_VERSION,
            device_name: opts.device_name,
            modules: local,
            listen_port,
        };
        let slots: HashMap<PeerId, Slot> = opts
            .peers
            .peer
            .into_iter()
            .map(|e| (e.id, Slot::new(e)))
            .collect();
        let ids: Vec<PeerId> = slots.keys().copied().collect();
        let (revoked, revoked_rx) = mpsc::unbounded_channel();
        let inner = Arc::new(Inner {
            id,
            identity: opts.identity,
            hello,
            local,
            endpoint,
            pins,
            server_verifier,
            modules: opts.modules,
            paused: AtomicBool::new(opts.paused),
            peers: Mutex::new(slots),
            blocks: Mutex::new(VecDeque::new()),
            removals: Mutex::new(VecDeque::new()),
            revoked,
            next_gen: AtomicU64::new(1),
            stop: watch::Sender::new(false),
        });
        tokio::spawn(inner.clone().accept_loop());
        for peer in ids {
            tokio::spawn(inner.clone().dial_loop(peer));
        }
        Ok(Self {
            inner,
            revoked: Mutex::new(Some(revoked_rx)),
        })
    }

    /// Peers that sent `Bye{Revoked}` over their authenticated mesh connection (v1.16). The agent
    /// takes this once and removes each from peers.toml, then calls `reload`.
    pub fn revocations(&self) -> Option<mpsc::UnboundedReceiver<PeerId>> {
        self.revoked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Peers that removed this device lately, newest first.
    pub fn recent_removals(&self) -> Vec<RemovalRecord> {
        self.inner
            .removals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }

    pub fn local_id(&self) -> PeerId {
        self.inner.id
    }

    pub fn local_addr(&self) -> Result<SocketAddr, CoreError> {
        self.inner.endpoint.local_addr().map_err(CoreError::Io)
    }

    /// Modules that are ON (announced in Hello).
    pub fn local_modules(&self) -> ModuleSet {
        self.inner.local
    }

    pub fn host(&self) -> Arc<dyn ModuleHost> {
        Arc::new(MeshHost(self.inner.clone()))
    }

    pub fn paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    /// Applies at once and tells every module (ModuleInput::Paused). Persisting is the agent's job.
    pub async fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::SeqCst);
        self.inner.broadcast(|| ModuleInput::Paused(paused)).await;
    }

    /// Certificates checked by the server verifier (tests: resumption is off).
    pub fn server_verifier_calls(&self) -> usize {
        self.inner.server_verifier.calls()
    }

    pub fn peer_statuses(&self) -> Vec<PeerStatus> {
        let peers = self.inner.lock();
        let mut out: Vec<PeerStatus> = peers
            .values()
            .map(|s| PeerStatus {
                name: s.entry.name.clone(),
                id: s.entry.id,
                addr: s.entry.addr,
                state: s.state.clone(),
                perms: s.entry.perms,
                remote_modules: s.active.as_ref().map(|a| a.remote).unwrap_or_default(),
                remote_version: s.active.as_ref().map_or(0, |a| a.version),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        out
    }

    /// A module reported a new `last_block` (no peer attached): it joins `recent_blocks`.
    pub fn record_module_block(&self, module: ModuleKind, reason: String) {
        self.inner.push_block(None, Some(module), reason);
    }

    /// What the mesh refused lately, newest first.
    pub fn recent_blocks(&self) -> Vec<BlockRecord> {
        self.inner
            .blocks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }

    /// New peers.toml (ReloadPeers): pins change at once; removed peers are told `Bye{Revoked}`
    /// (version 2 only, v1.16), closed and announced down; new peers get a dialer; changed
    /// permissions are announced with a fresh PeerUp.
    pub async fn reload(&self, peers: Peers) {
        let inner = &self.inner;
        inner.pins.replace(peers.pins());
        let keep: HashSet<PeerId> = peers.peer.iter().map(|e| e.id).collect();
        let (mut down, mut up, mut spawn) = (Vec::new(), Vec::new(), Vec::new());
        let mut closing = Vec::new();
        {
            let mut slots = inner.lock();
            slots.retain(|id, slot| {
                if keep.contains(id) {
                    return true;
                }
                if let Some(a) = &slot.active {
                    closing.push((a.conn.clone(), a.revoke.clone(), a.version));
                    down.push(*id);
                }
                slot.wake.notify_one();
                false
            });
            for entry in peers.peer {
                match slots.get_mut(&entry.id) {
                    Some(slot) => {
                        let changed =
                            slot.entry.perms != entry.perms || slot.entry.name != entry.name;
                        let readdressed = slot.entry.addr != entry.addr;
                        slot.entry = entry;
                        if changed && slot.active.is_some() {
                            up.push(slot.info());
                        }
                        if readdressed {
                            slot.wake.notify_one();
                        }
                    }
                    None => {
                        spawn.push(entry.id);
                        slots.insert(entry.id, Slot::new(entry));
                    }
                }
            }
        }
        // The control task writes the Bye; the close waits the same grace as a shutdown Bye.
        let mut told = false;
        for (_, revoke, version) in &closing {
            if *version >= REVOKED_MIN_VERSION {
                revoke.notify_one();
                told = true;
            }
        }
        if told {
            tokio::time::sleep(BYE_GRACE).await;
        }
        for (conn, _, _) in &closing {
            close(conn, CloseCode::UnknownPeer);
        }
        for id in spawn {
            tokio::spawn(inner.clone().dial_loop(id));
        }
        for id in down {
            inner.broadcast(|| ModuleInput::PeerDown(id)).await;
        }
        for info in up {
            inner.broadcast(|| ModuleInput::PeerUp(info.clone())).await;
        }
    }

    /// Bye on every control stream, then close every connection with CloseCode::Shutdown.
    pub async fn shutdown(&self) {
        let inner = &self.inner;
        inner.stop.send_replace(true);
        tokio::time::sleep(BYE_GRACE).await;
        let conns: Vec<Connection> = inner
            .lock()
            .values()
            .filter_map(|s| s.active.as_ref().map(|a| a.conn.clone()))
            .collect();
        for c in &conns {
            close(c, CloseCode::Shutdown);
        }
        inner
            .endpoint
            .close(varint(CloseCode::Shutdown), CloseCode::Shutdown.reason());
        let _ = timeout(Duration::from_secs(2), inner.endpoint.wait_idle()).await;
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, HashMap<PeerId, Slot>> {
        self.peers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn stopped(&self) -> bool {
        *self.stop.borrow()
    }

    async fn until_stop(&self) {
        let mut rx = self.stop.subscribe();
        let _ = rx.wait_for(|s| *s).await;
    }

    fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    fn perms_of(&self, peer: &PeerId) -> Option<PeerPerms> {
        self.lock().get(peer).map(|s| s.entry.perms)
    }

    /// Call with `peers` unlocked: the name is read from it.
    fn push_block(&self, peer: Option<PeerId>, module: Option<ModuleKind>, reason: String) {
        let peer_name = peer.map(|id| {
            self.lock()
                .get(&id)
                .map_or_else(|| id.short(), |s| s.entry.name.clone())
        });
        let at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let mut blocks = self.blocks.lock().unwrap_or_else(PoisonError::into_inner);
        blocks.push_front(BlockRecord {
            at_unix_ms,
            peer,
            peer_name,
            module,
            reason,
        });
        blocks.truncate(MAX_RECENT_BLOCKS);
    }

    /// The peer removed us: remember it (status) and ask the agent to remove it from peers.toml.
    fn revoked_by(&self, peer: PeerId) {
        let Some(peer_name) = self.lock().get(&peer).map(|s| s.entry.name.clone()) else {
            return;
        };
        tracing::info!(peer = %peer, "removed by the peer");
        let at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        {
            let mut removals = self.removals.lock().unwrap_or_else(PoisonError::into_inner);
            removals.push_front(RemovalRecord {
                at_unix_ms,
                peer,
                peer_name,
            });
            removals.truncate(MAX_RECENT_REMOVALS);
        }
        let _ = self.revoked.send(peer);
    }

    async fn broadcast(&self, make: impl Fn() -> ModuleInput) {
        for tx in self.modules.values() {
            let _ = tx.send(make()).await;
        }
    }

    async fn deliver(&self, kind: ModuleKind, input: ModuleInput) {
        if let Some(tx) = self.modules.get(&kind) {
            let _ = tx.send(input).await;
        }
    }

    fn set_state(&self, peer: PeerId, state: PeerState) {
        if let Some(slot) = self.lock().get_mut(&peer)
            && slot.active.is_none()
            && !matches!(slot.state, PeerState::Blocked { .. })
        {
            slot.state = state;
        }
    }

    async fn accept_loop(self: Arc<Self>) {
        loop {
            let incoming = tokio::select! {
                i = self.endpoint.accept() => i,
                _ = self.until_stop() => None,
            };
            let Some(incoming) = incoming else { break };
            tokio::spawn(self.clone().handle_incoming(incoming));
        }
    }

    async fn handle_incoming(self: Arc<Self>, incoming: quinn::Incoming) {
        let conn = match incoming.await {
            Ok(c) => c,
            // Unknown peers are logged by the verifier.
            Err(e) => return tracing::debug!(error = %e, "inbound handshake failed"),
        };
        let Some(peer) = tls::peer_id_of(&conn) else {
            return close(&conn, CloseCode::Protocol);
        };
        let hs = timeout(HELLO_TIMEOUT, async {
            let (mut send, mut recv) = conn
                .accept_bi()
                .await
                .map_err(|e| HelloFail::Lost(e.to_string()))?;
            let hello = self.read_hello(&mut send, &mut recv).await?;
            write_frame(
                &mut send,
                &Frame::Hello(self.hello.clone()),
                MAX_CONTROL_FRAME_BYTES,
            )
            .await?;
            Ok::<_, HelloFail>((send, recv, hello))
        })
        .await;
        match hs {
            Ok(Ok((send, recv, hello))) => self.install(peer, conn, peer, hello, send, recv).await,
            Ok(Err(HelloFail::Close(code))) => close(&conn, code),
            Ok(Err(HelloFail::Lost(e))) => {
                tracing::debug!(peer = %peer, error = %e, "inbound hello failed")
            }
            Err(_) => close(&conn, CloseCode::Protocol),
        }
    }

    async fn read_hello(
        &self,
        send: &mut SendStream,
        recv: &mut RecvStream,
    ) -> Result<Hello, HelloFail> {
        match read_control(send, recv).await? {
            Frame::Hello(h) if hello_compatible(&h) => Ok(h),
            Frame::Hello(h) => {
                tracing::warn!(
                    remote = h.version,
                    remote_min = h.min_version,
                    "{}",
                    msg::VERSION_MISMATCH
                );
                Err(HelloFail::Close(CloseCode::Version))
            }
            _ => Err(HelloFail::Close(CloseCode::Protocol)),
        }
    }

    /// Returns `None` when the peer is gone or blocked (the dialer stops).
    fn dial_target(&self, peer: PeerId) -> Option<(Option<SocketAddr>, Arc<Notify>, bool)> {
        let slots = self.lock();
        let slot = slots.get(&peer)?;
        if matches!(slot.state, PeerState::Blocked { .. }) {
            return None;
        }
        Some((slot.entry.addr, slot.wake.clone(), slot.active.is_some()))
    }

    async fn dial_loop(self: Arc<Self>, peer: PeerId) {
        let mut backoff = BACKOFF_MIN;
        // Rule 5 keeps the connection dialed by the smaller PeerId, so when we are the larger
        // side our dial would only make a duplicate (and lose frames sent on it). We give the
        // other side DEFER to dial first, once per disconnected spell.
        let mut deferred = false;
        loop {
            if self.stopped() {
                return;
            }
            let Some((addr, wake, connected)) = self.dial_target(peer) else {
                return;
            };
            let Some(addr) = addr.filter(|_| !connected) else {
                tokio::select! {
                    _ = wake.notified() => {}
                    _ = self.until_stop() => return,
                }
                backoff = BACKOFF_MIN;
                deferred = false;
                continue;
            };
            if self.id > peer && !deferred {
                deferred = true;
                tokio::select! {
                    _ = tokio::time::sleep(jitter(DEFER)) => {}
                    _ = self.until_stop() => return,
                }
                continue;
            }
            self.set_state(peer, PeerState::Connecting);
            match self.clone().dial_once(peer, addr).await {
                Ok(()) => {
                    backoff = BACKOFF_MIN;
                    deferred = false;
                    continue;
                }
                Err(DialFail::KeyChanged(got)) => {
                    tracing::warn!(peer = %peer, got = %got, addr = %addr, "{}", msg::KEY_CHANGED_LOG);
                    if let Some(slot) = self.lock().get_mut(&peer) {
                        slot.state = PeerState::Blocked {
                            reason: msg::KEY_CHANGED.to_string(),
                        };
                    }
                    self.push_block(Some(peer), None, msg::KEY_CHANGED.to_string());
                    return;
                }
                Err(DialFail::Other(e)) => {
                    tracing::debug!(peer = %peer, error = %e, "dial failed");
                    self.set_state(peer, PeerState::Disconnected);
                }
            }
            tokio::select! {
                _ = tokio::time::sleep(jitter(backoff)) => {}
                _ = wake.notified() => {}
                _ = self.until_stop() => return,
            }
            backoff = (backoff * 2).min(BACKOFF_MAX);
        }
    }

    async fn dial_once(self: Arc<Self>, peer: PeerId, addr: SocketAddr) -> Result<(), DialFail> {
        let other = |e: &dyn std::fmt::Display| DialFail::Other(e.to_string());
        let verifier = PinVerifier::new(VerifyMode::Expect(peer));
        let cfg = tls::client_config(&self.identity, verifier.clone(), ALPN_MESH)
            .map_err(|e| other(&e))?;
        let connecting = self
            .endpoint
            .connect_with(cfg, addr, crate::SERVER_NAME)
            .map_err(|e| other(&e))?;
        let conn = match timeout(DIAL_TIMEOUT, connecting).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                return Err(match verifier.last_rejected() {
                    Some(got) => DialFail::KeyChanged(got),
                    None => other(&e),
                });
            }
            Err(_) => return Err(DialFail::Other("dial timeout".into())),
        };
        let hs = timeout(HELLO_TIMEOUT, async {
            let (mut send, mut recv) = conn
                .open_bi()
                .await
                .map_err(|e| HelloFail::Lost(e.to_string()))?;
            write_frame(
                &mut send,
                &Frame::Hello(self.hello.clone()),
                MAX_CONTROL_FRAME_BYTES,
            )
            .await?;
            let hello = self.read_hello(&mut send, &mut recv).await?;
            Ok::<_, HelloFail>((send, recv, hello))
        })
        .await;
        match hs {
            Ok(Ok((send, recv, hello))) => {
                self.install(peer, conn, self.id, hello, send, recv).await;
                Ok(())
            }
            Ok(Err(HelloFail::Close(code))) => {
                close(&conn, code);
                Err(DialFail::Other(format!("closed: {code:?}")))
            }
            Ok(Err(HelloFail::Lost(e))) => {
                if let Some(quinn::ConnectionError::ConnectionClosed(c)) = conn.close_reason()
                    && tls::HANDSHAKE_REJECT_REASONS
                        .iter()
                        .any(|r| r.as_bytes() == &c.reason[..])
                {
                    tracing::warn!(peer = %peer, "{}", msg::REFUSED_BY_PEER);
                }
                Err(DialFail::Other(e))
            }
            Err(_) => {
                close(&conn, CloseCode::Protocol);
                Err(DialFail::Other("hello timeout".into()))
            }
        }
    }

    /// Hello went both ways: the connection counts. Duplicates keep the one dialed by the
    /// smaller PeerId (rule 5); a newer connection from the same dialer replaces the old one.
    async fn install(
        self: &Arc<Self>,
        peer: PeerId,
        conn: Connection,
        dialed_by: PeerId,
        hello: Hello,
        send: SendStream,
        recv: RecvStream,
    ) {
        let generation = self.next_gen.fetch_add(1, Ordering::SeqCst);
        let revoke = Arc::new(Notify::new());
        let outcome = {
            let mut slots = self.lock();
            match slots.get_mut(&peer) {
                None => None,
                Some(slot) => {
                    let winner = self.id.min(peer);
                    let replaces = match &slot.active {
                        None => true,
                        Some(old) => old.dialed_by == dialed_by || dialed_by == winner,
                    };
                    Some(if replaces {
                        let old = slot.active.take().map(|a| a.conn);
                        let fresh = old.is_none();
                        slot.active = Some(Active {
                            conn: conn.clone(),
                            revoke: revoke.clone(),
                            remote: hello.modules,
                            version: hello.version,
                            dialed_by,
                            generation,
                        });
                        slot.state = PeerState::Connected;
                        Installed::Won {
                            old,
                            fresh,
                            info: slot.info(),
                        }
                    } else {
                        Installed::Lost
                    })
                }
            }
        };
        let (old, fresh, info) = match outcome {
            None => return close(&conn, CloseCode::UnknownPeer),
            Some(Installed::Lost) => return close(&conn, CloseCode::Duplicate),
            Some(Installed::Won { old, fresh, info }) => (old, fresh, info),
        };
        if let Some(old) = old {
            close(&old, CloseCode::Duplicate);
        }
        tracing::info!(peer = %peer, name = %info.name, "connected");
        tokio::spawn(
            self.clone()
                .control_task(peer, conn.clone(), send, recv, revoke),
        );
        tokio::spawn(self.clone().uni_task(peer, conn.clone()));
        tokio::spawn(self.clone().datagram_task(peer, conn.clone()));
        let watcher = self.clone();
        tokio::spawn(async move {
            let reason = conn.closed().await;
            watcher.uninstall(peer, generation, reason).await;
        });
        if fresh {
            self.broadcast(|| ModuleInput::PeerUp(info.clone())).await;
        }
    }

    async fn uninstall(&self, peer: PeerId, generation: u64, reason: quinn::ConnectionError) {
        let lost = {
            let mut slots = self.lock();
            match slots.get_mut(&peer) {
                Some(slot)
                    if slot
                        .active
                        .as_ref()
                        .is_some_and(|a| a.generation == generation) =>
                {
                    slot.active = None;
                    if !matches!(slot.state, PeerState::Blocked { .. }) {
                        slot.state = PeerState::Disconnected;
                    }
                    slot.wake.notify_one();
                    true
                }
                _ => false,
            }
        };
        if lost {
            tracing::info!(peer = %peer, reason = %reason, "disconnected");
            self.broadcast(|| ModuleInput::PeerDown(peer)).await;
        }
    }

    /// Owns the dialer-opened control stream: Ping/Pong, Bye on shutdown.
    async fn control_task(
        self: Arc<Self>,
        peer: PeerId,
        conn: Connection,
        mut send: SendStream,
        mut recv: RecvStream,
        revoke: Arc<Notify>,
    ) {
        loop {
            let frame = tokio::select! {
                f = read_control(&mut send, &mut recv) => f,
                _ = self.until_stop() => {
                    let bye = Frame::Bye { code: CloseCode::Shutdown };
                    let _ = write_frame(&mut send, &bye, MAX_CONTROL_FRAME_BYTES).await;
                    let _ = send.finish();
                    return;
                }
                () = revoke.notified() => {
                    let bye = Frame::Bye { code: CloseCode::Revoked };
                    let _ = write_frame(&mut send, &bye, MAX_CONTROL_FRAME_BYTES).await;
                    let _ = send.finish();
                    return;
                }
            };
            match frame {
                Ok(Frame::Ping { nonce }) => {
                    let pong = Frame::Pong { nonce };
                    if write_frame(&mut send, &pong, MAX_CONTROL_FRAME_BYTES)
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(Frame::Pong { .. }) => {}
                // Only a connection that passed the pin and Hello gets here (ALPN_MESH).
                Ok(Frame::Bye {
                    code: CloseCode::Revoked,
                }) => return self.revoked_by(peer),
                Ok(Frame::Bye { code }) => return tracing::debug!(?code, "bye"),
                Ok(_)
                | Err(WireError::TooLarge { .. } | WireError::Decode | WireError::Invalid(_)) => {
                    return close(&conn, CloseCode::Protocol);
                }
                Err(WireError::Io(_)) => return,
            }
        }
    }

    async fn uni_task(self: Arc<Self>, from: PeerId, conn: Connection) {
        while let Ok(mut recv) = conn.accept_uni().await {
            let this = self.clone();
            let conn = conn.clone();
            tokio::spawn(async move {
                match read_frame(&mut recv, MAX_FRAME_BYTES).await {
                    Ok(frame) => this.handle_frame(from, frame, &conn).await,
                    Err(WireError::TooLarge { len, .. }) => {
                        let _ = recv.stop(varint(CloseCode::Protocol));
                        tracing::warn!(peer = %from, len, "module frame above the limit refused");
                    }
                    Err(e) => tracing::debug!(peer = %from, error = %e, "bad module frame"),
                }
            });
        }
    }

    async fn handle_frame(&self, from: PeerId, frame: Frame, conn: &Connection) {
        match frame {
            Frame::Module(m) => {
                let Some(perms) = self.perms_of(&from) else {
                    return;
                };
                match authorize_inbound(self.local, &perms, self.paused(), &m) {
                    Ok(()) => {
                        self.deliver(m.kind(), ModuleInput::Reliable { from, msg: m })
                            .await
                    }
                    Err(reason) => {
                        self.push_block(Some(from), Some(m.kind()), reason.pt().to_string());
                        let reply = Frame::Rejected {
                            kind: m.kind(),
                            ref_id: offer_id(&m),
                            reason,
                        };
                        if let Err(e) = send_uni(conn, &reply).await {
                            tracing::debug!(peer = %from, error = %e, "could not send Rejected");
                        }
                    }
                }
            }
            Frame::Rejected {
                kind,
                ref_id,
                reason,
            } => {
                self.deliver(
                    kind,
                    ModuleInput::Rejected {
                        from,
                        ref_id,
                        reason,
                    },
                )
                .await;
            }
            // Pairing frames on a mesh connection are a protocol error (§4).
            Frame::Pair(_) => close(conn, CloseCode::Protocol),
            other => {
                tracing::debug!(peer = %from, frame = ?other, "unexpected frame on a uni stream")
            }
        }
    }

    async fn datagram_task(self: Arc<Self>, from: PeerId, conn: Connection) {
        let mut dropped: u64 = 0;
        while let Ok(bytes) = conn.read_datagram().await {
            let Ok(Datagram::Input(dg)) = decode_datagram(&bytes) else {
                dropped += 1;
                continue;
            };
            let allowed = self
                .perms_of(&from)
                .is_some_and(|p| authorize_inbound_datagram(self.local, &p, self.paused()).is_ok());
            let delivered = allowed
                && self
                    .modules
                    .get(&ModuleKind::Input)
                    .is_some_and(|tx| tx.try_send(ModuleInput::Datagram { from, dg }).is_ok());
            if !delivered {
                dropped += 1;
            }
        }
        if dropped > 0 {
            tracing::debug!(peer = %from, dropped, "datagrams dropped");
        }
    }
}

async fn send_uni(conn: &Connection, frame: &Frame) -> Result<(), SendError> {
    let buf = encode_frame(frame, MAX_FRAME_BYTES).map_err(|e| match e {
        WireError::TooLarge { .. } => SendError::TooLarge,
        other => SendError::Transport(other.to_string()),
    })?;
    let transport = |e: &dyn std::fmt::Display| SendError::Transport(e.to_string());
    let mut s = conn.open_uni().await.map_err(|e| transport(&e))?;
    s.write_all(&buf).await.map_err(|e| transport(&e))?;
    s.finish().map_err(|e| transport(&e))?;
    Ok(())
}

/// The core's side of the module contract.
struct MeshHost(Arc<Inner>);

struct Route {
    conn: Connection,
    perms: PeerPerms,
    remote: ModuleSet,
    version: u16,
}

impl MeshHost {
    fn route(&self, to: &PeerId) -> Result<Route, SendError> {
        let slots = self.0.lock();
        let slot = slots.get(to).ok_or(SendError::NotConnected)?;
        let active = slot.active.as_ref().ok_or(SendError::NotConnected)?;
        Ok(Route {
            conn: active.conn.clone(),
            perms: slot.entry.perms,
            remote: active.remote,
            version: active.version,
        })
    }
}

#[async_trait::async_trait]
impl ModuleHost for MeshHost {
    fn local_id(&self) -> PeerId {
        self.0.id
    }

    fn local_name(&self) -> String {
        self.0.hello.device_name.clone()
    }

    fn peers(&self) -> Vec<PeerInfo> {
        let mut out: Vec<PeerInfo> = self.0.lock().values().map(Slot::info).collect();
        out.sort_by_key(|p| p.id);
        out
    }

    fn paused(&self) -> bool {
        self.0.paused()
    }

    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError> {
        let r = self.route(&to)?;
        authorize_outbound(self.0.local, &r.perms, r.remote, self.0.paused(), &msg)?;
        // A 0.1.0 peer closes the connection on a frame it cannot decode: send nothing newer.
        if r.version < min_version(&msg) {
            return Err(SendError::RemoteTooOld);
        }
        send_uni(&r.conn, &Frame::Module(msg)).await
    }

    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError> {
        let Route { conn, remote, .. } = self.route(&to)?;
        authorize_outbound_datagram(self.0.local, remote, self.0.paused())?;
        let buf = encode_datagram(&Datagram::Input(dg)).map_err(|e| match e {
            WireError::TooLarge { .. } => SendError::TooLarge,
            other => SendError::Transport(other.to_string()),
        })?;
        conn.send_datagram(buf.into()).map_err(|e| match e {
            quinn::SendDatagramError::TooLarge => SendError::TooLarge,
            other => SendError::Transport(other.to_string()),
        })
    }
}
