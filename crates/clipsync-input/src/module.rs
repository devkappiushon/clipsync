//! The input module: destination side (Enter / datagrams / Stats), the source-side loopback
//! driven by `ModuleCommand::InputLoopback` (contract §8 rules 1 to 3) and the continuous
//! capture derived from `layout.toml` (§12.6 item 2, in `capture.rs`).
mod organization;
use crate::adapter::lifecycle::LifecycleCursor;
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, Hotkey, InputError, Zone, ZoneRef,
    entry_events, same_zones,
};
use crate::capture::{
    self, Ctl, Engine, Note, SPAN_PROTOCOL, Target, ZONES_POLL, new_session, reject_code,
};
use crate::config::{InputConfig, map_pos, zone_ref};
use crate::layout::{DerivedBarrier, Endpoint, Layout, trim_dead_corner};
use crate::layout_document::LayoutDocument;
use crate::msg;
use crate::span::SpanTrack;
use async_trait::async_trait;
use clipsync_proto::{
    Edge, InputDatagram, InputMsg, InputState, LayoutOffer, LoopbackStats,
    MAX_INPUT_DATAGRAMS_PER_SEC, Module, ModuleCommand, ModuleError, ModuleHost, ModuleInput,
    ModuleKind, ModuleMsg, ModuleReply, ModuleStatus, OfferDevice, OfferEndpoint, OfferLink,
    OfferOutcome, OfferState, PeerId, PeerInfo, PeerZones, PendingOffer, ReceiveContext,
    RejectReason, SendError, ZoneInfo, mono_us,
};
use organization::OrganizationRuntime;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant;

const STATS_WAIT: Duration = Duration::from_secs(3);
const MAX_LATENCY_SAMPLES: usize = 100_000;

pub struct InputModule {
    cfg: InputConfig,
    cap: Box<dyn CaptureAdapter>,
    emu: Box<dyn EmulateAdapter>,
    layout: Option<Layout>,
    organization: OrganizationRuntime,
    layout_path: Option<PathBuf>,
}

/// No `layout.toml`: loopback only (contract §8 rule 1).
pub fn build_with_adapters(
    cfg: InputConfig,
    cap: Box<dyn CaptureAdapter>,
    emu: Box<dyn EmulateAdapter>,
) -> Box<dyn Module> {
    Box::new(InputModule {
        cfg,
        cap,
        emu,
        layout: None,
        organization: OrganizationRuntime::default(),
        layout_path: None,
    })
}

/// Reads `DIR/layout.toml` for the continuous capture. Links there and manual
/// `[[input.barrier]]` together are a `ModuleError::Config` (contract §12.6 item 3).
pub fn build_with_layout(
    cfg: InputConfig,
    cap: Box<dyn CaptureAdapter>,
    emu: Box<dyn EmulateAdapter>,
    profile_dir: &Path,
) -> Result<Box<dyn Module>, ModuleError> {
    let config = |e: crate::layout::LayoutError| ModuleError::Config(e.to_string());
    let path = profile_dir.join(crate::LAYOUT_FILE);
    let document = LayoutDocument::read(&path).map_err(config)?;
    let layout = document.as_ref().map(|d| d.legacy.clone());
    if let Some(l) = &layout {
        l.ensure_no_manual_barriers(&cfg).map_err(config)?;
    }
    Ok(Box::new(InputModule {
        cfg,
        cap,
        emu,
        layout,
        organization: OrganizationRuntime {
            document,
            ..Default::default()
        },
        layout_path: Some(path),
    }))
}

/// The continuous-capture task, which owns the capture adapter while it runs.
struct Cont {
    ctl: mpsc::UnboundedSender<Ctl>,
    task: JoinHandle<Box<dyn CaptureAdapter>>,
}

/// An `InputOffer` in flight: a refusal of the `LayoutOffer` arrives after the send.
struct OfferJob {
    outcomes: Vec<OfferOutcome>,
    reply: oneshot::Sender<ModuleReply>,
    until: Instant,
    waiting: usize,
    rejected: std::collections::HashSet<PeerId>,
}

struct OfferCompletion {
    notes: mpsc::UnboundedSender<Note>,
    peer: PeerId,
    state: OfferState,
}
impl OfferCompletion {
    fn set(&mut self, state: OfferState) {
        self.state = state;
    }
}
impl Drop for OfferCompletion {
    fn drop(&mut self) {
        let _ = self.notes.send(Note::OfferSent(self.peer, self.state));
    }
}

const OFFER_GRACE: Duration = Duration::from_millis(700);
use crate::capture::LAYOUT_PROTOCOL;

/// One barrier of a loopback and the destination side it maps to.
struct LoopBarrier {
    spec: BarrierSpec,
    peer_edge: Edge,
    local: (f32, f32),
    remote: (f32, f32),
}

/// Destination-side record of one controlling peer.
struct Incoming {
    context: ReceiveContext,
    session: u32,
    active: bool,
    /// Events received (not datagrams).
    received: u64,
    /// Datagrams missing by sequence gap.
    lost: u64,
    /// Datagrams that arrived with an old sequence number.
    out_of_order: u64,
    last_seq: Option<u32>,
    latencies: Vec<u64>,
    samples: u64,
    /// Entered with `EnterSpan`: this side decides the way back.
    span: Option<SpanTrack>,
    routes: Option<(
        crate::route_track::RouteTrack,
        clipsync_proto::InventoryRevisions,
    )>,
}

impl Incoming {
    fn new(session: u32, context: ReceiveContext) -> Self {
        Self {
            context,
            session,
            active: true,
            received: 0,
            lost: 0,
            out_of_order: 0,
            last_seq: None,
            latencies: Vec::new(),
            samples: 0,
            span: None,
            routes: None,
        }
    }

    fn stats(&self) -> LoopbackStats {
        let mut l = self.latencies.clone();
        l.sort_unstable();
        let pick = |p: usize| {
            if l.is_empty() {
                0
            } else {
                l[(l.len() * p / 100).min(l.len() - 1)]
            }
        };
        LoopbackStats {
            session: self.session,
            received: self.received,
            lost: self.lost,
            out_of_order: self.out_of_order,
            p50_us: pick(50),
            p95_us: pick(95),
            max_us: l.last().copied().unwrap_or(0),
        }
    }

    fn record_latency(&mut self, v: u64) {
        if self.latencies.len() < MAX_LATENCY_SAMPLES {
            self.latencies.push(v);
        } else {
            let i = (self.samples % MAX_LATENCY_SAMPLES as u64) as usize;
            self.latencies[i] = v;
        }
        self.samples += 1;
    }
}

/// Token bucket on the monotonic clock: `MAX_INPUT_DATAGRAMS_PER_SEC` tokens per second,
/// burst of the same size.
struct Bucket {
    tokens: f64,
    last: Instant,
}

impl Default for Bucket {
    fn default() -> Self {
        Self {
            tokens: f64::from(MAX_INPUT_DATAGRAMS_PER_SEC),
            last: Instant::now(),
        }
    }
}

impl Bucket {
    fn take(&mut self, now: Instant) -> bool {
        let cap = f64::from(MAX_INPUT_DATAGRAMS_PER_SEC);
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + elapsed * cap).min(cap);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

enum LoopSignal {
    Stop,
    /// "Travar nesta tela" while the loopback runs: the session ends as `lock`, not `stop`.
    Locked,
    Stats(LoopbackStats),
    Rejected(RejectReason),
    PeerDown,
    Paused,
}

struct LoopCtl {
    to: PeerId,
    session: Arc<AtomicU32>,
    tx: mpsc::UnboundedSender<LoopSignal>,
}

struct LoopDone {
    cap: Box<dyn CaptureAdapter>,
    outcome: Result<LoopbackStats, String>,
    reply: oneshot::Sender<ModuleReply>,
}

struct Entry<'a> {
    edge: Edge,
    monitor: Option<&'a str>,
    pos: f32,
    back: Option<(f32, f32)>,
}

#[derive(Default)]
struct StreamFrontier {
    connection: u64,
    closed: Option<u64>,
    entered: Option<u64>,
}

struct State {
    network: crate::network::Network,
    frontiers: HashMap<PeerId, StreamFrontier>,
    cfg: InputConfig,
    host: Arc<dyn ModuleHost>,
    emu: Box<dyn EmulateAdapter>,
    emu_started: bool,
    cap: Option<Box<dyn CaptureAdapter>>,
    cap_name: &'static str,
    emu_name: &'static str,
    peers: HashMap<PeerId, PeerInfo>,
    incoming: HashMap<PeerId, Incoming>,
    buckets: HashMap<PeerId, Bucket>,
    ctl: Option<LoopCtl>,
    ignored: u64,
    rate_dropped: u64,
    paused: bool,
    authority: capture::HostCursor,
    last_event: Option<String>,
    last_block: Option<String>,
    layout: Option<Layout>,
    organization: OrganizationRuntime,
    layout_path: Option<PathBuf>,
    /// "Travar nesta tela": barriers stay installed but never open a session.
    locked: bool,
    cont: Option<Cont>,
    notes: mpsc::UnboundedSender<Note>,
    /// Last zones the adapter reported (live once a capture session exists).
    local_zones: Vec<Zone>,
    /// Last `Zones` of each peer; kept after it disconnects.
    peer_zones: HashMap<PeerId, Vec<ZoneInfo>>,
    pending_offer: Option<PendingOffer>,
    active_peer: Option<PeerId>,
    offer_job: Option<OfferJob>,
    /// The capture adapter holds system-wide hooks: pausing stops it (§13.2 rule 2g).
    cap_hooks: bool,
    /// While paused and stopped: the resume combination, when the system allowed it.
    resume_rx: Option<mpsc::Receiver<CaptureEvent>>,
    /// Stopped because no link leads to a peer this machine trusts; it starts again when one does.
    suspended: bool,
    /// The links were there but their monitors were not: start when the zones show up.
    retry_on_zones: bool,
    /// The targets the engine runs with, to retarget only on a real change.
    targets_key: String,
    /// Last `routes.*` line emitted, so a route build is logged only when it changes.
    routes_logged: Option<String>,
    /// The adapter can start a capture without a crossing (`can_begin`): a switch by button works.
    cap_can_begin: bool,
    /// The system refused Win+Alt+P as the way to resume: only the tray and the button are left.
    resume_refused: bool,
}

