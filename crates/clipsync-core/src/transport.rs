//! QUIC mesh (contract §5 rules 4 to 10). Owned by Rivet.
//!
//! One endpoint per agent, server and client at once. Every trusted peer gets a dialer with
//! backoff. A connection only counts after the application Hello went both ways; until then
//! nothing reaches the modules. Module frames ride one uni stream each, input rides datagrams,
//! and every inbound message passes `authorize_inbound*` here, before any module sees it.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clipsync_proto::{
    ALPN_MESH, BlockRecord, ClipMsg, CloseCode, Datagram, EventId, Frame, Hello, InputDatagram,
    MAX_CONTROL_FRAME_BYTES, MAX_FRAME_BYTES, MAX_RECENT_BLOCKS, MAX_RECENT_REMOVALS,
    MIN_PROTOCOL_VERSION, ModuleHost, ModuleInput, ModuleKind, ModuleMsg, ModuleSet,
    PROTOCOL_VERSION, PeerId, PeerInfo, PeerInputContext, PeerPerms, PeerState, PeerStatus,
    ReceiveContext, RejectReason, RemovalRecord, SendError, WireError, authorize_inbound,
    authorize_inbound_datagram, authorize_outbound, authorize_outbound_datagram, decode_datagram,
    encode_datagram, encode_frame, min_version, read_frame, write_frame,
};
use quinn::{Connection, Endpoint, RecvStream, SendStream, VarInt};
use tokio::sync::{Notify, mpsc, watch};
use tokio::time::timeout;

use crate::tls::{self, PinSet, PinVerifier, VerifyMode};
use crate::{CoreError, Identity, LearnedAddr, PeerEntry, Peers, msg};

const RELIABLE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_READERS: usize = 32;
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const DIAL_TIMEOUT: Duration = Duration::from_secs(10);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);
/// The larger PeerId waits this long before dialing, so the smaller one dials first.
const DEFER: Duration = Duration::from_secs(2);
/// Time the control tasks get to put Bye on the wire before the connections close.
const BYE_GRACE: Duration = Duration::from_millis(100);
/// One name lookup may not hold the dialer longer than this (mDNS answers in well under it).
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(3);
/// A dial to an address found by name, now or on an earlier run. Such an address is on the
/// local network or it is wrong: it does not get the wait a pairing address gets.
const NAME_DIAL_TIMEOUT: Duration = Duration::from_secs(3);
/// Addresses found by name tried per round. The next round goes on from where this one stopped,
/// so an answer beyond the first few is reached too.
const MAX_RESOLVED: usize = 3;

/// Answers the addresses a peer's device name has now, with the port the pairing address had.
/// Asked only after the pairing address failed a dial (the peer got another address from DHCP).
pub type Resolver =
    Arc<dyn Fn(String, u16) -> Pin<Box<dyn Future<Output = Vec<SocketAddr>> + Send>> + Send + Sync>;

/// What the mesh tells the agent about the cache of addresses found by name. The cache lives
/// beside peers.toml and never in it: an address a name answered is not a trust anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressUpdate {
    /// The peer proved its key at `learned` while its pairing address `paired` answered nothing.
    Learned(LearnedAddr),
    /// What was cached for the peer is no longer true (another device answers there, or the
    /// pairing address changed).
    Forgotten(PeerId),
}

/// A device name the system resolver may be asked about: one host-name label, so a name typed
/// with spaces or accents never leaves this process as a query, and a dotted name never turns
/// into a query for somebody else's domain.
fn host_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// The resolver of the machine: the bare name (DNS, LLMNR or NetBIOS on Windows) and the mDNS
/// `.local` name (Avahi on Linux, native on Windows), each within RESOLVE_TIMEOUT.
pub fn system_resolver() -> Resolver {
    Arc::new(|name, port| {
        Box::pin(async move {
            if !host_name(&name) {
                return Vec::new();
            }
            let lookup = |host: String| async move {
                match timeout(RESOLVE_TIMEOUT, tokio::net::lookup_host((host, port))).await {
                    Ok(Ok(found)) => found.collect::<Vec<_>>(),
                    _ => Vec::new(),
                }
            };
            let (bare, mdns) = tokio::join!(lookup(name.clone()), lookup(format!("{name}.local")));
            let mut out: Vec<SocketAddr> = Vec::new();
            for addr in mdns.into_iter().chain(bare) {
                if !out.contains(&addr) {
                    out.push(addr);
                }
            }
            out
        })
    })
}
/// `Bye{Revoked}` only goes to a peer that decodes it (v1.16): a 0.1.0 would close with Protocol.
const REVOKED_MIN_VERSION: u16 = 2;

