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
use tokio::sync::{mpsc, oneshot, watch};

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
            .map_err(|e| {
                tracing::info!(evento = "portal.session.created", portal = "input_capture", motivo = "refused", erro = %e, "portal session refused");
                map_err(e)
            })?;
        tracing::info!(
            evento = "portal.session.created",
            portal = "input_capture",
            "portal session created"
        );
        self.ic = Some(ic);
        self.session = Some(session);
        self.watch_zones().await?;
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
    async fn watch_zones(&mut self) -> Result<(), InputError> {
        if self.zones_task.as_ref().is_some_and(|t| !t.is_finished()) {
            return Ok(());
        }
        let dirty = self.zones_dirty.clone();
        let (ready, subscribed) = oneshot::channel();
        self.zones_task = Some(tokio::spawn(async move {
            let ic = match InputCapture::new().await {
                Ok(ic) => ic,
                Err(_) => {
                    dirty.store(true, Ordering::SeqCst);
                    let _ = ready.send(false);
                    return;
                }
            };
            let mut changes = match ic.receive_zones_changed().await {
                Ok(s) => s,
                Err(_) => {
                    dirty.store(true, Ordering::SeqCst);
                    let _ = ready.send(false);
                    return;
                }
            };
            let _ = ready.send(true);
            while changes.next().await.is_some() {
                dirty.store(true, Ordering::SeqCst);
            }
            dirty.store(true, Ordering::SeqCst);
        }));
        if subscribed.await.unwrap_or(false) {
            Ok(())
        } else {
            Err(crate::inventory::unavailable())
        }
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
    fn known_monitor_observation(&self) -> Result<crate::inventory::ObservedMonitors, InputError> {
        if self.session.is_none() {
            return crate::inventory::linux_estimated_observation(std::path::Path::new(
                "/sys/class/drm",
            ));
        }
        if self.zones_dirty.load(Ordering::SeqCst)
            || self.zones_task.as_ref().is_none_or(|t| t.is_finished())
        {
            return Err(crate::inventory::unavailable());
        }
        crate::inventory::linux_observation(std::path::Path::new("/sys/class/drm"), &self.zones)
    }
    async fn prepare_monitor_observation(
        &mut self,
    ) -> Result<crate::inventory::ObservedMonitors, InputError> {
        self.ensure_session().await?;
        self.watch_zones().await?;
        // Clear before GetZones; a signal during the await leaves the result invalid.
        self.zones_dirty.store(false, Ordering::SeqCst);
        if let Err(e) = self.refresh_zones().await {
            self.zones_dirty.store(true, Ordering::SeqCst);
            return Err(e);
        }
        self.known_monitor_observation()
    }

    fn name(&self) -> &'static str {
        "portal-inputcapture"
    }

    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        self.ensure_session().await?;
        if self.zones_dirty.swap(false, Ordering::SeqCst)
            && let Err(e) = self.refresh_zones().await
        {
            self.zones_dirty.store(true, Ordering::SeqCst);
            return Err(e);
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
            .map_err(|e| {
                tracing::info!(evento = "portal.connect_eis", portal = "input_capture", motivo = "refused", erro = %e, "EIS connection refused");
                map_err(e)
            })?;
        tracing::info!(
            evento = "portal.connect_eis",
            portal = "input_capture",
            "EIS connected"
        );
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
        self.watch_zones().await?;
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
    /// Local number of this device in the EIS loop, only for the log.
    id: u64,
    /// reis remove as interfaces antes de DeviceRemoved: o log conserva o anúncio original.
    capabilities: String,
    dev: reis::event::Device,
    resumed: bool,
    emulating: bool,
    touched: bool,
    /// Key events sent to this device since it was added: a count, never a key.
    keys: u64,
    /// Eventos de tecla descartados enquanto este dispositivo estava indisponível.
    dropped_keys: u64,
}

/// The device capabilities, in a fixed order, for the `eis.device` log line. Names only.
fn capabilities_of(dev: &reis::event::Device) -> String {
    let mut caps = Vec::new();
    for (cap, name) in [
        (DeviceCapability::Pointer, "pointer"),
        (DeviceCapability::PointerAbsolute, "pointer_absolute"),
        (DeviceCapability::Button, "button"),
        (DeviceCapability::Scroll, "scroll"),
        (DeviceCapability::Keyboard, "keyboard"),
        (DeviceCapability::Touch, "touch"),
        (DeviceCapability::Text, "text"),
    ] {
        if dev.has_capability(cap) {
            caps.push(name);
        }
    }
    caps.join(",")
}

