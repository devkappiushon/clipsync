//! In-memory adapters for tests and for `capture = "fake"` / `emulate = "fake"`.
use super::{BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, Hotkey, InputError, Zone};
use async_trait::async_trait;
use clipsync_proto::{InputEvent, mono_us};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::{Mutex as AsyncMutex, mpsc, oneshot};
use tokio::task::JoinHandle;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the handle shares with the capture besides zones and barriers.
#[derive(Default)]
struct CapState {
    lifecycle: Mutex<Option<tokio::sync::watch::Sender<super::CaptureLifecycle>>>,
    observation: Mutex<Option<crate::inventory::ObservedMonitors>>,
    hooks: AtomicBool,
    started: AtomicBool,
    starts: AtomicUsize,
    resume_refused: AtomicBool,
    resume_tx: Mutex<Option<mpsc::Sender<CaptureEvent>>>,
    released_at: Mutex<Vec<(usize, f32)>>,
    begins: Mutex<Vec<(usize, f32)>>,
    cannot_begin: AtomicBool,
    screen_locked: AtomicBool,
}

pub struct FakeCapture {
    cap: Arc<CapState>,
    zones: Arc<Mutex<Vec<Zone>>>,
    barriers: Arc<Mutex<Vec<BarrierSpec>>>,
    src: Arc<AsyncMutex<mpsc::UnboundedReceiver<CaptureEvent>>>,
    /// Configured fake mode has no external injector; keep its source alive until adapter drop.
    _source_keepalive: Option<mpsc::UnboundedSender<CaptureEvent>>,
    forwarder: Option<JoinHandle<Vec<CaptureEvent>>>,
    stop_forwarder: Option<oneshot::Sender<()>>,
    lifecycle: Vec<CaptureEvent>,
    released: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct FakeCaptureHandle {
    cap: Arc<CapState>,
    tx: mpsc::UnboundedSender<CaptureEvent>,
    zones: Arc<Mutex<Vec<Zone>>>,
    barriers: Arc<Mutex<Vec<BarrierSpec>>>,
    released: Arc<AtomicUsize>,
}

impl FakeCapture {
    /// The configured fake has known unlocked state and no external event injector.
    /// The controlled constructor below deliberately retains its handle-drop/EOF behavior.
    pub(crate) fn standalone(zones: Vec<Zone>) -> Self {
        let (mut cap, handle) = Self::new(zones.clone());
        handle.enable_lifecycle(super::CaptureLifecycle {
            revision: 1,
            invalidation_generation: 0,
            valid: true,
            session_locked: Some(false),
            desktop_available: Some(true),
            zones,
            terminal_error: None,
        });
        cap._source_keepalive = Some(handle.tx);
        cap
    }

    pub fn new(zones: Vec<Zone>) -> (Self, FakeCaptureHandle) {
        let (tx, rx) = mpsc::unbounded_channel();
        let released = Arc::new(AtomicUsize::new(0));
        let zones = Arc::new(Mutex::new(zones));
        let cap = Arc::new(CapState::default());
        let barriers = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                cap: cap.clone(),
                zones: zones.clone(),
                barriers: barriers.clone(),
                src: Arc::new(AsyncMutex::new(rx)),
                _source_keepalive: None,
                forwarder: None,
                stop_forwarder: None,
                lifecycle: Vec::new(),
                released: released.clone(),
            },
            FakeCaptureHandle {
                cap,
                tx,
                zones,
                barriers,
                released,
            },
        )
    }
}

impl FakeCaptureHandle {
    /// Explicit regression harness input; configured fake mode never supplies this.
    pub fn set_monitor_observation(&self, value: Option<crate::inventory::ObservedMonitors>) {
        *lock(&self.cap.observation) = value;
    }

    /// Seleciona o canal independente antes de iniciar o módulo.
    pub fn enable_lifecycle(&self, snapshot: super::CaptureLifecycle) {
        *lock(&self.cap.lifecycle) = Some(tokio::sync::watch::channel(snapshot).0);
    }
    /// Publica somente estado, independentemente da fila de dados.
    pub fn publish_lifecycle(&self, snapshot: super::CaptureLifecycle) {
        if let Some(tx) = lock(&self.cap.lifecycle).as_ref() {
            tx.send_replace(snapshot);
        }
    }

