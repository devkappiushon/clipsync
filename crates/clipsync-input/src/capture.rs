//! Continuous capture (contract §12.6 item 2, §12.10 decision 9). The barriers derived from
//! the links in `layout.toml` stay installed while the agent runs. Crossing one opens a control
//! session on the linked peer; pushing the cursor back through the entry edge, a refusal, a
//! pause, the lock or the peer going down closes it and gives the cursor back.
use crate::adapter::lifecycle::LifecycleCursor;
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, Hotkey, InputError, Zone, same_zones,
};
use crate::config::{map_pos, zone_ref};
use crate::layout::trim_dead_corner;
use crate::msg;
use crate::{
    inventory::{InventoryCatalog, ObservedMonitors},
    route_session::{RoutePlan, RouteSession},
};
use clipsync_proto::{
    Edge, EventId, InputDatagram, InputEvent, InputMsg, ModuleHost, ModuleMsg, PeerId,
    RejectReason, SendError, ZoneInfo,
};
use clipsync_proto::{InventoryStamp, ReceiveContext, ReturnedRoute};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

/// Each owner observes host invalidations independently; never share a watch cursor.
pub(crate) struct HostCursor {
    rx: Option<tokio::sync::watch::Receiver<clipsync_proto::HostLifecycle>>,
    pub snapshot: clipsync_proto::HostLifecycle,
}
impl HostCursor {
    pub fn new(host: &dyn ModuleHost) -> Self {
        let mut rx = host.lifecycle_receiver();
        let mut snapshot = rx.as_mut().map_or(
            clipsync_proto::HostLifecycle {
                paused: host.paused(),
                pause_generation: 0,
                stopped: false,
            },
            |r| *r.borrow_and_update(),
        );
        snapshot.stopped |= rx.as_ref().is_some_and(|r| r.has_changed().is_err())
            || snapshot.pause_generation == u64::MAX;
        Self { rx, snapshot }
    }
    pub fn observed(&self) -> bool {
        self.rx.is_some()
    }
    /// (notification, invalidation). Closed observed sources stay terminal.
    pub fn refresh(&mut self) -> (bool, bool) {
        let Some(rx) = &mut self.rx else {
            return (false, false);
        };
        let changed = rx.has_changed().unwrap_or(true);
        let mut next = *rx.borrow_and_update();
        next.stopped |=
            self.snapshot.stopped || rx.has_changed().is_err() || next.pause_generation == u64::MAX;
        let invalidated = next.pause_generation != self.snapshot.pause_generation;
        let changed = changed || next != self.snapshot;
        self.snapshot = next;
        (changed, invalidated)
    }
    pub async fn changed(&mut self) {
        match &mut self.rx {
            Some(rx) => {
                let _ = rx.changed().await;
            }
            None => std::future::pending().await,
        }
    }
}

/// Pushing this far past the entry edge brings the cursor back; smaller moves are jitter.
pub(crate) const RETURN_SLACK_PX: f32 = 8.0;
/// Keeps the tracked depth finite without the peer's zones.
const MAX_DEPTH_PX: f32 = 1.0e7;
/// A refusal of the Enter arrives after the send: a switch waits this long for it.
const SWITCH_GRACE: Duration = Duration::from_millis(400);
/// Protocol version that understands `Zones`, `LayoutOffer` and `EnterAt` (§12.10 decision 7).
pub(crate) const LAYOUT_PROTOCOL: u16 = 2;
/// Protocol version that understands `EnterSpan`, `Returned` and `Refused` (§13.4).
pub(crate) const SPAN_PROTOCOL: u16 = 3;
/// How often the live zones are compared with the last ones seen (§13.8 item 2).
pub(crate) const ZONES_POLL: Duration = Duration::from_secs(2);

/// One installed barrier and where it leads.
#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub spec: BarrierSpec,
    pub peer: PeerId,
    /// Destination monitor; goes out in `EnterAt` once the peer speaks protocol 2.
    pub peer_monitor: Option<String>,
    pub peer_edge: Edge,
    /// Local `(from, to)` and destination `(peer_from, peer_to)` of the position mapping.
    pub local: (f32, f32),
    pub remote: (f32, f32),
}

/// Cleanup transferred to State, which polls it only outside layout transactions.
#[derive(Clone, Default)]
pub(crate) struct DeferredCleanup {
    queue: Arc<std::sync::Mutex<Vec<Sending>>>,
    count: Arc<std::sync::atomic::AtomicUsize>,
}
impl DeferredCleanup {
    fn push(&self, future: Sending) {
        struct Count(Arc<std::sync::atomic::AtomicUsize>);
        impl Drop for Count {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let count = Count(self.count.clone());
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Box::pin(async move {
                let _count = count;
                future.await
            }));
    }
    pub fn take(&self) -> Vec<Sending> {
        std::mem::take(&mut *self.queue.lock().unwrap_or_else(|e| e.into_inner()))
    }
    fn busy(&self) -> bool {
        self.count.load(std::sync::atomic::Ordering::SeqCst) != 0
    }
}

pub(crate) enum Ctl {
    KnownInventory {
        reply: oneshot::Sender<Result<ObservedMonitors, InputError>>,
    },
    Quiesce {
        reply: oneshot::Sender<Result<(), InputError>>,
    },
    Ready {
        reply: oneshot::Sender<bool>,
    },
    PrepareInventory {
        reply: oneshot::Sender<Result<ObservedMonitors, InputError>>,
    },
    InvalidateRoutes,
    RoutePlans(HashMap<usize, RoutePlan>),
    ReturnedRoute {
        from: PeerId,
        context: ReceiveContext,
        returned: ReturnedRoute,
    },
    Lock(bool),
    Paused(bool),
    /// The destination refused a message of the open session.
    Rejected {
        from: PeerId,
        reason: RejectReason,
    },
    PeerDown(PeerId),
    /// UI or hotkey: open a session on `to` through the first barrier that leads there.
    Switch {
        to: PeerId,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// The destination says the cursor left through the way back (protocol 3).
    Returned {
        from: PeerId,
        session: u32,
        pos: f32,
    },
    /// The destination does not emulate in this session (protocol 3).
    Refused {
        from: PeerId,
        session: u32,
        reason: RejectReason,
    },
    /// What a peer reported about its monitors (for the way-back span).
    PeerZones(PeerId, Vec<ZoneInfo>),
    /// New barriers for the same adapter session (zones or trust changed).
    Retarget(Vec<Target>),
    /// Hotkey of this machine's own device: give the cursor back.
    Home,
    /// Close the session and hand the adapter back.
    Stop,
}

/// What the module shows in `ModuleStatus` and `InputState`.
pub(crate) enum Note {
    PrepareInventory,
    Inventory(ObservedMonitors),
    Event(String),
    Block(String),
    Active(Option<PeerId>),
    /// The live zones once the capture session exists, or after they changed.
    Zones(Vec<Zone>),
    /// A shortcut the adapter recognized; the module decides what it does.
    Hotkey(Hotkey),
    /// A `Returned` or `Refused` that does not belong to the open session.
    Ignored,
    /// The adapter cannot swap barriers in place: the module restarts the capture.
    Restart,
    OfferSent(PeerId, clipsync_proto::OfferState),
}

pub(crate) struct Engine {
    pub cleanup: DeferredCleanup,
    pub expected_observation: Option<ObservedMonitors>,
    pub catalog: InventoryCatalog,
    pub route_plans: HashMap<usize, RoutePlan>,
    pub host: Arc<dyn ModuleHost>,
    pub cap: Box<dyn CaptureAdapter>,
    pub targets: Vec<Target>,
    pub ctl: mpsc::UnboundedReceiver<Ctl>,
    pub notes: mpsc::UnboundedSender<Note>,
    pub locked: bool,
    pub paused: bool,
    /// `options.dead_corner_px`, for the way-back span sent with `EnterSpan`.
    pub dead_corner_px: u32,
    pub peer_zones: HashMap<PeerId, Vec<ZoneInfo>>,
}

/// Where the forwarded motion put the cursor on the destination, along the axis normal to the
/// entry edge. The destination pins its cursor at the edge, so the tracked depth is pinned too;
/// what counts is how hard the cursor pushes against the edge from inside.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VirtualCursor {
    edge: Edge,
    depth: f32,
    overshoot: f32,
}