/// `eis.device`: the life of one EIS device (§17.8). Only capabilities and counts, never a key.
fn log_device(acao: &str, dev: &Dev, origem: &str) {
    tracing::info!(
        evento = "eis.device",
        acao,
        origem,
        dispositivo = dev.id,
        capacidades = %dev.capabilities,
        "EIS device"
    );
}

/// Fecho de cada dispositivo, com contagem de pedidos de tecla, nunca tecla ou keycode.
fn log_device_count(dev: &Dev, origem: &str) {
    tracing::info!(
        evento = "eis.device",
        acao = "removed",
        origem,
        dispositivo = dev.id,
        capacidades = %dev.capabilities,
        contagem = dev.keys,
        descartados = dev.dropped_keys,
        "EIS device"
    );
}

#[derive(Clone, Copy, Default)]
struct DevicesReady {
    pointer: bool,
    keyboard: bool,
    keyboard_dropping: bool,
}

struct Emulator {
    ctx: ei::Context,
    conn: reis::event::Connection,
    devs: Vec<Dev>,
    seq: u32,
    /// Number of devices announced in this loop, for the log.
    announced: u64,
    /// Pedidos de tecla enviados por toda a vida do laço, inclusive dispositivos removidos.
    keys_sent: u64,
    /// Descartes sem teclado pertencem ao laço, mesmo antes de existir um dispositivo.
    keys_dropped: u64,
    keyboard_dropping: bool,
    pressed: HashSet<(bool, u32)>,
    /// Solturas de presses já enviados, adiadas por indisponibilidade ou pelo fim da sessão.
    /// Só códigos internos: nunca serializar este conjunto nem reproduzir presses descartados.
    pending_releases: HashSet<u32>,
}

