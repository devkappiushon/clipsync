//! Continuous capture (contract §12.6 item 2, §12.10 decision 9). The barriers derived from
//! the links in `layout.toml` stay installed while the agent runs. Crossing one opens a control
//! session on the linked peer; pushing the cursor back through the entry edge, a refusal, a
//! pause, the lock or the peer going down closes it and gives the cursor back.
use crate::adapter::{BarrierSpec, CaptureAdapter, CaptureEvent, InputError};
use crate::config::map_pos;
use crate::msg;
use clipsync_proto::{
    Edge, EventId, InputDatagram, InputEvent, InputMsg, MAX_EVENTS_PER_DATAGRAM, ModuleHost,
    ModuleMsg, PeerId, RejectReason, SendError,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

/// Pushing this far past the entry edge brings the cursor back; smaller moves are jitter.
const RETURN_SLACK_PX: f32 = 8.0;
/// Keeps the tracked depth finite without the peer's zones.
const MAX_DEPTH_PX: f32 = 1.0e7;
/// A refusal of the Enter arrives after the send: a switch waits this long for it.
const SWITCH_GRACE: Duration = Duration::from_millis(400);
/// Protocol version that understands `Zones`, `LayoutOffer` and `EnterAt` (§12.10 decision 7).
pub(crate) const LAYOUT_PROTOCOL: u16 = 2;

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
    /// Close the session and hand the adapter back.
    Stop,
}

/// What the module shows in `ModuleStatus` and `InputState`.
pub(crate) enum Note {
    Event(String),
    Block(String),
    Active(Option<PeerId>),
    /// The live zones once the capture session exists.
    Zones(Vec<crate::adapter::Zone>),
}

pub(crate) struct Engine {
    pub host: Arc<dyn ModuleHost>,
    pub cap: Box<dyn CaptureAdapter>,
    pub targets: Vec<Target>,
    pub ctl: mpsc::UnboundedReceiver<Ctl>,
    pub notes: mpsc::UnboundedSender<Note>,
    pub locked: bool,
    pub paused: bool,
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
    cursor: VirtualCursor,
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
    if rx.is_some() {
        let _ = notes.send(Note::Zones(cap.known_zones()));
        let _ = notes.send(Note::Event(msg::capture_ready(targets.len())));
    }
    let mut st = Loop {
        host,
        cap,
        targets,
        notes,
        locked,
        paused,
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
        let Some(s) = self.session.take() else { return };
        if send_leave {
            let m = ModuleMsg::Input(InputMsg::Leave { session: s.id });
            if let Err(e) = self.host.send_reliable(s.peer, m).await {
                tracing::debug!("leave not sent: {e}");
            }
        }
        self.release().await;
        self.notes(Note::Active(None));
    }

    /// Opens a session on `t.peer` entering at `pos` (already in destination terms).
    async fn open(&mut self, t: &Target, pos: f32) -> Result<(), String> {
        self.end_session(true).await;
        let id = new_session();
        let speaks_v2 = self
            .host
            .peers()
            .iter()
            .any(|p| p.id == t.peer && p.remote_version >= LAYOUT_PROTOCOL);
        let m = ModuleMsg::Input(match (&t.peer_monitor, speaks_v2) {
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
            cursor: VirtualCursor::new(t.peer_edge),
        });
        let name = self.peer_name(t.peer);
        self.notes(Note::Active(Some(t.peer)));
        self.notes(Note::Event(msg::control_started(&name)));
        Ok(())
    }

    async fn event(&mut self, ev: CaptureEvent, rx: Option<&mut mpsc::Receiver<CaptureEvent>>) {
        match ev {
            CaptureEvent::Activated { barrier, pos, .. } => {
                if self.locked || self.paused {
                    self.release().await;
                    let text = if self.locked {
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
                for e in &events {
                    if let InputEvent::Motion { dx, dy } = e {
                        back |= s.cursor.step(*dx, *dy);
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
        }
    }

    async fn control(&mut self, c: Ctl) {
        match c {
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
