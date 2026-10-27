//! Clipboard module: sync policy over a `ClipboardAdapter` (contrato §7, rules 1..11).
use crate::adapter::{ClipError, ClipSnapshot, ClipboardAdapter};
use crate::config::{ClipMode, ClipboardConfig};
use crate::ledger::{Hash, SeenEvents, WriteLedger, hash_text};
use crate::msg;
use clipsync_proto::{
    CLIP_TTL_MS, ClipAck, ClipMsg, ClipOffer, DeliveryState, EventId, InboxItem, MAX_INBOX_ITEMS,
    MIME_TEXT_UTF8, Module, ModuleCommand, ModuleError, ModuleHost, ModuleInput, ModuleKind,
    ModuleMsg, ModuleReply, ModuleStatus, PeerId, PeerInfo, RejectReason, SendError, SendOutcome,
};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

const ACK_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);
const READ_RETRIES: usize = 3;

// ---- adapter thread (rule 11) ----

enum Cmd {
    /// Atomic "apply": refuses when the clipboard no longer holds what we last knew (rule 7).
    Write {
        text: String,
        expected: Option<Hash>,
        reply: oneshot::Sender<WriteResult>,
    },
    Read {
        reply: oneshot::Sender<Option<ClipSnapshot>>,
    },
    Stop,
}

enum WriteResult {
    Written,
    Conflict,
    Failed(ClipError),
}

enum AdapterEvent {
    Changed(Option<ClipSnapshot>),
    ReadFailed(String),
    Fatal(String),
}

