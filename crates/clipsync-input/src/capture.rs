//! Continuous capture (contract §12.6 item 2, §12.10 decision 9). The barriers derived from
//! the links in `layout.toml` stay installed while the agent runs. Crossing one opens a control
//! session on the linked peer; pushing the cursor back through the entry edge, a refusal, a
//! pause, the lock or the peer going down closes it and gives the cursor back.
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, Hotkey, InputError, Zone, same_zones,
};
use crate::config::{map_pos, zone_ref};
use crate::layout::trim_dead_corner;
use crate::msg;
use clipsync_proto::{
    Edge, EventId, InputDatagram, InputEvent, InputMsg, MAX_EVENTS_PER_DATAGRAM, ModuleHost,
    ModuleMsg, PeerId, RejectReason, SendError, ZoneInfo,
};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

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

pub(crate) enum Ctl {
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
}

pub(crate) struct Engine {
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
    peer: PeerId,
    id: u32,
    seq: u32,
    /// Protocol 2 peers only; with protocol 3 the destination decides the way back.
    cursor: Option<VirtualCursor>,
    target: Target,
}

struct PendingSwitch {
    peer: PeerId,
    reply: oneshot::Sender<Result<(), String>>,
    until: Instant,
}

pub(crate) fn new_session() -> u32 {
    let b = EventId::random().0;
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]).max(1)
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