#[async_trait]
impl Module for InputModule {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Input
    }

    fn adapter_name(&self) -> String {
        format!("{}+{}", self.cap.name(), self.emu.name())
    }

    async fn run(
        self: Box<Self>,
        host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        let me = *self;
        let authority = capture::HostCursor::new(host.as_ref());
        let paused = authority.snapshot.paused || authority.snapshot.stopped;
        let (notes, mut note_rx) = mpsc::unbounded_channel();
        let mut st = State {
            network: Default::default(),
            frontiers: HashMap::new(),
            cap_name: me.cap.name(),
            emu_name: me.emu.name(),
            cfg: me.cfg,
            authority,
            host,
            emu: me.emu,
            emu_started: false,
            cap: Some(me.cap),
            peers: HashMap::new(),
            incoming: HashMap::new(),
            buckets: HashMap::new(),
            ctl: None,
            ignored: 0,
            rate_dropped: 0,
            paused,
            last_event: None,
            last_block: None,
            layout: me.layout,
            organization: me.organization,
            layout_path: me.layout_path,
            locked: false,
            cont: None,
            notes,
            local_zones: Vec::new(),
            peer_zones: HashMap::new(),
            pending_offer: None,
            active_peer: None,
            offer_job: None,
            cap_hooks: false,
            resume_rx: None,
            suspended: false,
            retry_on_zones: false,
            targets_key: String::new(),
            routes_logged: None,
            resume_refused: false,
            cap_can_begin: true,
        };
        st.cap_hooks = st.cap.as_ref().is_some_and(|c| c.holds_hooks());
        st.cap_can_begin = st.cap.as_ref().is_none_or(|c| c.can_begin());
        st.local_zones = st.cap.as_ref().map(|c| c.known_zones()).unwrap_or_default();
        if paused && st.cap_hooks {
            st.arm_resume().await;
        }
        st.refresh_known_inventory().await;
        st.start_continuous().await;
        let mut task: Option<JoinHandle<LoopDone>> = None;
        let mut tick = tokio::time::interval(ZONES_POLL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut cleanup = std::collections::VecDeque::new();
        loop {
            cleanup.extend(st.organization.cleanup.take());
            st.reconcile_host().await;
            if !st.preparation_valid() {
                st.cancel_preparation().await;
            }
            if st.authority.snapshot.stopped {
                break;
            }
            tokio::select! {
                _ = async { if let Some(job) = cleanup.front_mut() { let _ = job.await; } }, if !cleanup.is_empty() => { cleanup.pop_front(); },
                done = async { match st.organization.preparation.as_mut() { Some(p) => p.future.as_mut().await, None => std::future::pending().await } }, if st.organization.preparation.is_some() => {
                    st.finish_preparation(done, false).await;
                },
                () = st.authority.changed() => {},
                input = rx.recv() => {
                    let Some(input) = input else { break };
                    match input {
                        ModuleInput::Shutdown => break,
                        ModuleInput::Command { cmd, reply } => {
                            if let Some(t) = st.command(cmd, reply, &mut task).await {
                                task = Some(t);
                            }
                        }
                        other => st.handle(other).await,
                    }
                }
                done = async { match task.as_mut() { Some(t) => Some(t.await), None => None } },
                    if task.is_some() =>
                {
                    task = None;
                    st.ctl = None;
                    match done {
                        Some(Ok(d)) => st.finish_loopback(d).await,
                        _ => {
                            // The task panicked: the capture adapter is gone with it.
                            st.last_block = Some(msg::adapter_failed("loopback"));
                        }
                    }
                }
                n = note_rx.recv() => {
                    if let Some(n) = n {
                        st.on_note(n).await;
                    }
                }
                ev = async { match st.resume_rx.as_mut() { Some(r) => r.recv().await, None => None } },
                    if st.resume_rx.is_some() =>
                {
                    match ev {
                        Some(CaptureEvent::Hotkey(h)) => st.on_hotkey(h).await,
                        Some(_) => {}
                        None => st.resume_rx = None,
                    }
                }
                _ = tick.tick() => st.on_tick().await,
                () = async { if let Some(j) = &st.offer_job { tokio::time::sleep_until(j.until).await } },
                    if st.offer_job.is_some() =>
                {
                    st.finish_offer();
                }
            }
        }
        st.release_all().await;
        st.cancel_preparation().await;
        if let Some(t) = task {
            if let Some(c) = &st.ctl {
                let _ = c.tx.send(LoopSignal::Stop);
            }
            if let Ok(done) = t.await {
                st.cap = Some(done.cap);
                let _ = done
                    .reply
                    .send(ModuleReply::Error(msg::SESSION_CANCELLED.to_owned()));
            }
        }
        if let Some(j) = st.offer_job.take() {
            let _ = j
                .reply
                .send(ModuleReply::Error(msg::SESSION_CANCELLED.to_owned()));
        }
        st.stop_continuous().await;
        cleanup.extend(st.organization.cleanup.take());
        let _ = tokio::time::timeout(crate::network::SEND_WAIT, async {
            for job in cleanup {
                let _ = job.await;
            }
        })
        .await;
        let _ = st.emu.stop().await;
        st.network.stop().await;
        Ok(())
    }
}

impl State {
    async fn reconcile_host(&mut self) {
        let (changed, invalidated) = self.authority.refresh();
        if !changed {
            return;
        }
        let now = self.authority.snapshot;
        if invalidated && !self.paused {
            self.apply_pause(true).await;
        }
        let paused = now.paused || now.stopped;
        if paused != self.paused {
            self.apply_pause(paused).await;
        }
        for peer in self.incoming.keys().copied().collect::<Vec<_>>() {
            self.reconcile_incoming(peer).await;
        }
        self.sync_links().await;
    }

    async fn apply_pause(&mut self, p: bool) {
        self.paused = p;
        if p {
            self.cancel_preparation().await;
        }
        self.send_ctl(Ctl::Paused(p));
        if p {
            for i in self.incoming.values_mut() {
                i.active = false;
            }
            self.release_all().await;
            if let Some(c) = &self.ctl {
                let _ = c.tx.send(LoopSignal::Paused);
            }
            // Hooks do not stay installed while paused (§13.2 rule 2g); only the system
            // hotkey for the way back is left, if the system allows it.
            if self.cap_hooks {
                if self.cont.is_some() {
                    self.stop_continuous().await;
                }
                self.arm_resume().await;
            }
        } else {
            self.disarm_resume().await;
            self.start_continuous().await;
        }
    }

    async fn finish_loopback(&mut self, d: LoopDone) {
        self.cap = Some(d.cap);
        let reply = match d.outcome {
            Ok(s) => {
                self.last_event = Some(format!("loopback: {} eventos recebidos", s.received));
                ModuleReply::InputLoopback(s)
            }
            Err(e) => {
                self.last_block = Some(e.clone());
                ModuleReply::Error(e)
            }
        };
        let _ = d.reply.send(reply);
        self.start_continuous().await;
    }

    async fn on_note(&mut self, n: Note) {
        match n {
            Note::Event(t) => self.last_event = Some(t),
            Note::Block(t) => self.last_block = Some(t),
            Note::Active(p) => self.active_peer = p,
            Note::PrepareInventory => {
                if !self.organization.inventory_blocked {
                    self.begin_preparation(None).await;
                }
            }
            Note::Inventory(o) => {
                if self.organization.inventory_blocked {
                    return;
                }
                if let Err(e) = self.observe_inventory(o).await {
                    self.organization.problem = Some(e.to_string());
                }
            }
            Note::Zones(z) => self.on_zones(z).await,
            Note::Hotkey(h) => self.on_hotkey(h).await,
            Note::Ignored => self.ignored += 1,
            Note::OfferSent(peer, state) => {
                if let Some(job) = &mut self.offer_job {
                    if !job.rejected.contains(&peer)
                        && let Some(o) = job.outcomes.iter_mut().find(|o| o.peer == peer)
                    {
                        o.state = state;
                    }
                    job.waiting = job.waiting.saturating_sub(1);
                    if job.waiting == 0 {
                        job.until = Instant::now() + OFFER_GRACE;
                    }
                }
            }
            Note::Restart => {
                self.stop_continuous().await;
                self.start_continuous().await;
            }
        }
    }

    /// The monitors may have changed (§13.8): a change by content is announced to the peers that
    /// may control this machine, and the barriers follow without a new capture session.
    async fn on_zones(&mut self, z: Vec<Zone>) {
        if z.is_empty() || same_zones(&self.local_zones, &z) {
            return;
        }
        self.local_zones = z;
        self.announce_zones_to_all();
        if self.cont.is_some() {
            self.retarget().await;
        } else if self.retry_on_zones {
            self.start_continuous().await;
        }
    }

    /// Every 2 s, in every platform: the zones the adapter knows (the engine does the same while
    /// it owns the adapter) and whether the links still lead to trusted peers.
    async fn on_tick(&mut self) {
        self.refresh_known_inventory().await;
        for peer in self.incoming.keys().copied().collect::<Vec<_>>() {
            self.reconcile_incoming(peer).await;
        }
        let known = self.cap.as_ref().map(|c| c.known_zones());
        if let Some(z) = known {
            self.on_zones(z).await;
        }
        self.sync_links().await;
    }

    /// What a recognized shortcut does (§13.6 item 5).
    async fn on_hotkey(&mut self, h: Hotkey) {
        if self.host.paused() && h != Hotkey::TogglePause {
            return;
        }
        match h {
            Hotkey::GoTo(n) => {
                let me = self.host.local_id();
                let target = self
                    .layout
                    .as_ref()
                    .and_then(|l| l.devices.iter().find(|d| d.hotkey == Some(n)))
                    .map(|d| d.peer);
                match target {
                    Some(peer) if peer == me => {
                        self.send_ctl(Ctl::Home);
                        if let Some(c) = &self.ctl {
                            let _ = c.tx.send(LoopSignal::Stop);
                        }
                    }
                    Some(peer) => {
                        let (tx, rx) = oneshot::channel();
                        self.switch(peer, tx);
                        let notes = self.notes.clone();
                        self.network.spawn_for(Duration::from_secs(5), async move {
                            if let Ok(ModuleReply::Error(e)) = rx.await {
                                let _ = notes.send(Note::Block(e));
                            }
                        });
                    }
                    None => self.last_block = Some(msg::no_device_for_hotkey(n)),
                }
            }
            Hotkey::ToggleLock => {
                let on = !self.locked;
                self.set_locked(on);
            }
            Hotkey::TogglePause => self.host.request_pause(!self.host.paused()),
        }
    }

