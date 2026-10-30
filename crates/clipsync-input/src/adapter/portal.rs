//! Wayland adapters: xdg-desktop-portal InputCapture / RemoteDesktop (ashpd) plus libei/EIS (reis).
//!
//! Hard rules from the spikes: never call `supported_capabilities()` (GNOME 50 returns 15, an
//! unknown bit, and the parse fails); the right-edge barrier sits at `x = zone.x + width`; the
//! RemoteDesktop EIS offers relative motion only.
use super::session::LockWatch;
use super::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, InputError, TokenStore, Zone,
    ZoneRef, barrier_segment, edge_position,
};
use ashpd::desktop::input_capture::{
    Barrier, Capabilities, ConnectToEISOptions, CreateSessionOptions, EnableOptions,
    GetZonesOptions, InputCapture, ReleaseOptions, SetPointerBarriersOptions,
};
use ashpd::desktop::remote_desktop::{
    ConnectToEISOptions as RdConnectOptions, DeviceType, RemoteDesktop, SelectDevicesOptions,
    StartOptions,
};
use ashpd::desktop::{CreateSessionOptions as RdCreateOptions, PersistMode, Session};
use async_trait::async_trait;
use clipsync_proto::{Edge, InputEvent, mono_us};
use futures_util::StreamExt;
use reis::ei;
use reis::event::{DeviceCapability, EiEvent};
use std::collections::HashSet;
use std::num::NonZero;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

const INWARD_PX: f32 = 50.0;
const DEVICE_WAIT: Duration = Duration::from_secs(5);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn map_err(e: ashpd::Error) -> InputError {
    use ashpd::desktop::ResponseError;
    match e {
        ashpd::Error::Response(ResponseError::Cancelled) => InputError::PermissionDenied,
        ashpd::Error::PortalNotFound(_) | ashpd::Error::RequiresVersion(..) => {
            InputError::PortalUnavailable(e.to_string())
        }
        other => InputError::Other(other.to_string()),
    }
}

fn eis_err(e: impl std::fmt::Display) -> InputError {
    InputError::Eis(e.to_string())
}

// ---------------------------------------------------------------- capture

/// Where the cursor goes back to on `release()`.
#[derive(Clone, Copy, Default)]
struct ActiveInfo {
    activation_id: Option<u32>,
    cursor: Option<(f64, f64)>,
}

#[derive(Clone)]
struct Geom {
    zone: Zone,
    edge: Edge,
}

pub struct PortalCapture {
    ic: Option<InputCapture>,
    session: Option<Session<InputCapture>>,
    zone_set: u32,
    zones: Vec<Zone>,
    /// The portal said the zones changed (`ZonesChanged`): the next `zones()` asks again.
    zones_dirty: Arc<AtomicBool>,
    zones_task: Option<tokio::task::JoinHandle<()>>,
    /// The barriers the capture loop maps an activation through; `set_barriers` swaps them.
    geoms: Arc<Mutex<Vec<Geom>>>,
    active: Arc<Mutex<ActiveInfo>>,
    thread: Option<std::thread::JoinHandle<()>>,
    stop: Option<oneshot::Sender<()>>,
    lock_watch: Option<LockWatch>,
}

impl Default for PortalCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PortalCapture {
    pub fn new() -> Self {
        Self {
            ic: None,
            session: None,
            zone_set: 0,
            zones: Vec::new(),
            zones_dirty: Arc::new(AtomicBool::new(false)),
            zones_task: None,
            geoms: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(Mutex::new(ActiveInfo::default())),
            thread: None,
            stop: None,
            lock_watch: None,
        }
    }

    /// Creates the session (dialog on first use) and reads the zones.
    async fn ensure_session(&mut self) -> Result<(), InputError> {
        if self.session.is_some() {
            return Ok(());
        }
        let ic = InputCapture::new().await.map_err(map_err)?;
        let (session, _granted) = ic
            .create_session(
                None,
                CreateSessionOptions::default()
                    .set_capabilities(Capabilities::Keyboard | Capabilities::Pointer),
            )
            .await
            .map_err(map_err)?;
        self.ic = Some(ic);
        self.session = Some(session);
        self.refresh_zones().await
    }