fn read_with_retries(ad: &mut dyn ClipboardAdapter) -> Result<Option<ClipSnapshot>, ClipError> {
    let mut last = ClipError::Busy;
    for i in 0..READ_RETRIES {
        match ad.read_text() {
            Ok(v) => return Ok(v),
            Err(ClipError::Busy) => {
                last = ClipError::Busy;
                if i + 1 < READ_RETRIES {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

fn handle_cmd(ad: &mut dyn ClipboardAdapter, cmd: Cmd) -> bool {
    match cmd {
        Cmd::Stop => return false,
        Cmd::Read { reply } => {
            let _ = reply.send(read_with_retries(ad).ok().flatten());
        }
        Cmd::Write {
            text,
            expected,
            reply,
        } => {
            let res = match read_with_retries(ad) {
                Ok(Some(cur)) if Some(hash_text(&cur.text)) != expected => WriteResult::Conflict,
                Ok(_) => match ad.write_text(&text) {
                    Ok(()) => WriteResult::Written,
                    Err(e) => WriteResult::Failed(e),
                },
                Err(e) => WriteResult::Failed(e),
            };
            let _ = reply.send(res);
        }
    }
    true
}

fn adapter_thread(
    mut ad: Box<dyn ClipboardAdapter>,
    cmd_rx: std_mpsc::Receiver<Cmd>,
    ev_tx: mpsc::Sender<AdapterEvent>,
) {
    loop {
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    if !handle_cmd(ad.as_mut(), cmd) {
                        return;
                    }
                }
                Err(std_mpsc::TryRecvError::Empty) => break,
                Err(std_mpsc::TryRecvError::Disconnected) => return,
            }
        }
        match ad.wait_change(POLL) {
            Ok(Some(_)) => {
                let ev = match read_with_retries(ad.as_mut()) {
                    Ok(snap) => AdapterEvent::Changed(snap),
                    Err(e) => AdapterEvent::ReadFailed(e.to_string()),
                };
                if ev_tx.blocking_send(ev).is_err() {
                    return;
                }
            }
            Ok(None) => {}
            Err(e) => {
                let _ = ev_tx.blocking_send(AdapterEvent::Fatal(e.to_string()));
                return;
            }
        }
    }
}

// ---- shared bookkeeping ----

type PendingAcks = Arc<Mutex<HashMap<(EventId, PeerId), oneshot::Sender<DeliveryState>>>>;

#[derive(Default)]
struct Status {
    last_event: Option<String>,
    last_block: Option<String>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn send_error_state(e: &SendError) -> Option<DeliveryState> {
    match e {
        SendError::NotAuthorized => Some(DeliveryState::Rejected(RejectReason::NotAuthorized)),
        SendError::RemoteModuleDisabled | SendError::LocalModuleDisabled => {
            Some(DeliveryState::Rejected(RejectReason::ModuleDisabled))
        }
        SendError::Paused => Some(DeliveryState::Rejected(RejectReason::Paused)),
        SendError::TooLarge => Some(DeliveryState::Rejected(RejectReason::TooLarge)),
        // No wire verdict exists for these; the caller reports them as "not reached".
        SendError::NotConnected | SendError::Transport(_) => None,
    }
}

/// Sends one offer and waits up to ACK_TIMEOUT for the final state. `None` = not reached.
async fn send_offer(
    host: Arc<dyn ModuleHost>,
    pending: PendingAcks,
    peer: PeerInfo,
    offer: ClipOffer,
) -> Option<SendOutcome> {
    let key = (offer.id, peer.id);
    let (tx, rx) = oneshot::channel();
    lock(&pending).insert(key, tx);
    let sent = host
        .send_reliable(peer.id, ModuleMsg::Clip(ClipMsg::Offer(offer)))
        .await;
    let out = match sent {
        Ok(()) => {
            let state = tokio::time::timeout(ACK_TIMEOUT, rx)
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or(DeliveryState::Received);
            Some(SendOutcome {
                peer: peer.name,
                state,
            })
        }
        Err(e) => send_error_state(&e).map(|state| SendOutcome {
            peer: peer.name,
            state,
        }),
    };
    lock(&pending).remove(&key);
    out
}

// ---- the module ----

pub struct ClipboardModule {
    cfg: ClipboardConfig,
    adapter: Box<dyn ClipboardAdapter>,
}

pub fn build_with_adapter(
    cfg: ClipboardConfig,
    adapter: Box<dyn ClipboardAdapter>,
) -> Box<dyn Module> {
    Box::new(ClipboardModule { cfg, adapter })
}

#[async_trait::async_trait]
impl Module for ClipboardModule {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Clipboard
    }
    fn adapter_name(&self) -> String {
        self.adapter.name().to_owned()
    }
    async fn run(
        self: Box<Self>,
        host: Arc<dyn ModuleHost>,
        rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        let ClipboardModule { cfg, adapter } = *self;
        let adapter_name = adapter.name().to_owned();
        let (cmd_tx, cmd_rx) = std_mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel(64);
        let thread = tokio::task::spawn_blocking(move || adapter_thread(adapter, cmd_rx, ev_tx));
        let mut st = State::new(cfg, adapter_name, host, cmd_tx.clone());
        st.init_current().await;
        let res = st.event_loop(rx, ev_rx).await;
        let _ = cmd_tx.send(Cmd::Stop);
        let _ = thread.await;
        res
    }
}

struct State {
    cfg: ClipboardConfig,
    adapter_name: String,
    host: Arc<dyn ModuleHost>,
    cmd_tx: std_mpsc::Sender<Cmd>,
    paused: bool,
    /// Hash of the clipboard content we currently know (last read or last write of ours).
    current: Option<Hash>,
    ledger: WriteLedger,
    seen: SeenEvents,
    lamport_max: u64,
    /// Highest (lamport, origin) applied or copied locally.
    last: Option<(u64, PeerId)>,
    inbox: VecDeque<InboxItem>,
    pending: PendingAcks,
    status: Arc<Mutex<Status>>,
}

impl State {
    fn new(
        cfg: ClipboardConfig,
        adapter_name: String,
        host: Arc<dyn ModuleHost>,
        cmd_tx: std_mpsc::Sender<Cmd>,
    ) -> Self {
        let paused = host.paused();
        Self {
            cfg,
            adapter_name,
            host,
            cmd_tx,
            paused,
            current: None,
            ledger: WriteLedger::default(),
            seen: SeenEvents::new(Duration::from_millis(u64::from(CLIP_TTL_MS))),
            lamport_max: 0,
            last: None,
            inbox: VecDeque::new(),
            pending: Arc::default(),
            status: Arc::default(),
        }
    }

    fn is_paused(&self) -> bool {
        self.paused || self.host.paused()
    }

    async fn read_now(&self) -> Option<ClipSnapshot> {
        let (reply, rx) = oneshot::channel();
        self.cmd_tx.send(Cmd::Read { reply }).ok()?;
        rx.await.ok().flatten()
    }

    /// Whatever is in the clipboard when we start (or resume) is NOT a new copy (rule 10).
    async fn init_current(&mut self) {
        if let Some(s) = self.read_now().await {
            self.current = Some(hash_text(&s.text));
        }
    }

    async fn event_loop(
        &mut self,
        mut rx: mpsc::Receiver<ModuleInput>,
        mut ev_rx: mpsc::Receiver<AdapterEvent>,
    ) -> Result<(), ModuleError> {
        loop {
            tokio::select! {
                input = rx.recv() => match input {
                    None | Some(ModuleInput::Shutdown) => return Ok(()),
                    Some(i) => self.on_input(i).await,
                },
                ev = ev_rx.recv() => match ev {
                    Some(AdapterEvent::Changed(snap)) => self.on_local_change(snap),
                    Some(AdapterEvent::ReadFailed(d)) => lock(&self.status).last_block = Some(msg::adapter_read_failed(&d)),
                    Some(AdapterEvent::Fatal(d)) => return Err(ModuleError::Adapter(d)),
                    None => return Err(ModuleError::Adapter("adapter thread ended".into())),
                },
            }
        }
    }

    async fn on_input(&mut self, input: ModuleInput) {
        match input {
            ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Clip(ClipMsg::Offer(o)),
            } => self.on_offer(from, o).await,
            ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Clip(ClipMsg::Ack(a)),
            } => self.resolve(a.id, from, a.state),
            ModuleInput::Rejected {
                from,
                ref_id: Some(id),
                reason,
            } => {
                lock(&self.status).last_block =
                    Some(format!("{}: {}", self.peer_name(from), msg::REFUSED));
                self.resolve(id, from, DeliveryState::Rejected(reason));
            }
            ModuleInput::Paused(p) => {
                self.paused = p;
                if !p {
                    // resume: whatever was copied during the pause is not re-sent
                    if let Some(s) = self.read_now().await {
                        self.current = Some(hash_text(&s.text));
                    }
                }
            }
            ModuleInput::Command { cmd, reply } => self.on_command(cmd, reply),
            _ => {}
        }
    }

    fn resolve(&self, id: EventId, from: PeerId, state: DeliveryState) {
        if let Some(tx) = lock(&self.pending).remove(&(id, from)) {
            let _ = tx.send(state);
        }
    }

    fn peer_name(&self, id: PeerId) -> String {
        self.host
            .peers()
            .into_iter()
            .find(|p| p.id == id)
            .map_or_else(|| id.short(), |p| p.name)
    }

    // ---- local copy -> offers (rules 1..4, 10) ----

    fn on_local_change(&mut self, snap: Option<ClipSnapshot>) {
        let Some(snap) = snap else { return }; // rule 4: empty never offers, hash untouched
        let h = hash_text(&snap.text);
        let now = Instant::now();
        if self.ledger.is_echo(&h, now) {
            self.current = Some(h); // rule 2
            return;
        }
        if self.current == Some(h) {
            return; // rule 3: unchanged content (Mutter takeover, repeated copy)
        }
        self.current = Some(h);
        if self.is_paused() {
            return; // rule 10: silent
        }
        if snap.text.len() > self.cfg.max_text_bytes {
            lock(&self.status).last_block = Some(msg::blocked_too_large(
                snap.text.len(),
                self.cfg.max_text_bytes,
            ));
            return;
        }
        let offer = self.new_offer(snap.text);
        self.last = Some((offer.lamport, self.host.local_id()));
        let targets: Vec<PeerInfo> = self
            .host
            .peers()
            .into_iter()
            .filter(|p| p.connected && p.perms.clipboard_out && p.remote_modules.clipboard)
            .collect();
        if targets.is_empty() {
            return;
        }
        lock(&self.status).last_event = Some(msg::sent_to(targets.len()));
        for peer in targets {
            tokio::spawn(send_offer(
                self.host.clone(),
                self.pending.clone(),
                peer,
                offer.clone(),
            ));
        }
    }

    fn new_offer(&mut self, text: String) -> ClipOffer {
        self.lamport_max += 1;
        ClipOffer {
            id: EventId::random(),
            lamport: self.lamport_max,
            mime: MIME_TEXT_UTF8.into(),
            text,
            created_unix_ms: now_unix_ms(),
            ttl_ms: CLIP_TTL_MS,
        }
    }

    // ---- received offers (rules 5..9) ----

    async fn on_offer(&mut self, from: PeerId, o: ClipOffer) {
        let id = o.id;
        let bytes = o.text.len();
        let name = self.peer_name(from);
        if o.mime != MIME_TEXT_UTF8 || o.text.is_empty() {
            return self
                .ack(from, id, DeliveryState::Rejected(RejectReason::Invalid))
                .await;
        }
        if bytes > self.cfg.max_text_bytes {
            return self
                .ack(from, id, DeliveryState::Rejected(RejectReason::TooLarge))
                .await;
        }
        if !self.seen.insert_new(id, Instant::now()) {
            return self
                .ack(from, id, DeliveryState::Rejected(RejectReason::Duplicate))
                .await;
        }
        self.lamport_max = self.lamport_max.max(o.lamport);
        if self.is_paused() {
            return self
                .ack(from, id, DeliveryState::Rejected(RejectReason::Paused))
                .await;
        }
        let state = if o.ttl_ms == 0 {
            DeliveryState::Expired
        } else if self.last.is_some_and(|l| (o.lamport, from) <= l) {
            DeliveryState::Rejected(RejectReason::Stale)
        } else if self.cfg.mode == ClipMode::Manual {
            DeliveryState::InInbox
        } else {
            self.apply(from, &o).await
        };
        let label = match state {
            DeliveryState::Applied => msg::APPLIED,
            DeliveryState::InInbox => msg::IN_INBOX,
            DeliveryState::Rejected(RejectReason::Stale) => msg::STALE,
            _ => msg::REFUSED,
        };
        lock(&self.status).last_event = Some(msg::received_from(&name, label));
        self.push_inbox(&o, name, state);
        self.ack(from, id, state).await;
    }

    async fn apply(&mut self, from: PeerId, o: &ClipOffer) -> DeliveryState {
        let h = hash_text(&o.text);
        self.ledger.register(h, Instant::now()); // BEFORE the write (rule 2)
        let (reply, rx) = oneshot::channel();
        let sent = self.cmd_tx.send(Cmd::Write {
            text: o.text.clone(),
            expected: self.current,
            reply,
        });
        let res = match (sent, rx.await) {
            (Ok(()), Ok(r)) => r,
            _ => WriteResult::Failed(ClipError::Other("adapter thread gone".into())),
        };
        match res {
            WriteResult::Written => {
                self.current = Some(h);
                self.last = Some((o.lamport, from));
                DeliveryState::Applied
            }
            WriteResult::Conflict => {
                self.ledger.forget(&h);
                DeliveryState::InInbox // rule 7: the user copied something newer
            }
            WriteResult::Failed(e) => {
                self.ledger.forget(&h);
                let reason = if matches!(e, ClipError::Busy) {
                    RejectReason::Locked
                } else {
                    RejectReason::ApplyFailed
                };
                DeliveryState::Rejected(reason)
            }
        }
    }

    fn push_inbox(&mut self, o: &ClipOffer, from: String, state: DeliveryState) {
        self.inbox.push_front(InboxItem {
            id: o.id,
            from,
            received_unix_ms: now_unix_ms(),
            mime: o.mime.clone(),
            bytes: o.text.len(),
            preview: o.text.chars().take(200).collect(),
            state,
        });
        self.inbox.truncate(MAX_INBOX_ITEMS);
    }

    async fn ack(&self, to: PeerId, id: EventId, state: DeliveryState) {
        let r = self
            .host
            .send_reliable(to, ModuleMsg::Clip(ClipMsg::Ack(ClipAck { id, state })))
            .await;
        if let Err(e) = r {
            tracing::debug!(peer = %to, error = %e, "clipboard ack not delivered");
        }
    }

    // ---- control commands ----

    fn on_command(&mut self, cmd: ModuleCommand, reply: oneshot::Sender<ModuleReply>) {
        match cmd {
            ModuleCommand::Status => {
                let peers = self.host.peers();
                let s = lock(&self.status);
                let _ = reply.send(ModuleReply::Status(ModuleStatus {
                    kind: ModuleKind::Clipboard,
                    enabled: true,
                    adapter: Some(self.adapter_name.clone()),
                    authorized_peers: peers
                        .iter()
                        .filter(|p| p.perms.clipboard_in || p.perms.clipboard_out)
                        .map(|p| p.name.clone())
                        .collect(),
                    last_event: s.last_event.clone(),
                    last_block: s.last_block.clone(),
                }));
            }
            ModuleCommand::ClipInbox => {
                let _ = reply.send(ModuleReply::ClipInbox(self.inbox.iter().cloned().collect()));
            }
            ModuleCommand::ClipSend { text, to } => self.clip_send(text, to, reply),
            ModuleCommand::InputLoopback { .. } => {
                let _ = reply.send(ModuleReply::Error(msg::UNSUPPORTED_COMMAND.into()));
            }
        }
    }

    fn clip_send(&mut self, text: String, to: Option<PeerId>, reply: oneshot::Sender<ModuleReply>) {
        if text.is_empty() {
            let _ = reply.send(ModuleReply::Error(msg::EMPTY_TEXT.into()));
            return;
        }
        if text.len() > self.cfg.max_text_bytes {
            let _ = reply.send(ModuleReply::Error(msg::TOO_LARGE.into()));
            return;
        }
        // An explicit send is an origin event: it advances the clock but is not a local copy.
        let offer = self.new_offer(text);
        let peers: Vec<PeerInfo> = self
            .host
            .peers()
            .into_iter()
            .filter(|p| to.is_none_or(|t| t == p.id))
            .collect();
        let (host, pending) = (self.host.clone(), self.pending.clone());
        tokio::spawn(async move {
            let sends = peers
                .into_iter()
                .map(|p| tokio::spawn(send_offer(host.clone(), pending.clone(), p, offer.clone())));
            let mut outcomes = Vec::new();
            for h in sends {
                if let Ok(Some(o)) = h.await {
                    outcomes.push(o);
                }
            }
            let _ = reply.send(if outcomes.is_empty() {
                ModuleReply::Error(msg::NO_PEER_REACHED.into())
            } else {
                ModuleReply::ClipSent(outcomes)
            });
        });
    }
}