impl VirtualCursor {
    pub(crate) fn new(edge: Edge) -> Self {
        Self {
            edge,
            depth: 0.0,
            overshoot: 0.0,
        }
    }

    /// True when the cursor came back through the entry edge.
    pub(crate) fn step(&mut self, dx: f32, dy: f32) -> bool {
        let d = match self.edge {
            Edge::Left => dx,
            Edge::Right => -dx,
            Edge::Top => dy,
            Edge::Bottom => -dy,
        };
        if !d.is_finite() {
            return false;
        }
        let next = self.depth + d;
        if next < 0.0 {
            self.overshoot += -next;
            self.depth = 0.0;
            self.overshoot > RETURN_SLACK_PX
        } else {
            self.depth = next.min(MAX_DEPTH_PX);
            self.overshoot = 0.0;
            false
        }
    }
}

struct Session {
    routes: Option<RouteSession>,
    connection: u64,
    peer: PeerId,
    id: u32,
    seq: u32,
    /// Attempt that opened this session (contract §17.2).
    attempt: u32,
    /// Datagrams sent and dropped, reported in `session.closed`.
    sent: u64,
    dropped: u64,
    /// Protocol 2 peers only; with protocol 3 the destination decides the way back.
    cursor: Option<VirtualCursor>,
    target: Target,
}

struct PendingSwitch {
    peer: PeerId,
    reply: oneshot::Sender<Result<(), String>>,
    until: Option<Instant>,
}

use crate::network::Sending;
struct Opening {
    session: Session,
    send: Sending,
    started: bool,
    begin: Option<(usize, f32)>,
}
const OPEN_BUFFER: usize = 256;

pub(crate) fn new_session() -> u32 {
    let b = EventId::random().0;
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]).max(1)
}

/// Why `open` did not start a session: `code` for the log, `text` for the interface.
struct OpenError {
    code: &'static str,
    text: String,
}
impl OpenError {
    fn new(code: &'static str, text: impl Into<String>) -> Self {
        Self {
            code,
            text: text.into(),
        }
    }
}
impl From<OpenError> for String {
    fn from(e: OpenError) -> Self {
        e.text
    }
}

/// `SendError` in the snake_case the log vocabulary uses (contract §17.2).
pub(crate) fn send_error_code(e: &SendError) -> &'static str {
    match e {
        SendError::NotConnected => "not_connected",
        SendError::NotAuthorized => "not_authorized",
        SendError::RemoteModuleDisabled => "remote_module_disabled",
        SendError::LocalModuleDisabled => "local_module_disabled",
        SendError::Paused => "paused",
        SendError::RemoteTooOld => "remote_too_old",
        SendError::TooLarge => "too_large",
        SendError::Transport(_) => "transport",
    }
}

/// `RejectReason` in the snake_case the log vocabulary uses (contract §17.2).
pub(crate) fn reject_code(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::ModuleDisabled => "module_disabled",
        RejectReason::NotAuthorized => "not_authorized",
        RejectReason::Paused => "paused",
        RejectReason::TooLarge => "too_large",
        RejectReason::Invalid => "invalid",
        RejectReason::Duplicate => "duplicate",
        RejectReason::Stale => "stale",
        RejectReason::Expired => "expired",
        RejectReason::ApplyFailed => "apply_failed",
        RejectReason::Locked => "locked",
        RejectReason::VersionMismatch => "version_mismatch",
    }
}

static ATTEMPTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Next attempt id for this process (contract §17.2): shared by the continuous capture and the
/// loopback, so ids never repeat inside one run; never 0.
pub(crate) fn next_attempt() -> u32 {
    loop {
        let n = ATTEMPTS
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .wrapping_add(1);
        if n != 0 {
            return n;
        }
    }
}

/// A fraction with three decimals, the precision the log vocabulary fixes for `pos`.
pub(crate) fn frac3(pos: f32) -> f64 {
    (f64::from(pos) * 1000.0).round() / 1000.0
}

/// `attempt.filtered`: the attempt ended before a session existed.
fn log_filtered(attempt: u32, motivo: &str) {
    tracing::info!(
        evento = "attempt.filtered",
        attempt,
        motivo,
        "attempt filtered"
    );
}

pub(crate) fn send_error_text(e: &SendError) -> String {
    match e {
        SendError::NotConnected => msg::PEER_OFFLINE.to_owned(),
        SendError::NotAuthorized => msg::NOT_AUTHORIZED.to_owned(),
        SendError::RemoteModuleDisabled => msg::REMOTE_INPUT_OFF.to_owned(),
        SendError::LocalModuleDisabled => clipsync_proto::msg::MODULE_DISABLED.to_owned(),
        SendError::Paused => RejectReason::Paused.pt().to_owned(),
        SendError::RemoteTooOld => msg::PEER_TOO_OLD.to_owned(),
        SendError::TooLarge | SendError::Transport(_) => msg::send_failed(&e.to_string()),
    }
}

fn edge_code(edge: Edge) -> &'static str {
    match edge {
        Edge::Left => "left",
        Edge::Right => "right",
        Edge::Top => "top",
        Edge::Bottom => "bottom",
    }
}

/// `capture.start`: the adapter is about to install the barriers (contract §17.2).
fn log_capture_start(adaptador: &str) {
    tracing::info!(evento = "capture.start", adaptador, "capture starting");
}

/// `capture.failed`: the adapter refused to start; the reason is coarse by design.
fn log_capture_failed(adaptador: &str, e: &InputError) {
    let motivo = if matches!(e, InputError::PermissionDenied) {
        "portal_denied"
    } else {
        "adapter_failed"
    };
    tracing::info!(evento = "capture.failed", adaptador, motivo, erro = %e, "capture did not start");
}