    /// Asks the portal for the zones again (a new `zone_set` after a change).
    async fn refresh_zones(&mut self) -> Result<(), InputError> {
        let (Some(ic), Some(session)) = (&self.ic, &self.session) else {
            return Err(InputError::Other("no session".into()));
        };
        let zones = ic
            .zones(session, GetZonesOptions::default())
            .await
            .map_err(map_err)?
            .response()
            .map_err(map_err)?;
        self.zone_set = zones.zone_set();
        let outputs = super::outputs::connected_outputs();
        self.zones = zones
            .regions()
            .iter()
            .enumerate()
            .map(|(i, r)| Zone {
                index: i as u32,
                name: (outputs.len() == zones.regions().len())
                    .then(|| outputs.get(i).cloned())
                    .flatten(),
                x: r.x_offset(),
                y: r.y_offset(),
                width: r.width(),
                height: r.height(),
                scale: 1.0,
            })
            .collect();
        Ok(())
    }

    /// The portal's `ZonesChanged` signal only marks the zones dirty; the module's 2 s poll calls
    /// `zones()`, which asks again (§13.8 item 2).
    fn watch_zones(&mut self) {
        if self.zones_task.is_some() {
            return;
        }
        let dirty = self.zones_dirty.clone();
        self.zones_task = Some(tokio::spawn(async move {
            let Ok(ic) = InputCapture::new().await else {
                return;
            };
            let Ok(mut changes) = ic.receive_zones_changed().await else {
                return;
            };
            while changes.next().await.is_some() {
                dirty.store(true, Ordering::SeqCst);
            }
        }));
    }

    /// The barriers of `barriers` on the current zones, as the portal wants them.
    fn portal_barriers(
        &self,
        barriers: &[BarrierSpec],
    ) -> Result<(Vec<Barrier>, Vec<Geom>), InputError> {
        let mut geoms = Vec::new();
        let mut portal = Vec::new();
        for (i, b) in barriers.iter().enumerate() {
            let zone = self.resolve(&b.zone)?.clone();
            let seg = barrier_segment(&zone, b.edge, b.from, b.to);
            let id = NonZero::new(i as u32 + 1).ok_or_else(|| InputError::Other("id".into()))?;
            portal.push(Barrier::new(id, (seg.x1, seg.y1, seg.x2, seg.y2)));
            geoms.push(Geom { zone, edge: b.edge });
        }
        Ok((portal, geoms))
    }

    fn resolve<'a>(&'a self, r: &ZoneRef) -> Result<&'a Zone, InputError> {
        match r {
            ZoneRef::Index(i) => self.zones.iter().find(|z| z.index == *i),
            ZoneRef::Name(n) => self.zones.iter().find(|z| z.name.as_deref() == Some(n)),
        }
        .ok_or_else(|| {
            InputError::ZoneNotFound(match r {
                ZoneRef::Index(i) => format!("#{i}"),
                ZoneRef::Name(n) => n.clone(),
            })
        })
    }
}