    pub fn activate(&self, barrier: usize, pos: f32) {
        let _ = self.tx.send(CaptureEvent::Activated {
            barrier,
            pos,
            t_mono_us: mono_us(),
        });
    }
    pub fn push(&self, ev: InputEvent) {
        let _ = self.tx.send(CaptureEvent::Input {
            t_mono_us: mono_us(),
            ev,
        });
    }
    pub fn deactivate(&self) {
        let _ = self.tx.send(CaptureEvent::Deactivated);
    }
    /// How many times the module called `release()` or `release_at()`.
    pub fn released(&self) -> usize {
        self.released.load(Ordering::SeqCst)
    }
    /// A recognized shortcut, as the real adapters report it.
    pub fn hotkey(&self, h: Hotkey) {
        let _ = self.tx.send(CaptureEvent::Hotkey(h));
    }
    /// The monitors changed: `known_zones()` answers with `z` from now on and the module is told.
    pub fn set_zones(&self, z: Vec<Zone>) {
        *lock(&self.zones) = z.clone();
        if let Some(tx) = lock(&self.cap.lifecycle).as_ref() {
            tx.send_modify(|s| {
                s.zones = z;
                s.revision = s.revision.saturating_add(1);
            });
        } else {
            let _ = self.tx.send(CaptureEvent::ZonesChanged(z));
        }
    }
    /// This machine's screen locked (`true`) or came back.
    pub fn session_locked(&self, on: bool) {
        self.cap.screen_locked.store(on, Ordering::SeqCst);
        if let Some(tx) = lock(&self.cap.lifecycle).as_ref() {
            tx.send_modify(|s| {
                s.revision = s.revision.saturating_add(1);
                if on {
                    s.invalidation_generation = s.invalidation_generation.saturating_add(1);
                }
                s.session_locked = Some(on);
            });
        } else if self.cap.started.load(Ordering::SeqCst) {
            let _ = self.tx.send(CaptureEvent::SessionLocked(on));
        }
    }
    /// The barriers the module installed last, by `start` or `set_barriers`.
    pub fn barriers(&self) -> Vec<BarrierSpec> {
        lock(&self.barriers).clone()
    }
    /// Makes the adapter behave like one that holds system-wide hooks (`holds_hooks`). Call it
    /// before the module runs.
    pub fn set_hooks(&self, on: bool) {
        self.cap.hooks.store(on, Ordering::SeqCst);
    }
    /// True between `start` and `stop`: a hook-holding adapter is installed exactly then.
    pub fn started(&self) -> bool {
        self.cap.started.load(Ordering::SeqCst)
    }
    /// How many times the module called `start`.
    pub fn starts(&self) -> usize {
        self.cap.starts.load(Ordering::SeqCst)
    }
    /// `(barrier, pos)` of every `release_at` call, in order.
    pub fn released_at(&self) -> Vec<(usize, f32)> {
        lock(&self.cap.released_at).clone()
    }
    /// `(barrier, pos)` of every `begin` call: a switch by shortcut or button, never a crossing.
    pub fn begins(&self) -> Vec<(usize, f32)> {
        lock(&self.cap.begins).clone()
    }
    /// The adapter cannot start a capture without a crossing (the Wayland portal).
    pub fn set_can_begin(&self, on: bool) {
        self.cap.cannot_begin.store(!on, Ordering::SeqCst);
    }
    /// True while the module listens for the resume combination (`watch_resume`).
    pub fn resume_watching(&self) -> bool {
        lock(&self.cap.resume_tx).is_some()
    }
    /// The system refuses to register the resume combination (the tray stays as the way back).
    pub fn refuse_resume_hotkey(&self, on: bool) {
        self.cap.resume_refused.store(on, Ordering::SeqCst);
    }
    /// The user pressed the resume combination: reaches the module only while it is watching.
    pub fn resume_hotkey(&self) {
        if let Some(tx) = lock(&self.cap.resume_tx).as_ref() {
            let _ = tx.try_send(CaptureEvent::Hotkey(Hotkey::TogglePause));
        }
    }
}