    /// "Travar nesta tela", by command or shortcut.
    fn set_locked(&mut self, on: bool) {
        self.locked = on;
        if on && let Some(c) = &self.ctl {
            let _ = c.tx.send(LoopSignal::Locked);
        }
        self.send_ctl(Ctl::Lock(on));
        self.last_event = Some(if on {
            msg::LOCKED_ON.to_owned()
        } else {
            msg::LOCKED_OFF.to_owned()
        });
    }

    /// Paused with the capture stopped: listen for the resume combination, nothing else.
    async fn arm_resume(&mut self) {
        let Some(cap) = self.cap.as_mut() else { return };
        match cap.watch_resume().await {
            Ok(rx) => {
                self.resume_rx = rx;
                self.resume_refused = false;
            }
            Err(e) => {
                tracing::warn!("resume hotkey refused: {e}");
                self.last_block = Some(msg::RESUME_HOTKEY_REFUSED.to_owned());
                self.resume_refused = true;
            }
        }
    }

    async fn disarm_resume(&mut self) {
        self.resume_rx = None;
        self.resume_refused = false;
        if let Some(cap) = self.cap.as_mut() {
            cap.unwatch_resume().await;
        }
    }

    fn input_state(&self) -> InputState {
        let mut peers: Vec<PeerZones> = self
            .peer_zones
            .iter()
            .map(|(p, z)| PeerZones {
                peer: *p,
                zones: z.clone(),
            })
            .collect();
        peers.sort_by_key(|p| p.peer);
        InputState {
            inventories: self.organization.catalog.snapshot(),
            inventory_problem: self.organization.problem.clone(),
            pending_monitor_offer: self.organization.pending.clone(),
            local: self.local_zones.iter().map(zone_info).collect(),
            peers,
            screen_locked: self.locked,
            active_peer: self.active_peer,
            pending_offer: self.pending_offer.clone(),
            keyboard_layout: self.emu.keyboard_layout(),
            resume_hotkey_refused: self.resume_refused,
            switch_supported: self.cap_can_begin,
        }
    }

    /// `InputReload`: reads `layout.toml` again and reinstalls the barriers. A bad file keeps
    /// what was installed and reports why.
    async fn reload_layout(&mut self) -> Result<(), String> {
        let Some(path) = self.layout_path.clone() else {
            return Err(msg::NO_LAYOUT_PATH.to_owned());
        };
        let document = LayoutDocument::read(&path).map_err(|e| e.to_string())?;
        let layout = document.as_ref().map(|d| d.legacy.clone());
        if let Some(l) = &layout {
            l.ensure_no_manual_barriers(&self.cfg)
                .map_err(|e| e.to_string())?;
        }
        if self
            .organization
            .document
            .as_ref()
            .is_some_and(|d| d.organization.is_some())
            || document.as_ref().is_some_and(|d| d.organization.is_some())
        {
            self.quiesce_organization().await.map_err(|e| e.code)?;
        } else {
            self.stop_continuous().await;
        }
        self.layout = layout;
        self.organization.document = document;
        self.organization.suspended = false;
        self.organization.ingress_suspended = false;
        self.start_continuous().await;
        self.last_event = Some(msg::LAYOUT_RELOADED.to_owned());
        Ok(())
    }

    /// `InputSwitch`: the continuous capture opens the session; the answer comes when the peer
    /// had its chance to refuse.
    fn switch(&mut self, to: PeerId, reply: oneshot::Sender<ModuleReply>) {
        let Some(cont) = &self.cont else {
            let _ = reply.send(ModuleReply::Error(msg::NOT_LINKED.to_owned()));
            return;
        };
        if !self.network.has_capacity() {
            let _ = reply.send(ModuleReply::Error(msg::NETWORK_BUSY.to_owned()));
            return;
        }
        let (tx, rx) = oneshot::channel();
        let _ = cont.ctl.send(Ctl::Switch { to, reply: tx });
        self.network.spawn_for(Duration::from_secs(5), async move {
            let mut reply = reply;
            let result =
                tokio::select! { biased; () = reply.closed() => return, result = rx => result };
            let r = match result {
                Ok(Ok(())) => ModuleReply::Done,
                Ok(Err(e)) => ModuleReply::Error(e),
                Err(_) => ModuleReply::Error(msg::adapter_failed("captura")),
            };
            let _ = reply.send(r);
        });
    }

    /// `InputOffer`: one `LayoutOffer` per peer; `Offered` becomes `Denied` if the peer refuses
    /// within `OFFER_GRACE`.
    async fn offer(&mut self, to: Vec<PeerId>, reply: oneshot::Sender<ModuleReply>) {
        if self
            .organization
            .document
            .as_ref()
            .is_some_and(|d| d.organization.is_some())
        {
            let _ = reply.send(ModuleReply::Error("mapa v2 exige oferta v4".into()));
            return;
        }

        if self.offer_job.is_some() {
            let _ = reply.send(ModuleReply::Error(msg::OFFER_BUSY.to_owned()));
            return;
        }
        let Some(offer) = self.layout.as_ref().map(layout_offer) else {
            let _ = reply.send(ModuleReply::Error(msg::NO_LAYOUT.to_owned()));
            return;
        };
        if !self.network.has_capacity() {
            let _ = reply.send(ModuleReply::Error(msg::NETWORK_BUSY.to_owned()));
            return;
        }
        let mut outcomes = Vec::new();
        let mut waiting = 0;
        for peer in to {
            let state = match self.peers.get(&peer) {
                Some(p) if !p.connected => OfferState::Offline,
                None => OfferState::Offline,
                Some(p) if p.remote_version < LAYOUT_PROTOCOL => OfferState::Unsupported,
                Some(_) => {
                    if self.network.has_capacity()
                        && let Some(context) = self.host.peer_input_context(peer)
                    {
                        let host = self.host.clone();
                        let offer = offer.clone();
                        let mut completion = OfferCompletion {
                            notes: self.notes.clone(),
                            peer,
                            state: OfferState::Denied,
                        };
                        self.network.spawn(async move {
                            let state = match host
                                .send_reliable_on(
                                    peer,
                                    context.connection,
                                    ModuleMsg::Input(InputMsg::LayoutOffer(offer)),
                                )
                                .await
                            {
                                Ok(()) => OfferState::Offered,
                                Err(SendError::RemoteTooOld) => OfferState::Unsupported,
                                Err(SendError::NotConnected) => OfferState::Offline,
                                Err(_) => OfferState::Denied,
                            };
                            completion.set(state);
                        });
                        waiting += 1;
                    }
                    OfferState::Denied
                }
            };
            outcomes.push(OfferOutcome { peer, state });
        }
        if waiting > 0 {
            self.offer_job = Some(OfferJob {
                outcomes,
                reply,
                waiting,
                rejected: Default::default(),
                until: Instant::now() + crate::network::SEND_WAIT + OFFER_GRACE,
            });
        } else {
            let _ = reply.send(ModuleReply::InputOffered(outcomes));
        }
    }

    fn finish_offer(&mut self) {
        if let Some(j) = self.offer_job.take() {
            let _ = j.reply.send(ModuleReply::InputOffered(j.outcomes));
        }
    }

    /// `Zones` to a peer that speaks protocol 2 and may control me (§12.10 decision 8). The
    /// host refuses it otherwise, so nothing leaves for older peers or peers without `input_in`.
    fn announce_zones(&mut self, info: &PeerInfo) {
        if !info.connected
            || info.remote_version < LAYOUT_PROTOCOL
            || !info.perms.input_in
            || self.local_zones.is_empty()
        {
            return;
        }
        let host = self.host.clone();
        let to = info.id;
        let zones = self.local_zones.iter().map(zone_info).collect();
        let Some(context) = host.peer_input_context(to) else {
            return;
        };
        if !self.network.spawn(async move {
            host.send_reliable_on(
                to,
                context.connection,
                ModuleMsg::Input(InputMsg::Zones { zones }),
            )
            .await
        }) {
            self.last_block = Some(msg::NETWORK_BUSY.to_owned());
        }
    }

    /// Monitors changed: every eligible peer hears about it.
    fn announce_zones_to_all(&mut self) {
        for p in self.peers.values().cloned().collect::<Vec<_>>() {
            self.announce_zones(&p);
        }
    }

    fn send_ctl(&self, c: Ctl) {
        if let Some(cont) = &self.cont {
            let _ = cont.ctl.send(c);
        }
    }

    /// The links of `layout.toml` that start on this machine and lead to a peer it trusts. A peer
    /// that was revoked is no longer in `host.peers()`, so its links stop counting (the hook goes
    /// away with the last one, §13.2 rule 2a and the Sentinel's R2).
    fn derived_links(&self) -> Vec<DerivedBarrier> {
        let me = self.host.local_id();
        let trusted: Vec<PeerId> = self.host.peers().iter().map(|p| p.id).collect();
        self.layout
            .as_ref()
            .map(|l| l.barriers_for(me))
            .unwrap_or_default()
            .into_iter()
            .filter(|b| trusted.contains(&b.peer))
            .collect()
    }

    /// The barriers that exist on these zones. A link whose monitor is missing waits for it
    /// (§13.8 item 3); `report` says why in `last_block`.
    fn build_targets(&mut self, zones: &[Zone], report: bool) -> Vec<Target> {
        let organized = self
            .organization
            .document
            .as_ref()
            .is_some_and(|d| d.organization.is_some());
        let legacy_links = if organized {
            0
        } else {
            self.derived_links().len()
        };
        let targets = self.compute_targets(zones, report, organized);
        let (rotas, motivo) = if organized {
            (
                self.organization.route_links,
                self.organization.route_reason,
            )
        } else if legacy_links == 0 {
            (0, Some("no_links"))
        } else {
            (
                legacy_links,
                targets.is_empty().then_some("local_monitor_missing"),
            )
        };
        log_routes(&mut self.routes_logged, rotas, &targets, motivo);
        targets
    }