#[async_trait]
impl CaptureAdapter for PortalCapture {
    fn name(&self) -> &'static str {
        "portal-inputcapture"
    }

    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        self.ensure_session().await?;
        if self.zones_dirty.swap(false, Ordering::SeqCst) {
            self.refresh_zones().await?;
        }
        Ok(self.zones.clone())
    }

    /// No dialog: the session's zones when it exists, else the DRM estimate.
    fn known_zones(&self) -> Vec<Zone> {
        if self.session.is_some() {
            self.zones.clone()
        } else {
            super::outputs::sysfs_zones()
        }
    }

    async fn start(
        &mut self,
        barriers: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        self.ensure_session().await?;
        let (Some(ic), Some(session)) = (&self.ic, &self.session) else {
            return Err(InputError::Other("no session".into()));
        };
        let (portal_barriers, geoms) = self.portal_barriers(barriers)?;
        *lock(&self.geoms) = geoms;
        let resp = ic
            .set_pointer_barriers(
                session,
                &portal_barriers,
                self.zone_set,
                SetPointerBarriersOptions::default(),
            )
            .await
            .map_err(map_err)?
            .response()
            .map_err(map_err)?;
        if !resp.failed_barriers().is_empty() {
            return Err(InputError::Other(format!(
                "portal refused {} barrier(s)",
                resp.failed_barriers().len()
            )));
        }
        let fd = ic
            .connect_to_eis(session, ConnectToEISOptions::default())
            .await
            .map_err(map_err)?;
        let stream = UnixStream::from(fd);
        let (tx, rx) = mpsc::channel(4096);
        // Publish lock state before any producer can enqueue activation/input, also avoiding
        // waiting on a full event queue before start() has returned its receiver.
        self.lock_watch = Some(LockWatch::start(Some(tx.clone())).await);
        let (ready_tx, ready_rx) = oneshot::channel();
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let active = self.active.clone();
        let geoms = self.geoms.clone();
        let spawned = std::thread::Builder::new()
            .name("clipsync-input-capture".into())
            .spawn(move || {
                // reis streams are !Send: the EIS loop owns a current-thread runtime.
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = ready_tx.send(Err("runtime".to_owned()));
                    return;
                };
                rt.block_on(capture_loop(stream, geoms, active, tx, ready_tx, stop_rx));
            });
        match spawned {
            Ok(h) => self.thread = Some(h),
            Err(e) => return Err(InputError::Other(e.to_string())),
        }
        self.stop = Some(stop_tx);
        match ready_rx.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(InputError::Other(e)),
            Err(_) => return Err(InputError::Other("capture thread ended".into())),
        }
        ic.enable(session, EnableOptions::default())
            .await
            .map_err(map_err)?;
        self.watch_zones();
        Ok(rx)
    }

    /// Swaps the barriers on the live session without a new dialog: the portal keeps one
    /// capture session per agent run, so only the barriers go through `SetPointerBarriers` again
    /// (with the current `zone_set`, hence the zones are asked first).
    async fn set_barriers(&mut self, barriers: &[BarrierSpec]) -> Result<(), InputError> {
        self.ensure_session().await?;
        self.refresh_zones().await?;
        self.zones_dirty.store(false, Ordering::SeqCst);
        let (portal_barriers, geoms) = self.portal_barriers(barriers)?;
        let (Some(ic), Some(session)) = (&self.ic, &self.session) else {
            return Err(InputError::Other("no session".into()));
        };
        // Barriers are replaced while the session is disabled; `enable` brings the capture back.
        ic.disable(session, Default::default())
            .await
            .map_err(map_err)?;
        let resp = ic
            .set_pointer_barriers(
                session,
                &portal_barriers,
                self.zone_set,
                SetPointerBarriersOptions::default(),
            )
            .await
            .map_err(map_err)?
            .response()
            .map_err(map_err)?;
        let refused = resp.failed_barriers().len();
        ic.enable(session, EnableOptions::default())
            .await
            .map_err(map_err)?;
        if refused > 0 {
            return Err(InputError::Other(format!(
                "portal refused {refused} barrier(s)"
            )));
        }
        *lock(&self.geoms) = geoms;
        Ok(())
    }

    async fn release(&mut self) -> Result<(), InputError> {
        let (Some(ic), Some(session)) = (&self.ic, &self.session) else {
            return Ok(());
        };
        let info = *lock(&self.active);
        let opts = ReleaseOptions::default()
            .set_activation_id(info.activation_id)
            .set_cursor_position(info.cursor);
        ic.release(session, opts).await.map_err(map_err)
    }

    /// The InputCapture portal activates a capture only when the pointer crosses a barrier: there
    /// is no call that starts one. A switch by button would hand the other machine the control
    /// while the keyboard and the mouse stayed here, so it is refused (`can_begin`).
    fn can_begin(&self) -> bool {
        false
    }

    async fn stop(&mut self) -> Result<(), InputError> {
        self.stop = None;
        self.lock_watch = None;
        if let Some(t) = self.zones_task.take() {
            t.abort();
        }
        if let Some(t) = self.thread.take() {
            let _ = tokio::task::spawn_blocking(move || t.join()).await;
        }
        // The session survives so the next loopback does not ask for the dialog again
        // (InputCapture v1 has no restore token: one dialog per agent session).
        if let (Some(ic), Some(session)) = (&self.ic, &self.session) {
            let _ = ic.disable(session, Default::default()).await;
        }
        Ok(())
    }
}

