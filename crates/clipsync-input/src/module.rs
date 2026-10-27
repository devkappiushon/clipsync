//! The input module: destination side (Enter / datagrams / Stats) and the source-side loopback
//! driven by `ModuleCommand::InputLoopback` (contract §8 rules 1 to 3).
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, InputError, ZoneRef,
};
use crate::config::InputConfig;
use crate::msg;
use async_trait::async_trait;
use clipsync_proto::{
    Edge, EventId, InputDatagram, InputEvent, InputMsg, LoopbackStats, MAX_EVENTS_PER_DATAGRAM,
    MAX_INPUT_DATAGRAMS_PER_SEC, Module, ModuleCommand, ModuleError, ModuleHost, ModuleInput,
    ModuleKind, ModuleMsg, ModuleReply, ModuleStatus, PeerId, PeerInfo, RejectReason, mono_us,
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant;

const STATS_WAIT: Duration = Duration::from_secs(3);
const MAX_LATENCY_SAMPLES: usize = 100_000;
/// Reference length of the entry edge (no destination size is known): 1080p.
const ENTRY_REF_LEN_PX: f32 = 1080.0;
/// Saturating burst toward an edge/corner.
const SATURATE_PX: f32 = 4000.0;

pub struct InputModule {
    cfg: InputConfig,
    cap: Box<dyn CaptureAdapter>,
    emu: Box<dyn EmulateAdapter>,
}

pub fn build_with_adapters(
    cfg: InputConfig,
    cap: Box<dyn CaptureAdapter>,
    emu: Box<dyn EmulateAdapter>,
) -> Box<dyn Module> {
    Box::new(InputModule { cfg, cap, emu })
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
    window_start: Instant,
    window_count: u32,
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
            window_start: Instant::now(),
            window_count: 0,
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
    ctl: Option<LoopCtl>,
    ignored: u64,
    rate_dropped: u64,
    paused: bool,
    last_event: Option<String>,
    last_block: Option<String>,
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
            ctl: None,
            ignored: 0,
            rate_dropped: 0,
            paused: false,
            last_event: None,
            last_block: None,
        };
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
                        Some(Ok(d)) => st.finish_loopback(d),
                        _ => {
                            // The task panicked: the capture adapter is gone with it.
                            st.last_block = Some(msg::adapter_failed("loopback"));
                        }
                    }
                }
            }
        }
        if let Some(t) = task {
            t.abort();
        }
        st.release_all().await;
        let _ = st.emu.stop().await;
        Ok(())
    }
}