/// `capture.ready`: only with at least one barrier installed, listing where each one leads.
fn log_capture_ready(adaptador: &str, targets: &[Target]) {
    if targets.is_empty() {
        return;
    }
    let barreiras: Vec<serde_json::Value> = targets
        .iter()
        .map(|t| {
            serde_json::json!({
                "peer": t.peer.short(),
                "monitor": t.peer_monitor,
                "borda": edge_code(t.spec.edge),
                "trecho": [t.spec.from, t.spec.to],
            })
        })
        .collect();
    let barreiras = serde_json::to_string(&barreiras).unwrap_or_else(|_| "[]".into());
    tracing::info!(evento = "capture.ready", adaptador, barreiras = %barreiras, "capture ready");
}

/// Runs until `Ctl::Stop`; returns the adapter so the module can reuse it.
pub(crate) async fn run(engine: Engine) -> Box<dyn CaptureAdapter> {
    let Engine {
        cleanup,
        expected_observation,
        catalog,
        route_plans,
        host,
        mut cap,
        targets,
        mut ctl,
        notes,
        locked,
        paused,
        dead_corner_px,
        peer_zones,
    } = engine;
    let specs: Vec<BarrierSpec> = targets.iter().map(|t| t.spec.clone()).collect();
    let mut authority = HostCursor::new(host.as_ref());
    authority.refresh();
    let paused = paused || host.paused() || authority.snapshot.paused || authority.snapshot.stopped;
    let mut lifecycle = LifecycleCursor::new(cap.as_ref());
    let mut rx = if paused {
        None
    } else {
        log_capture_start(cap.name());
        match cap.start(&specs).await {
            Ok(rx) => Some(rx),
            Err(e) => {
                log_capture_failed(cap.name(), &e);
                let text = match e {
                    InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                    other => msg::adapter_failed(&other.to_string()),
                };
                let _ = notes.send(Note::Block(text));
                None
            }
        }
    };
    let observed = cap.known_monitor_observation().ok();
    if expected_observation.as_ref().is_some_and(|expected| {
        observed
            .as_ref()
            .is_none_or(|actual| !crate::inventory::same_observation(expected, actual))
    }) {
        catalog.invalidate(host.local_id());
    }
    if let Some(o) = &observed {
        let _ = notes.send(Note::Inventory(o.clone()));
    }
    let last_zones = cap.known_zones();
    if rx.is_some() {
        log_capture_ready(cap.name(), &targets);
        let _ = notes.send(Note::Zones(last_zones.clone()));
        let _ = notes.send(Note::Event(msg::capture_ready(targets.len())));
    }
    let mut zones_tick = tokio::time::interval(ZONES_POLL);
    zones_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut st = Loop {
        cleanup,
        quiesce_reply: None,
        prepare_reply: None,
        quiesced: false,
        catalog,
        route_plans,
        observed,
        host,
        cap,
        targets,
        notes,
        locked,
        paused,
        session_locked: true,
        snapshot_ready: false,
        dead_corner_px,
        peer_zones,
        last_zones,
        session: None,
        opening: None,
        closing: None,
        reset_rx: false,
        close_reason: None,
        close_detail: None,
        available: rx.is_some(),
        pending: None,
        queue: VecDeque::new(),
    };
    'capture: loop {
        // Stop wins even if a full data queue left the main module blocked in emulation.
        if !st.reconcile_host(&mut authority).await {
            break;
        }
        lifecycle.refresh();
        st.reconcile_lifecycle(&lifecycle).await;
        if st.reset_rx {
            st.reset_rx = false;
            st.available = false;
            // Closing admission wakes producers blocked in send, so stop can join them.
            // Keep queued lifecycle signals until the joined generation is drained below.
            if let Some(old) = rx.as_mut() {
                old.close();
            }
            let stopped = st.cap.stop().await;
            if let Some(mut old) = rx.take() {
                // Producer is stopped before discarding its activation generation. Lifecycle
                // notifications survive the boundary; old input/activation never does.
                while let Ok(ev) = old.try_recv() {
                    if !matches!(
                        ev,
                        CaptureEvent::Input { .. } | CaptureEvent::Activated { .. }
                    ) {
                        st.event(ev).await;
                    }
                }
            }
            lifecycle.acknowledge_cleanup();
            st.snapshot_ready = false;
            st.reconcile_lifecycle(&lifecycle).await;
            // stop/join may have yielded while the caller cancelled or the module paused.
            // Observe those changes before installing another producer. This adapter is stopped:
            // retarget only updates the next start, and Switch fails via `available = false`.
            loop {
                match ctl.try_recv() {
                    Ok(Ctl::Stop) | Err(mpsc::error::TryRecvError::Disconnected) => break 'capture,
                    Ok(c) => st.control(c).await,
                    Err(mpsc::error::TryRecvError::Empty) => {
                        if st.pending.as_ref().is_some_and(|p| p.reply.is_closed()) {
                            st.end_session(true).await;
                            // Recheck controls that arrived while releasing the cancelled attempt.
                            continue;
                        }
                        break;
                    }
                }
            }
            if !st.reconcile_host(&mut authority).await {
                break;
            }
            if let Some(reply) = st.quiesce_reply.take() {
                let result = stopped
                    .as_ref()
                    .map(|_| ())
                    .map_err(|_| crate::inventory::unavailable());
                let _ = reply.send(result);
            }
            st.reset_rx = false;
            if let Err(e) = stopped {
                if let Some(reply) = st.prepare_reply.take() {
                    let _ = reply.send(Err(crate::inventory::unavailable()));
                }
                st.notes(Note::Block(msg::adapter_failed(&e.to_string())));
                st.close_reason = Some("error");
                st.close_detail = Some("capture_stop_failed".into());
                st.end_session(true).await;
                st.reset_rx = false;
            } else if st.prepare_reply.is_none()
                && !st.paused
                && !st.host.paused()
                && !st.locked
                && !st.targets.is_empty()
                && !lifecycle
                    .snapshot
                    .as_ref()
                    .is_some_and(|s| s.terminal_error.is_some())
            {
                // Locked screens need a fresh producer to observe unlock, with no crossing barriers.
                let observing_lock = lifecycle.snapshot.as_ref().map_or(st.session_locked, |s| {
                    s.session_locked == Some(true) || s.desktop_available == Some(false)
                });
                let specs: Vec<_> = if observing_lock {
                    Vec::new()
                } else {
                    st.targets.iter().map(|t| t.spec.clone()).collect()
                };
                // O lock herdado continua autoritativo no produtor sem barreiras.
                // Seu primeiro unlock exige outro start; nunca libera este rx para ativar.
                st.snapshot_ready = observing_lock && lifecycle.snapshot.is_none();
                st.session_locked = true;
                log_capture_start(st.cap.name());
                match st.cap.start(&specs).await {
                    Ok(new_rx) => {
                        if !observing_lock {
                            log_capture_ready(st.cap.name(), &st.targets);
                        }
                        rx = Some(new_rx);
                        lifecycle.reacquire(st.cap.as_ref());
                        st.reconcile_lifecycle(&lifecycle).await;
                        st.available = true;
                    }
                    Err(e) => {
                        log_capture_failed(st.cap.name(), &e);
                        st.notes(Note::Block(msg::adapter_failed(&e.to_string())));
                    }
                }
            }
        }
        if st.reset_rx {
            continue;
        }
        if let Some(mut reply) = st.prepare_reply.take() {
            // Borrow the existing adapter in a cancellable scope; no task can outlive its owner.
            enum Prepared {
                Done(Result<ObservedMonitors, InputError>),
                Control(Option<Ctl>),
                Cancel,
            }
            let peers: Vec<_> = st
                .host
                .peers()
                .iter()
                .map(|p| (p.id, st.host.peer_input_context(p.id)))
                .collect();
            let result = {
                let prepare = st.cap.prepare_monitor_observation();
                tokio::pin!(prepare);
                let mut check = tokio::time::interval(ZONES_POLL);
                loop {
                    tokio::select! {
                        biased;
                        c = ctl.recv() => break Prepared::Control(c),
                        () = reply.closed() => break Prepared::Cancel,
                        () = authority.changed() => break Prepared::Cancel,
                        () = lifecycle.changed() => break Prepared::Cancel,
                        _ = check.tick() => {
                            if st.host.paused() || peers.iter().any(|(p,c)| st.host.peer_input_context(*p) != *c) { break Prepared::Cancel; }
                        }
                        r = &mut prepare => break Prepared::Done(r),
                    }
                }
            };
            match result {
                Prepared::Done(r) => {
                    if let Ok(o) = &r {
                        st.observed = Some(o.clone());
                    }
                    let _ = reply.send(r);
                }
                Prepared::Control(c) => {
                    let _ = reply.send(Err(crate::inventory::unavailable()));
                    match c {
                        None | Some(Ctl::Stop) => break,
                        Some(c) => st.control(c).await,
                    }
                }
                Prepared::Cancel => {
                    let _ = reply.send(Err(crate::inventory::unavailable()));
                }
            }
            continue;
        }
        let switch_until = st.pending.as_ref().and_then(|p| p.until);
        tokio::select! {
            biased;
            c = ctl.recv() => {
                match c { Some(Ctl::Stop) | None => break, Some(c) => st.control(c).await }
            }
            () = async { if let Some(p) = st.pending.as_mut() { p.reply.closed().await; } }, if st.pending.is_some() => { st.end_session(true).await; }
            () = authority.changed() => continue,
            () = lifecycle.changed() => continue,
            _ = async { if let Some(f) = &mut st.closing { let _ = f.await; } }, if st.closing.is_some() => { st.closing = None; }
            ev = async { match rx.as_mut() { Some(r) => r.recv().await, None => None } }, if rx.is_some() => {
                match ev {
                    Some(ev) => {
                        if !st.reconcile_host(&mut authority).await { break; }
                        lifecycle.refresh();
                        st.reconcile_lifecycle(&lifecycle).await;
                        if !st.reset_rx { st.event(ev).await; }
                    },
                    None => {
                        st.notes(Note::Block(msg::adapter_failed("captura encerrada")));
                        st.close_reason = Some("error");
                        st.close_detail = Some("capture_ended".into());
                        st.end_session(true).await;
                        st.reset_rx = true;
                    }
                }
            }
            result = async {
                if let Some(o) = &mut st.opening { o.started = true; o.send.as_mut().await } else { Ok(()) }
            }, if st.opening.is_some() && st.closing.is_none() => {
                if !st.reconcile_host(&mut authority).await { break; }
                lifecycle.refresh();
                st.reconcile_lifecycle(&lifecycle).await;
                if !st.reset_rx { st.opened(result).await; }
            }
            _ = std::future::ready(()), if st.session.is_some() && !st.queue.is_empty() => {
                if !st.reconcile_host(&mut authority).await { break; }
                lifecycle.refresh();
                st.reconcile_lifecycle(&lifecycle).await;
                if st.reset_rx { continue; }
                if let Some(CaptureEvent::Input { t_mono_us, ev }) = st.queue.pop_front() { st.forward(t_mono_us, ev).await; }
            }
            _ = zones_tick.tick() => st.poll_zones().await,
            () = async { if let Some(until) = switch_until { tokio::time::sleep_until(until).await; } },
                if switch_until.is_some() => {
                if let Some(p) = st.pending.take() { let _ = p.reply.send(Ok(())); }
            }
        }
    }
    st.end_session(true).await;
    if let Some(old) = rx.as_mut() {
        old.close();
    }
    let _ = st.cap.stop().await;
    drop(rx);
    if let Some(closing) = st.closing.take() {
        let _ = closing.await;
    }
    st.cap
}