async fn capture_loop(
    stream: UnixStream,
    geoms: Arc<Mutex<Vec<Geom>>>,
    active: Arc<Mutex<ActiveInfo>>,
    tx: mpsc::Sender<CaptureEvent>,
    ready_tx: oneshot::Sender<Result<(), String>>,
    mut stop_rx: oneshot::Receiver<()>,
) {
    let Ok(ctx) = ei::Context::new(stream) else {
        let _ = ready_tx.send(Err("eis context".to_owned()));
        return;
    };
    let Ok((_conn, mut events)) = ctx
        .handshake_tokio("clipsync-input", ei::handshake::ContextType::Receiver)
        .await
    else {
        let _ = ready_tx.send(Err("eis handshake".to_owned()));
        return;
    };
    // A second proxy owns the signal streams so the task is 'static.
    let Ok(ic2) = InputCapture::new().await else {
        let _ = ready_tx.send(Err("signal proxy".to_owned()));
        return;
    };
    let (Ok(mut activated), Ok(mut deactivated)) = (
        ic2.receive_activated().await,
        ic2.receive_deactivated().await,
    ) else {
        let _ = ready_tx.send(Err("signal subscription".to_owned()));
        return;
    };
    let _ = ready_tx.send(Ok(()));
    let mut capturing = false;
    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            Some(a) = activated.next() => {
                let barrier = match a.barrier_id() {
                    Some(ashpd::desktop::input_capture::ActivatedBarrier::Barrier(id)) =>
                        (id.get() as usize).saturating_sub(1),
                    _ => 0,
                };
                let g = {
                    let all = lock(&geoms);
                    all.get(barrier).or_else(|| all.first()).cloned()
                };
                let g = g.as_ref();
                let cur = a.cursor_position();
                let pos = match (g, cur) {
                    (Some(g), Some((cx, cy))) => edge_position(&g.zone, g.edge, cx, cy),
                    _ => 0.0,
                };
                let back = match (g, cur) {
                    (Some(g), Some((cx, cy))) => Some(match g.edge {
                        Edge::Right => (g.zone.x as f32 + g.zone.width as f32 - INWARD_PX, cy),
                        Edge::Left => (g.zone.x as f32 + INWARD_PX, cy),
                        Edge::Top => (cx, g.zone.y as f32 + INWARD_PX),
                        Edge::Bottom => (cx, g.zone.y as f32 + g.zone.height as f32 - INWARD_PX),
                    }),
                    _ => None,
                };
                *lock(&active) = ActiveInfo {
                    activation_id: a.activation_id(),
                    cursor: back.map(|(x, y)| (f64::from(x), f64::from(y))),
                };
                capturing = true;
                if tx.send(CaptureEvent::Activated { barrier, pos, t_mono_us: mono_us() }).await.is_err() { break; }
            }
            Some(_d) = deactivated.next() => {
                capturing = false;
                if tx.send(CaptureEvent::Deactivated).await.is_err() { break; }
            }
            ev = events.next() => {
                let Some(ev) = ev else { break };
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => { let _ = tx.send(CaptureEvent::Error(e.to_string())).await; break; }
                };
                if let EiEvent::SeatAdded(s) = &ev {
                    s.seat.bind_capabilities(
                        DeviceCapability::Pointer | DeviceCapability::Keyboard
                        | DeviceCapability::Button | DeviceCapability::Scroll);
                    let _ = ctx.flush();
                    continue;
                }
                if !capturing { continue; }
                let mapped = match ev {
                    EiEvent::PointerMotion(m) => Some(InputEvent::Motion { dx: m.dx, dy: m.dy }),
                    EiEvent::Button(b) => Some(InputEvent::Button {
                        code: b.button,
                        pressed: b.state == ei::button::ButtonState::Press,
                    }),
                    EiEvent::ScrollDelta(s) => Some(InputEvent::Scroll { dx: s.dx, dy: s.dy }),
                    EiEvent::ScrollDiscrete(s) => Some(InputEvent::ScrollDiscrete {
                        dx: s.discrete_dx, dy: s.discrete_dy }),
                    EiEvent::KeyboardKey(k) => Some(InputEvent::Key {
                        code: k.key,
                        pressed: k.state == ei::keyboard::KeyState::Press,
                    }),
                    _ => None,
                };
                if let Some(ev) = mapped
                    && tx.send(CaptureEvent::Input { t_mono_us: mono_us(), ev }).await.is_err()
                {
                    break;
                }
            }
            else => break,
        }
    }
}

// --------------------------------------------------------------- emulation

enum Cmd {
    Emit(Vec<InputEvent>, oneshot::Sender<Result<(), InputError>>),
    ReleaseAll(oneshot::Sender<Result<(), InputError>>),
}