    fn compute_targets(&mut self, zones: &[Zone], report: bool, organized: bool) -> Vec<Target> {
        if organized {
            return self.organization_targets(zones);
        }

        let dead = self.layout.as_ref().map_or(0, |l| l.options.dead_corner_px);
        let mut targets = Vec::new();
        for b in self.derived_links() {
            let Some(zone) = find_zone(zones, &b.monitor) else {
                tracing::warn!(link = %b.link, "monitor of the link not found");
                if report {
                    self.last_block = Some(msg::layout_monitor_missing(&b.link, &b.monitor));
                }
                continue;
            };
            let len = match b.edge {
                Edge::Left | Edge::Right => zone.height,
                Edge::Top | Edge::Bottom => zone.width,
            };
            let Some((from, to)) = trim_dead_corner(b.from, b.to, len, dead) else {
                if report {
                    self.last_block = Some(msg::layout_segment_in_dead_corner(&b.link));
                }
                continue;
            };
            targets.push(Target {
                spec: BarrierSpec {
                    zone: zone_ref(&b.monitor),
                    edge: b.edge,
                    from,
                    to,
                },
                peer: b.peer,
                peer_monitor: Some(b.peer_monitor),
                peer_edge: b.peer_edge,
                local: (b.from, b.to),
                remote: (b.peer_from, b.peer_to),
            });
        }
        targets
    }

    /// Installs the barriers derived from `layout.toml` (contract §12.6 item 2). Nothing to do
    /// without links; the loopback owns the adapter while it runs and restarts us after. Paused
    /// with a hook-holding adapter: nothing is installed (§13.2 rule 2g).
    async fn start_continuous(&mut self) {
        if self.cont.is_some()
            || self.host.paused()
            || self.paused
            || self.authority.snapshot.stopped
        {
            return;
        }
        if self.organization.suspended
            || (self.derived_links().is_empty()
                && !self
                    .organization
                    .document
                    .as_ref()
                    .is_some_and(|d| d.organization.as_ref().is_some_and(|o| !o.links.is_empty())))
        {
            return;
        }
        self.suspended = false;
        let Some(mut cap) = self.cap.take() else {
            return;
        };
        let zones = match cap.zones().await {
            Ok(z) => z,
            Err(e) => {
                tracing::warn!("zones unavailable: {e}");
                let motivo = if matches!(e, InputError::PermissionDenied) {
                    "portal_denied"
                } else {
                    "adapter_failed"
                };
                tracing::info!(evento = "capture.failed", adaptador = cap.name(), motivo, erro = %e, "zones unavailable before the capture engine");
                self.last_block = Some(match e {
                    InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                    other => msg::adapter_failed(&other.to_string()),
                });
                self.cap = Some(cap);
                return;
            }
        };
        if self.host.paused() {
            self.cap = Some(cap);
            return;
        }
        let targets = self.build_targets(&zones, true);
        if targets.is_empty() {
            self.cap = Some(cap);
            self.retry_on_zones = true;
            return;
        }
        self.retry_on_zones = false;
        let barreiras = targets.len();
        self.targets_key = format!(
            "{}{:?}",
            targets_key(&targets),
            self.organization.catalog.snapshot()
        );
        let (ctl, ctl_rx) = mpsc::unbounded_channel();
        let engine = Engine {
            cleanup: self.organization.cleanup.clone(),
            expected_observation: self.organization.observed.clone(),
            host: self.host.clone(),
            cap,
            targets,
            ctl: ctl_rx,
            notes: self.notes.clone(),
            locked: self.locked,
            paused: self.paused,
            dead_corner_px: self.layout.as_ref().map_or(0, |l| l.options.dead_corner_px),
            peer_zones: self.peer_zones.clone(),
            catalog: self.organization.catalog.clone(),
            route_plans: self.organization.plans.clone(),
        };
        self.cont = Some(Cont {
            ctl,
            task: tokio::spawn(capture::run(engine)),
        });
        tracing::info!(
            evento = "module.input.start",
            barreiras,
            "continuous capture started"
        );
    }

    /// The zones or the trusted peers changed: swap the barriers in place when they differ, stop
    /// when no link leads to a trusted peer any more.
    async fn retarget(&mut self) {
        if self.cont.is_none() {
            return;
        }
        if self.organization.suspended
            || (self.derived_links().is_empty()
                && !self
                    .organization
                    .document
                    .as_ref()
                    .is_some_and(|d| d.organization.as_ref().is_some_and(|o| !o.links.is_empty())))
        {
            self.stop_continuous().await;
            self.suspended = true;
            return;
        }
        let zones = self.local_zones.clone();
        let targets = self.build_targets(&zones, false);
        let key = format!(
            "{}{:?}",
            targets_key(&targets),
            self.organization.catalog.snapshot()
        );
        if key != self.targets_key {
            self.targets_key = key;
            self.send_ctl(Ctl::RoutePlans(self.organization.plans.clone()));
            self.send_ctl(Ctl::Retarget(targets));
        }
    }

    /// Peers came or went: follow the trust in the links.
    async fn sync_links(&mut self) {
        if self.cont.is_some() {
            self.retarget().await;
        } else if !self.organization.suspended
            && (self.suspended && !self.derived_links().is_empty()
                || self
                    .organization
                    .document
                    .as_ref()
                    .is_some_and(|d| d.organization.is_some()))
        {
            self.start_continuous().await;
        }
    }

    /// Closes the open session and takes the adapter back.
    async fn stop_continuous(&mut self) {
        let Some(c) = self.cont.as_mut() else { return };
        let _ = c.ctl.send(Ctl::Stop);
        match (&mut c.task).await {
            Ok(cap) => self.cap = Some(cap),
            Err(_) => self.last_block = Some(msg::adapter_failed("captura")),
        }
        self.cont = None;
        tracing::info!(evento = "module.input.stop", "continuous capture stopped");
    }

    /// Barriers of a loopback toward `to`: the links of `layout.toml`, else the manual
    /// `[[input.barrier]]` of that peer, else the upper half of the right edge of zone 0.
    fn loop_barriers(&self, to: PeerId, peer_name: &str) -> Vec<LoopBarrier> {
        let me = self.host.local_id();
        let mut out: Vec<LoopBarrier> = self
            .layout
            .iter()
            .flat_map(|l| l.barriers_for(me))
            .filter(|b| b.peer == to)
            .map(|b| LoopBarrier {
                spec: BarrierSpec {
                    zone: zone_ref(&b.monitor),
                    edge: b.edge,
                    from: b.from,
                    to: b.to,
                },
                peer_edge: b.peer_edge,
                local: (b.from, b.to),
                remote: (b.peer_from, b.peer_to),
            })
            .collect();
        if out.is_empty() {
            out = self
                .cfg
                .barriers
                .iter()
                .filter(|b| b.peer == peer_name)
                .map(|b| LoopBarrier {
                    spec: BarrierSpec {
                        zone: zone_ref(&b.monitor),
                        edge: b.edge,
                        from: b.from,
                        to: b.to,
                    },
                    peer_edge: b.peer_edge,
                    local: (b.from, b.to),
                    remote: b.remote_segment(),
                })
                .collect();
        }
        if out.is_empty() {
            out.push(LoopBarrier {
                spec: BarrierSpec {
                    zone: ZoneRef::Index(0),
                    edge: Edge::Right,
                    from: 0.0,
                    to: 0.5,
                },
                peer_edge: Edge::Left,
                local: (0.0, 0.5),
                remote: (0.0, 0.5),
            });
        }
        out
    }

    async fn command(
        &mut self,
        cmd: ModuleCommand,
        reply: oneshot::Sender<ModuleReply>,
        task: &mut Option<JoinHandle<LoopDone>>,
    ) -> Option<JoinHandle<LoopDone>> {
        match cmd {
            ModuleCommand::InputPrepareInventory => {
                self.begin_preparation(Some(reply)).await;
                None
            }
            ModuleCommand::InputPreviewOrganization(r) => {
                let result = self.preview_organization(r).await;
                let _ = reply.send(match result {
                    Ok(v) => ModuleReply::OrganizationPreview(v),
                    Err(e) => ModuleReply::OrganizationFailure(e),
                });
                None
            }
            ModuleCommand::InputApplyOrganization(r) => {
                let result = self.apply_organization(r).await;
                let _ = reply.send(match result {
                    Ok(v) => ModuleReply::OrganizationApplied(v),
                    Err(e) => ModuleReply::OrganizationFailure(e),
                });
                None
            }
            ModuleCommand::InputSuspendOrganization => {
                let result = self.quiesce_organization().await;
                let _ = reply.send(match result {
                    Ok(()) => ModuleReply::Done,
                    Err(e) => ModuleReply::OrganizationFailure(e),
                });
                None
            }
            ModuleCommand::InputRestoreLayoutV1 { base_rev } => {
                let result = self.restore_layout_v1(base_rev).await;
                let _ = reply.send(match result {
                    Ok(v) => ModuleReply::OrganizationApplied(v),
                    Err(e) => ModuleReply::OrganizationFailure(e),
                });
                None
            }
            ModuleCommand::InputOfferV4 { to } => {
                self.offer_v4(to, reply).await;
                None
            }
            ModuleCommand::InputAnswerOfferV4 {
                from,
                connection,
                rev,
                base_rev,
                accept,
            } => {
                let result = self
                    .answer_offer_v4(from, connection, rev, base_rev, accept)
                    .await;
                let _ = reply.send(match result {
                    Ok(Some(v)) => ModuleReply::OrganizationApplied(v),
                    Ok(None) => ModuleReply::Done,
                    Err(e) => ModuleReply::OrganizationFailure(e),
                });
                None
            }
            ModuleCommand::Status => {
                let _ = reply.send(ModuleReply::Status(self.status()));
                None
            }
            ModuleCommand::InputLoopback { to, seconds } => {
                if self.host.paused() {
                    let _ = reply.send(ModuleReply::Error(RejectReason::Paused.pt().to_owned()));
                    return None;
                }
                if task.is_some() {
                    let _ = reply.send(ModuleReply::Error(msg::LOOPBACK_BUSY.into()));
                    return None;
                }
                let Some(peer) = self.peers.get(&to).cloned() else {
                    let _ = reply.send(ModuleReply::Error(msg::PEER_UNKNOWN.into()));
                    return None;
                };
                if !peer.connected {
                    let _ = reply.send(ModuleReply::Error(msg::PEER_OFFLINE.into()));
                    return None;
                }
                // The continuous capture owns the adapter: pause it for the test.
                if self.cap.is_none() {
                    self.stop_continuous().await;
                }
                let Some(cap) = self.cap.take() else {
                    let _ = reply.send(ModuleReply::Error(msg::LOOPBACK_BUSY.into()));
                    return None;
                };
                let session = Arc::new(AtomicU32::new(0));
                let (tx, sig_rx) = mpsc::unbounded_channel();
                self.ctl = Some(LoopCtl {
                    to,
                    session: session.clone(),
                    tx,
                });
                let job = LoopJob {
                    host: self.host.clone(),
                    cap,
                    barriers: self.loop_barriers(to, &peer.name),
                    to,
                    seconds: seconds.clamp(1, 300),
                    session,
                    signals: sig_rx,
                    reply,
                };
                Some(tokio::spawn(run_loopback(job)))
            }
            ModuleCommand::InputState => {
                let _ = reply.send(ModuleReply::InputState(self.input_state()));
                None
            }
            ModuleCommand::InputReload => {
                let r = match self.reload_layout().await {
                    Ok(()) => ModuleReply::Done,
                    Err(e) => ModuleReply::Error(e),
                };
                let _ = reply.send(r);
                None
            }
            ModuleCommand::InputLock { locked } => {
                self.set_locked(locked);
                let _ = reply.send(ModuleReply::Done);
                None
            }
            ModuleCommand::InputSwitch { to } => {
                self.switch(to, reply);
                None
            }
            ModuleCommand::InputOffer { to } => {
                self.offer(to, reply).await;
                None
            }
            ModuleCommand::InputOfferClear => {
                self.pending_offer = None;
                let _ = reply.send(ModuleReply::Done);
                None
            }
            _ => {
                let _ = reply.send(ModuleReply::Error(msg::UNSUPPORTED_COMMAND.into()));
                None
            }
        }
    }