struct Loop {
    cleanup: DeferredCleanup,
    quiesce_reply: Option<oneshot::Sender<Result<(), InputError>>>,
    prepare_reply: Option<oneshot::Sender<Result<ObservedMonitors, InputError>>>,
    quiesced: bool,
    catalog: InventoryCatalog,
    route_plans: HashMap<usize, RoutePlan>,
    observed: Option<ObservedMonitors>,
    host: Arc<dyn ModuleHost>,
    cap: Box<dyn CaptureAdapter>,
    targets: Vec<Target>,
    notes: mpsc::UnboundedSender<Note>,
    locked: bool,
    paused: bool,
    /// This machine's screen is locked: no barrier may open a session (§13.6 item 6).
    session_locked: bool,
    snapshot_ready: bool,
    dead_corner_px: u32,
    peer_zones: HashMap<PeerId, Vec<ZoneInfo>>,
    /// The zones the module was last told about, to see a change by content.
    last_zones: Vec<Zone>,
    session: Option<Session>,
    opening: Option<Opening>,
    closing: Option<Sending>,
    reset_rx: bool,
    available: bool,
    pending: Option<PendingSwitch>,
    queue: VecDeque<CaptureEvent>,
    /// Why the session being closed ended; `close_session` logs and clears it.
    close_reason: Option<&'static str>,
    close_detail: Option<String>,
}