struct Dev {
    dev: reis::event::Device,
    resumed: bool,
    emulating: bool,
    touched: bool,
}

struct Emulator {
    ctx: ei::Context,
    conn: reis::event::Connection,
    devs: Vec<Dev>,
    seq: u32,
    pressed: HashSet<(bool, u32)>,
}

impl Emulator {
    fn pick(&mut self, cap: DeviceCapability) -> Option<&mut Dev> {
        self.devs
            .iter_mut()
            .find(|d| d.resumed && d.dev.has_capability(cap))
    }

    fn begin(&mut self, idx: usize) {
        let serial = self.conn.serial();
        self.seq += 1;
        let seq = self.seq;
        if let Some(d) = self.devs.get_mut(idx) {
            if !d.emulating {
                d.dev.device().start_emulating(serial, seq);
                d.emulating = true;
            }
            d.touched = true;
        }
    }

    fn index_of(&self, cap: DeviceCapability) -> Option<usize> {
        self.devs
            .iter()
            .position(|d| d.resumed && d.dev.has_capability(cap))
    }

    fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        for ev in events {
            let cap = match ev {
                InputEvent::Motion { .. } => DeviceCapability::Pointer,
                InputEvent::Button { .. } => DeviceCapability::Button,
                InputEvent::Scroll { .. } | InputEvent::ScrollDiscrete { .. } => {
                    DeviceCapability::Scroll
                }
                InputEvent::Key { .. } => DeviceCapability::Keyboard,
            };
            let idx = self
                .index_of(cap)
                .ok_or_else(|| InputError::Eis(format!("no device for {cap:?}")))?;
            self.begin(idx);
            let dev = &self.devs[idx].dev;
            match *ev {
                InputEvent::Motion { dx, dy } => {
                    if let Some(p) = dev.interface::<ei::Pointer>() {
                        p.motion_relative(dx, dy);
                    }
                }
                InputEvent::Button { code, pressed } => {
                    if let Some(b) = dev.interface::<ei::Button>() {
                        b.button(code, button_state(pressed));
                    }
                    set_pressed(&mut self.pressed, true, code, pressed);
                }
                InputEvent::Scroll { dx, dy } => {
                    if let Some(s) = dev.interface::<ei::Scroll>() {
                        s.scroll(dx, dy);
                    }
                }
                InputEvent::ScrollDiscrete { dx, dy } => {
                    if let Some(s) = dev.interface::<ei::Scroll>() {
                        s.scroll_discrete(dx, dy);
                    }
                }
                InputEvent::Key { code, pressed } => {
                    if let Some(k) = dev.interface::<ei::Keyboard>() {
                        k.key(code, key_state(pressed));
                    }
                    set_pressed(&mut self.pressed, false, code, pressed);
                }
            }
        }
        self.frame()
    }

    fn frame(&mut self) -> Result<(), InputError> {
        let serial = self.conn.serial();
        let now = mono_us();
        for d in &mut self.devs {
            if d.touched {
                d.dev.device().frame(serial, now);
                d.touched = false;
            }
        }
        self.ctx.flush().map_err(eis_err)
    }

    fn release_all(&mut self) -> Result<(), InputError> {
        let pressed: Vec<(bool, u32)> = self.pressed.drain().collect();
        for (is_button, code) in pressed {
            let cap = if is_button {
                DeviceCapability::Button
            } else {
                DeviceCapability::Keyboard
            };
            let Some(idx) = self.index_of(cap) else {
                continue;
            };
            self.begin(idx);
            let dev = &self.devs[idx].dev;
            if is_button {
                if let Some(b) = dev.interface::<ei::Button>() {
                    b.button(code, button_state(false));
                }
            } else if let Some(k) = dev.interface::<ei::Keyboard>() {
                k.key(code, key_state(false));
            }
        }
        self.frame()
    }

    fn stop_emulating(&mut self) {
        let serial = self.conn.serial();
        for d in &mut self.devs {
            if d.emulating {
                d.dev.device().stop_emulating(serial);
                d.emulating = false;
            }
        }
        let _ = self.ctx.flush();
    }
}

fn button_state(pressed: bool) -> ei::button::ButtonState {
    if pressed {
        ei::button::ButtonState::Press
    } else {
        ei::button::ButtonState::Released
    }
}