impl Emulator {
    fn devices_ready(&self) -> DevicesReady {
        DevicesReady {
            pointer: self.index_of(DeviceCapability::Pointer).is_some(),
            keyboard: self.index_of(DeviceCapability::Keyboard).is_some(),
            keyboard_dropping: self.keyboard_dropping,
        }
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

    fn drop_key(&mut self) {
        self.keys_dropped = self.keys_dropped.saturating_add(1);
        if let Some(d) = self
            .devs
            .iter_mut()
            .find(|d| d.dev.has_capability(DeviceCapability::Keyboard))
        {
            d.dropped_keys = d.dropped_keys.saturating_add(1);
        }
        if !self.keyboard_dropping {
            self.keyboard_dropping = true;
            tracing::info!(
                evento = "eis.device",
                acao = "discarding",
                origem = "emit",
                capacidades = "keyboard",
                motivo = "not_ready",
                descartados = self.keys_dropped,
                "EIS keyboard unavailable; key events discarded"
            );
        }
    }

    fn reconcile_keys(&mut self, origem: &'static str) -> Result<(), InputError> {
        if self.pending_releases.is_empty() {
            return Ok(());
        }
        let Some(idx) = self.index_of(DeviceCapability::Keyboard) else {
            return Ok(());
        };
        let mut pending: Vec<u32> = self.pending_releases.iter().copied().collect();
        pending.sort_unstable();
        self.begin(idx);
        let Some(keyboard) = self.devs[idx].dev.interface::<ei::Keyboard>() else {
            return Err(InputError::Eis("no device for Keyboard".into()));
        };
        for code in &pending {
            keyboard.key(*code, key_state(false));
            self.devs[idx].keys = self.devs[idx].keys.saturating_add(1);
            self.keys_sent = self.keys_sent.saturating_add(1);
        }
        // O frame das solturas precede qualquer nova tecla; só aposenta pendentes após flush.
        self.frame()?;
        self.pending_releases.clear();
        tracing::info!(
            evento = "eis.device",
            acao = "reconciled",
            origem,
            dispositivo = self.devs[idx].id,
            capacidades = "keyboard",
            contagem = pending.len() as u64,
            "EIS pending releases sent"
        );
        Ok(())
    }

    fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        self.reconcile_keys("emit")?;
        let mut missing = None;
        for ev in events {
            let cap = match ev {
                InputEvent::Motion { .. } => DeviceCapability::Pointer,
                InputEvent::Button { .. } => DeviceCapability::Button,
                InputEvent::Scroll { .. } | InputEvent::ScrollDiscrete { .. } => {
                    DeviceCapability::Scroll
                }
                InputEvent::Key { .. } => DeviceCapability::Keyboard,
            };
            let Some(idx) = self.index_of(cap) else {
                if cap == DeviceCapability::Keyboard {
                    // Não guarda presses descartados. Uma soltura de press já enviado
                    // precisa neutralizar o estado anterior assim que houver teclado pronto.
                    if let InputEvent::Key {
                        code,
                        pressed: false,
                    } = ev
                        && self.pressed.remove(&(false, *code))
                    {
                        self.pending_releases.insert(*code);
                    }
                    self.drop_key();
                    continue;
                }
                // One missing device does not throw away the events already queued for the
                // others: the frame below commits them and the caller still sees the error.
                missing.get_or_insert_with(|| InputError::Eis(format!("no device for {cap:?}")));
                continue;
            };
            self.begin(idx);
            let mut key_sent = false;
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
                        key_sent = true;
                    }
                    set_pressed(&mut self.pressed, false, code, pressed);
                }
            }
            if key_sent {
                self.devs[idx].keys = self.devs[idx].keys.saturating_add(1);
                self.keys_sent = self.keys_sent.saturating_add(1);
            }
        }
        self.frame()?;
        match missing {
            Some(e) => Err(e),
            None => Ok(()),
        }
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
        let pressed = std::mem::take(&mut self.pressed);
        for (is_button, code) in pressed {
            if !is_button {
                // Leave/pausa encerram a sessão de controle, não necessariamente o laço EI.
                // Só aposenta solturas de teclado depois de enviá-las com flush bem-sucedido.
                self.pending_releases.insert(code);
                continue;
            }
            // Botões conservam a política anterior: tenta soltar no dispositivo disponível.
            let Some(idx) = self.index_of(DeviceCapability::Button) else {
                continue;
            };
            self.begin(idx);
            if let Some(button) = self.devs[idx].dev.interface::<ei::Button>() {
                button.button(code, button_state(false));
            }
        }
        self.reconcile_keys("release_all")?;
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
    ready_tx: watch::Sender<DevicesReady>,
    layout: Arc<Mutex<Option<String>>>,
) {
    let Ok(ctx) = ei::Context::new(stream) else {
        tracing::info!(
            evento = "eis.loop",
            motivo = "context",
            "EIS loop ended before the handshake"
        );
        return;
    };
    let Ok((conn, mut events)) = ctx
        .handshake_tokio("clipsync-input-emit", ei::handshake::ContextType::Sender)
        .await
    else {
        tracing::info!(
            evento = "eis.loop",
            motivo = "handshake",
            "EIS loop ended before the handshake"
        );
        return;
    };
    let mut em = Emulator {
        ctx: ctx.clone(),
        conn,
        devs: Vec::new(),
        seq: 0,
        announced: 0,
        keys_sent: 0,
        keys_dropped: 0,
        keyboard_dropping: false,
        pressed: HashSet::new(),
        pending_releases: HashSet::new(),
    };
    let (motivo, erro) = loop {
        tokio::select! {
            ev = events.next() => {
                let Some(ev) = ev else { break ("eof", None) };
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(reis::Error::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        break ("eof", None);
                    }
                    Err(e) => {
                        // Só a categoria fixa: texto de erro pode carregar dados do protocolo.
                        let (motivo, erro) = loop_error(&e);
                        break (motivo, Some(erro));
                    }
                };
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
                        em.announced += 1;
                        let dev = Dev {
                            id: em.announced,
                            capabilities: capabilities_of(&d.device),
                            dev: d.device,
                            resumed: false,
                            emulating: false,
                            touched: false,
                            keys: 0,
                            dropped_keys: 0,
                        };
                        log_device("added", &dev, "added");
                        em.devs.push(dev);
                    }
                    EiEvent::DeviceResumed(d) => {
                        if let Some(x) = em.devs.iter_mut().find(|x| x.dev == d.device) {
                            x.resumed = true;
                            log_device("resumed", x, "resumed");
                        }
                    }
                    EiEvent::DevicePaused(d) => {
                        if let Some(x) = em.devs.iter_mut().find(|x| x.dev == d.device) {
                            x.resumed = false;
                            x.emulating = false;
                            x.touched = false;
                            log_device("paused", x, "paused");
                        }
                    }
                    // stop_emulating vindo do servidor é exclusivo de receiver no protocolo
                    // EI e na API libeis 1.5.0. Nosso contexto é sender; não usar esse evento
                    // para inventar uma transição ou atribuir a causa do incidente.
                    EiEvent::DeviceRemoved(d) => {
                        if let Some(x) = em.devs.iter().find(|x| x.dev == d.device) {
                            log_device_count(x, "removed");
                        }
                        em.devs.retain(|x| x.dev != d.device);
                    }
                    EiEvent::Disconnected(_) => break ("disconnected", None),
                    _ => {}
                }
                // Retomada/recriação reconcilia solturas antes de anunciar disponibilidade
                // ou aceitar outra tecla. Presses descartados nunca são reproduzidos.
                if em.index_of(DeviceCapability::Keyboard).is_some() {
                    if em.reconcile_keys("resumed").is_err() { break ("io", Some("io")); }
                    em.keyboard_dropping = false;
                }
                // Snapshot atual, não um sinal histórico: pausa/remoção também o invalidam.
                let _ = ready_tx.send(em.devices_ready());
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(Cmd::Emit(evs, reply)) => {
                        let result = em.emit(&evs);
                        // Publica antes de responder ao módulo: o aviso não depende de outra tecla.
                        let _ = ready_tx.send(em.devices_ready());
                        let _ = reply.send(result);
                    }
                    Some(Cmd::ReleaseAll(reply)) => {
                        let result = em.release_all();
                        let _ = ready_tx.send(em.devices_ready());
                        let _ = reply.send(result);
                    }
                    None => break ("closed", None),
                }
            }
        }
    };
    em.stop_emulating();
    // The loop ending closes the EIS connection, so every device left is gone with it.
    for dev in &em.devs {
        log_device_count(dev, "loop");
    }
    let contagem = em.keys_sent;
    match erro {
        Some(e) => tracing::info!(
            evento = "eis.loop",
            motivo,
            erro = %e,
            dispositivos = em.announced,
            contagem,
            descartados = em.keys_dropped,
            "EIS loop ended"
        ),
        None => tracing::info!(
            evento = "eis.loop",
            motivo,
            dispositivos = em.announced,
            contagem,
            descartados = em.keys_dropped,
            "EIS loop ended"
        ),
    }
}