    fn status(&self) -> ModuleStatus {
        let mut authorized: Vec<String> = self
            .peers
            .values()
            .filter(|p| p.perms.input_in)
            .map(|p| p.name.clone())
            .collect();
        authorized.sort();
        let mut block = self.last_block.clone();
        if block.is_none() && self.ignored > 0 {
            block = Some(msg::ignored_count(self.ignored));
        }
        if block.is_none() && self.rate_dropped > 0 {
            block = Some(format!(
                "{} datagrama(s) descartado(s) por limite de taxa",
                self.rate_dropped
            ));
        }
        ModuleStatus {
            kind: ModuleKind::Input,
            enabled: true,
            adapter: Some(format!("{}+{}", self.cap_name, self.emu_name)),
            authorized_peers: authorized,
            last_event: self.last_event.clone(),
            last_block: block,
        }
    }

    async fn handle(&mut self, input: ModuleInput) {
        match input {
            ModuleInput::Reliable { from, msg, context } => {
                if let ModuleMsg::Input(m) = msg {
                    self.on_msg(from, m, context).await;
                }
            }
            ModuleInput::Datagram { from, dg, context } => {
                self.on_datagram(from, dg, context).await
            }
            ModuleInput::Rejected { from, reason, .. } => {
                self.last_block = Some(reason.pt().to_owned());
                if let Some(c) = self.ctl.as_ref().filter(|c| c.to == from) {
                    let _ = c.tx.send(LoopSignal::Rejected(reason));
                }
                self.send_ctl(Ctl::Rejected { from, reason });
                if let Some(j) = &mut self.offer_job {
                    j.rejected.insert(from);
                    for o in j.outcomes.iter_mut().filter(|o| o.peer == from) {
                        if o.state == OfferState::Offered {
                            o.state = OfferState::Denied;
                        }
                    }
                }
            }
            ModuleInput::PeerUp(info) => {
                self.reconcile_incoming(info.id).await;
                self.announce_zones(&info);
                self.announce_inventory(&info);
                self.peers.insert(info.id, info);
                self.sync_links().await;
            }
            ModuleInput::PeerDown(id) => {
                self.cancel_preparation().await;
                self.organization.catalog.invalidate(id);
                self.peers.remove(&id);
                self.buckets.remove(&id);
                self.frontiers.remove(&id);
                if self.incoming.remove(&id).is_some_and(|i| i.active) {
                    self.release_all().await;
                }
                if let Some(c) = self.ctl.as_ref().filter(|c| c.to == id) {
                    let _ = c.tx.send(LoopSignal::PeerDown);
                }
                self.send_ctl(Ctl::PeerDown(id));
                // A revoked peer is gone from `host.peers()`: its links stop counting.
                self.sync_links().await;
            }
            ModuleInput::Paused(p) => {
                if self.authority.observed() {
                    self.reconcile_host().await;
                } else {
                    self.apply_pause(p).await;
                }
            }
            ModuleInput::Command { .. } | ModuleInput::Shutdown => {}
        }
    }

    async fn on_msg(&mut self, from: PeerId, m: InputMsg, context: ReceiveContext) {
        if self
            .host
            .peer_input_context(from)
            .is_none_or(|c| !c.connected || c.connection != context.connection)
        {
            self.ignored += 1;
            return;
        }
        match m {
            InputMsg::InventoryV4(inv) => self.receive_inventory(from, context, inv).await,
            InputMsg::EnterRoutes(entry) => self.enter_routes(from, entry, context).await,
            InputMsg::ReturnedRoute(returned) => self.send_ctl(Ctl::ReturnedRoute {
                from,
                context,
                returned,
            }),
            InputMsg::LayoutOfferV4(offer) => {
                self.organization.pending = Some(clipsync_proto::PendingMonitorOffer {
                    from,
                    connection: context.connection,
                    offer,
                });
            }
            InputMsg::Enter { session, edge, pos } => {
                tracing::info!(evento = "enter.received", session, peer = %from, mensagem = "enter", "entry received");
                self.on_enter(
                    from,
                    session,
                    Entry {
                        edge,
                        monitor: None,
                        pos,
                        back: None,
                    },
                    context,
                )
                .await
            }
            InputMsg::EnterAt {
                session,
                edge,
                monitor,
                pos,
            } => {
                tracing::info!(evento = "enter.received", session, peer = %from, mensagem = "enter_at", "entry received");
                self.on_enter(
                    from,
                    session,
                    Entry {
                        edge,
                        monitor: monitor.as_deref(),
                        pos,
                        back: None,
                    },
                    context,
                )
                .await
            }
            InputMsg::EnterSpan {
                session,
                edge,
                monitor,
                pos,
                back_from,
                back_to,
            } => {
                tracing::info!(evento = "enter.received", session, peer = %from, mensagem = "enter_span", "entry received");
                self.on_enter(
                    from,
                    session,
                    Entry {
                        edge,
                        monitor: monitor.as_deref(),
                        pos,
                        back: Some((back_from, back_to)),
                    },
                    context,
                )
                .await
            }
            // Only for the session this machine opened toward that peer (the capture checks it).
            InputMsg::Returned { session, pos } => match &self.cont {
                Some(c) => {
                    let _ = c.ctl.send(Ctl::Returned { from, session, pos });
                }
                None => self.ignored += 1,
            },
            InputMsg::Refused { session, reason } => {
                if let Some(c) = self
                    .ctl
                    .as_ref()
                    .filter(|c| c.to == from && c.session.load(Ordering::SeqCst) == session)
                {
                    let _ = c.tx.send(LoopSignal::Rejected(reason));
                } else if let Some(c) = &self.cont {
                    let _ = c.ctl.send(Ctl::Refused {
                        from,
                        session,
                        reason,
                    });
                } else {
                    self.ignored += 1;
                }
            }
            InputMsg::Zones { zones } => {
                self.send_ctl(Ctl::PeerZones(from, zones.clone()));
                self.peer_zones.insert(from, zones);
                let name = self.peer_name(from);
                self.last_event = Some(msg::zones_from(&name));
            }
            InputMsg::LayoutOffer(offer) => {
                let name = self.peer_name(from);
                self.pending_offer = Some(PendingOffer { from, offer });
                self.last_event = Some(msg::offer_from(&name));
            }
            InputMsg::Leave { session } => {
                if let Some(index) = context.stream {
                    let f = self.frontier(from, context.connection);
                    f.closed = Some(f.closed.map_or(index, |old| old.max(index)));
                }
                if let Some(i) = self.incoming.get_mut(&from).filter(|i| {
                    i.session == session
                        && i.context.connection == context.connection
                        && i.context.stream < context.stream
                }) {
                    i.active = false;
                    self.release_all().await;
                    let name = self.peer_name(from);
                    self.last_event = Some(msg::session_closed_by(&name));
                } else {
                    self.ignored += 1;
                }
            }
            InputMsg::ReleaseAll { session } => {
                if self
                    .incoming
                    .get(&from)
                    .is_some_and(|i| i.session == session)
                {
                    self.release_all().await;
                } else {
                    self.ignored += 1;
                }
            }
            InputMsg::StatsRequest { session } => {
                match self.incoming.get(&from).filter(|i| i.session == session) {
                    Some(i) => {
                        let stats = i.stats();
                        let host = self.host.clone();
                        if !self.network.spawn(async move {
                            host.send_reliable_on(
                                from,
                                context.connection,
                                ModuleMsg::Input(InputMsg::Stats(stats)),
                            )
                            .await
                        }) {
                            self.last_block = Some(msg::NETWORK_BUSY.to_owned());
                        }
                    }
                    None => self.ignored += 1,
                }
            }
            InputMsg::Stats(s) => match &self.ctl {
                Some(c)
                    if c.to == from
                        && s.session != 0
                        && s.session == c.session.load(Ordering::SeqCst) =>
                {
                    let _ = c.tx.send(LoopSignal::Stats(s));
                }
                _ => self.ignored += 1,
            },
        }
    }

    fn peer_name(&self, id: PeerId) -> String {
        self.peers
            .get(&id)
            .map_or_else(|| id.short(), |p| p.name.clone())
    }