pub fn varint(code: CloseCode) -> VarInt {
    VarInt::from_u32(code.code())
}

/// Closes with the code and its fixed token only (rule 9).
pub fn close(conn: &Connection, code: CloseCode) {
    conn.close(varint(code), code.reason());
}

/// A CONNECTION_CLOSE whose reason is one of the fixed rustls texts of a refused certificate.
fn refused_our_certificate(e: &quinn::ConnectionError) -> bool {
    matches!(e, quinn::ConnectionError::ConnectionClosed(c)
        if tls::HANDSHAKE_REJECT_REASONS.iter().any(|r| r.as_bytes() == &c.reason[..]))
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
    /// Taken once by the agent (`pause_requests`).
    pausing: Mutex<Option<mpsc::UnboundedReceiver<bool>>>,
    /// Taken once by the agent (`address_updates`).
    readdressed: Mutex<Option<mpsc::UnboundedReceiver<AddressUpdate>>>,
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
    lifecycle: watch::Sender<clipsync_proto::HostLifecycle>,
    peers: Mutex<HashMap<PeerId, Slot>>,
    /// Newest first, at most MAX_RECENT_BLOCKS. Locked on its own, never inside `peers`.
    blocks: Mutex<VecDeque<BlockRecord>>,
    /// Peers that removed this device, newest first, at most MAX_RECENT_REMOVALS. Locked on its
    /// own, never inside `peers`.
    removals: Mutex<VecDeque<RemovalRecord>>,
    /// To the agent: this peer sent `Bye{Revoked}`, take it out of peers.toml.
    revoked: mpsc::UnboundedSender<PeerId>,
    /// To the agent: a module asked to pause (true) or resume (`ModuleHost::request_pause`).
    pausing: mpsc::UnboundedSender<bool>,
    /// To the agent: what to write to, or drop from, the cache of addresses found by name.
    readdressed: mpsc::UnboundedSender<AddressUpdate>,
    /// Off until `resolve_names_with`: a mesh without it only dials the addresses it was given.
    resolver: Mutex<Option<Resolver>>,
    next_gen: AtomicU64,
    stop: watch::Sender<bool>,
}

struct Slot {
    input_epoch: u64,
    entry: PeerEntry,
    /// Where the peer last proved its key while `entry.addr` answered nothing. A cache: dialed
    /// first, dropped when another device answers there or when `entry.addr` changes, and never
    /// a reason to block the peer.
    learned: Option<SocketAddr>,
    state: PeerState,
    active: Option<Active>,
    /// Wakes the dialer: connection lost, peers reloaded.
    wake: Arc<Notify>,
    /// The peer refused our certificate and that is already in recent_blocks: one entry while
    /// it repeats, cleared when a connection comes up again.
    refused: bool,
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
            input_epoch: 0,
            entry,
            learned: None,
            state: PeerState::Disconnected,
            active: None,
            wake: Arc::new(Notify::new()),
            refused: false,
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

/// What the dialer of one peer works from, read under the lock at each step.
struct DialTarget {
    /// The address in peers.toml, from the pairing or typed by the user.
    paired: Option<SocketAddr>,
    learned: Option<SocketAddr>,
    wake: Arc<Notify>,
    connected: bool,
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
        Self::start_with_cache(opts, Vec::new())
    }