/// Runs until `Ctl::Stop`; returns the adapter so the module can reuse it.
pub(crate) async fn run(engine: Engine) -> Box<dyn CaptureAdapter> {
    let Engine {
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
    let mut rx = match cap.start(&specs).await {
        Ok(rx) => Some(rx),
        Err(e) => {
            let text = match e {
                InputError::PermissionDenied => msg::PORTAL_DENIED.to_owned(),
                other => msg::adapter_failed(&other.to_string()),
            };
            let _ = notes.send(Note::Block(text));
            None
        }
    };
    let last_zones = cap.known_zones();
    if rx.is_some() {
        let _ = notes.send(Note::Zones(last_zones.clone()));
        let _ = notes.send(Note::Event(msg::capture_ready(targets.len())));
    }
    let mut zones_tick = tokio::time::interval(ZONES_POLL);
    zones_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut st = Loop {
        host,
        cap,
        targets,
        notes,
        locked,
        paused,
        session_locked: false,
        dead_corner_px,
        peer_zones,
        last_zones,
        session: None,
        pending: None,
        queue: VecDeque::new(),
    };
    loop {
        let ev = if let Some(e) = st.queue.pop_front() {
            Some(e)
        } else {
            tokio::select! {
                e = async { match rx.as_mut() { Some(r) => r.recv().await, None => None } },
                    if rx.is_some() =>
                {
                    match e {
                        Some(e) => Some(e),
                        None => {
                            // The adapter is gone; wait for Stop so the module can rebuild.
                            rx = None;
                            st.notes(Note::Block(msg::adapter_failed("captura encerrada")));
                            st.end_session(true).await;
                            continue;
                        }
                    }
                }
                c = ctl.recv() => {
                    match c {
                        Some(Ctl::Stop) | None => break,
                        Some(c) => st.control(c).await,
                    }
                    continue;
                }
                _ = zones_tick.tick() => {
                    st.poll_zones();
                    continue;
                }
                () = async { if let Some(p) = &st.pending { tokio::time::sleep_until(p.until).await } },
                    if st.pending.is_some() =>
                {
                    if let Some(p) = st.pending.take() {
                        let _ = p.reply.send(Ok(()));
                    }
                    continue;
                }
            }
        };
        if let Some(ev) = ev {
            st.event(ev, rx.as_mut()).await;
        }
    }
    st.end_session(true).await;
    let _ = st.cap.stop().await;
    st.cap
}

struct Loop {
    host: Arc<dyn ModuleHost>,
    cap: Box<dyn CaptureAdapter>,
    targets: Vec<Target>,
    notes: mpsc::UnboundedSender<Note>,
    locked: bool,
    paused: bool,
    /// This machine's screen is locked: no barrier may open a session (§13.6 item 6).
    session_locked: bool,
    dead_corner_px: u32,
    peer_zones: HashMap<PeerId, Vec<ZoneInfo>>,
    /// The zones the module was last told about, to see a change by content.
    last_zones: Vec<Zone>,
    session: Option<Session>,
    pending: Option<PendingSwitch>,
    queue: VecDeque<CaptureEvent>,
}

impl Loop {
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
        let Some(s) = self.session.take() else { return };
        if send_leave {
            let m = ModuleMsg::Input(InputMsg::Leave { session: s.id });
            if let Err(e) = self.host.send_reliable(s.peer, m).await {
                tracing::debug!("leave not sent: {e}");
            }
        }
        match back_at {
            Some((barrier, pos)) => {
                if let Err(e) = self.cap.release_at(barrier, pos).await {
                    tracing::debug!("release_at failed: {e}");
                }
            }
            None => self.release().await,
        }
        self.notes(Note::Active(None));
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
    fn poll_zones(&mut self) {
        let now = self.cap.known_zones();
        if !now.is_empty() && !same_zones(&self.last_zones, &now) {
            self.last_zones = now.clone();
            self.notes(Note::Zones(now));
        }
    }

    /// Opens a session on `t.peer` entering at `pos` (already in destination terms).
    async fn open(&mut self, t: &Target, pos: f32) -> Result<(), String> {
        self.end_session(true).await;
        let id = new_session();
        let version = self
            .host
            .peers()
            .iter()
            .find(|p| p.id == t.peer)
            .map_or(0, |p| p.remote_version);
        let v3 = version >= SPAN_PROTOCOL;
        let m = ModuleMsg::Input(match (&t.peer_monitor, version >= LAYOUT_PROTOCOL) {
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
        if let Err(e) = self.host.send_reliable(t.peer, m).await {
            self.release().await;
            let text = send_error_text(&e);
            self.notes(Note::Block(text.clone()));
            return Err(text);
        }
        self.session = Some(Session {
            peer: t.peer,
            id,
            seq: 0,
            cursor: (!v3).then(|| VirtualCursor::new(t.peer_edge)),
            target: t.clone(),
        });
        let name = self.peer_name(t.peer);
        self.notes(Note::Active(Some(t.peer)));
        self.notes(Note::Event(msg::control_started(&name)));
        Ok(())
    }

    async fn event(&mut self, ev: CaptureEvent, rx: Option<&mut mpsc::Receiver<CaptureEvent>>) {
        match ev {
            CaptureEvent::Activated { barrier, pos, .. } => {
                if self.locked || self.paused || self.session_locked {
                    self.release().await;
                    let text = if self.session_locked {
                        msg::SOURCE_SCREEN_LOCKED.to_owned()
                    } else if self.locked {
                        msg::LOCKED.to_owned()
                    } else {
                        RejectReason::Paused.pt().to_owned()
                    };
                    self.notes(Note::Event(text));
                    return;
                }
                let Some(t) = self.targets.get(barrier).cloned() else {
                    self.release().await;
                    return;
                };
                let dest = map_pos(t.local, t.remote, pos);
                let _ = self.open(&t, dest).await;
            }
            CaptureEvent::Input { t_mono_us, ev } => {
                if self.session.is_none() {
                    return;
                }
                let mut events = vec![ev];
                if let Some(rx) = rx {
                    while events.len() < MAX_EVENTS_PER_DATAGRAM {
                        match rx.try_recv() {
                            Ok(CaptureEvent::Input { ev, .. }) => events.push(ev),
                            Ok(other) => {
                                self.queue.push_back(other);
                                break;
                            }
                            Err(_) => break,
                        }
                    }
                }
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
                s.seq = s.seq.wrapping_add(1);
                let peer = s.peer;
                if let Err(e) = self.host.send_datagram(peer, dg) {
                    tracing::warn!("datagram not sent: {e}");
                }
                if back {
                    let name = self.peer_name(peer);
                    self.end_session(true).await;
                    self.notes(Note::Event(msg::returned_from(&name)));
                }
            }
            CaptureEvent::Deactivated => {
                if self.session.is_some() {
                    self.end_session(true).await;
                }
            }
            CaptureEvent::Error(e) => {
                self.notes(Note::Block(msg::adapter_failed(&e)));
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
                self.session_locked = on;
                if on {
                    self.close_for_lock().await;
                    self.notes(Note::Event(msg::SOURCE_SCREEN_LOCKED.to_owned()));
                }
            }
        }
    }

    /// This machine's screen locked: `ReleaseAll` and `Leave` to the destination, cursor back
    /// (§13.6 item 6).
    async fn close_for_lock(&mut self) {
        let Some(s) = &self.session else { return };
        let m = ModuleMsg::Input(InputMsg::ReleaseAll { session: s.id });
        if let Err(e) = self.host.send_reliable(s.peer, m).await {
            tracing::debug!("release-all not sent: {e}");
        }
        self.end_session(true).await;
    }

    async fn control(&mut self, c: Ctl) {
        match c {
            Ctl::Returned { from, session, pos } => {
                let Some(s) = self
                    .session
                    .as_ref()
                    .filter(|s| s.peer == from && s.id == session)
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
                self.close_session(true, barrier.map(|i| (i, back))).await;
                self.notes(Note::Event(msg::returned_from(&name)));
            }
            Ctl::Refused {
                from,
                session,
                reason,
            } => {
                if self
                    .session
                    .as_ref()
                    .is_some_and(|s| s.peer == from && s.id == session)
                {
                    // The peer refused: no `Leave`, just the cursor back.
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
                self.targets = targets;
                let specs: Vec<BarrierSpec> = self.targets.iter().map(|t| t.spec.clone()).collect();
                if let Err(e) = self.cap.set_barriers(&specs).await {
                    tracing::debug!("set_barriers failed: {e}");
                    self.notes(Note::Restart);
                }
            }
            Ctl::Home => {
                if self.session.is_some() {
                    self.end_session(true).await;
                }
            }
            Ctl::Lock(on) => {
                self.locked = on;
                if on {
                    self.end_session(true).await;
                }
            }
            Ctl::Paused(on) => {
                self.paused = on;
                if on {
                    self.end_session(true).await;
                }
            }
            Ctl::Rejected { from, reason } => {
                if self.session.as_ref().is_some_and(|s| s.peer == from) {
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
                if self.session.as_ref().is_some_and(|s| s.peer == id) {
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
                            let _ = p.reply.send(Ok(()));
                        }
                        self.pending = Some(PendingSwitch {
                            peer: to,
                            reply,
                            until: Instant::now() + SWITCH_GRACE,
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
        if self.locked {
            return Err(msg::LOCKED.to_owned());
        }
        if self.paused {
            return Err(RejectReason::Paused.pt().to_owned());
        }
        if self.session_locked {
            return Err(msg::SOURCE_SCREEN_LOCKED.to_owned());
        }
        let Some(t) = self.targets.iter().find(|t| t.peer == to).cloned() else {
            return Err(msg::NOT_LINKED.to_owned());
        };
        if !self.host.peers().iter().any(|p| p.id == to && p.connected) {
            return Err(msg::PEER_OFFLINE.to_owned());
        }
        let middle = (t.remote.0 + t.remote.1) / 2.0;
        self.open(&t, middle).await
    }
}

/// The same barrier on both sides of a retarget: same peer, edge and segments.
fn same_barrier(a: &Target, b: &Target) -> bool {
    a.peer == b.peer
        && a.peer_monitor == b.peer_monitor
        && a.peer_edge == b.peer_edge
        && a.local == b.local
        && a.remote == b.remote
}