/// A spawned EIS sender loop: the command channel, the readiness signal and the thread.
struct EmulateTask {
    cmd: mpsc::Sender<Cmd>,
    ready: watch::Receiver<DevicesReady>,
    thread: std::thread::JoinHandle<()>,
}

/// Starts `emulate_loop` on its own thread with a current-thread runtime. Both
/// `PortalEmulate::start` and the test harness (`PortalEmulate::attach`) come through here, so the test
/// drives the same code the app does.
fn spawn_emulate_loop(
    stream: UnixStream,
    layout: Arc<Mutex<Option<String>>>,
) -> std::io::Result<EmulateTask> {
    let (cmd, cmd_rx) = mpsc::channel::<Cmd>(64);
    let (ready_tx, ready) = watch::channel(DevicesReady::default());
    // reis streams are !Send: the EIS loop owns a current-thread runtime.
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let thread = std::thread::Builder::new()
        .name("clipsync-input-emulate".into())
        .spawn(move || {
            let _trace = tracing::dispatcher::set_default(&dispatch);
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                tracing::info!(
                    evento = "eis.loop",
                    motivo = "runtime",
                    "EIS loop did not start"
                );
                return;
            };
            rt.block_on(emulate_loop(stream, cmd_rx, ready_tx, layout));
        })?;
    Ok(EmulateTask { cmd, ready, thread })
}

/// Categorias estáticas do erro; nunca serializa evento, tecla, keycode ou payload remoto.
fn loop_error(error: &reis::Error) -> (&'static str, &'static str) {
    match error {
        reis::Error::Io(_) => ("io", "io"),
        reis::Error::Event(_) => ("converter", "event"),
        reis::Error::Parse(_) => ("converter", "parse"),
        reis::Error::Request(_) => ("converter", "request"),
        reis::Error::Handshake(_) | reis::Error::UnexpectedHandshakeEvent => {
            ("converter", "handshake")
        }
        reis::Error::InvalidInterfaceVersion(_, _) => ("converter", "interface_version"),
    }
}

/// Retorna no tempo do ponteiro. Teclado não bloqueia Enter nem o atendimento de Stats.
async fn wait_emulate_ready(ready: &mut watch::Receiver<DevicesReady>) -> Result<(), InputError> {
    let ended = || InputError::Eis("emulation task ended".into());
    tokio::time::timeout(DEVICE_WAIT, async {
        loop {
            if ready.has_changed().is_err() {
                return Err(ended());
            }
            let state = *ready.borrow_and_update();
            if state.pointer {
                if !state.keyboard {
                    tracing::info!(
                        evento = "eis.device",
                        acao = "missing",
                        origem = "start",
                        capacidades = "keyboard",
                        motivo = "not_ready",
                        "EIS keyboard unavailable; pointer ready"
                    );
                }
                return Ok(());
            }
            ready.changed().await.map_err(|_| ended())?;
        }
    })
    .await
    .map_err(|_| InputError::Eis("no pointer device became ready".into()))?
}