    /// `start`, taking back the cache of addresses found by name that a previous run wrote. An
    /// entry only counts for a peer that is still trusted and whose pairing address is still
    /// the one the entry was found for. The dialers start with it: the cached address is the
    /// first one dialed.
    pub fn start_with_cache(
        opts: MeshOptions,
        cached: Vec<LearnedAddr>,
    ) -> Result<Self, CoreError> {
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
        let mut slots: HashMap<PeerId, Slot> = opts
            .peers
            .peer
            .into_iter()
            .map(|e| (e.id, Slot::new(e)))
            .collect();
        for entry in cached {
            if let Some(slot) = slots.get_mut(&entry.peer)
                && slot.entry.addr == Some(entry.paired)
            {
                slot.learned = Some(entry.learned);
            }
        }
        let ids: Vec<PeerId> = slots.keys().copied().collect();
        let (revoked, revoked_rx) = mpsc::unbounded_channel();
        let (pausing, pausing_rx) = mpsc::unbounded_channel();
        let (readdressed, readdressed_rx) = mpsc::unbounded_channel();
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
            lifecycle: watch::Sender::new(clipsync_proto::HostLifecycle {
                paused: opts.paused,
                pause_generation: 0,
                stopped: false,
            }),
            peers: Mutex::new(slots),
            blocks: Mutex::new(VecDeque::new()),
            removals: Mutex::new(VecDeque::new()),
            revoked,
            pausing,
            readdressed,
            resolver: Mutex::new(None),
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
            pausing: Mutex::new(Some(pausing_rx)),
            readdressed: Mutex::new(Some(readdressed_rx)),
        })
    }

    /// From now on a peer whose pairing address fails a dial is looked up by its device name and
    /// dialed at what `resolver` answers. The agent turns this on with `system_resolver`.
    pub fn resolve_names_with(&self, resolver: Resolver) {
        *self
            .inner
            .resolver
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(resolver);
    }

    /// Changes to the cache of addresses found by name, already applied in this mesh. The agent
    /// takes this once and mirrors each to the cache file, so a restart dials the address first.
    pub fn address_updates(&self) -> Option<mpsc::UnboundedReceiver<AddressUpdate>> {
        self.readdressed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// What modules asked through `ModuleHost::request_pause`, in order. The agent takes this
    /// once and applies each like the control's `pause` and `resume` (memory, then state.toml).
    pub fn pause_requests(&self) -> Option<mpsc::UnboundedReceiver<bool>> {
        self.pausing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
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
        {
            let mut slots = self.inner.lock();
            if self.inner.paused.swap(paused, Ordering::SeqCst) != paused {
                for slot in slots.values_mut() {
                    slot.input_epoch = self.inner.next_gen.fetch_add(1, Ordering::SeqCst);
                }
            }
            // Same lock as authority: concurrent Pause/Resume cannot publish out of order.
            self.inner.lifecycle.send_modify(|s| {
                if paused {
                    s.pause_generation = s.pause_generation.saturating_add(1);
                }
                s.stopped |= s.pause_generation == u64::MAX;
                s.paused = paused;
            });
        }
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
                addr: s.learned.or(s.entry.addr),
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
                // Whatever is cached for a peer that left goes with it (S1b-PRIV-01). Told under
                // the lock, in the order the cache changes (R3-N02).
                let _ = inner.readdressed.send(AddressUpdate::Forgotten(*id));
                slot.wake.notify_one();
                false
            });
            for entry in peers.peer {
                match slots.get_mut(&entry.id) {
                    Some(slot) => {
                        let changed =
                            slot.entry.perms != entry.perms || slot.entry.name != entry.name;
                        let readdressed = slot.entry.addr != entry.addr;
                        if slot.entry.perms.input_in != entry.perms.input_in {
                            slot.input_epoch = inner.next_gen.fetch_add(1, Ordering::SeqCst);
                        }
                        slot.entry = entry;
                        if changed && slot.active.is_some() {
                            up.push(slot.info());
                        }
                        if readdressed {
                            // What was learned stood in for the old pairing address only.
                            if slot.learned.take().is_some() {
                                let _ = inner
                                    .readdressed
                                    .send(AddressUpdate::Forgotten(slot.entry.id));
                            }
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
        inner.lifecycle.send_modify(|_| {});
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
        inner.lifecycle.send_modify(|s| s.stopped = true);
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

    fn input_context(&self, peer: PeerId) -> Option<PeerInputContext> {
        let slots = self.lock();
        let slot = slots.get(&peer)?;
        let active = slot.active.as_ref()?;
        Some(PeerInputContext {
            connection: active.generation,
            input_epoch: slot.input_epoch,
            input_in: slot.entry.perms.input_in,
            paused: self.paused(),
            connected: active.conn.close_reason().is_none(),
        })
    }
    fn receive_context(
        &self,
        peer: PeerId,
        generation: u64,
        stream: Option<u64>,
    ) -> Option<ReceiveContext> {
        let c = self.input_context(peer)?;
        (c.connection == generation && c.connected).then_some(ReceiveContext {
            connection: generation,
            input_epoch: c.input_epoch,
            stream,
        })
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
        push_block_record(
            &mut blocks,
            BlockRecord {
                occurrences: 1,
                last_unix_ms: at_unix_ms,
                at_unix_ms,
                peer,
                peer_name,
                module,
                reason,
            },
        );
    }

    /// The peer refused our certificate: it no longer has us paired (revoked while we were away,
    /// or re-created). One recent_blocks entry while it repeats; nothing on the refusing side,
    /// where an unknown key could be anyone on the network.
    fn refused_by(&self, peer: PeerId) {
        let first = match self.lock().get_mut(&peer) {
            Some(slot) if !slot.refused => {
                slot.refused = true;
                true
            }
            _ => false,
        };
        if first {
            tracing::warn!(peer = %peer, "{}", msg::REFUSED_BY_PEER);
            self.push_block(Some(peer), None, msg::PEER_REFUSED_US.to_string());
        }
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
        // A stalled clipboard queue must not prevent Input's PeerDown/Paused notification.
        let mut waiting = tokio::task::JoinSet::new();
        for tx in self.modules.values() {
            match tx.try_send(make()) {
                Ok(()) | Err(mpsc::error::TrySendError::Closed(_)) => {}
                Err(mpsc::error::TrySendError::Full(message)) => {
                    let tx = tx.clone();
                    waiting.spawn(async move {
                        let _ = timeout(RELIABLE_TIMEOUT, tx.send(message)).await;
                    });
                }
            }
        }
        while waiting.join_next().await.is_some() {}
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
    fn dial_target(&self, peer: PeerId) -> Option<DialTarget> {
        let slots = self.lock();
        let slot = slots.get(&peer)?;
        if matches!(slot.state, PeerState::Blocked { .. }) {
            return None;
        }
        Some(DialTarget {
            paired: slot.entry.addr,
            learned: slot.learned,
            wake: slot.wake.clone(),
            connected: slot.active.is_some(),
        })
    }

    async fn dial_loop(self: Arc<Self>, peer: PeerId) {
        let mut backoff = BACKOFF_MIN;
        // Rule 5 keeps the connection dialed by the smaller PeerId, so when we are the larger
        // side our dial would only make a duplicate (and lose frames sent on it). We give the
        // other side DEFER to dial first, once per disconnected spell.
        let mut deferred = false;
        // Where the next round of addresses found by name starts (see MAX_RESOLVED).
        let mut name_cursor = 0usize;
        loop {
            if self.stopped() {
                return;
            }
            let Some(DialTarget {
                paired,
                learned,
                wake,
                connected,
            }) = self.dial_target(peer)
            else {
                return;
            };
            let Some(addr) = paired.filter(|_| !connected) else {
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
            // The cached address first: it is where the peer was last time. It is only a cache,
            // so another device answering there gets it dropped and nothing else (S1-SEG-01).
            if let Some(cached) = learned.filter(|cached| *cached != addr) {
                match self
                    .clone()
                    .dial_once(peer, cached, NAME_DIAL_TIMEOUT)
                    .await
                {
                    Ok(()) => {
                        backoff = BACKOFF_MIN;
                        deferred = false;
                        continue;
                    }
                    Err(DialFail::KeyChanged(_)) => {
                        tracing::debug!(peer = %peer, "another device answers at the cached address");
                        self.forget(peer, Some(cached));
                    }
                    Err(DialFail::Other(e)) => {
                        tracing::debug!(peer = %peer, error = %e, "dial to the cached address failed");
                    }
                }
                // That dial took time: a reload may have given the peer another pairing
                // address, or the peer may have connected from its side. What follows is only
                // about the pairing address this round started with (R3-N01).
                match self.dial_target(peer) {
                    Some(target) if target.paired == Some(addr) && !target.connected => {}
                    _ => continue,
                }
            }
            match self.clone().dial_once(peer, addr, DIAL_TIMEOUT).await {
                Ok(()) => {
                    // The pairing address works: whatever was cached in its place is stale
                    // (R3-N03).
                    self.forget(peer, None);
                    backoff = BACKOFF_MIN;
                    deferred = false;
                    continue;
                }
                Err(DialFail::KeyChanged(got)) => {
                    // Another key at the pairing address blocks the peer, as before. Only if
                    // this still is its pairing address: an answer about an address a reload
                    // has replaced meanwhile says nothing about the new one (R3-N01).
                    let blocked = match self.lock().get_mut(&peer) {
                        Some(slot) if slot.entry.addr == Some(addr) => {
                            slot.state = PeerState::Blocked {
                                reason: msg::KEY_CHANGED.to_string(),
                            };
                            true
                        }
                        _ => false,
                    };
                    if !blocked {
                        continue;
                    }
                    tracing::warn!(peer = %peer, got = %got, addr = %addr, "{}", msg::KEY_CHANGED_LOG);
                    self.push_block(Some(peer), None, msg::KEY_CHANGED.to_string());
                    return;
                }
                Err(DialFail::Other(e)) => {
                    tracing::debug!(peer = %peer, error = %e, "dial failed");
                    if self.dial_by_name(peer, addr, &mut name_cursor).await {
                        backoff = BACKOFF_MIN;
                        deferred = false;
                        continue;
                    }
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

    /// The pairing address answered nothing: the peer may have another one now (DHCP). Asks the
    /// resolver for the peer's device name and dials what it answers, a few addresses per round
    /// starting at `cursor`. The pin decides who the peer is, as in any dial; another device at
    /// a resolved address is skipped and never blocks the peer, since a name answer proves
    /// nothing about who holds the address.
    async fn dial_by_name(
        self: &Arc<Self>,
        peer: PeerId,
        paired: SocketAddr,
        cursor: &mut usize,
    ) -> bool {
        let resolver = self
            .resolver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let Some(resolver) = resolver else {
            return false;
        };
        let Some(name) = self.lock().get(&peer).map(|s| s.entry.name.clone()) else {
            return false;
        };
        let found = tokio::select! {
            f = resolver(name, paired.port()) => f,
            _ = self.until_stop() => return false,
        };
        let mut candidates: Vec<SocketAddr> = Vec::new();
        for addr in found {
            if addr != paired && addr.is_ipv4() == paired.is_ipv4() && !candidates.contains(&addr) {
                candidates.push(addr);
            }
        }
        for _ in 0..candidates.len().min(MAX_RESOLVED) {
            let addr = candidates[*cursor % candidates.len()];
            *cursor = cursor.wrapping_add(1);
            // Gone, blocked, given another pairing address by a reload, or connected from the
            // other side meanwhile.
            match self.dial_target(peer) {
                Some(target) if target.paired == Some(paired) && !target.connected => {
                    // Just tried, as the cached address, in this same round.
                    if target.learned == Some(addr) {
                        continue;
                    }
                }
                _ => return false,
            }
            if self.stopped() {
                return false;
            }
            match self.clone().dial_once(peer, addr, NAME_DIAL_TIMEOUT).await {
                Ok(()) => {
                    self.learn(peer, paired, addr);
                    return true;
                }
                Err(DialFail::KeyChanged(_)) => {
                    tracing::debug!(peer = %peer, "another device answers at a resolved address");
                }
                Err(DialFail::Other(e)) => {
                    tracing::debug!(peer = %peer, error = %e, "dial by name failed");
                }
            }
        }
        false
    }

    /// The peer proved its key at `learned` while `paired` answered nothing. Cached only if
    /// `paired` is still the peer's pairing address: a result that started before a reload gave
    /// the peer another one is dropped (R1-D01), though the connection it made stands. The
    /// agent is told under the lock, so it sees the cache change in the order it happened.
    fn learn(&self, peer: PeerId, paired: SocketAddr, learned: SocketAddr) {
        let mut slots = self.lock();
        let Some(slot) = slots.get_mut(&peer) else {
            return;
        };
        if slot.entry.addr != Some(paired) || slot.learned == Some(learned) {
            return;
        }
        slot.learned = Some(learned);
        let _ = self.readdressed.send(AddressUpdate::Learned(LearnedAddr {
            peer,
            paired,
            learned,
        }));
        drop(slots);
        tracing::info!(evento = "peer.readdressed", peer = %peer, origem = "name", "peer found at a new address");
    }

    /// Drops the cache entry of the peer: `only` names the address that must still be the
    /// cached one (another device answers there), `None` drops whatever is cached (the pairing
    /// address works again).
    fn forget(&self, peer: PeerId, only: Option<SocketAddr>) {
        let mut slots = self.lock();
        let Some(slot) = slots.get_mut(&peer) else {
            return;
        };
        if slot.learned.is_none() || only.is_some_and(|addr| slot.learned != Some(addr)) {
            return;
        }
        slot.learned = None;
        let _ = self.readdressed.send(AddressUpdate::Forgotten(peer));
    }

    async fn dial_once(
        self: Arc<Self>,
        peer: PeerId,
        addr: SocketAddr,
        wait: Duration,
    ) -> Result<(), DialFail> {
        let other = |e: &dyn std::fmt::Display| DialFail::Other(e.to_string());
        let verifier = PinVerifier::new(VerifyMode::Expect(peer));
        let cfg = tls::client_config(&self.identity, verifier.clone(), ALPN_MESH)
            .map_err(|e| other(&e))?;
        let connecting = self
            .endpoint
            .connect_with(cfg, addr, crate::SERVER_NAME)
            .map_err(|e| other(&e))?;
        let conn = match timeout(wait, connecting).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                return Err(match verifier.last_rejected() {
                    Some(got) => DialFail::KeyChanged(got),
                    None => {
                        // The close text is not authenticated before the handshake ends: only a
                        // server that proved it holds the peer's key can be the peer refusing
                        // us (S6-21).
                        if refused_our_certificate(&e) && verifier.proven() == Some(peer) {
                            self.refused_by(peer);
                        }
                        other(&e)
                    }
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
                // TLS 1.3: our side may finish the handshake before the peer checks our
                // certificate, so its refusal can also arrive here. The handshake ended, so
                // the peer's identity is proven.
                if conn
                    .close_reason()
                    .is_some_and(|r| refused_our_certificate(&r))
                {
                    self.refused_by(peer);
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
                        slot.input_epoch = generation;
                        slot.state = PeerState::Connected;
                        slot.refused = false;
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
        self.lifecycle.send_modify(|_| {});
        if let Some(old) = old {
            close(&old, CloseCode::Duplicate);
        }
        tracing::info!(evento = "peer.connected", peer = %peer, name = %info.name, "connected");
        tokio::spawn(
            self.clone()
                .control_task(peer, conn.clone(), send, recv, revoke),
        );
        tokio::spawn(self.clone().uni_task(peer, generation, conn.clone()));
        tokio::spawn(self.clone().datagram_task(peer, generation, conn.clone()));
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
            self.lifecycle.send_modify(|_| {});
            tracing::info!(evento = "peer.disconnected", peer = %peer, reason = %reason, "disconnected");
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

    async fn uni_task(self: Arc<Self>, from: PeerId, generation: u64, conn: Connection) {
        let mut readers = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                biased;
                _=conn.closed()=>break,
                done=readers.join_next(), if !readers.is_empty()=> {
                    if matches!(done,Some(Ok(true))) {break;}
                }
                accepted=conn.accept_uni(), if readers.len()<MAX_READERS=> {
                    let Ok(mut recv)=accepted else {break};
                    let deadline=tokio::time::Instant::now()+RELIABLE_TIMEOUT;
                    let Some(context)=self.receive_context(from,generation,Some(recv.id().index())) else {break};
                    let this=self.clone(); let connection=conn.clone();
                    readers.spawn(async move {
                        let work=async {
                            match read_frame(&mut recv,MAX_FRAME_BYTES).await {
                                Ok(frame)=>this.handle_frame(from,frame,&connection,context).await,
                                Err(WireError::TooLarge{..})=>{let _=recv.stop(varint(CloseCode::Protocol));}
                                Err(_)=>{}
                            }
                        };
                        if tokio::time::timeout_at(deadline,work).await.is_err() {
                            let _=recv.stop(varint(CloseCode::Protocol));
                            close(&connection,CloseCode::Protocol);
                            true
                        } else {false}
                    });
                }
            }
        }
        readers.abort_all();
        while readers.join_next().await.is_some() {}
        // The watcher may win this race; uninstall checks the generation and publishes once.
        if let Some(reason) = conn.close_reason() {
            self.uninstall(from, generation, reason).await;
        }
    }

    async fn handle_frame(
        &self,
        from: PeerId,
        frame: Frame,
        conn: &Connection,
        context: ReceiveContext,
    ) {
        match frame {
            Frame::Module(m) => {
                let Some(perms) = self.perms_of(&from) else {
                    return;
                };
                match authorize_inbound(self.local, &perms, self.paused(), &m) {
                    Ok(()) => {
                        self.deliver(
                            m.kind(),
                            ModuleInput::Reliable {
                                from,
                                msg: m,
                                context,
                            },
                        )
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

    async fn datagram_task(self: Arc<Self>, from: PeerId, generation: u64, conn: Connection) {
        let mut dropped: u64 = 0;
        while let Ok(bytes) = conn.read_datagram().await {
            let Ok(Datagram::Input(dg)) = decode_datagram(&bytes) else {
                dropped += 1;
                continue;
            };
            let Some(context) = self.receive_context(from, generation, None) else {
                break;
            };
            let allowed = self.input_context(from).is_some_and(|c| c.permits(context))
                && self.perms_of(&from).is_some_and(|p| {
                    authorize_inbound_datagram(self.local, &p, self.paused()).is_ok()
                });
            let delivered = allowed
                && self.modules.get(&ModuleKind::Input).is_some_and(|tx| {
                    tx.try_send(ModuleInput::Datagram { from, dg, context })
                        .is_ok()
                });
            if !delivered {
                dropped += 1;
            }
        }
        if dropped > 0 {
            tracing::debug!(peer = %from, dropped, "datagrams dropped");
        }
    }
}

/// Quinn's ordinary Drop finishes a prefix. An interrupted frame must be reset instead.
struct PendingSend {
    stream: SendStream,
    finished: bool,
}
impl Drop for PendingSend {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.stream.reset(varint(CloseCode::Protocol));
        }
    }
}
async fn send_uni(conn: &Connection, frame: &Frame) -> Result<(), SendError> {
    let deadline = tokio::time::Instant::now() + RELIABLE_TIMEOUT;
    let buf = encode_frame(frame, MAX_FRAME_BYTES).map_err(|e| match e {
        WireError::TooLarge { .. } => SendError::TooLarge,
        other => SendError::Transport(other.to_string()),
    })?;
    tokio::time::timeout_at(deadline, async {
        let transport = |e: &dyn std::fmt::Display| SendError::Transport(e.to_string());
        let stream = conn.open_uni().await.map_err(|e| transport(&e))?;
        let mut pending = PendingSend {
            stream,
            finished: false,
        };
        pending
            .stream
            .write_all(&buf)
            .await
            .map_err(|e| transport(&e))?;
        pending.stream.finish().map_err(|e| transport(&e))?;
        pending.finished = true;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| {
        Err(SendError::Transport(
            "reliable send deadline exceeded".into(),
        ))
    })
}

/// The core's side of the module contract.
struct MeshHost(Arc<Inner>);

struct Route {
    generation: u64,
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
            generation: active.generation,
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

    fn lifecycle_receiver(&self) -> Option<watch::Receiver<clipsync_proto::HostLifecycle>> {
        Some(self.0.lifecycle.subscribe())
    }

    fn peer_input_context(&self, peer: PeerId) -> Option<PeerInputContext> {
        self.0.input_context(peer)
    }
    async fn send_reliable_on(
        &self,
        to: PeerId,
        connection: u64,
        msg: ModuleMsg,
    ) -> Result<(), SendError> {
        let r = self.route(&to)?;
        if r.generation != connection {
            return Err(SendError::NotConnected);
        }
        authorize_outbound(self.0.local, &r.perms, r.remote, self.0.paused(), &msg)?;
        if r.version < min_version(&msg) {
            return Err(SendError::RemoteTooOld);
        }
        send_uni(&r.conn, &Frame::Module(msg)).await
    }
    async fn reject_input_on(
        &self,
        to: PeerId,
        connection: u64,
        reason: RejectReason,
    ) -> Result<(), SendError> {
        let r = self.route(&to)?;
        if r.generation != connection {
            return Err(SendError::NotConnected);
        }
        send_uni(
            &r.conn,
            &Frame::Rejected {
                kind: ModuleKind::Input,
                ref_id: None,
                reason,
            },
        )
        .await
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

    fn request_pause(&self, paused: bool) {
        let _ = self.0.pausing.send(paused);
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod send_backpressure {
    include!("../tests/support/send_backpressure.rs");
}

// Ordem por chegada; separado do relógio do SO para testar regressão de tempo de parede.
fn push_block_record(blocks: &mut VecDeque<BlockRecord>, mut record: BlockRecord) {
    if let Some(index) = blocks.iter().position(|old| {
        old.peer == record.peer && old.module == record.module && old.reason == record.reason
    }) && let Some(old) = blocks.remove(index)
    {
        record.at_unix_ms = old.at_unix_ms;
        record.occurrences = old.occurrences.saturating_add(record.occurrences);
    }
    if record.last_unix_ms == 0 {
        record.last_unix_ms = record.at_unix_ms;
    }
    blocks.push_front(record);
    blocks.truncate(MAX_RECENT_BLOCKS);
}

#[cfg(test)]
mod block_groups_tests {
    use super::*;
    fn record(reason: &str, at: u64) -> BlockRecord {
        BlockRecord {
            at_unix_ms: at,
            last_unix_ms: at,
            occurrences: 1,
            peer: Some(PeerId([1; 32])),
            peer_name: Some("nome".into()),
            module: Some(ModuleKind::Input),
            reason: reason.into(),
        }
    }
    #[test]
    fn thousand_repeats_share_one_group() {
        let mut groups = VecDeque::new();
        for at in 1..=1000 {
            push_block_record(&mut groups, record("A", at));
        }
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].occurrences, 1000);
        assert_eq!(groups[0].at_unix_ms, 1);
        assert_eq!(groups[0].last_unix_ms, 1000);
    }
    #[test]
    fn arrival_lru_preserves_other_causes_and_ignores_names_and_wall_clock() {
        let mut groups = VecDeque::new();
        push_block_record(&mut groups, record("A", 100));
        push_block_record(&mut groups, record("B", 200));
        let mut renamed = record("A", 50);
        renamed.peer_name = Some("outro nome".into());
        push_block_record(&mut groups, renamed);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].reason, "A");
        assert_eq!(groups[1].reason, "B");
        assert_eq!(groups[0].at_unix_ms, 100);
        assert_eq!(groups[0].last_unix_ms, 50);
        assert_eq!(groups[0].occurrences, 2);
        assert_eq!(groups[0].peer_name.as_deref(), Some("outro nome"));
        for n in 0..19 {
            push_block_record(&mut groups, record(&format!("nova{n}"), 1));
        }
        assert_eq!(groups.len(), 20);
        assert!(!groups.iter().any(|b| b.reason == "B"));
        assert!(groups.iter().any(|b| b.reason == "A"));
    }
    #[test]
    fn counts_saturate_without_merging_different_modules_or_peers() {
        let mut max = record("A", 10);
        max.occurrences = u64::MAX;
        let mut groups = VecDeque::from([max]);
        push_block_record(&mut groups, record("A", 11));
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].occurrences, u64::MAX);
        let mut other = record("A", 12);
        other.module = Some(ModuleKind::Clipboard);
        push_block_record(&mut groups, other);
        let mut other = record("A", 13);
        other.peer = Some(PeerId([2; 32]));
        push_block_record(&mut groups, other);
        assert_eq!(groups.len(), 3);
    }
}