impl Loop {
    fn route_valid(&self, s: &RouteSession) -> bool {
        self.catalog.matches(&[
            InventoryStamp {
                peer: self.host.local_id(),
                connection: 0,
                rev: s.inventories.source,
            },
            InventoryStamp {
                peer: s.peer,
                connection: s.connection,
                rev: s.inventories.target,
            },
        ]) && self.cap.known_monitor_observation().ok().is_some_and(|o| {
            self.observed
                .as_ref()
                .is_some_and(|old| crate::inventory::same_observation(old, &o))
        })
    }
    async fn observe_monitors(&mut self) {
        match self.cap.known_monitor_observation() {
            Ok(o) => {
                if self
                    .observed
                    .as_ref()
                    .is_none_or(|old| !crate::inventory::same_observation(old, &o))
                {
                    if self.observed.is_some() {
                        self.catalog.invalidate(self.host.local_id());
                        self.end_session(true).await;
                        self.reset_rx = true;
                    }
                    self.observed = Some(o.clone());
                    self.notes(Note::Inventory(o));
                }
            }
            Err(_) => {
                self.catalog.invalidate(self.host.local_id());
                if self.current().is_some_and(|s| s.routes.is_some()) {
                    self.end_session(true).await;
                    self.reset_rx = true;
                }
                // State releases incoming emulation before scheduling preparation on this owner.
                if !self.quiesced
                    && self
                        .observed
                        .as_ref()
                        .is_some_and(|o| o.source == clipsync_proto::GeometrySource::Observed)
                {
                    self.notes(Note::PrepareInventory);
                }
            }
        }
    }
    async fn reconcile_host(&mut self, authority: &mut HostCursor) -> bool {
        let (_, invalidated) = authority.refresh();
        let state = authority.snapshot;
        if authority.observed()
            && (invalidated || (state.paused || self.quiesced) != self.paused || state.stopped)
        {
            self.paused = state.paused || state.stopped || self.quiesced;
            self.close_reason = Some(if state.stopped { "stop" } else { "pause" });
            self.end_session(true).await;
            self.reset_rx = true;
        }
        if self
            .current()
            .and_then(|s| s.routes.as_ref())
            .is_some_and(|r| !self.route_valid(r))
        {
            self.end_session(true).await;
            self.reset_rx = true;
        }
        // Losing/replacing the destination also cancels while the module cannot consume PeerDown.
        if self.current().is_some_and(|s| {
            self.host
                .peer_input_context(s.peer)
                .is_none_or(|c| !c.connected || c.connection != s.connection)
        }) {
            self.end_session(false).await;
        }
        !state.stopped
    }

    async fn reconcile_lifecycle(&mut self, lifecycle: &LifecycleCursor) {
        let Some(snapshot) = lifecycle.snapshot.as_ref() else {
            return;
        };
        let blocked = !snapshot.allows_capture();
        if lifecycle.pending_invalidation {
            self.session_locked = true;
            self.close_reason = Some("lock");
            self.end_session(true).await;
            self.queue.clear();
            self.reset_rx = true;
        }
        if self.snapshot_ready && self.session_locked != blocked {
            self.reset_rx = true;
        }
        self.session_locked = blocked;
        self.snapshot_ready = true;
        if blocked {
            self.close_reason = Some("lock");
            self.end_session(true).await;
        }
        if !same_zones(&self.last_zones, &snapshot.zones) && !snapshot.zones.is_empty() {
            self.last_zones = snapshot.zones.clone();
            self.notes(Note::Zones(snapshot.zones.clone()));
        }
        if let Some(error) = snapshot.terminal_error {
            self.notes(Note::Block(msg::adapter_failed(error)));
        }
    }

    fn notes(&self, n: Note) {
        let _ = self.notes.send(n);
    }

    fn peer_name(&self, id: PeerId) -> String {
        self.host
            .peers()
            .into_iter()
            .find(|p| p.id == id)
            .map_or_else(|| id.short(), |p| p.name)
    }

    async fn release(&mut self) {
        if let Err(e) = self.cap.release().await {
            tracing::debug!("release failed: {e}");
        }
    }

    /// Closes the open session: `Leave` to the peer (unless it refused us) and the cursor back.
    async fn end_session(&mut self, send_leave: bool) {
        self.close_session(send_leave, None).await;
    }

    /// Like `end_session`, but the cursor comes back at `(barrier, pos)` when given.
    async fn close_session(&mut self, send_leave: bool, back_at: Option<(usize, f32)>) {
        // The reason belongs to this close only: taken here so a close without a session never
        // hands it to the next one.
        let motivo = self.close_reason.take().unwrap_or("stop");
        let erro = self.close_detail.take().unwrap_or_default();
        let opening = self.opening.take();
        let had_opening = opening.is_some();
        let s = self.session.take();
        if s.is_none() && !had_opening {
            return;
        }
        // Release the physical device before cancelling or polling any network work.
        match back_at {
            Some((barrier, pos)) => {
                let _ = self.cap.release_at(barrier, pos).await;
            }
            None => self.release().await,
        }
        let cancelled = opening.map(|o| (o.session, o.started)); // drops the send future now
        let attempt = s.map(|s| (s, send_leave)).or(cancelled);
        if let Some((s, _)) = attempt.as_ref() {
            let totais = format!("{{\"enviados\":{},\"descartados\":{}}}", s.sent, s.dropped);
            tracing::info!(evento = "session.closed", attempt = s.attempt, session = s.id, peer = %s.peer, motivo, erro = %erro, totais = %totais, "session closed");
        }
        self.queue.clear();
        self.reset_rx = true;
        if let Some(p) = self.pending.take() {
            let _ = p.reply.send(Err(msg::SESSION_CANCELLED.to_owned()));
        }
        self.notes(Note::Active(None));
        if let Some((s, true)) = attempt {
            let host = self.host.clone();
            let release_all = self.session_locked;
            // At most one cleanup; a following Enter is never polled before it completes.
            self.closing = Some(Box::pin(async move {
                tokio::time::timeout(crate::network::SEND_WAIT, async move {
                    host.send_reliable_on(
                        s.peer,
                        s.connection,
                        ModuleMsg::Input(InputMsg::Leave { session: s.id }),
                    )
                    .await?;
                    if release_all {
                        host.send_reliable_on(
                            s.peer,
                            s.connection,
                            ModuleMsg::Input(InputMsg::ReleaseAll { session: s.id }),
                        )
                        .await?;
                    }
                    Ok(())
                })
                .await
                .unwrap_or_else(|_| {
                    Err(SendError::Transport(
                        "input cleanup deadline exceeded".into(),
                    ))
                })
            }));
        }
    }

    fn current(&self) -> Option<&Session> {
        self.session
            .as_ref()
            .or_else(|| self.opening.as_ref().map(|o| &o.session))
    }

    async fn opened(&mut self, result: Result<(), SendError>) {
        let Some((session, peer, attempt)) = self.current().map(|s| (s.id, s.peer, s.attempt))
        else {
            return;
        };
        if let Err(e) = result {
            let text = send_error_text(&e);
            self.notes(Note::Block(text));
            self.close_reason = Some("error");
            self.close_detail = Some(send_error_code(&e).to_owned());
            self.end_session(true).await;
            return;
        }
        tracing::info!(evento = "session.sent", attempt, session, peer = %peer, "entry sent");
        if self
            .current()
            .and_then(|s| s.routes.as_ref())
            .is_some_and(|r| !self.route_valid(r))
        {
            self.close_reason = Some("inventory");
            self.end_session(true).await;
            return;
        }
        if self.host.paused()
            || self.opening.as_ref().is_some_and(|o| {
                self.host
                    .peer_input_context(o.session.peer)
                    .is_none_or(|c| !c.connected || c.connection != o.session.connection)
            })
        {
            self.close_reason = Some(if self.host.paused() {
                "pause"
            } else {
                "peer_down"
            });
            self.end_session(true).await;
            return;
        }
        let Some(o) = self.opening.take() else {
            return;
        };
        self.session = Some(o.session);
        if let Some((barrier, pos)) = o.begin
            && let Err(e) = self.cap.begin(barrier, pos).await
        {
            self.notes(Note::Block(msg::adapter_failed(&e.to_string())));
            self.close_reason = Some("error");
            self.close_detail = Some("begin_failed".into());
            self.end_session(true).await;
            return;
        }
        tracing::info!(evento = "session.active", attempt, session, peer = %peer, "session active");
        if let Some(p) = self.pending.as_mut() {
            p.until = Some(Instant::now() + SWITCH_GRACE);
        }
        self.notes(Note::Active(Some(peer)));
        self.notes(Note::Event(msg::control_started(&self.peer_name(peer))));
    }