fn key_state(pressed: bool) -> ei::keyboard::KeyState {
    if pressed {
        ei::keyboard::KeyState::Press
    } else {
        ei::keyboard::KeyState::Released
    }
}

fn set_pressed(set: &mut HashSet<(bool, u32)>, is_button: bool, code: u32, pressed: bool) {
    if pressed {
        set.insert((is_button, code));
    } else {
        set.remove(&(is_button, code));
    }
}

async fn emulate_loop(
    stream: UnixStream,
    mut cmd_rx: mpsc::Receiver<Cmd>,
    ready_tx: oneshot::Sender<()>,
    layout: Arc<Mutex<Option<String>>>,
) {
    let mut ready_tx = Some(ready_tx);
    let Ok(ctx) = ei::Context::new(stream) else {
        return;
    };
    let Ok((conn, mut events)) = ctx
        .handshake_tokio("clipsync-input-emit", ei::handshake::ContextType::Sender)
        .await
    else {
        return;
    };
    let mut em = Emulator {
        ctx: ctx.clone(),
        conn,
        devs: Vec::new(),
        seq: 0,
        pressed: HashSet::new(),
    };
    loop {
        tokio::select! {
            ev = events.next() => {
                let Some(Ok(ev)) = ev else { break };
                match ev {
                    EiEvent::SeatAdded(s) => {
                        s.seat.bind_capabilities(
                            DeviceCapability::Pointer | DeviceCapability::Keyboard
                            | DeviceCapability::Button | DeviceCapability::Scroll);
                        let _ = ctx.flush();
                    }
                    EiEvent::DeviceAdded(d) => {
                        if let Some(name) = d.device.keymap().and_then(keymap_symbols) {
                            *lock(&layout) = Some(name);
                        }
                        em.devs.push(Dev {
                            dev: d.device, resumed: false, emulating: false, touched: false });
                    }
                    EiEvent::DeviceResumed(d) => {
                        if let Some(x) = em.devs.iter_mut().find(|x| x.dev == d.device) {
                            x.resumed = true;
                        }
                        if em.pick(DeviceCapability::Pointer).is_some()
                            && let Some(r) = ready_tx.take()
                        {
                            let _ = r.send(());
                        }
                    }
                    EiEvent::DevicePaused(d) => {
                        if let Some(x) = em.devs.iter_mut().find(|x| x.dev == d.device) {
                            x.resumed = false;
                            x.emulating = false;
                        }
                    }
                    EiEvent::DeviceRemoved(d) => em.devs.retain(|x| x.dev != d.device),
                    EiEvent::Disconnected(_) => break,
                    _ => {}
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(Cmd::Emit(evs, reply)) => { let _ = reply.send(em.emit(&evs)); }
                    Some(Cmd::ReleaseAll(reply)) => { let _ = reply.send(em.release_all()); }
                    None => break,
                }
            }
        }
    }
    em.stop_emulating();
}

pub struct PortalEmulate {
    store: TokenStore,
    session: Option<Session<RemoteDesktop>>,
    cmd: Option<mpsc::Sender<Cmd>>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// A locked screen takes no emulation (§13.6 item 8).
    lock_watch: Option<LockWatch>,
    /// The `xkb_symbols` name of the keymap the EIS announced, for the diagnostics only.
    layout: Arc<Mutex<Option<String>>>,
}

impl PortalEmulate {
    pub fn new(store: TokenStore) -> Self {
        Self {
            store,
            session: None,
            cmd: None,
            thread: None,
            lock_watch: None,
            layout: Arc::new(Mutex::new(None)),
        }
    }

    async fn call(
        &self,
        make: impl FnOnce(oneshot::Sender<Result<(), InputError>>) -> Cmd,
    ) -> Result<(), InputError> {
        let tx = self
            .cmd
            .as_ref()
            .ok_or_else(|| InputError::Other("emulator not started".into()))?;
        let (r_tx, r_rx) = oneshot::channel();
        tx.send(make(r_tx))
            .await
            .map_err(|_| InputError::Eis("emulation task ended".into()))?;
        r_rx.await
            .map_err(|_| InputError::Eis("emulation task ended".into()))?
    }
}

