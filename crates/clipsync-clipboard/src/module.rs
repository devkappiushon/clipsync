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
const INBOX_SWEEP: Duration = Duration::from_secs(5);
/// Logical steps, not elapsed time: larger remote advances are deliberately incompatible.
const MAX_REMOTE_AHEAD: u64 = 1_000_000;

// ---- adapter thread (rule 11) ----

enum Cmd {
    Write {
        text: String,
        guard: WriteGuard,
        reply: oneshot::Sender<WriteResult>,
    },
    Read {
        reply: oneshot::Sender<Option<ClipSnapshot>>,
    },
    Stop,
}

/// What may still stop a write between the decision and the adapter call.
enum WriteGuard {
    /// Automatic apply of an offer: refused if the agent paused meanwhile or the clipboard no
    /// longer holds what we last knew (rule 7). Both are judged right before the write.
    Offer { expected: Option<Hash> },
    /// Explicit user action (`ClipApply`, contrato §12.6): writes even under pause and over
    /// newer content.
    Explicit,
}

enum WriteResult {
    Written,
    Conflict,
    /// The agent was paused between the authorization and the write.
    Paused,
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

fn handle_cmd(ad: &mut dyn ClipboardAdapter, host: &dyn ModuleHost, cmd: Cmd) -> bool {
    match cmd {
        Cmd::Stop => return false,
        Cmd::Read { reply } => {
            let _ = reply.send(read_with_retries(ad).ok().flatten());
        }
        Cmd::Write { text, guard, reply } => {
            let res = match guard {
                WriteGuard::Explicit => write_now(ad, &text),
                WriteGuard::Offer { expected } => match read_with_retries(ad) {
                    Err(e) => WriteResult::Failed(e),
                    // The read can be slow. Pause and the known content are judged AFTER it,
                    // right before the write, because both may have changed since the offer
                    // was authorized.
                    Ok(cur) => {
                        if host.paused() {
                            WriteResult::Paused
                        } else if cur.is_some_and(|c| Some(hash_text(&c.text)) != expected) {
                            WriteResult::Conflict
                        } else {
                            write_now(ad, &text)
                        }
                    }
                },
            };
            let _ = reply.send(res);
        }
    }
    true
}

fn write_now(ad: &mut dyn ClipboardAdapter, text: &str) -> WriteResult {
    match ad.write_text(text) {
        Ok(()) => WriteResult::Written,
        Err(e) => WriteResult::Failed(e),
    }
}

fn adapter_thread(
    mut ad: Box<dyn ClipboardAdapter>,
    cmd_rx: std_mpsc::Receiver<Cmd>,
    ev_tx: mpsc::Sender<AdapterEvent>,
    host: Arc<dyn ModuleHost>,
) {
    loop {
        loop {
            match cmd_rx.try_recv() {
                Ok(cmd) => {
                    if !handle_cmd(ad.as_mut(), host.as_ref(), cmd) {
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

fn send_error_state(e: &SendError) -> DeliveryState {
    match e {
        SendError::NotAuthorized => DeliveryState::Rejected(RejectReason::NotAuthorized),
        SendError::RemoteModuleDisabled | SendError::LocalModuleDisabled => {
            DeliveryState::Rejected(RejectReason::ModuleDisabled)
        }
        SendError::Paused => DeliveryState::Rejected(RejectReason::Paused),
        SendError::TooLarge => DeliveryState::Rejected(RejectReason::TooLarge),
        SendError::NotConnected | SendError::Transport(_) => DeliveryState::Unreachable,
        SendError::RemoteTooOld => DeliveryState::Rejected(RejectReason::VersionMismatch),
    }
}

/// Sends one offer and waits up to ACK_TIMEOUT for the final state. No Ack in time = `Unreachable`.
async fn send_offer(
    host: Arc<dyn ModuleHost>,
    pending: PendingAcks,
    peer: PeerInfo,
    offer: ClipOffer,
) -> SendOutcome {
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
                .unwrap_or(DeliveryState::Unreachable);
            SendOutcome {
                peer: peer.name,
                state,
            }
        }
        Err(e) => SendOutcome {
            peer: peer.name,
            state: send_error_state(&e),
        },
    };
    lock(&pending).remove(&key);
    out
}

/// Inbox item plus the local monotonic receive time that the ttl counts from (rule 6).
struct InboxEntry {
    item: InboxItem,
    received: Instant,
    ttl: Duration,
    /// Full text, kept only while the item can still be applied by hand (`in_inbox` or stale):
    /// until the ttl, a discard or the apply (contrato §12.6). Memory only: no Debug, no log.
    text: Option<String>,
    /// Origin of the offer, so a manual apply advances the local ordering like an automatic one.
    lamport: u64,
    from: PeerId,
}

/// Only items in the inbox or stale ones can be applied by hand, so only they keep the text.
fn keeps_text(state: DeliveryState) -> bool {
    matches!(
        state,
        DeliveryState::InInbox | DeliveryState::Rejected(RejectReason::Stale)
    )
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
        let thread_host = host.clone();
        let thread = tokio::task::spawn_blocking(move || {
            adapter_thread(adapter, cmd_rx, ev_tx, thread_host)
        });
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
    inbox: VecDeque<InboxEntry>,
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
        let mut sweep = tokio::time::interval(INBOX_SWEEP);
        sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = sweep.tick() => self.expire_inbox(),
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
                ..
            } => self.on_offer(from, o).await,
            ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Clip(ClipMsg::Ack(a)),
                ..
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
            ModuleInput::Command { cmd, reply } => self.on_command(cmd, reply).await,
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
        let offer = match self.new_offer(snap.text) {
            Ok(offer) => offer,
            Err(reason) => {
                lock(&self.status).last_block = Some(reason.into());
                return;
            }
        };
        let origin = (offer.lamport, self.host.local_id());
        self.last = Some(self.last.map_or(origin, |last| last.max(origin)));
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

    fn new_offer(&mut self, text: String) -> Result<ClipOffer, &'static str> {
        let next = self
            .lamport_max
            .checked_add(1)
            .filter(|next| *next != u64::MAX)
            .ok_or(msg::CLOCK_EXHAUSTED)?;
        self.lamport_max = next;
        Ok(ClipOffer {
            id: EventId::random(),
            lamport: self.lamport_max,
            mime: MIME_TEXT_UTF8.into(),
            text,
            created_unix_ms: now_unix_ms(),
            ttl_ms: CLIP_TTL_MS,
        })
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
        if self.is_paused() {
            return self
                .ack(from, id, DeliveryState::Rejected(RejectReason::Paused))
                .await;
        }
        // Expired offers retain the existing inbox/TTL behavior, but cannot poison the clock.
        if o.ttl_ms != 0 {
            if o.lamport == 0
                || o.lamport == u64::MAX
                || o.lamport.saturating_sub(self.lamport_max) > MAX_REMOTE_AHEAD
            {
                lock(&self.status).last_block = Some(msg::CLOCK_AHEAD.into());
                return self
                    .ack(from, id, DeliveryState::Rejected(RejectReason::Invalid))
                    .await;
            }
            self.lamport_max = self.lamport_max.max(o.lamport);
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
        self.push_inbox(from, &o, name, state);
        self.ack(from, id, state).await;
    }

    /// One write through the adapter thread; a dead thread counts as a failed write.
    async fn write(&self, text: String, guard: WriteGuard) -> WriteResult {
        let (reply, rx) = oneshot::channel();
        let sent = self.cmd_tx.send(Cmd::Write { text, guard, reply });
        match (sent, rx.await) {
            (Ok(()), Ok(r)) => r,
            _ => WriteResult::Failed(ClipError::Other("adapter thread gone".into())),
        }
    }

    async fn apply(&mut self, from: PeerId, o: &ClipOffer) -> DeliveryState {
        let h = hash_text(&o.text);
        self.ledger.register(h, Instant::now()); // BEFORE the write (rule 2)
        let guard = WriteGuard::Offer {
            expected: self.current,
        };
        match self.write(o.text.clone(), guard).await {
            WriteResult::Written => {
                self.current = Some(h);
                let origin = (o.lamport, from);
                self.last = Some(self.last.map_or(origin, |last| last.max(origin)));
                DeliveryState::Applied
            }
            WriteResult::Paused => {
                self.ledger.forget(&h);
                DeliveryState::Rejected(RejectReason::Paused)
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

    fn push_inbox(&mut self, from: PeerId, o: &ClipOffer, name: String, state: DeliveryState) {
        let expired = state == DeliveryState::Expired;
        self.inbox.push_front(InboxEntry {
            text: keeps_text(state).then(|| o.text.clone()),
            lamport: o.lamport,
            from,
            item: InboxItem {
                id: o.id,
                from: name,
                received_unix_ms: now_unix_ms(),
                mime: o.mime.clone(),
                bytes: o.text.len(),
                // an expired item never keeps its text in memory
                preview: if expired {
                    String::new()
                } else {
                    o.text.chars().take(200).collect()
                },
                state,
            },
            received: Instant::now(),
            ttl: Duration::from_millis(u64::from(o.ttl_ms.min(CLIP_TTL_MS))),
        });
        self.inbox.truncate(MAX_INBOX_ITEMS);
    }

    /// Rule 6: an item older than its ttl (local monotonic clock) becomes Expired and drops its text.
    fn expire_inbox(&mut self) {
        let now = Instant::now();
        for e in &mut self.inbox {
            if e.item.state != DeliveryState::Expired && now.duration_since(e.received) > e.ttl {
                e.item.state = DeliveryState::Expired;
                e.item.preview = String::new();
                e.text = None;
            }
        }
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

    async fn on_command(&mut self, cmd: ModuleCommand, reply: oneshot::Sender<ModuleReply>) {
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
                self.expire_inbox();
                let _ = reply.send(ModuleReply::ClipInbox(
                    self.inbox.iter().map(|e| e.item.clone()).collect(),
                ));
            }
            ModuleCommand::ClipSend { text, to } => self.clip_send(text, to, reply),
            ModuleCommand::ClipApply { id } => {
                let _ = reply.send(self.apply_item(id).await);
            }
            ModuleCommand::ClipDiscard { id } => {
                let _ = reply.send(self.discard_item(id));
            }
            ModuleCommand::ClipSendCurrent { to } => self.send_current(to, reply).await,
            ModuleCommand::InputLoopback { .. }
            | ModuleCommand::InputState
            | ModuleCommand::InputReload
            | ModuleCommand::InputLock { .. }
            | ModuleCommand::InputSwitch { .. }
            | ModuleCommand::InputOffer { .. }
            | ModuleCommand::InputOfferClear
            | ModuleCommand::InputPrepareInventory
            | ModuleCommand::InputPreviewOrganization(_)
            | ModuleCommand::InputApplyOrganization(_)
            | ModuleCommand::InputRestoreLayoutV1 { .. }
            | ModuleCommand::InputSuspendOrganization
            | ModuleCommand::InputOfferV4 { .. }
            | ModuleCommand::InputAnswerOfferV4 { .. } => {
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
        let offer = match self.new_offer(text) {
            Ok(offer) => offer,
            Err(reason) => {
                lock(&self.status).last_block = Some(reason.into());
                let _ = reply.send(ModuleReply::Error(reason.into()));
                return;
            }
        };
        // `Some` = exactly that peer (an unknown one is an error); `None` = every peer we may send to.
        let peers: Vec<PeerInfo> = self
            .host
            .peers()
            .into_iter()
            .filter(|p| to.map_or(p.perms.clipboard_out, |t| t == p.id))
            .collect();
        if peers.is_empty() {
            let _ = reply.send(ModuleReply::Error(msg::NO_TARGET.into()));
            return;
        }
        let (host, pending) = (self.host.clone(), self.pending.clone());
        tokio::spawn(async move {
            let sends: Vec<_> = peers
                .into_iter()
                .map(|p| {
                    let name = p.name.clone();
                    let task =
                        tokio::spawn(send_offer(host.clone(), pending.clone(), p, offer.clone()));
                    (name, task)
                })
                .collect();
            let mut outcomes = Vec::new();
            for (peer, task) in sends {
                // a send task that died still leaves its peer in the list
                outcomes.push(task.await.unwrap_or(SendOutcome {
                    peer,
                    state: DeliveryState::Unreachable,
                }));
            }
            let _ = reply.send(ModuleReply::ClipSent(outcomes));
        });
    }

    // ---- inbox actions and explicit send (contrato §12.6, L5) ----

    /// `ClipApply`: writes an inbox item to the clipboard by hand. Works under pause, never offers
    /// (the echo is dropped by the ledger) and drops the stored text once applied. A failed write
    /// leaves the item as it was, so the user can try again.
    async fn apply_item(&mut self, id: EventId) -> ModuleReply {
        self.expire_inbox();
        let Some(index) = self.inbox.iter().position(|e| e.item.id == id) else {
            return ModuleReply::Error(msg::NO_ITEM.into());
        };
        let Some(text) = self.inbox[index].text.clone() else {
            return ModuleReply::Error(msg::WRONG_STATE.into());
        };
        let h = hash_text(&text);
        self.ledger.register(h, Instant::now()); // BEFORE the write (rule 2)
        let state = match self.write(text, WriteGuard::Explicit).await {
            WriteResult::Written => DeliveryState::Applied,
            WriteResult::Failed(ClipError::Busy) => DeliveryState::Rejected(RejectReason::Locked),
            WriteResult::Failed(_) | WriteResult::Paused | WriteResult::Conflict => {
                DeliveryState::Rejected(RejectReason::ApplyFailed)
            }
        };
        if state != DeliveryState::Applied {
            self.ledger.forget(&h);
            return ModuleReply::ClipApplied(state);
        }
        self.current = Some(h);
        let entry = &mut self.inbox[index];
        entry.item.state = DeliveryState::Applied;
        entry.text = None;
        let origin = (entry.lamport, entry.from);
        let name = entry.item.from.clone();
        if self.last.is_none_or(|l| origin > l) {
            self.last = Some(origin);
        }
        lock(&self.status).last_event = Some(msg::applied_from_inbox(&name));
        ModuleReply::ClipApplied(state)
    }

    /// `ClipDiscard`: the item leaves the inbox and its text leaves memory.
    fn discard_item(&mut self, id: EventId) -> ModuleReply {
        match self.inbox.iter().position(|e| e.item.id == id) {
            Some(index) => {
                self.inbox.remove(index);
                ModuleReply::Done
            }
            None => ModuleReply::Error(msg::NO_ITEM.into()),
        }
    }

    /// `ClipSendCurrent`: what the clipboard holds now, down the `ClipSend` path (rule 12).
    async fn send_current(&mut self, to: Option<PeerId>, reply: oneshot::Sender<ModuleReply>) {
        match self.read_now().await {
            Some(snap) => self.clip_send(snap.text, to, reply),
            None => {
                let _ = reply.send(ModuleReply::Error(msg::EMPTY_CLIPBOARD.into()));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod clock_tests {
    use super::*;
    use crate::adapter::{FakeClipboard, FakeClipboardHandle};
    use clipsync_proto::{MemHub, MemNode, ModuleSet, PeerPerms};

    const BOTH: ModuleSet = ModuleSet {
        clipboard: true,
        input: false,
    };
    const ALL: PeerPerms = PeerPerms {
        clipboard_in: true,
        clipboard_out: true,
        input_in: false,
    };
    const EXHAUSTED: &str = "relógio lógico do clipboard esgotado; nenhuma oferta foi enviada";
    const AHEAD: &str =
        "relógio remoto inválido: a janela lógica permite até 1000000 passos de avanço";

    struct Fixture {
        st: State,
        remote: MemNode,
        fake: FakeClipboardHandle,
        worker: tokio::task::JoinHandle<()>,
        events: mpsc::Receiver<AdapterEvent>,
    }
    impl Fixture {
        fn new(mode: ClipMode) -> Self {
            let hub = MemHub::new();
            let local = hub.add_node("local", BOTH);
            let remote = hub.add_node("remoto", BOTH);
            hub.link(local.id, remote.id, ALL, ALL);
            let (adapter, fake) = FakeClipboard::new();
            let (cmd_tx, cmd_rx) = std_mpsc::channel();
            let (ev_tx, events) = mpsc::channel(64);
            let host = local.host.clone();
            let worker = tokio::task::spawn_blocking(move || {
                adapter_thread(Box::new(adapter), cmd_rx, ev_tx, host)
            });
            let st = State::new(
                ClipboardConfig {
                    mode,
                    ..ClipboardConfig::default()
                },
                "fake".into(),
                local.host,
                cmd_tx,
            );
            Self {
                st,
                remote,
                fake,
                worker,
                events,
            }
        }
        async fn stop(self) {
            self.st.cmd_tx.send(Cmd::Stop).unwrap();
            self.worker.await.unwrap();
        }
        async fn ack(&mut self) -> DeliveryState {
            loop {
                let input = tokio::time::timeout(Duration::from_secs(2), self.remote.rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                if let ModuleInput::Reliable {
                    msg: ModuleMsg::Clip(ClipMsg::Ack(a)),
                    ..
                } = input
                {
                    return a.state;
                }
            }
        }
        async fn receive(&mut self, o: ClipOffer) -> DeliveryState {
            self.st.on_offer(self.remote.id, o).await;
            self.ack().await
        }
    }
    fn offer(lamport: u64, text: &str) -> ClipOffer {
        ClipOffer {
            id: EventId::random(),
            lamport,
            mime: MIME_TEXT_UTF8.into(),
            text: text.into(),
            created_unix_ms: 0,
            ttl_ms: CLIP_TTL_MS,
        }
    }
    type Snapshot = (
        Option<Hash>,
        Option<(u64, PeerId)>,
        u64,
        Vec<(EventId, DeliveryState, Option<String>)>,
    );
    fn snapshot(st: &State) -> Snapshot {
        (
            st.current,
            st.last,
            st.lamport_max,
            st.inbox
                .iter()
                .map(|e| (e.item.id, e.item.state, e.text.clone()))
                .collect(),
        )
    }
    async fn command(st: &mut State, cmd: ModuleCommand) -> ModuleReply {
        let (reply, rx) = oneshot::channel();
        st.on_command(cmd, reply).await;
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .unwrap()
            .unwrap()
    }

    // Catches early clock incorporation, off-by-one admission, and a poisoned next local copy.
    #[tokio::test]
    async fn clock_admission_preserves_state() {
        let mut f = Fixture::new(ClipMode::Manual);
        f.st.current = Some(hash_text("anterior"));
        f.st.last = Some((7, f.st.host.local_id()));
        f.st.lamport_max = 7;
        for remote in [0, u64::MAX, 1_000_008] {
            let before = snapshot(&f.st);
            assert_eq!(
                f.receive(offer(remote, "recusado")).await,
                DeliveryState::Rejected(RejectReason::Invalid)
            );
            assert_eq!(snapshot(&f.st), before, "remote={remote}");
            assert_eq!(lock(&f.st.status).last_block.as_deref(), Some(AHEAD));
            assert_eq!((f.fake.writes(), f.fake.contents()), (0, None));
        }
        // Content, duplicate, pause and zero TTL take precedence over extreme clock rejection.
        let mut invalid = offer(u64::MAX, "");
        invalid.mime = "image/png".into();
        let before = snapshot(&f.st);
        assert_eq!(
            f.receive(invalid).await,
            DeliveryState::Rejected(RejectReason::Invalid)
        );
        assert_eq!(snapshot(&f.st), before);
        let mut large = offer(u64::MAX, "grande");
        f.st.cfg.max_text_bytes = 1;
        assert_eq!(
            f.receive(large.clone()).await,
            DeliveryState::Rejected(RejectReason::TooLarge)
        );
        assert_eq!(snapshot(&f.st), before);
        f.st.cfg.max_text_bytes = ClipboardConfig::default().max_text_bytes;
        f.st.paused = true;
        assert_eq!(
            f.receive(offer(u64::MAX, "pausado")).await,
            DeliveryState::Rejected(RejectReason::Paused)
        );
        assert_eq!(snapshot(&f.st), before);
        f.st.paused = false;
        large.ttl_ms = 0;
        assert_eq!(f.receive(large).await, DeliveryState::Expired);
        assert_eq!(
            (f.st.current, f.st.last, f.st.lamport_max),
            (before.0, before.1, 7)
        );
        assert_eq!(f.st.inbox.len(), 1);
        assert_eq!(f.st.inbox[0].text, None);
        let before = snapshot(&f.st);
        let duplicate = offer(u64::MAX, "duplicado");
        assert!(f.st.seen.insert_new(duplicate.id, Instant::now()));
        assert_eq!(
            f.receive(duplicate).await,
            DeliveryState::Rejected(RejectReason::Duplicate)
        );
        assert_eq!(snapshot(&f.st), before);
        // Exact limit relative to 7 is admitted, without claiming it was applied.
        assert_eq!(
            f.receive(offer(1_000_007, "limite")).await,
            DeliveryState::InInbox
        );
        assert_eq!(
            (f.st.current, f.st.last, f.st.lamport_max),
            (before.0, before.1, 1_000_007)
        );
        f.st.on_local_change(Some(ClipSnapshot {
            mime: MIME_TEXT_UTF8.into(),
            text: "cópia seguinte".into(),
        }));
        assert_eq!(f.st.lamport_max, 1_000_008);
        assert_eq!(f.st.last, Some((1_000_008, f.st.host.local_id())));
        // Check the actual emitted timestamp, not only the internal counter.
        loop {
            let input = tokio::time::timeout(Duration::from_secs(2), f.remote.rx.recv())
                .await
                .unwrap()
                .unwrap();
            if let ModuleInput::Reliable {
                msg: ModuleMsg::Clip(ClipMsg::Offer(o)),
                ..
            } = input
            {
                assert_eq!((o.lamport, o.text.as_str()), (1_000_008, "cópia seguinte"));
                break;
            }
        }
        f.stop().await;
    }

    // Catches MAX emission/overflow in each creator and failure to return to the module loop.
    #[tokio::test]
    async fn local_clock_exhaustion_keeps_module_alive() {
        for clock in [u64::MAX - 1, u64::MAX] {
            let mut f = Fixture::new(ClipMode::Auto);
            f.st.lamport_max = u64::MAX - 2;
            assert_eq!(
                f.st.new_offer("último válido".into()).unwrap().lamport,
                u64::MAX - 1
            );
            f.st.lamport_max = clock;
            f.st.last = Some((5, f.remote.id));
            let last = f.st.last;
            f.fake.user_copy("cópia local");
            f.st.on_local_change(Some(ClipSnapshot {
                mime: MIME_TEXT_UTF8.into(),
                text: "cópia local".into(),
            }));
            assert_eq!(f.st.last, last, "failed copy cannot claim a new origin");
            assert_eq!(f.st.lamport_max, clock);
            assert_eq!(f.st.current, Some(hash_text("cópia local")));
            assert_eq!(lock(&f.st.status).last_block.as_deref(), Some(EXHAUSTED));
            for cmd in [
                ModuleCommand::ClipSend {
                    text: "envio explícito".into(),
                    to: Some(f.remote.id),
                },
                ModuleCommand::ClipSendCurrent {
                    to: Some(f.remote.id),
                },
            ] {
                assert!(
                    matches!(command(&mut f.st, cmd).await, ModuleReply::Error(e) if e == EXHAUSTED)
                );
                assert_eq!((f.st.lamport_max, f.st.last), (clock, last));
            }
            assert_eq!(f.fake.writes(), 0);
            assert!(f.st.inbox.is_empty());
            // Run the actual loop after exhaustion; status/pause/shutdown must still be handled.
            let (tx, rx) = mpsc::channel(8);
            let status = f.st.status.clone();
            let task = tokio::spawn(async move {
                f.st.event_loop(rx, f.events).await.unwrap();
                f.st
            });
            tx.send(ModuleInput::Paused(true)).await.unwrap();
            let (reply, response) = oneshot::channel();
            tx.send(ModuleInput::Command {
                cmd: ModuleCommand::Status,
                reply,
            })
            .await
            .unwrap();
            let response = tokio::time::timeout(Duration::from_secs(2), response)
                .await
                .unwrap()
                .unwrap();
            assert!(
                matches!(response, ModuleReply::Status(s) if s.last_block.as_deref() == Some(EXHAUSTED))
            );
            tx.send(ModuleInput::Shutdown).await.unwrap();
            let st = task.await.unwrap();
            assert!(st.paused);
            assert_eq!((st.lamport_max, st.last), (clock, last));
            assert_eq!(lock(&status).last_block.as_deref(), Some(EXHAUSTED));
            // No offer of any timestamp (including MAX or 0) may have escaped.
            while let Ok(input) = f.remote.rx.try_recv() {
                assert!(!matches!(
                    input,
                    ModuleInput::Reliable {
                        msg: ModuleMsg::Clip(ClipMsg::Offer(_)),
                        ..
                    }
                ));
            }
            st.cmd_tx.send(Cmd::Stop).unwrap();
            f.worker.await.unwrap();
        }
    }

    // Catches reversed tie order and pretending an inbox entry/failed write was locally applied.
    #[tokio::test]
    async fn equal_timestamp_tiebreak_and_last() {
        let low = PeerId([1; 32]);
        let high = PeerId([2; 32]);
        for order in [[low, high], [high, low]] {
            let mut f = Fixture::new(ClipMode::Auto);
            for from in order {
                f.st.on_offer(from, offer(10, if from == low { "menor" } else { "maior" }))
                    .await;
            }
            assert_eq!(f.st.last, Some((10, high)));
            assert_eq!(f.fake.contents().as_deref(), Some("maior"));
            assert_eq!(f.fake.writes(), if order[0] == low { 2 } else { 1 });
            assert_eq!(
                f.st.inbox[0].item.state,
                if order[0] == low {
                    DeliveryState::Applied
                } else {
                    DeliveryState::Rejected(RejectReason::Stale)
                }
            );
            // Explicit stale apply may change content, but must not roll back ordering.
            let stale = offer(9, "stale explícito");
            f.st.on_offer(low, stale.clone()).await;
            assert!(matches!(
                f.st.apply_item(stale.id).await,
                ModuleReply::ClipApplied(DeliveryState::Applied)
            ));
            assert_eq!(f.fake.contents().as_deref(), Some("stale explícito"));
            assert_eq!(f.st.last, Some((10, high)));
            f.st.on_offer(low, offer(10, "ainda antigo")).await;
            assert_eq!(f.fake.contents().as_deref(), Some("stale explícito"));
            assert_eq!(f.st.last, Some((10, high)));
            f.stop().await;
        }
        let mut f = Fixture::new(ClipMode::Auto);
        f.st.last = Some((5, low));
        f.fake.fail_writes(true);
        assert_eq!(
            f.receive(offer(6, "falha")).await,
            DeliveryState::Rejected(RejectReason::Locked)
        );
        assert_eq!(
            (f.st.last, f.st.current, f.st.lamport_max),
            (Some((5, low)), None, 6)
        );
        f.fake.fail_writes(false);
        f.fake.user_copy("mudança concorrente");
        assert_eq!(
            f.receive(offer(7, "conflito")).await,
            DeliveryState::InInbox
        );
        assert_eq!(
            (f.st.last, f.st.current, f.st.lamport_max),
            (Some((5, low)), None, 7)
        );
        let conflicted = f.st.inbox[0].item.id;
        f.fake.fail_writes(true);
        assert!(matches!(
            f.st.apply_item(conflicted).await,
            ModuleReply::ClipApplied(DeliveryState::Rejected(RejectReason::Locked))
        ));
        assert_eq!((f.st.last, f.st.current), (Some((5, low)), None));
        f.fake.fail_writes(false);
        f.st.cfg.mode = ClipMode::Manual;
        let manual = offer(8, "manual");
        assert_eq!(f.receive(manual.clone()).await, DeliveryState::InInbox);
        assert_eq!(
            (f.st.last, f.st.current, f.st.lamport_max),
            (Some((5, low)), None, 8)
        );
        assert!(matches!(
            f.st.apply_item(manual.id).await,
            ModuleReply::ClipApplied(DeliveryState::Applied)
        ));
        assert_eq!(
            (f.st.last, f.st.current),
            (Some((8, f.remote.id)), Some(hash_text("manual")))
        );
        assert_eq!(
            (f.fake.writes(), f.fake.contents().as_deref()),
            (1, Some("manual"))
        );
        f.stop().await;
    }
}