impl State {
    fn finish_loopback(&mut self, d: LoopDone) {
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
                    barriers: self.cfg.barriers.clone(),
                    peer_name: peer.name,
                    to,
                    seconds: seconds.clamp(1, 300),
                    session,
                    signals: sig_rx,
                    reply,
                };
                Some(tokio::spawn(run_loopback(job)))
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
            }
            ModuleInput::PeerUp(info) => {
                self.peers.insert(info.id, info);
            }
            ModuleInput::PeerDown(id) => {
                self.peers.remove(&id);
                if self.incoming.remove(&id).is_some_and(|i| i.active) {
                    self.release_all().await;
                }
                if let Some(c) = self.ctl.as_ref().filter(|c| c.to == id) {
                    let _ = c.tx.send(LoopSignal::PeerDown);
                }
            }
            ModuleInput::Paused(p) => {
                self.paused = p;
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
            InputMsg::Enter { session, edge, pos } => self.on_enter(from, session, edge, pos).await,
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

    async fn on_enter(&mut self, from: PeerId, session: u32, edge: Edge, pos: f32) {
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
        let entry = entry_events(edge, pos);
        if let Err(e) = self.emu.emit(&entry).await {
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
        let now = Instant::now();
        if now.duration_since(inc.window_start) >= Duration::from_secs(1) {
            inc.window_start = now;
            inc.window_count = 0;
        }
        inc.window_count += 1;
        if inc.window_count > MAX_INPUT_DATAGRAMS_PER_SEC {
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

/// Relative-only positioning (the EIS has no absolute motion): saturate into the entry corner,
/// then walk along the edge to `pos`.
fn entry_events(edge: Edge, pos: f32) -> Vec<InputEvent> {
    let toward_edge = match edge {
        Edge::Left => (-SATURATE_PX, 0.0),
        Edge::Right => (SATURATE_PX, 0.0),
        Edge::Top => (0.0, -SATURATE_PX),
        Edge::Bottom => (0.0, SATURATE_PX),
    };
    let toward_start = match edge {
        Edge::Left | Edge::Right => (0.0, -SATURATE_PX),
        Edge::Top | Edge::Bottom => (-SATURATE_PX, 0.0),
    };
    let walk = pos.clamp(0.0, 1.0) * ENTRY_REF_LEN_PX;
    let along = match edge {
        Edge::Left | Edge::Right => (0.0, walk),
        Edge::Top | Edge::Bottom => (walk, 0.0),
    };
    let mk = |(dx, dy): (f32, f32)| InputEvent::Motion { dx, dy };
    vec![mk(toward_edge), mk(toward_start), mk(along)]
}

struct LoopJob {
    host: Arc<dyn ModuleHost>,
    cap: Box<dyn CaptureAdapter>,
    barriers: Vec<crate::config::BarrierConfig>,
    peer_name: String,
    to: PeerId,
    seconds: u32,
    session: Arc<AtomicU32>,
    signals: mpsc::UnboundedReceiver<LoopSignal>,
    reply: oneshot::Sender<ModuleReply>,
}

fn opposite(e: Edge) -> Edge {
    match e {
        Edge::Left => Edge::Right,
        Edge::Right => Edge::Left,
        Edge::Top => Edge::Bottom,
        Edge::Bottom => Edge::Top,
    }
}

/// Barriers for `peer`: from `[[input.barrier]]`, or the upper half of the right edge of zone 0.
/// Returns the specs and, per spec, the edge the destination should enter through.
fn pick_barriers(
    cfgs: &[crate::config::BarrierConfig],
    peer: &str,
) -> (Vec<BarrierSpec>, Vec<Edge>) {
    let mut specs = Vec::new();
    let mut entries = Vec::new();
    for b in cfgs.iter().filter(|b| b.peer == peer) {
        let zone = b
            .monitor
            .parse::<u32>()
            .map_or_else(|_| ZoneRef::Name(b.monitor.clone()), ZoneRef::Index);
        specs.push(BarrierSpec {
            zone,
            edge: b.edge,
            from: b.from,
            to: b.to,
        });
        entries.push(b.peer_edge);
    }
    if specs.is_empty() {
        specs.push(BarrierSpec {
            zone: ZoneRef::Index(0),
            edge: Edge::Right,
            from: 0.0,
            to: 0.5,
        });
        entries.push(opposite(Edge::Right));
    }
    (specs, entries)
}

fn new_session() -> u32 {
    let b = EventId::random().0;
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]).max(1)
}

async fn run_loopback(job: LoopJob) -> LoopDone {
    let LoopJob {
        host,
        mut cap,
        barriers,
        peer_name,
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
        &peer_name,
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

#[allow(clippy::too_many_arguments)]
async fn drive(
    host: &Arc<dyn ModuleHost>,
    cap: &mut dyn CaptureAdapter,
    barriers: &[crate::config::BarrierConfig],
    peer_name: &str,
    to: PeerId,
    seconds: u32,
    session_cell: &AtomicU32,
    signals: &mut mpsc::UnboundedReceiver<LoopSignal>,
) -> Result<LoopbackStats, String> {
    let (specs, entries) = pick_barriers(barriers, peer_name);
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
                let edge = entries.get(barrier).copied().unwrap_or(Edge::Left);
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