    /// Length in px of `edge` of the peer's monitor `monitor`, when the peer told us.
    fn peer_edge_len(&self, peer: PeerId, monitor: &str, edge: Edge) -> Option<u32> {
        let zones = self.peer_zones.get(&peer)?;
        let z = match zone_ref(monitor) {
            crate::adapter::ZoneRef::Index(i) => zones.iter().find(|z| z.index == i),
            crate::adapter::ZoneRef::Name(n) => {
                zones.iter().find(|z| z.name.as_deref() == Some(n.as_str()))
            }
        }?;
        Some(match edge {
            Edge::Left | Edge::Right => z.height,
            Edge::Top | Edge::Bottom => z.width,
        })
    }

    /// The way back sent with `EnterSpan`: the destination segment of the link, without the dead
    /// corners of the destination monitor (§13.6 item 3).
    fn back_span(&self, t: &Target) -> (f32, f32) {
        let (from, to) = t.remote;
        if from >= to {
            return (0.0, 1.0);
        }
        t.peer_monitor
            .as_deref()
            .and_then(|m| self.peer_edge_len(t.peer, m, t.peer_edge))
            .and_then(|len| trim_dead_corner(from, to, len, self.dead_corner_px))
            .unwrap_or((from, to))
    }

    /// Compares the adapter's zones with the last ones seen and tells the module on a change.
    async fn poll_zones(&mut self) {
        self.observe_monitors().await;
        let now = self.cap.known_zones();
        if !now.is_empty() && !same_zones(&self.last_zones, &now) {
            self.last_zones = now.clone();
            self.notes(Note::Zones(now));
        }
    }

    /// Opens a session on `t.peer` entering at `pos` (already in destination terms).
    async fn open(&mut self, t: &Target, pos: f32, attempt: u32) -> Result<(), OpenError> {
        if self.cleanup.busy() {
            // The adapter may already have captured physically in this new Engine.
            // Release before external network cleanup and retire this producer generation,
            // including queued activations; only a fresh activation may retry.
            self.release().await;
            self.queue.clear();
            self.reset_rx = true;
            return Err(OpenError::new("cleanup_pending", msg::SESSION_CANCELLED));
        }
        if self.current().is_some() {
            self.close_reason = Some("switch");
        }
        self.end_session(true).await;
        let id = new_session();
        let version = self
            .host
            .peers()
            .iter()
            .find(|p| p.id == t.peer)
            .map_or(0, |p| p.remote_version);
        let v3 = version >= SPAN_PROTOCOL;
        let plan = self
            .targets
            .iter()
            .position(|x| same_barrier(x, t))
            .and_then(|i| self.route_plans.get(&i))
            .cloned();
        if plan
            .as_ref()
            .is_some_and(|p| version < 4 || !self.route_valid(&p.session))
        {
            return Err(OpenError::new(
                "inventory_changed",
                "inventário de rota alterado",
            ));
        }
        let m = ModuleMsg::Input(match (&t.peer_monitor, version >= LAYOUT_PROTOCOL) {
            _ if plan.is_some() => {
                let mut entry = plan
                    .as_ref()
                    .map(|p| p.entry.clone())
                    .ok_or_else(|| OpenError::new("route_missing", "rota ausente"))?;
                entry.session = id;
                entry.pos = pos;
                InputMsg::EnterRoutes(entry)
            }
            // protocol 3: the way back travels with the entry and the destination decides it
            _ if v3 => {
                let (back_from, back_to) = self.back_span(t);
                InputMsg::EnterSpan {
                    session: id,
                    edge: t.peer_edge,
                    monitor: t.peer_monitor.clone(),
                    pos,
                    back_from,
                    back_to,
                }
            }
            // the destination monitor only travels to a peer that decodes EnterAt
            (Some(monitor), true) => InputMsg::EnterAt {
                session: id,
                edge: t.peer_edge,
                monitor: Some(monitor.clone()),
                pos,
            },
            _ => InputMsg::Enter {
                session: id,
                edge: t.peer_edge,
                pos,
            },
        });
        let connection = self
            .host
            .peer_input_context(t.peer)
            .filter(|c| c.connected)
            .ok_or_else(|| OpenError::new("peer_offline", msg::PEER_OFFLINE))?
            .connection;
        let host = self.host.clone();
        let peer = t.peer;
        let mensagem = match &m {
            ModuleMsg::Input(InputMsg::EnterRoutes(_)) => "enter_routes",
            ModuleMsg::Input(InputMsg::EnterSpan { .. }) => "enter_span",
            ModuleMsg::Input(InputMsg::EnterAt { .. }) => "enter_at",
            _ => "enter",
        };
        let rota = self
            .targets
            .iter()
            .position(|x| same_barrier(x, t))
            .map_or(-1, |i| i as i64);
        tracing::info!(evento = "session.open", attempt, session = id, peer = %peer, mensagem, rota, "session opening");
        self.opening = Some(Opening {
            session: Session {
                routes: plan.map(|p| RouteSession {
                    session: id,
                    ..p.session
                }),
                peer,
                connection,
                id,
                seq: 0,
                attempt,
                sent: 0,
                dropped: 0,
                cursor: (!v3).then(|| VirtualCursor::new(t.peer_edge)),
                target: t.clone(),
            },
            send: Box::pin(async move {
                tokio::time::timeout(
                    crate::network::SEND_WAIT,
                    host.send_reliable_on(peer, connection, m),
                )
                .await
                .unwrap_or_else(|_| {
                    Err(SendError::Transport(
                        "input opening deadline exceeded".into(),
                    ))
                })
            }),
            started: false,
            begin: None,
        });
        Ok(())
    }