    /// Protocol 3 message to `to`; kept for the backpressure harness, which drives the queue
    /// directly. Production paths log through `send_return` and `refuse_on_with`.
    #[cfg(test)]
    fn send_v3(&mut self, to: PeerId, connection: u64, m: InputMsg) {
        if !self
            .host
            .peers()
            .iter()
            .any(|p| p.id == to && p.remote_version >= SPAN_PROTOCOL)
        {
            return;
        }
        let host = self.host.clone();
        if !self.network.spawn(async move {
            host.send_reliable_on(to, connection, ModuleMsg::Input(m))
                .await
        }) {
            self.last_block = Some(msg::NETWORK_BUSY.into());
        }
    }

    /// Sends the way back and logs `return.sent` only after the send succeeds; a failure, a
    /// full queue or the send budget running out is `return.failed` with the reason (§17.2).
    fn send_return(&mut self, to: PeerId, connection: u64, m: InputMsg, session: u32, pos: f32) {
        let (recebidos, perdidos, fora_de_ordem) = self
            .incoming
            .get(&to)
            .map_or((0, 0, 0), |i| (i.received, i.lost, i.out_of_order));
        let totais = format!(
            "{{\"recebidos\":{recebidos},\"perdidos\":{perdidos},\"fora_de_ordem\":{fora_de_ordem}}}"
        );
        let pos = capture::frac3(pos);
        if !self
            .host
            .peers()
            .iter()
            .any(|p| p.id == to && p.remote_version >= SPAN_PROTOCOL)
        {
            tracing::info!(evento = "return.failed", session, peer = %to, erro = "peer_too_old", "way back not sent");
            return;
        }
        if !self.network.has_capacity() {
            tracing::info!(evento = "return.failed", session, peer = %to, erro = "network_busy", "way back not queued");
            self.last_block = Some(msg::NETWORK_BUSY.into());
            return;
        }
        let host = self.host.clone();
        // The guard exists from admission: a job aborted before its first poll, or cancelled by
        // the send budget, still leaves its line in the log.
        let pendente = Pendente(Some(move || {
            tracing::info!(evento = "return.failed", session, peer = %to, erro = "timeout", "way back cancelled before completion");
        }));
        let _ = self.network.spawn(async move {
            let mut pendente = pendente;
            let sent = host
                .send_reliable_on(to, connection, ModuleMsg::Input(m))
                .await;
            pendente.0.take();
            match &sent {
                Ok(()) => tracing::info!(evento = "return.sent", session, peer = %to, pos, totais = %totais, "cursor returned to the source"),
                Err(e) => tracing::info!(evento = "return.failed", session, peer = %to, erro = capture::send_error_code(e), "way back not sent"),
            }
            sent
        });
    }

    /// `Enter`, `EnterAt` and `EnterSpan` (`back` is the way-back span of the last one).
    async fn on_enter(
        &mut self,
        from: PeerId,
        session: u32,
        entry: Entry<'_>,
        context: ReceiveContext,
    ) {
        let Entry {
            edge,
            monitor,
            pos,
            back,
        } = entry;
        if session == 0 {
            self.ignored += 1;
            return;
        }
        if !self.authorized(from, context) {
            self.ignored += 1;
            let reason = if self.paused || self.host.paused() {
                RejectReason::Paused
            } else {
                RejectReason::NotAuthorized
            };
            self.refuse_on(from, session, context.connection, reason);
            return;
        }
        let Some(index) = context.stream else {
            self.ignored += 1;
            return;
        };
        let frontier = self.frontier(from, context.connection);
        if frontier.closed.is_some_and(|n| index <= n)
            || frontier.entered.is_some_and(|n| index <= n)
        {
            self.ignored += 1;
            return;
        }
        frontier.entered = Some(index);
        if self.incoming.get(&from).is_some_and(|i| i.active) {
            self.release_all().await;
        }
        if !self.authorized(from, context) {
            self.release_all().await;
            self.refuse_on(
                from,
                session,
                context.connection,
                RejectReason::NotAuthorized,
            );
            return;
        }
        if self.emu_started && !self.emu.alive() {
            tracing::warn!("emulator ended by the platform, starting it again");
            let _ = self.emu.stop().await;
            self.emu_started = false;
        }
        tracing::info!(evento = "emulate.start", session, peer = %from, adaptador = self.emu.name(), reutilizado = self.emu_started, "emulator in use");
        if !self.emu_started {
            match self.emu.start().await {
                Ok(()) => self.emu_started = true,
                Err(e) => {
                    tracing::warn!("emulator start failed: {e}");
                    tracing::info!(evento = "emulate.error", session, peer = %from, adaptador = self.emu.name(), erro = %e, "emulator did not start");
                    self.last_block = Some(match e {
                        InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                        _ => msg::EMULATOR_FAILED.to_owned(),
                    });
                    self.refuse_on_with(
                        from,
                        session,
                        context.connection,
                        RejectReason::ApplyFailed,
                        "emulator_start_failed",
                    );
                    return;
                }
            }
        }
        if !self.authorized(from, context) {
            self.release_all().await;
            self.refuse_on(
                from,
                session,
                context.connection,
                RejectReason::NotAuthorized,
            );
            return;
        }
        self.incoming.remove(&from);
        let zone = monitor.and_then(|m| find_zone(&self.local_zones, m));
        let placed = match zone {
            Some(z) => self.emu.enter(z, &self.local_zones, edge, pos).await,
            None => self.emu.emit(&entry_events(edge, pos)).await,
        };
        match placed {
            Err(InputError::Locked) => {
                tracing::info!(evento = "emulate.error", session, peer = %from, adaptador = self.emu.name(), erro = "locked", "entry refused by the lock");
                self.incoming.remove(&from);
                self.release_all().await;
                self.refuse_on(from, session, context.connection, RejectReason::Locked);
                return;
            }
            Err(e) => {
                tracing::warn!("entry positioning failed: {e}");
                tracing::info!(evento = "emulate.error", session, peer = %from, adaptador = self.emu.name(), erro = %e, "entry positioning failed");
            }
            Ok(()) => {
                tracing::info!(evento = "emulate.positioned", session, peer = %from, adaptador = self.emu.name(), pos = capture::frac3(pos), "entry positioned");
            }
        }
        if !self.authorized(from, context) {
            self.release_all().await;
            self.refuse_on(
                from,
                session,
                context.connection,
                RejectReason::NotAuthorized,
            );
            return;
        }
        self.incoming.insert(from, Incoming::new(session, context));
        if let Some(span) = back {
            let entry_zone = zone
                .or_else(|| self.local_zones.first())
                .cloned()
                .unwrap_or_else(reference_zone);
            if let Some(i) = self.incoming.get_mut(&from) {
                i.span = Some(SpanTrack::new(entry_zone, edge, pos, span));
            }
        }
        let name = self.peer_name(from);
        self.last_event = Some(msg::control_started(&name));
    }

    fn frontier(&mut self, peer: PeerId, connection: u64) -> &mut StreamFrontier {
        let f = self.frontiers.entry(peer).or_default();
        if f.connection != connection {
            *f = StreamFrontier {
                connection,
                ..Default::default()
            };
        }
        f
    }
    fn authorized(&self, from: PeerId, context: ReceiveContext) -> bool {
        self.organization.preparation.is_none()
            && !self.paused
            && !self.host.paused()
            && self
                .host
                .peer_input_context(from)
                .is_some_and(|c| c.permits(context))
    }
    async fn reconcile_incoming(&mut self, from: PeerId) {
        let invalid = self
            .incoming
            .get(&from)
            .is_some_and(|i| !self.authorized(from, i.context));
        if invalid && let Some(i) = self.incoming.remove(&from) {
            if i.active {
                self.release_all().await;
            }
            self.refuse_on(
                from,
                i.session,
                i.context.connection,
                RejectReason::NotAuthorized,
            );
        }
    }
    fn refuse_on(&mut self, to: PeerId, session: u32, connection: u64, reason: RejectReason) {
        self.refuse_on_with(to, session, connection, reason, reject_code(reason));
    }

    /// Same `Rejected`/`Refused` on the wire, with a finer `motivo` in the log (contract §17.2).
    fn refuse_on_with(
        &mut self,
        to: PeerId,
        session: u32,
        connection: u64,
        reason: RejectReason,
        motivo: &'static str,
    ) {
        let host = self.host.clone();
        let v3 = host
            .peers()
            .iter()
            .any(|p| p.id == to && p.remote_version >= SPAN_PROTOCOL);
        // One `enter.refused` per refusal, emitted with the real state of the notice: after the
        // send, at once when it could not be queued, or by the guard when the job is cancelled
        // (before its first poll or by the send budget).
        if !self.network.has_capacity() {
            tracing::info!(evento = "enter.refused", session, peer = %to, motivo, enviada = false, erro = "network_busy", "entry refused, notice not queued");
            self.last_block = Some(msg::NETWORK_BUSY.into());
            return;
        }
        let pendente = Pendente(Some(move || {
            tracing::info!(evento = "enter.refused", session, peer = %to, motivo, enviada = false, erro = "timeout", "entry refused, notice cancelled before completion");
        }));
        let _ = self.network.spawn(async move {
            let mut pendente = pendente;
            let sent = if v3 {
                host.send_reliable_on(
                    to,
                    connection,
                    ModuleMsg::Input(InputMsg::Refused { session, reason }),
                )
                .await
            } else {
                host.reject_input_on(to, connection, reason).await
            };
            pendente.0.take();
            match &sent {
                Ok(()) => tracing::info!(evento = "enter.refused", session, peer = %to, motivo, enviada = true, "entry refused"),
                Err(e) => tracing::info!(evento = "enter.refused", session, peer = %to, motivo, enviada = false, erro = capture::send_error_code(e), "entry refused, notice not sent"),
            }
            sent
        });
    }