pub struct PortalEmulate {
    store: TokenStore,
    /// Snapshot somente de disponibilidade/descarte, sem conteúdo de entrada.
    state: Option<watch::Receiver<DevicesReady>>,
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
            state: None,
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
        call_cmd(tx, make).await
    }

    /// Test-only entry (`tests/eis_*.rs`): attaches the emulation loop to an EIS sender stream
    /// the caller already owns, skipping the portal. It spawns the same loop as `start()` and
    /// waits on the same readiness rule; the store is never read. Not part of the contract.
    #[doc(hidden)]
    pub async fn attach(store: TokenStore, stream: UnixStream) -> Result<Self, InputError> {
        let mut this = Self::new(store);
        let mut task = spawn_emulate_loop(stream, this.layout.clone())
            .map_err(|e| InputError::Other(e.to_string()))?;
        this.thread = Some(task.thread);
        wait_emulate_ready(&mut task.ready).await?;
        this.state = Some(task.ready);
        this.cmd = Some(task.cmd);
        Ok(this)
    }
}

async fn call_cmd(
    cmd: &mpsc::Sender<Cmd>,
    make: impl FnOnce(oneshot::Sender<Result<(), InputError>>) -> Cmd,
) -> Result<(), InputError> {
    let (r_tx, r_rx) = oneshot::channel();
    cmd.send(make(r_tx))
        .await
        .map_err(|_| InputError::Eis("emulation task ended".into()))?;
    r_rx.await
        .map_err(|_| InputError::Eis("emulation task ended".into()))?
}

#[async_trait]
impl EmulateAdapter for PortalEmulate {
    fn name(&self) -> &'static str {
        "portal-remotedesktop"
    }

    async fn start(&mut self) -> Result<(), InputError> {
        if self.cmd.is_some() {
            if self.alive() {
                return Ok(());
            }
            // The EIS loop ended (GNOME closed the EIS or the portal session): drop the dead
            // session and open a new one, silent with the restore token.
            self.stop().await?;
        }
        let rd = RemoteDesktop::new().await.map_err(map_err)?;
        let session = rd
            .create_session(RdCreateOptions::default())
            .await
            .map_err(|e| {
                tracing::info!(evento = "portal.session.created", portal = "remote_desktop", motivo = "refused", erro = %e, "portal session refused");
                map_err(e)
            })?;
        tracing::info!(
            evento = "portal.session.created",
            portal = "remote_desktop",
            "portal session created"
        );
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
            .map_err(|e| {
                tracing::info!(evento = "portal.connect_eis", portal = "remote_desktop", motivo = "refused", erro = %e, "EIS connection refused");
                map_err(e)
            })?;
        tracing::info!(
            evento = "portal.connect_eis",
            portal = "remote_desktop",
            "EIS connected"
        );
        let stream = UnixStream::from(fd);
        let layout = self.layout.clone();
        let mut task =
            spawn_emulate_loop(stream, layout).map_err(|e| InputError::Other(e.to_string()))?;
        self.thread = Some(task.thread);
        wait_emulate_ready(&mut task.ready).await?;
        self.state = Some(task.ready);
        self.session = Some(session);
        self.cmd = Some(task.cmd);
        self.lock_watch = Some(LockWatch::start(None).await);
        Ok(())
    }

    fn alive(&self) -> bool {
        !self.cmd.as_ref().is_some_and(mpsc::Sender::is_closed)
    }

    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        if self.lock_watch.as_ref().is_some_and(LockWatch::is_locked) {
            return Err(InputError::Locked);
        }
        let evs = events.to_vec();
        self.call(|r| Cmd::Emit(evs, r)).await
    }

    fn block_reason(&self) -> Option<&'static str> {
        let state = self.state.as_ref()?;
        let dropping = state.borrow().keyboard_dropping;
        dropping.then_some(crate::msg::KEYBOARD_UNAVAILABLE)
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
        self.state = None;
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

impl Drop for PortalCapture {
    fn drop(&mut self) {
        if let Some(t) = self.zones_task.take() {
            t.abort();
        }
    }
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
