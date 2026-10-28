//! The input module: destination side (Enter / datagrams / Stats), the source-side loopback
//! driven by `ModuleCommand::InputLoopback` (contract §8 rules 1 to 3) and the continuous
//! capture derived from `layout.toml` (§12.6 item 2, in `capture.rs`).
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, InputError, Zone, ZoneRef,
    entry_events,
};
use crate::capture::{self, Ctl, Engine, Note, Target, new_session};
use crate::config::{InputConfig, map_pos, zone_ref};
use crate::layout::{Endpoint, Layout, trim_dead_corner};
use crate::msg;
use async_trait::async_trait;
use clipsync_proto::{
    Edge, InputDatagram, InputMsg, InputState, LayoutOffer, LoopbackStats, MAX_EVENTS_PER_DATAGRAM,
    MAX_INPUT_DATAGRAMS_PER_SEC, Module, ModuleCommand, ModuleError, ModuleHost, ModuleInput,
    ModuleKind, ModuleMsg, ModuleReply, ModuleStatus, OfferDevice, OfferEndpoint, OfferLink,
    OfferOutcome, OfferState, PeerId, PeerInfo, PeerZones, PendingOffer, RejectReason, SendError,
    ZoneInfo, mono_us,
};
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
    let layout = Layout::read(&path).map_err(config)?;
    if let Some(l) = &layout {
        l.ensure_no_manual_barriers(&cfg).map_err(config)?;
    }
    Ok(Box::new(InputModule {
        cfg,
        cap,
        emu,
        layout,
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
}

impl Incoming {
    fn new(session: u32) -> Self {
        Self {
            session,
            active: true,
            received: 0,
            lost: 0,
            out_of_order: 0,
            last_seq: None,
            latencies: Vec::new(),
            samples: 0,
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

struct State {
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
    last_event: Option<String>,
    last_block: Option<String>,
    layout: Option<Layout>,
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
        let (notes, mut note_rx) = mpsc::unbounded_channel();
        let mut st = State {
            cap_name: me.cap.name(),
            emu_name: me.emu.name(),
            cfg: me.cfg,
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
            paused: false,
            last_event: None,
            last_block: None,
            layout: me.layout,
            layout_path: me.layout_path,
            locked: false,
            cont: None,
            notes,
            local_zones: Vec::new(),
            peer_zones: HashMap::new(),
            pending_offer: None,
            active_peer: None,
            offer_job: None,
        };
        st.local_zones = st.cap.as_ref().map(|c| c.known_zones()).unwrap_or_default();
        st.start_continuous().await;
        let mut task: Option<JoinHandle<LoopDone>> = None;
        loop {
            tokio::select! {
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
                        st.on_note(n);
                    }
                }
                () = async { if let Some(j) = &st.offer_job { tokio::time::sleep_until(j.until).await } },
                    if st.offer_job.is_some() =>
                {
                    st.finish_offer();
                }
            }
        }
        if let Some(t) = task {
            t.abort();
        }
        st.stop_continuous().await;
        st.release_all().await;
        let _ = st.emu.stop().await;
        Ok(())
    }
}

impl State {
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

    fn on_note(&mut self, n: Note) {
        match n {
            Note::Event(t) => self.last_event = Some(t),
            Note::Block(t) => self.last_block = Some(t),
            Note::Active(p) => self.active_peer = p,
            Note::Zones(z) => {
                if !z.is_empty() && z.len() != self.local_zones.len() {
                    self.local_zones = z;
                    self.announce_zones_to_all();
                } else if !z.is_empty() {
                    self.local_zones = z;
                }
            }
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
            local: self.local_zones.iter().map(zone_info).collect(),
            peers,
            screen_locked: self.locked,
            active_peer: self.active_peer,
            pending_offer: self.pending_offer.clone(),
            keyboard_layout: None,
        }
    }

    /// `InputReload`: reads `layout.toml` again and reinstalls the barriers. A bad file keeps
    /// what was installed and reports why.
    async fn reload_layout(&mut self) -> Result<(), String> {
        let Some(path) = self.layout_path.clone() else {
            return Err(msg::NO_LAYOUT_PATH.to_owned());
        };
        let layout = Layout::read(&path).map_err(|e| e.to_string())?;
        if let Some(l) = &layout {
            l.ensure_no_manual_barriers(&self.cfg)
                .map_err(|e| e.to_string())?;
        }
        self.stop_continuous().await;
        self.layout = layout;
        self.start_continuous().await;
        self.last_event = Some(msg::LAYOUT_RELOADED.to_owned());
        Ok(())
    }

    /// `InputSwitch`: the continuous capture opens the session; the answer comes when the peer
    /// had its chance to refuse.
    fn switch(&self, to: PeerId, reply: oneshot::Sender<ModuleReply>) {
        let Some(cont) = &self.cont else {
            let _ = reply.send(ModuleReply::Error(msg::NOT_LINKED.to_owned()));
            return;
        };
        let (tx, rx) = oneshot::channel();
        let _ = cont.ctl.send(Ctl::Switch { to, reply: tx });
        tokio::spawn(async move {
            let r = match rx.await {
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
        if self.offer_job.is_some() {
            let _ = reply.send(ModuleReply::Error(msg::OFFER_BUSY.to_owned()));
            return;
        }
        let Some(offer) = self.layout.as_ref().map(layout_offer) else {
            let _ = reply.send(ModuleReply::Error(msg::NO_LAYOUT.to_owned()));
            return;
        };
        let mut outcomes = Vec::new();
        for peer in to {
            let state = match self.peers.get(&peer) {
                Some(p) if !p.connected => OfferState::Offline,
                None => OfferState::Offline,
                Some(p) if p.remote_version < LAYOUT_PROTOCOL => OfferState::Unsupported,
                Some(_) => {
                    let m = ModuleMsg::Input(InputMsg::LayoutOffer(offer.clone()));
                    match self.host.send_reliable(peer, m).await {
                        Ok(()) => OfferState::Offered,
                        Err(SendError::RemoteTooOld) => OfferState::Unsupported,
                        Err(SendError::NotConnected) => OfferState::Offline,
                        Err(_) => OfferState::Denied,
                    }
                }
            };
            outcomes.push(OfferOutcome { peer, state });
        }
        if outcomes.iter().any(|o| o.state == OfferState::Offered) {
            self.offer_job = Some(OfferJob {
                outcomes,
                reply,
                until: Instant::now() + OFFER_GRACE,
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
    fn announce_zones(&self, info: &PeerInfo) {
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
        tokio::spawn(async move {
            let m = ModuleMsg::Input(InputMsg::Zones { zones });
            if let Err(e) = host.send_reliable(to, m).await {
                tracing::debug!(peer = %to, "zones not sent: {e}");
            }
        });
    }

    /// Monitors changed: every eligible peer hears about it.
    fn announce_zones_to_all(&self) {
        for p in self.peers.values() {
            self.announce_zones(p);
        }
    }

    fn send_ctl(&self, c: Ctl) {
        if let Some(cont) = &self.cont {
            let _ = cont.ctl.send(c);
        }
    }

    /// Installs the barriers derived from `layout.toml` (contract §12.6 item 2). Nothing to do
    /// without links; the loopback owns the adapter while it runs and restarts us after.
    async fn start_continuous(&mut self) {
        if self.cont.is_some() {
            return;
        }
        let me = self.host.local_id();
        let (derived, dead) = match &self.layout {
            Some(l) => (l.barriers_for(me), l.options.dead_corner_px),
            None => return,
        };
        if derived.is_empty() {
            return;
        }
        let Some(mut cap) = self.cap.take() else {
            return;
        };
        let zones = match cap.zones().await {
            Ok(z) => z,
            Err(e) => {
                tracing::warn!("zones unavailable: {e}");
                self.last_block = Some(match e {
                    InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                    other => msg::adapter_failed(&other.to_string()),
                });
                self.cap = Some(cap);
                return;
            }
        };
        let mut targets = Vec::new();
        for b in derived {
            let Some(zone) = find_zone(&zones, &b.monitor) else {
                tracing::warn!(link = %b.link, "monitor of the link not found");
                self.last_block = Some(msg::layout_monitor_missing(&b.link, &b.monitor));
                continue;
            };
            let len = match b.edge {
                Edge::Left | Edge::Right => zone.height,
                Edge::Top | Edge::Bottom => zone.width,
            };
            let Some((from, to)) = trim_dead_corner(b.from, b.to, len, dead) else {
                self.last_block = Some(msg::layout_segment_in_dead_corner(&b.link));
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
        if targets.is_empty() {
            self.cap = Some(cap);
            return;
        }
        let (ctl, ctl_rx) = mpsc::unbounded_channel();
        let engine = Engine {
            host: self.host.clone(),
            cap,
            targets,
            ctl: ctl_rx,
            notes: self.notes.clone(),
            locked: self.locked,
            paused: self.paused,
        };
        self.cont = Some(Cont {
            ctl,
            task: tokio::spawn(capture::run(engine)),
        });
    }

    /// Closes the open session and takes the adapter back.
    async fn stop_continuous(&mut self) {
        let Some(c) = self.cont.take() else { return };
        let _ = c.ctl.send(Ctl::Stop);
        match c.task.await {
            Ok(cap) => self.cap = Some(cap),
            Err(_) => self.last_block = Some(msg::adapter_failed("captura")),
        }
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
            ModuleCommand::Status => {
                let _ = reply.send(ModuleReply::Status(self.status()));
                None
            }
            ModuleCommand::InputLoopback { to, seconds } => {
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
                self.locked = locked;
                self.send_ctl(Ctl::Lock(locked));
                self.last_event = Some(if locked {
                    msg::LOCKED_ON.to_owned()
                } else {
                    msg::LOCKED_OFF.to_owned()
                });
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
            ModuleInput::Reliable { from, msg } => {
                if let ModuleMsg::Input(m) = msg {
                    self.on_msg(from, m).await;
                }
            }
            ModuleInput::Datagram { from, dg } => self.on_datagram(from, dg).await,
            ModuleInput::Rejected { from, reason, .. } => {
                self.last_block = Some(reason.pt().to_owned());
                if let Some(c) = self.ctl.as_ref().filter(|c| c.to == from) {
                    let _ = c.tx.send(LoopSignal::Rejected(reason));
                }
                self.send_ctl(Ctl::Rejected { from, reason });
                if let Some(j) = &mut self.offer_job {
                    for o in j.outcomes.iter_mut().filter(|o| o.peer == from) {
                        if o.state == OfferState::Offered {
                            o.state = OfferState::Denied;
                        }
                    }
                }
            }
            ModuleInput::PeerUp(info) => {
                self.announce_zones(&info);
                self.peers.insert(info.id, info);
            }
            ModuleInput::PeerDown(id) => {
                self.peers.remove(&id);
                self.buckets.remove(&id);
                if self.incoming.remove(&id).is_some_and(|i| i.active) {
                    self.release_all().await;
                }
                if let Some(c) = self.ctl.as_ref().filter(|c| c.to == id) {
                    let _ = c.tx.send(LoopSignal::PeerDown);
                }
                self.send_ctl(Ctl::PeerDown(id));
            }
            ModuleInput::Paused(p) => {
                self.paused = p;
                self.send_ctl(Ctl::Paused(p));
                if p {
                    for i in self.incoming.values_mut() {
                        i.active = false;
                    }
                    self.release_all().await;
                    if let Some(c) = &self.ctl {
                        let _ = c.tx.send(LoopSignal::Paused);
                    }
                }
            }
            ModuleInput::Command { .. } | ModuleInput::Shutdown => {}
        }
    }

    async fn on_msg(&mut self, from: PeerId, m: InputMsg) {
        match m {
            InputMsg::Enter { session, edge, pos } => {
                self.on_enter(from, session, edge, None, pos).await
            }
            InputMsg::EnterAt {
                session,
                edge,
                monitor,
                pos,
            } => {
                self.on_enter(from, session, edge, monitor.as_deref(), pos)
                    .await
            }
            // Protocol 3. The way back (`back_from..back_to`) and the replies come with the S7
            // module work (§13.6 items 1 to 3); until then an `EnterSpan` enters like an `EnterAt`.
            InputMsg::EnterSpan {
                session,
                edge,
                monitor,
                pos,
                ..
            } => {
                self.on_enter(from, session, edge, monitor.as_deref(), pos)
                    .await
            }
            InputMsg::Returned { .. } | InputMsg::Refused { .. } => self.ignored += 1,
            InputMsg::Zones { zones } => {
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
                if let Some(i) = self
                    .incoming
                    .get_mut(&from)
                    .filter(|i| i.session == session)
                {
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
                        tokio::spawn(async move {
                            let m = ModuleMsg::Input(InputMsg::Stats(stats));
                            if let Err(e) = host.send_reliable(from, m).await {
                                tracing::warn!("could not send input stats: {e}");
                            }
                        });
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

    async fn on_enter(
        &mut self,
        from: PeerId,
        session: u32,
        edge: Edge,
        monitor: Option<&str>,
        pos: f32,
    ) {
        if self.paused || session == 0 {
            self.ignored += 1;
            return;
        }
        if self.incoming.get(&from).is_some_and(|i| i.active) {
            self.release_all().await;
        }
        if !self.emu_started {
            match self.emu.start().await {
                Ok(()) => self.emu_started = true,
                Err(e) => {
                    tracing::warn!("emulator start failed: {e}");
                    self.last_block = Some(match e {
                        InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                        _ => msg::EMULATOR_FAILED.to_owned(),
                    });
                    return;
                }
            }
        }
        self.incoming.insert(from, Incoming::new(session));
        let zone = monitor.and_then(|m| find_zone(&self.local_zones, m));
        let placed = match zone {
            Some(z) => self.emu.enter(z, &self.local_zones, edge, pos).await,
            None => self.emu.emit(&entry_events(edge, pos)).await,
        };
        if let Err(e) = placed {
            tracing::warn!("entry positioning failed: {e}");
        }
        let name = self.peer_name(from);
        self.last_event = Some(msg::control_started(&name));
    }

    async fn on_datagram(&mut self, from: PeerId, dg: InputDatagram) {
        let Some(inc) = self.incoming.get_mut(&from) else {
            self.ignored += 1;
            return;
        };
        if !inc.active || inc.session != dg.session || self.paused {
            self.ignored += 1;
            return;
        }
        // Per peer, not per session: a new Enter must not refill the budget.
        if !self.buckets.entry(from).or_default().take(Instant::now()) {
            self.rate_dropped += 1;
            return;
        }
        match inc.last_seq {
            None => inc.last_seq = Some(dg.seq),
            Some(last) if dg.seq > last => {
                inc.lost += u64::from(dg.seq - last - 1);
                inc.last_seq = Some(dg.seq);
            }
            Some(_) => {
                inc.out_of_order += 1;
                inc.lost = inc.lost.saturating_sub(1);
            }
        }
        inc.record_latency(mono_us().saturating_sub(dg.t_mono_us));
        inc.received += dg.events.len() as u64;
        if self.emu_started
            && let Err(e) = self.emu.emit(&dg.events).await
        {
            tracing::warn!("emit failed: {e}");
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
    let specs: Vec<BarrierSpec> = barriers.iter().map(|b| b.spec.clone()).collect();
    let mut ev_rx = cap.start(&specs).await.map_err(|e| match e {
        InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
        other => msg::adapter_failed(&other.to_string()),
    })?;
    let deadline = Instant::now() + Duration::from_secs(u64::from(seconds));
    let mut session = 0u32;
    let mut active = false;
    let mut seq = 0u32;
    let mut queue: VecDeque<CaptureEvent> = VecDeque::new();
    let mut failure: Option<String> = None;

    'outer: loop {
        let ev = if let Some(e) = queue.pop_front() {
            e
        } else {
            tokio::select! {
                e = ev_rx.recv() => match e { Some(e) => e, None => break },
                s = signals.recv() => {
                    match s {
                        Some(LoopSignal::Rejected(r)) => failure = Some(r.pt().to_owned()),
                        Some(LoopSignal::PeerDown) => failure = Some(msg::PEER_OFFLINE.to_owned()),
                        Some(LoopSignal::Paused) => failure = Some(RejectReason::Paused.pt().to_owned()),
                        Some(LoopSignal::Stats(_)) | None => {}
                    }
                    if failure.is_some() { break 'outer; }
                    continue;
                }
                () = tokio::time::sleep_until(deadline) => break,
            }
        };
        match ev {
            CaptureEvent::Activated { barrier, pos, .. } => {
                session = new_session();
                session_cell.store(session, Ordering::SeqCst);
                seq = 0;
                active = true;
                let (edge, pos) = match barriers.get(barrier) {
                    Some(b) => (b.peer_edge, map_pos(b.local, b.remote, pos)),
                    None => (Edge::Left, pos),
                };
                let m = ModuleMsg::Input(InputMsg::Enter { session, edge, pos });
                if let Err(e) = host.send_reliable(to, m).await {
                    failure = Some(e.to_string());
                    break;
                }
            }
            CaptureEvent::Input { t_mono_us, ev } if active => {
                let mut events = vec![ev];
                while events.len() < MAX_EVENTS_PER_DATAGRAM {
                    match ev_rx.try_recv() {
                        Ok(CaptureEvent::Input { ev, .. }) => events.push(ev),
                        Ok(other) => {
                            queue.push_back(other);
                            break;
                        }
                        Err(_) => break,
                    }
                }
                let dg = InputDatagram {
                    session,
                    seq,
                    t_mono_us,
                    events,
                };
                seq = seq.wrapping_add(1);
                if let Err(e) = host.send_datagram(to, dg) {
                    tracing::warn!("datagram not sent: {e}");
                }
            }
            CaptureEvent::Input { .. } => {}
            CaptureEvent::Deactivated => {
                if active {
                    break;
                }
            }
            CaptureEvent::Error(e) => {
                failure = Some(msg::adapter_failed(&e));
                break;
            }
            // The loopback test does not follow shortcuts, zones or the session lock.
            CaptureEvent::Hotkey(_)
            | CaptureEvent::ZonesChanged(_)
            | CaptureEvent::SessionLocked(_) => {}
        }
    }

    let _ = cap.release().await;
    if let Some(f) = failure {
        if session != 0 {
            let _ = host
                .send_reliable(to, ModuleMsg::Input(InputMsg::Leave { session }))
                .await;
        }
        return Err(f);
    }
    if session == 0 {
        return Err(msg::NO_ACTIVATION.to_owned());
    }
    let _ = host
        .send_reliable(to, ModuleMsg::Input(InputMsg::Leave { session }))
        .await;
    host.send_reliable(to, ModuleMsg::Input(InputMsg::StatsRequest { session }))
        .await
        .map_err(|e| e.to_string())?;
    let wait = Instant::now() + STATS_WAIT;
    tokio::select! {
        s = signals.recv() => match s {
            Some(LoopSignal::Stats(s)) => Ok(s),
            Some(LoopSignal::Rejected(r)) => Err(r.pt().to_owned()),
            Some(LoopSignal::PeerDown) => Err(msg::PEER_OFFLINE.to_owned()),
            Some(LoopSignal::Paused) => Err(RejectReason::Paused.pt().to_owned()),
            None => Err(msg::NO_STATS.to_owned()),
        },
        () = tokio::time::sleep_until(wait) => Err(msg::NO_STATS.to_owned()),
    }
}