#[async_trait]
impl EmulateAdapter for PortalEmulate {
    fn name(&self) -> &'static str {
        "portal-remotedesktop"
    }

    async fn start(&mut self) -> Result<(), InputError> {
        if self.cmd.is_some() {
            return Ok(());
        }
        let rd = RemoteDesktop::new().await.map_err(map_err)?;
        let session = rd
            .create_session(RdCreateOptions::default())
            .await
            .map_err(map_err)?;
        let token = self.store.load();
        rd.select_devices(
            &session,
            SelectDevicesOptions::default()
                .set_devices(DeviceType::Pointer | DeviceType::Keyboard)
                .set_persist_mode(PersistMode::ExplicitlyRevoked)
                .set_restore_token(token.as_deref()),
        )
        .await
        .map_err(map_err)?
        .response()
        .map_err(map_err)?;
        let selected = rd
            .start(&session, None, StartOptions::default())
            .await
            .map_err(map_err)?
            .response()
            .map_err(map_err)?;
        match selected.restore_token() {
            Some(t) => {
                if let Err(e) = self.store.save(t) {
                    tracing::warn!("could not store restore token: {e}");
                }
            }
            None if token.is_some() => self.store.discard(),
            None => {}
        }
        let fd = rd
            .connect_to_eis(&session, RdConnectOptions::default())
            .await
            .map_err(map_err)?;
        let stream = UnixStream::from(fd);
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(64);
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let layout = self.layout.clone();
        // reis streams are !Send: the EIS loop owns a current-thread runtime.
        let spawned = std::thread::Builder::new()
            .name("clipsync-input-emulate".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(emulate_loop(stream, cmd_rx, ready_tx, layout));
            });
        match spawned {
            Ok(h) => self.thread = Some(h),
            Err(e) => return Err(InputError::Other(e.to_string())),
        }
        match tokio::time::timeout(DEVICE_WAIT, ready_rx).await {
            Ok(Ok(())) => {}
            _ => return Err(InputError::Eis("no pointer device became ready".into())),
        }
        self.session = Some(session);
        self.cmd = Some(cmd_tx);
        self.lock_watch = Some(LockWatch::start(None).await);
        Ok(())
    }

    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        if self.lock_watch.as_ref().is_some_and(LockWatch::is_locked) {
            return Err(InputError::Locked);
        }
        let evs = events.to_vec();
        self.call(|r| Cmd::Emit(evs, r)).await
    }

    fn keyboard_layout(&self) -> Option<String> {
        lock(&self.layout).clone()
    }

    async fn release_all(&mut self) -> Result<(), InputError> {
        if self.cmd.is_none() {
            return Ok(());
        }
        self.call(Cmd::ReleaseAll).await
    }

    async fn stop(&mut self) -> Result<(), InputError> {
        if self.cmd.is_some() {
            let _ = self.call(Cmd::ReleaseAll).await;
        }
        self.cmd = None;
        self.lock_watch = None;
        if let Some(t) = self.thread.take() {
            let _ = tokio::task::spawn_blocking(move || t.join()).await;
        }
        if let Some(s) = self.session.take() {
            let _ = s.close().await;
        }
        Ok(())
    }
}

/// The `xkb_symbols` name in the keymap the EIS announced (`pc+br(abnt2)+inet(evdev)`), or `None`
/// when it cannot be read. Only for the diagnostics: the layout is never used to translate keys.
fn keymap_symbols(k: &reis::event::Keymap) -> Option<String> {
    use std::io::Read;
    let fd = k.fd.try_clone().ok()?;
    let mut text = String::new();
    std::fs::File::from(fd)
        .take(u64::from(k.size).min(1 << 20))
        .read_to_string(&mut text)
        .ok()?;
    symbols_name(&text)
}

fn symbols_name(keymap: &str) -> Option<String> {
    let at = keymap.find("xkb_symbols")?;
    let rest = &keymap[at..];
    let start = rest.find('"')? + 1;
    let end = start + rest[start..].find('"')?;
    Some(rest[start..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::symbols_name;

    #[test]
    fn the_symbols_name_comes_out_of_the_keymap_text() {
        let text = "xkb_keymap {\n xkb_keycodes \"evdev+aliases(qwerty)\" {};\n xkb_symbols \"pc+br(abnt2)+inet(evdev)\" {};\n};";
        assert_eq!(
            symbols_name(text).as_deref(),
            Some("pc+br(abnt2)+inet(evdev)")
        );
        assert_eq!(symbols_name("nothing here"), None);
    }
}