    async fn on_datagram(&mut self, from: PeerId, dg: InputDatagram, context: ReceiveContext) {
        self.reconcile_incoming(from).await;
        if !self.authorized(from, context) {
            self.ignored += 1;
            return;
        }
        if let Some(rev) = self
            .incoming
            .get(&from)
            .and_then(|i| i.routes.as_ref())
            .map(|(_, r)| r.target)
            && (!self.confirm_local_inventory(rev).await || !self.authorized(from, context))
        {
            self.incoming.remove(&from);
            self.release_all().await;
            return;
        }
        let Some(inc) = self.incoming.get_mut(&from) else {
            self.ignored += 1;
            return;
        };
        if !inc.active
            || inc.session != dg.session
            || self.paused
            || inc.context.connection != context.connection
            || inc.context.input_epoch != context.input_epoch
        {
            self.ignored += 1;
            return;
        }
        // Per peer, not per session: a new Enter must not refill the budget.
        if !self.buckets.entry(from).or_default().take(Instant::now()) {
            self.rate_dropped += 1;
            return;
        }
        match inc.last_seq {
            None => {
                inc.last_seq = Some(dg.seq);
                tracing::info!(evento = "datagram.first", session = dg.session, peer = %from, "first datagram received");
            }
            Some(last) if dg.seq > last => {
                inc.lost += u64::from(dg.seq - last - 1);
                inc.last_seq = Some(dg.seq);
            }
            // An exact duplicate of the last datagram fills no gap; an older one is assumed to.
            Some(last) if dg.seq == last => inc.out_of_order += 1,
            Some(_) => {
                inc.out_of_order += 1;
                inc.lost = inc.lost.saturating_sub(1);
            }
        }
        inc.record_latency(mono_us().saturating_sub(dg.t_mono_us));
        inc.received += dg.events.len() as u64;
        let session = dg.session;
        let mut events = dg.events;
        // The way back: the destination follows the cursor and takes the push out of the motion.
        let mut returned_route = None;
        if let Some((track, revisions)) = inc.routes.as_mut() {
            if let Some((x, y)) = self.emu.cursor() {
                track.sync(x, y);
            }
            returned_route =
                track
                    .step(&mut events)
                    .map(|(route, pos)| clipsync_proto::ReturnedRoute {
                        session,
                        inventories: *revisions,
                        route,
                        pos,
                    });
        }
        let v4 = inc.routes.is_some();
        let mut returned = None;
        if let Some(track) = inc.span.as_mut() {
            if let Some((x, y)) = self.emu.cursor() {
                track.sync(x, y);
            }
            returned = track.step(&self.local_zones, &mut events);
        }
        if self.emu_started
            && !events.is_empty()
            && let Err(e) = self.emu.emit(&events).await
        {
            tracing::info!(evento = "emulate.error", session, peer = %from, adaptador = self.emu.name(), erro = %e, "emulation failed after entry");
            if v4 || matches!(e, InputError::Locked) {
                // The screen locked in the middle of the session: the source gets the cursor back.
                self.incoming.remove(&from);
                self.release_all().await;
                self.refuse_on(from, session, context.connection, RejectReason::Locked);
                return;
            }
            tracing::warn!("emit failed: {e}");
        }
        if !self.authorized(from, context) {
            self.reconcile_incoming(from).await;
            return;
        }
        if v4 {
            let rev = self
                .incoming
                .get(&from)
                .and_then(|i| i.routes.as_ref())
                .map(|(_, r)| r.target);
            if !self.confirm_local_inventory(rev.unwrap_or(0)).await
                || !self.authorized(from, context)
            {
                self.incoming.remove(&from);
                self.release_all().await;
                return;
            }
        }
        if let Some(returned) = returned_route {
            let pos = returned.pos;
            if let Some(i) = self.incoming.get_mut(&from) {
                i.active = false;
            }
            self.release_all().await;
            self.send_return(
                from,
                context.connection,
                InputMsg::ReturnedRoute(returned),
                session,
                pos,
            );
        }
        if let Some(pos) = returned {
            if let Some(i) = self.incoming.get_mut(&from) {
                i.active = false;
            }
            self.release_all().await;
            self.send_return(
                from,
                context.connection,
                InputMsg::Returned { session, pos },
                session,
                pos,
            );
            let name = self.peer_name(from);
            self.last_event = Some(msg::returned_from(&name));
        }
    }

    async fn release_all(&mut self) {
        if self.emu_started
            && let Err(e) = self.emu.release_all().await
        {
            tracing::warn!("release_all failed: {e}");
        }
    }
}

struct LoopJob {
    host: Arc<dyn ModuleHost>,
    cap: Box<dyn CaptureAdapter>,
    barriers: Vec<LoopBarrier>,
    to: PeerId,
    seconds: u32,
    session: Arc<AtomicU32>,
    signals: mpsc::UnboundedReceiver<LoopSignal>,
    reply: oneshot::Sender<ModuleReply>,
}

/// What the way back is measured on when the destination knows no monitor of its own: 1080p.
fn reference_zone() -> Zone {
    Zone {
        index: 0,
        name: None,
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}

/// `"#0"` is an index; anything else a connector name.
fn find_zone<'a>(zones: &'a [Zone], monitor: &str) -> Option<&'a Zone> {
    match zone_ref(monitor) {
        ZoneRef::Index(i) => zones.iter().find(|z| z.index == i),
        ZoneRef::Name(n) => zones.iter().find(|z| z.name.as_deref() == Some(n.as_str())),
    }
}

fn zone_info(z: &Zone) -> ZoneInfo {
    ZoneInfo {
        index: z.index,
        name: z.name.clone(),
        x: z.x,
        y: z.y,
        width: z.width,
        height: z.height,
        scale: z.scale,
    }
}

fn offer_endpoint(e: &Endpoint) -> OfferEndpoint {
    OfferEndpoint {
        peer: e.peer,
        monitor: e.monitor.clone(),
        edge: e.edge,
        from: e.from,
        to: e.to,
    }
}

/// The wire form of `layout.toml` for "copy the layout to the peers".
fn layout_offer(l: &Layout) -> LayoutOffer {
    LayoutOffer {
        rev: l.rev,
        devices: l
            .devices
            .iter()
            .map(|d| OfferDevice {
                peer: d.peer,
                x: d.x,
                y: d.y,
            })
            .collect(),
        links: l
            .links
            .iter()
            .map(|k| OfferLink {
                a: offer_endpoint(&k.a),
                b: offer_endpoint(&k.b),
            })
            .collect(),
    }
}

async fn run_loopback(job: LoopJob) -> LoopDone {
    let LoopJob {
        host,
        mut cap,
        barriers,
        to,
        seconds,
        session,
        mut signals,
        reply,
    } = job;
    let outcome = drive(
        &host,
        cap.as_mut(),
        &barriers,
        to,
        seconds,
        &session,
        &mut signals,
    )
    .await;
    let _ = cap.stop().await;
    LoopDone {
        cap,
        outcome,
        reply,
    }
}