    async fn event(&mut self, ev: CaptureEvent) {
        match ev {
            CaptureEvent::Activated { barrier, pos, .. } => {
                let attempt = next_attempt();
                tracing::info!(
                    evento = "attempt.begin",
                    attempt,
                    origem = "barrier",
                    "attempt begins"
                );
                match self.targets.get(barrier) {
                    Some(t) => {
                        tracing::info!(evento = "barrier.activated", attempt, barreira = barrier, peer = %t.peer, pos = frac3(pos), "barrier activated")
                    }
                    None => tracing::info!(
                        evento = "barrier.activated",
                        attempt,
                        barreira = barrier,
                        pos = frac3(pos),
                        "barrier activated"
                    ),
                }
                if self.locked || self.paused || self.session_locked {
                    self.release().await;
                    let (text, motivo) = if self.session_locked {
                        (msg::SOURCE_SCREEN_LOCKED.to_owned(), "screen_locked")
                    } else if self.locked {
                        (msg::LOCKED.to_owned(), "locked")
                    } else {
                        (RejectReason::Paused.pt().to_owned(), "paused")
                    };
                    log_filtered(attempt, motivo);
                    self.notes(Note::Event(text));
                    return;
                }
                let Some(t) = self.targets.get(barrier).cloned() else {
                    self.release().await;
                    log_filtered(attempt, "unknown_barrier");
                    return;
                };
                let dest = map_pos(t.local, t.remote, pos);
                if let Err(e) = self.open(&t, dest, attempt).await {
                    log_filtered(attempt, e.code);
                }
            }
            CaptureEvent::Input { t_mono_us, ev } => {
                if self.opening.is_some() {
                    if self.queue.len() == OPEN_BUFFER {
                        self.notes(Note::Block(msg::OPEN_BUFFER_FULL.to_owned()));
                        self.close_reason = Some("error");
                        self.close_detail = Some("open_buffer_full".into());
                        self.end_session(true).await;
                    } else {
                        self.queue.push_back(CaptureEvent::Input { t_mono_us, ev });
                    }
                    return;
                }
                if self.session.is_none() {
                    return;
                }
                if self.queue.is_empty() {
                    self.forward(t_mono_us, ev).await;
                } else {
                    let oldest = self.queue.pop_front();
                    self.queue.push_back(CaptureEvent::Input { t_mono_us, ev });
                    if let Some(CaptureEvent::Input { t_mono_us, ev }) = oldest {
                        self.forward(t_mono_us, ev).await;
                    }
                }
            }
            CaptureEvent::Deactivated => {
                if self.current().is_some() {
                    self.close_reason = Some("stop");
                    self.end_session(true).await;
                }
            }
            CaptureEvent::Error(e) => {
                self.notes(Note::Block(msg::adapter_failed(&e)));
                self.close_reason = Some("error");
                self.close_detail = Some("capture_error".into());
                self.end_session(true).await;
            }
            CaptureEvent::Hotkey(h) => self.notes(Note::Hotkey(h)),
            CaptureEvent::ZonesChanged(z) => {
                if !z.is_empty() && !same_zones(&self.last_zones, &z) {
                    self.last_zones = z.clone();
                    self.notes(Note::Zones(z));
                }
            }
            CaptureEvent::SessionLocked(on) => {
                if self.snapshot_ready && self.session_locked != on {
                    self.reset_rx = true;
                }
                self.snapshot_ready = true;
                self.session_locked = on;
                if on {
                    self.close_for_lock().await;
                    self.notes(Note::Event(msg::SOURCE_SCREEN_LOCKED.to_owned()));
                }
            }
        }
    }

    async fn forward(&mut self, t_mono_us: u64, ev: InputEvent) {
        let events = vec![ev];
        let mut back = false;
        let Some(s) = self.session.as_mut() else {
            return;
        };
        if let Some(cursor) = s.cursor.as_mut() {
            for e in &events {
                if let InputEvent::Motion { dx, dy } = e {
                    back |= cursor.step(*dx, *dy);
                }
            }
        }
        let dg = InputDatagram {
            session: s.id,
            seq: s.seq,
            t_mono_us,
            events,
        };
        let first = s.sent == 0;
        s.seq = s.seq.wrapping_add(1);
        let peer = s.peer;
        let session = s.id;
        match self.host.send_datagram(peer, dg) {
            Ok(()) => {
                s.sent += 1;
                if first {
                    tracing::info!(evento = "datagram.first", session, peer = %peer, "first datagram sent");
                }
            }
            Err(e) => {
                s.dropped += 1;
                tracing::warn!("datagram not sent: {e}");
            }
        }
        if back {
            let name = self.peer_name(peer);
            self.close_reason = Some("return");
            self.end_session(true).await;
            self.notes(Note::Event(msg::returned_from(&name)));
        }
    }

    /// This machine's screen locked: `ReleaseAll` and `Leave` to the destination, cursor back
    /// (§13.6 item 6).
    async fn close_for_lock(&mut self) {
        self.close_reason = Some("lock");
        self.end_session(true).await;
    }