#[async_trait]
impl CaptureAdapter for FakeCapture {
    fn known_monitor_observation(&self) -> Result<crate::inventory::ObservedMonitors, InputError> {
        lock(&self.cap.observation)
            .clone()
            .ok_or_else(crate::inventory::unavailable)
    }
    async fn prepare_monitor_observation(
        &mut self,
    ) -> Result<crate::inventory::ObservedMonitors, InputError> {
        self.known_monitor_observation()
    }

    fn lifecycle_receiver(&self) -> Option<tokio::sync::watch::Receiver<super::CaptureLifecycle>> {
        lock(&self.cap.lifecycle).as_ref().map(|tx| tx.subscribe())
    }
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        Ok(lock(&self.zones).clone())
    }
    fn known_zones(&self) -> Vec<Zone> {
        lock(&self.zones).clone()
    }
    async fn start(
        &mut self,
        barriers: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        if self.forwarder.is_some() || self.cap.starts.load(Ordering::SeqCst) > 0 {
            self.stop().await?;
        }
        *lock(&self.barriers) = barriers.to_vec();
        self.cap.started.store(true, Ordering::SeqCst);
        self.cap.starts.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel(4096);
        let (stop_tx, mut stop_rx) = oneshot::channel();
        self.stop_forwarder = Some(stop_tx);
        let src = self.src.clone();
        let mut initial = std::mem::take(&mut self.lifecycle);
        if lock(&self.cap.lifecycle).is_none() {
            initial.push(CaptureEvent::SessionLocked(
                self.cap.screen_locked.load(Ordering::SeqCst),
            ));
        }
        self.forwarder = Some(tokio::spawn(async move {
            let mut src = src.lock().await;
            let mut initial = std::collections::VecDeque::from(initial);
            loop {
                let ev = if let Some(ev) = initial.pop_front() {
                    ev
                } else {
                    tokio::select! { biased; _ = &mut stop_rx => break, ev = src.recv() => match ev { Some(ev) => ev, None => break } }
                };
                tokio::select! {
                    biased;
                    _ = &mut stop_rx => { initial.push_front(ev); break; }
                    permit = tx.reserve() => match permit { Ok(permit) => permit.send(ev), Err(_) => { initial.push_front(ev); break; } }
                }
            }
            initial.into_iter().filter(is_lifecycle).collect()
        }));
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.released.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn set_barriers(&mut self, barriers: &[BarrierSpec]) -> Result<(), InputError> {
        *lock(&self.barriers) = barriers.to_vec();
        Ok(())
    }
    async fn release_at(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        lock(&self.cap.released_at).push((barrier, pos));
        self.released.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn begin(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        lock(&self.cap.begins).push((barrier, pos));
        Ok(())
    }
    fn can_begin(&self) -> bool {
        !self.cap.cannot_begin.load(Ordering::SeqCst)
    }
    fn holds_hooks(&self) -> bool {
        self.cap.hooks.load(Ordering::SeqCst)
    }
    async fn watch_resume(&mut self) -> Result<Option<mpsc::Receiver<CaptureEvent>>, InputError> {
        if self.cap.resume_refused.load(Ordering::SeqCst) {
            return Err(InputError::Other("hotkey refused".into()));
        }
        let (tx, rx) = mpsc::channel(8);
        *lock(&self.cap.resume_tx) = Some(tx);
        Ok(Some(rx))
    }
    async fn unwatch_resume(&mut self) {
        *lock(&self.cap.resume_tx) = None;
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.cap.started.store(false, Ordering::SeqCst);
        if let Some(stop) = self.stop_forwarder.take() {
            let _ = stop.send(());
        }
        if let Some(h) = self.forwarder.take()
            && let Ok(pending) = h.await
        {
            self.lifecycle.extend(pending);
        }
        let mut src = self.src.lock().await;
        while let Ok(ev) = src.try_recv() {
            if is_lifecycle(&ev) {
                self.lifecycle.push(ev);
            }
        }
        Ok(())
    }
}

fn is_lifecycle(ev: &CaptureEvent) -> bool {
    !matches!(
        ev,
        CaptureEvent::Input { .. } | CaptureEvent::Activated { .. }
    )
}

#[derive(Default)]
struct EmuState {
    started: bool,
    emitted: usize,
    /// (is_button, code)
    pressed: HashSet<(bool, u32)>,
    release_calls: usize,
    locked: bool,
    events: Vec<InputEvent>,
}

pub struct FakeEmulate {
    st: Arc<Mutex<EmuState>>,
}

#[derive(Clone)]
pub struct FakeEmulateHandle {
    st: Arc<Mutex<EmuState>>,
}

impl FakeEmulate {
    pub fn new() -> (Self, FakeEmulateHandle) {
        let st = Arc::new(Mutex::new(EmuState::default()));
        (Self { st: st.clone() }, FakeEmulateHandle { st })
    }
}

impl FakeEmulateHandle {
    pub fn emitted(&self) -> usize {
        lock(&self.st).emitted
    }
    pub fn pressed_now(&self) -> usize {
        lock(&self.st).pressed.len()
    }
    /// True once the module called `start()` (and not `stop()` since).
    pub fn started(&self) -> bool {
        lock(&self.st).started
    }
    pub fn release_calls(&self) -> usize {
        lock(&self.st).release_calls
    }
    /// While on, `emit` and `enter` answer `InputError::Locked` and apply nothing (screen locked
    /// or secure desktop on the destination).
    pub fn set_locked(&self, on: bool) {
        lock(&self.st).locked = on;
    }
    /// Every event applied so far, in order.
    pub fn events(&self) -> Vec<InputEvent> {
        lock(&self.st).events.clone()
    }
}

#[async_trait]
impl EmulateAdapter for FakeEmulate {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn start(&mut self) -> Result<(), InputError> {
        lock(&self.st).started = true;
        Ok(())
    }
    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        let mut s = lock(&self.st);
        if !s.started {
            return Err(InputError::Other("emulator not started".into()));
        }
        if s.locked {
            return Err(InputError::Locked);
        }
        for ev in events {
            s.emitted += 1;
            s.events.push(*ev);
            match *ev {
                InputEvent::Key { code, pressed } => {
                    if pressed {
                        s.pressed.insert((false, code));
                    } else {
                        s.pressed.remove(&(false, code));
                    }
                }
                InputEvent::Button { code, pressed } => {
                    if pressed {
                        s.pressed.insert((true, code));
                    } else {
                        s.pressed.remove(&(true, code));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    async fn release_all(&mut self) -> Result<(), InputError> {
        let mut s = lock(&self.st);
        s.pressed.clear();
        s.release_calls += 1;
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        lock(&self.st).started = false;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod standalone_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn standalone_has_explicit_snapshot_before_start_and_stop_joins_each_producer() {
        let mut cap = FakeCapture::standalone(vec![crate::fake_zone()]);
        let state = cap
            .lifecycle_receiver()
            .expect("standalone must publish known fake state before start");
        assert!(state.borrow().allows_capture());
        assert_eq!(state.borrow().zones.len(), 1);
        for _ in 0..2 {
            let mut rx = cap.start(&[]).await.unwrap();
            assert!(state.borrow().allows_capture());
            assert!(
                tokio::time::timeout(Duration::from_millis(25), rx.recv())
                    .await
                    .is_err(),
                "quiet standalone must stay alive without an external injector"
            );
            cap.stop().await.unwrap();
            assert!(cap.forwarder.is_none(), "stop must join the owned producer");
            assert!(cap.stop_forwarder.is_none());
            assert!(
                tokio::time::timeout(Duration::from_millis(100), rx.recv())
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[tokio::test]
    async fn standalone_source_does_not_eof_without_an_injector() {
        let mut cap = FakeCapture::standalone(vec![crate::fake_zone()]);
        let mut rx = cap.start(&[]).await.unwrap();
        let ended = tokio::time::timeout(Duration::from_millis(25), async {
            while rx.recv().await.is_some() {}
        })
        .await;
        cap.stop().await.unwrap();
        assert!(
            ended.is_err(),
            "standalone source closed when its injector was dropped"
        );
    }

    #[tokio::test]
    async fn controlled_fake_still_reaches_eof_when_its_handle_is_dropped() {
        let (mut cap, handle) = FakeCapture::new(vec![crate::fake_zone()]);
        drop(handle);
        let mut rx = cap.start(&[]).await.unwrap();
        assert!(matches!(
            rx.recv().await,
            Some(CaptureEvent::SessionLocked(false))
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), rx.recv())
                .await
                .unwrap()
                .is_none()
        );
        cap.stop().await.unwrap();
        assert!(cap.forwarder.is_none());
    }
}