async fn drive(
    host: &Arc<dyn ModuleHost>,
    cap: &mut dyn CaptureAdapter,
    barriers: &[LoopBarrier],
    to: PeerId,
    seconds: u32,
    session_cell: &AtomicU32,
    signals: &mut mpsc::UnboundedReceiver<LoopSignal>,
) -> Result<LoopbackStats, String> {
    let mut authority = capture::HostCursor::new(host.as_ref());
    authority.refresh();
    if host.paused() || authority.snapshot.paused || authority.snapshot.stopped {
        return Err(RejectReason::Paused.pt().to_owned());
    }
    let mut lifecycle = LifecycleCursor::new(cap);
    let specs: Vec<BarrierSpec> = barriers.iter().map(|b| b.spec.clone()).collect();
    let attempt = capture::next_attempt();
    tracing::info!(evento = "attempt.begin", attempt, origem = "test", peer = %to, "attempt begins");
    let mut ev_rx = cap.start(&specs).await.map_err(|e| {
        let (motivo, text) = match e {
            InputError::PermissionDenied => ("portal_denied", msg::PORTAL_DENIED.to_owned()),
            other => ("adapter_failed", msg::adapter_failed(&other.to_string())),
        };
        tracing::info!(
            evento = "attempt.filtered",
            attempt,
            motivo,
            "attempt filtered"
        );
        text
    })?;
    let deadline = Instant::now() + Duration::from_secs(u64::from(seconds));
    let context = host
        .peer_input_context(to)
        .ok_or_else(|| msg::PEER_OFFLINE.to_owned())?;
    let mut session = 0;
    let mut seq = 0;
    let mut active = false;
    lifecycle.refresh();
    let mut snapshot_ready = lifecycle
        .snapshot
        .as_ref()
        .is_some_and(|s| s.allows_capture());
    let mut release_all = false;
    let mut opening: Option<crate::network::Sending> = None;
    let mut queue = VecDeque::new();
    let mut failure = None;
    // Why the loopback session ended, for `session.closed` (contract §17.2); `return` is the
    // normal end at the deadline with the session open.
    let mut close = "return";
    let mut sent: u64 = 0;
    let mut dropped: u64 = 0;
    loop {
        let (_, invalidated) = authority.refresh();
        if invalidated || authority.snapshot.paused || authority.snapshot.stopped {
            release_all = true;
            close = if authority.snapshot.stopped {
                "stop"
            } else {
                "pause"
            };
            failure = Some(RejectReason::Paused.pt().to_owned());
            break;
        }
        if host
            .peer_input_context(to)
            .is_none_or(|c| !c.connected || c.connection != context.connection)
        {
            release_all = true;
            close = "peer_down";
            failure = Some(msg::PEER_OFFLINE.to_owned());
            break;
        }
        lifecycle.refresh();
        if lifecycle.pending_invalidation
            || lifecycle
                .snapshot
                .as_ref()
                .is_some_and(|s| !s.allows_capture())
        {
            release_all = true;
            // A terminal adapter error outranks the lock classification (contract §17.2).
            match lifecycle.snapshot.as_ref().and_then(|s| s.terminal_error) {
                Some(error) => {
                    close = "error";
                    failure = Some(msg::adapter_failed(error));
                }
                None => {
                    close = "lock";
                    failure = Some(msg::SOURCE_SCREEN_LOCKED.to_owned());
                }
            }
            break;
        }
        let mut replay = false;
        let ev = tokio::select! {
            biased;
            s = signals.recv() => {
                failure = match s {
                    Some(LoopSignal::Stats(_)) => continue,
                    Some(LoopSignal::Rejected(r)) => {
                        if session != 0 {
                            tracing::info!(evento = "session.refused", session, peer = %to, motivo = capture::reject_code(r), "entry refused by the peer");
                        }
                        close = "refused";
                        Some(r.pt().to_owned())
                    }
                    Some(LoopSignal::PeerDown) => { close = "peer_down"; Some(msg::PEER_OFFLINE.to_owned()) }
                    Some(LoopSignal::Paused) => { close = "pause"; Some(RejectReason::Paused.pt().to_owned()) }
                    Some(LoopSignal::Locked) => { close = "lock"; Some(msg::LOCKED.to_owned()) }
                    Some(LoopSignal::Stop) | None => { close = "stop"; Some(msg::SESSION_CANCELLED.to_owned()) }
                };
                break;
            }
            () = authority.changed() => continue,
            () = lifecycle.changed() => continue,
            ev = ev_rx.recv() => match ev { Some(ev) => ev, None => { close = "error"; failure = Some(msg::SESSION_CANCELLED.to_owned()); break; } },
            result = async { match opening.as_mut() { Some(f) => f.await, None => Ok(()) } }, if opening.is_some() => {
                let (_, invalidated) = authority.refresh();
                if invalidated || authority.snapshot.paused || authority.snapshot.stopped {
                    release_all = true;
                    close = if authority.snapshot.stopped { "stop" } else { "pause" };
                    failure = Some(RejectReason::Paused.pt().to_owned());
                    break;
                }
                lifecycle.refresh();
                if lifecycle.pending_invalidation || lifecycle.snapshot.as_ref().is_some_and(|s| !s.allows_capture()) {
                    release_all = true;
                    match lifecycle.snapshot.as_ref().and_then(|s| s.terminal_error) {
                        Some(error) => {
                            close = "error";
                            failure = Some(msg::adapter_failed(error));
                        }
                        None => {
                            close = "lock";
                            failure = Some(msg::SOURCE_SCREEN_LOCKED.to_owned());
                        }
                    }
                    break;
                }
                opening = None;
                match result {
                    Ok(()) => {
                        tracing::info!(evento = "session.sent", attempt, session, peer = %to, "entry sent");
                        tracing::info!(evento = "session.active", attempt, session, peer = %to, "session active");
                        active = true
                    }
                    Err(e) => {
                        close = "error";
                        failure = Some(e.to_string());
                        break;
                    }
                }
                continue;
            }
            _ = std::future::ready(()), if active && !queue.is_empty() => { if let Some(ev) = queue.pop_front() { replay = true; ev } else { continue } },
            () = tokio::time::sleep_until(deadline) => break,
        };
        let (_, invalidated) = authority.refresh();
        if invalidated || authority.snapshot.paused || authority.snapshot.stopped {
            release_all = true;
            close = if authority.snapshot.stopped {
                "stop"
            } else {
                "pause"
            };
            failure = Some(RejectReason::Paused.pt().to_owned());
            break;
        }
        lifecycle.refresh();
        if lifecycle.pending_invalidation
            || lifecycle
                .snapshot
                .as_ref()
                .is_some_and(|s| !s.allows_capture())
        {
            release_all = true;
            match lifecycle.snapshot.as_ref().and_then(|s| s.terminal_error) {
                Some(error) => {
                    close = "error";
                    failure = Some(msg::adapter_failed(error));
                }
                None => {
                    close = "lock";
                    failure = Some(msg::SOURCE_SCREEN_LOCKED.to_owned());
                }
            }
            break;
        }
        match ev {
            CaptureEvent::Activated { .. } if !snapshot_ready => continue,
            CaptureEvent::Activated { barrier, pos, .. } => {
                tracing::info!(evento = "barrier.activated", attempt, barreira = barrier, peer = %to, pos = capture::frac3(pos), "barrier activated");
                if session != 0 {
                    close = "error";
                    failure = Some(msg::SESSION_CANCELLED.to_owned());
                    break;
                }
                session = new_session();
                session_cell.store(session, Ordering::SeqCst);
                tracing::info!(evento = "session.open", attempt, session, peer = %to, mensagem = "enter", rota = barrier, "session opening");
                let (edge, pos) = barriers.get(barrier).map_or((Edge::Left, pos), |b| {
                    (b.peer_edge, map_pos(b.local, b.remote, pos))
                });
                let host = host.clone();
                opening = Some(Box::pin(async move {
                    tokio::time::timeout(
                        crate::network::SEND_WAIT,
                        host.send_reliable_on(
                            to,
                            context.connection,
                            ModuleMsg::Input(InputMsg::Enter { session, edge, pos }),
                        ),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        Err(SendError::Transport(
                            "input opening deadline exceeded".into(),
                        ))
                    })
                }));
            }
            CaptureEvent::Input { t_mono_us, ev } if opening.is_some() => {
                if queue.len() == 256 {
                    close = "error";
                    failure = Some(msg::OPEN_BUFFER_FULL.to_owned());
                    break;
                }
                queue.push_back(CaptureEvent::Input { t_mono_us, ev });
            }
            CaptureEvent::Input { t_mono_us, ev } if active => {
                let (t_mono_us, ev) = if !replay && !queue.is_empty() {
                    let oldest = queue.pop_front();
                    queue.push_back(CaptureEvent::Input { t_mono_us, ev });
                    match oldest {
                        Some(CaptureEvent::Input { t_mono_us, ev }) => (t_mono_us, ev),
                        _ => continue,
                    }
                } else {
                    (t_mono_us, ev)
                };
                match host.send_datagram(
                    to,
                    InputDatagram {
                        session,
                        seq,
                        t_mono_us,
                        events: vec![ev],
                    },
                ) {
                    Ok(()) => {
                        if sent == 0 {
                            tracing::info!(evento = "datagram.first", session, peer = %to, "first datagram sent");
                        }
                        sent += 1;
                    }
                    Err(_) => dropped += 1,
                }
                seq = seq.wrapping_add(1);
            }
            CaptureEvent::Input { .. } => {}
            CaptureEvent::Deactivated => {
                close = "stop";
                break;
            }
            CaptureEvent::Error(e) => {
                close = "error";
                failure = Some(msg::adapter_failed(&e));
                break;
            }
            CaptureEvent::SessionLocked(true) => {
                release_all = true;
                close = "lock";
                failure = Some(msg::SOURCE_SCREEN_LOCKED.to_owned());
                break;
            }
            CaptureEvent::SessionLocked(false) => snapshot_ready = true,
            CaptureEvent::Hotkey(_) | CaptureEvent::ZonesChanged(_) => {}
        }
    }
    // No network progress is required to hand the physical device back.
    let _ = cap.release().await;
    drop(opening);
    // A producer may be waiting inside send; wake it before awaiting its join.
    ev_rx.close();
    let _ = cap.stop().await;
    drop(ev_rx);
    queue.clear();
    if session != 0 {
        let _ = tokio::time::timeout(crate::network::SEND_WAIT, async {
            host.send_reliable_on(
                to,
                context.connection,
                ModuleMsg::Input(InputMsg::Leave { session }),
            )
            .await?;
            if release_all {
                host.send_reliable_on(
                    to,
                    context.connection,
                    ModuleMsg::Input(InputMsg::ReleaseAll { session }),
                )
                .await?;
            }
            Ok::<(), SendError>(())
        })
        .await;
    }
    if session != 0 {
        let motivo = close;
        let totais = format!("{{\"enviados\":{sent},\"descartados\":{dropped}}}");
        tracing::info!(evento = "session.closed", attempt, session, peer = %to, motivo, erro = failure.as_deref().unwrap_or(""), totais = %totais, "session closed");
    }
    if let Some(f) = failure {
        if session == 0 {
            let motivo = match close {
                "lock" => "locked",
                "pause" => "paused",
                "peer_down" => "peer_offline",
                "stop" => "stopped",
                _ => "error",
            };
            tracing::info!(evento = "attempt.filtered", attempt, motivo, erro = %f, "attempt filtered");
        }
        return Err(f);
    }
    if session == 0 {
        tracing::info!(
            evento = "attempt.filtered",
            attempt,
            motivo = "no_activation",
            "attempt filtered"
        );
        return Err(msg::NO_ACTIVATION.to_owned());
    }
    tokio::time::timeout(
        crate::network::SEND_WAIT,
        host.send_reliable_on(
            to,
            context.connection,
            ModuleMsg::Input(InputMsg::StatsRequest { session }),
        ),
    )
    .await
    .map_err(|_| msg::NO_STATS.to_owned())?
    .map_err(|e| e.to_string())?;
    let wait = Instant::now() + STATS_WAIT;
    tokio::select! {
        s = signals.recv() => match s {
            Some(LoopSignal::Stats(s)) => Ok(s),
            Some(LoopSignal::Rejected(r)) => Err(r.pt().to_owned()),
            Some(LoopSignal::PeerDown) => Err(msg::PEER_OFFLINE.to_owned()),
            Some(LoopSignal::Paused) => Err(RejectReason::Paused.pt().to_owned()),
            Some(LoopSignal::Locked) => Err(msg::LOCKED.to_owned()),
            Some(LoopSignal::Stop) | None => Err(msg::SESSION_CANCELLED.to_owned()),
        },
        () = tokio::time::sleep_until(wait) => Err(msg::NO_STATS.to_owned()),
    }
}

/// A comparable summary of a set of targets: a retarget only happens when it changes.
fn targets_key(targets: &[Target]) -> String {
    format!("{targets:?}")
}

/// Runs the closure if the holder is dropped before being disarmed: a send cancelled by the
/// network budget still leaves its line in the log.
struct Pendente<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Pendente<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

/// `routes.built` or `routes.empty`, once per distinct result (contract §17.2).
fn log_routes(
    last: &mut Option<String>,
    rotas: usize,
    targets: &[Target],
    motivo: Option<&'static str>,
) {
    let mut pares: Vec<String> = targets.iter().map(|t| t.peer.short()).collect();
    pares.sort();
    pares.dedup();
    let key = format!("{rotas}:{}:{pares:?}:{motivo:?}", targets.len());
    if last.as_deref() == Some(key.as_str()) {
        return;
    }
    *last = Some(key);
    if targets.is_empty() {
        tracing::info!(
            evento = "routes.empty",
            motivo = motivo.unwrap_or("no_links"),
            rotas,
            "no barrier derived"
        );
    } else {
        let pares = serde_json::to_string(&pares).unwrap_or_else(|_| "[]".into());
        tracing::info!(evento = "routes.built", rotas, barreiras = targets.len(), pares = %pares, "barriers derived");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod backpressure {
    include!("../tests/support/module_backpressure.rs");
}