    async fn control(&mut self, c: Ctl) {
        match c {
            Ctl::KnownInventory { reply } => {
                let _ = reply.send(self.cap.known_monitor_observation());
            }
            Ctl::Ready { reply } => {
                let _ = reply.send(
                    self.available
                        && !self.paused
                        && self
                            .route_plans
                            .values()
                            .all(|p| self.route_valid(&p.session)),
                );
            }
            Ctl::Quiesce { reply } => {
                self.quiesced = true;
                self.paused = true;
                self.reset_rx = true;
                self.queue.clear();
                if let Some(closing) = self.closing.take() {
                    self.cleanup.push(closing);
                }
                self.end_session(true).await;
                if let Some(closing) = self.closing.take() {
                    self.cleanup.push(closing);
                }
                // The outer loop closes rx before stop/join and acknowledges only afterward.
                self.available = false;
                self.quiesce_reply = Some(reply);
            }
            Ctl::PrepareInventory { reply } => {
                self.catalog.invalidate(self.host.local_id());
                self.quiesced = true;
                self.paused = true;
                if let Some(closing) = self.closing.take() {
                    self.cleanup.push(closing);
                }
                self.end_session(true).await;
                if let Some(closing) = self.closing.take() {
                    self.cleanup.push(closing);
                }
                self.reset_rx = true;
                self.prepare_reply = Some(reply);
            }
            Ctl::InvalidateRoutes => {
                if self.current().is_some_and(|s| s.routes.is_some()) {
                    self.close_reason = Some("inventory");
                    self.end_session(true).await;
                }
                self.reset_rx = true;
            }
            Ctl::RoutePlans(plans) => {
                self.close_reason = Some("inventory");
                self.end_session(true).await;
                self.reset_rx = true;
                self.route_plans = plans;
            }
            Ctl::ReturnedRoute {
                from,
                context,
                returned,
            } => {
                let resolved = self
                    .current()
                    .and_then(|s| s.routes.as_ref())
                    .filter(|s| self.route_valid(s) && !self.locked)
                    .and_then(|s| s.resolve(from, context.connection, &returned, s.inventories));
                if let Some((barrier, pos)) = resolved {
                    if let Some(s) = self.current() {
                        tracing::info!(evento = "return.received", session = s.id, peer = %from, pos = frac3(pos), "cursor returned");
                    }
                    self.close_reason = Some("return");
                    self.close_session(true, Some((barrier, pos))).await;
                } else {
                    self.notes(Note::Ignored);
                }
            }
            Ctl::Returned { from, session, pos } => {
                let Some(s) = self
                    .current()
                    .filter(|s| s.peer == from && s.id == session && s.routes.is_none())
                else {
                    self.notes(Note::Ignored);
                    return;
                };
                // With the lock on, the destination keeps the cursor.
                if self.locked {
                    return;
                }
                let t = s.target.clone();
                let span = t.remote.1 - t.remote.0;
                let k = if span > 0.0 {
                    ((pos - t.remote.0) / span).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let back = t.local.0 + k * (t.local.1 - t.local.0);
                let barrier = self.targets.iter().position(|x| same_barrier(x, &t));
                let name = self.peer_name(from);
                tracing::info!(evento = "return.received", session, peer = %from, pos = frac3(pos), "cursor returned");
                self.close_reason = Some("return");
                self.close_session(true, barrier.map(|i| (i, back))).await;
                self.notes(Note::Event(msg::returned_from(&name)));
            }
            Ctl::Refused {
                from,
                session,
                reason,
            } => {
                if self
                    .current()
                    .is_some_and(|s| s.peer == from && s.id == session)
                {
                    // The peer refused: no `Leave`, just the cursor back.
                    tracing::info!(evento = "session.refused", session, peer = %from, motivo = reject_code(reason), "entry refused by the peer");
                    self.close_reason = Some("refused");
                    self.end_session(false).await;
                    let text = msg::refused_by(&self.peer_name(from), reason);
                    self.notes(Note::Block(text.clone()));
                    if self.pending.as_ref().is_some_and(|p| p.peer == from)
                        && let Some(p) = self.pending.take()
                    {
                        let _ = p.reply.send(Err(text));
                    }
                } else {
                    self.notes(Note::Ignored);
                }
            }
            Ctl::PeerZones(peer, zones) => {
                self.peer_zones.insert(peer, zones);
            }
            Ctl::Retarget(targets) => {
                if self.current().is_some_and(|s| s.routes.is_some()) {
                    self.end_session(true).await;
                    self.reset_rx = true;
                }
                if self
                    .current()
                    .is_some_and(|s| !targets.iter().any(|t| same_barrier(t, &s.target)))
                {
                    self.close_reason = Some("inventory");
                    self.end_session(true).await;
                }
                self.targets = targets;
                let specs: Vec<BarrierSpec> = self.targets.iter().map(|t| t.spec.clone()).collect();
                if self.available
                    && let Err(e) = self.cap.set_barriers(&specs).await
                {
                    tracing::debug!("set_barriers failed: {e}");
                    self.notes(Note::Restart);
                }
            }
            Ctl::Home => {
                if self.current().is_some() {
                    self.end_session(true).await;
                }
            }
            Ctl::Lock(on) => {
                self.locked = on;
                if !on || self.session_locked {
                    self.reset_rx = true;
                }
                if on {
                    self.close_reason = Some("lock");
                    self.end_session(true).await;
                }
            }
            Ctl::Paused(on) => {
                // Queued compatibility values can be older than the independent authority.
                if self.host.lifecycle_receiver().is_some() {
                    return;
                }
                self.paused = on || self.quiesced;
                self.reset_rx = true;
                if on {
                    self.close_reason = Some("pause");
                    self.end_session(true).await;
                }
            }
            Ctl::Rejected { from, reason } => {
                if let Some(session) = self.current().filter(|s| s.peer == from).map(|s| s.id) {
                    tracing::info!(evento = "session.refused", session, peer = %from, motivo = reject_code(reason), "entry rejected by the peer");
                    self.close_reason = Some("refused");
                    self.end_session(false).await;
                    self.notes(Note::Block(reason.pt().to_owned()));
                }
                if self.pending.as_ref().is_some_and(|p| p.peer == from)
                    && let Some(p) = self.pending.take()
                {
                    let _ = p.reply.send(Err(reason.pt().to_owned()));
                }
            }
            Ctl::PeerDown(id) => {
                if self.current().is_some_and(|s| s.peer == id) {
                    self.close_reason = Some("peer_down");
                    self.end_session(false).await;
                    self.notes(Note::Block(msg::PEER_OFFLINE.to_owned()));
                }
                if self.pending.as_ref().is_some_and(|p| p.peer == id)
                    && let Some(p) = self.pending.take()
                {
                    let _ = p.reply.send(Err(msg::PEER_OFFLINE.to_owned()));
                }
            }
            Ctl::Switch { to, reply } => {
                let res = self.switch(to).await;
                match res {
                    Ok(()) => {
                        if let Some(p) = self.pending.take() {
                            let _ = p.reply.send(Err(msg::SESSION_CANCELLED.to_owned()));
                        }
                        self.pending = Some(PendingSwitch {
                            peer: to,
                            reply,
                            until: None,
                        });
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Ctl::Stop => {}
        }
    }

    async fn switch(&mut self, to: PeerId) -> Result<(), String> {
        let attempt = next_attempt();
        tracing::info!(evento = "attempt.begin", attempt, origem = "switch", peer = %to, "attempt begins");
        let filtered = |motivo: &'static str, text: String| {
            log_filtered(attempt, motivo);
            Err(text)
        };
        // A deliberate gate remains the reason after cancellation stops the producer.
        if self.locked {
            return filtered("locked", msg::LOCKED.to_owned());
        }
        if self.paused {
            return filtered("paused", RejectReason::Paused.pt().to_owned());
        }
        if !self.available {
            return filtered(
                "capture_unavailable",
                msg::adapter_failed("captura indisponível"),
            );
        }
        if self.session_locked {
            return filtered("screen_locked", msg::SOURCE_SCREEN_LOCKED.to_owned());
        }
        let Some(t) = self.targets.iter().find(|t| t.peer == to).cloned() else {
            return filtered("not_linked", msg::NOT_LINKED.to_owned());
        };
        if !self.host.peers().iter().any(|p| p.id == to && p.connected) {
            return filtered("peer_offline", msg::PEER_OFFLINE.to_owned());
        }
        if !self.cap.can_begin() {
            return filtered("switch_unavailable", msg::SWITCH_UNAVAILABLE.to_owned());
        }
        let middle = (t.remote.0 + t.remote.1) / 2.0;
        if let Err(e) = self.open(&t, middle, attempt).await {
            log_filtered(attempt, e.code);
            return Err(e.into());
        }
        // The session is open: now the local keyboard and mouse have to follow it (a crossing
        // does this by itself; a switch does not cross anything).
        let barrier = self
            .targets
            .iter()
            .position(|x| same_barrier(x, &t))
            .unwrap_or(0);
        let here = (t.local.0 + t.local.1) / 2.0;
        if let Some(o) = self.opening.as_mut() {
            o.begin = Some((barrier, here));
        }

        Ok(())
    }
}

/// The same barrier on both sides of a retarget: same peer, edge and segments.
fn same_barrier(a: &Target, b: &Target) -> bool {
    a.peer == b.peer
        && a.peer_monitor == b.peer_monitor
        && a.peer_edge == b.peer_edge
        && a.local == b.local
        && a.remote == b.remote
        && a.spec.edge == b.spec.edge
        && a.spec.from == b.spec.from
        && a.spec.to == b.spec.to
        && match (&a.spec.zone, &b.spec.zone) {
            (crate::adapter::ZoneRef::Index(a), crate::adapter::ZoneRef::Index(b)) => a == b,
            (crate::adapter::ZoneRef::Name(a), crate::adapter::ZoneRef::Name(b)) => a == b,
            _ => false,
        }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod backpressure {
    include!("../tests/support/capture_backpressure.rs");
}
