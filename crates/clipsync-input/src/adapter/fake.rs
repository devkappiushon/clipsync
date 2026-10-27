//! In-memory adapters for tests and for `capture = "fake"` / `emulate = "fake"`.
use super::{BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, InputError, Zone};
use async_trait::async_trait;
use clipsync_proto::{InputEvent, mono_us};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio::task::JoinHandle;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct FakeCapture {
    zones: Vec<Zone>,
    src: Arc<AsyncMutex<mpsc::UnboundedReceiver<CaptureEvent>>>,
    forwarder: Option<JoinHandle<()>>,
    released: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct FakeCaptureHandle {
    tx: mpsc::UnboundedSender<CaptureEvent>,
    released: Arc<AtomicUsize>,
}

impl FakeCapture {
    pub fn new(zones: Vec<Zone>) -> (Self, FakeCaptureHandle) {
        let (tx, rx) = mpsc::unbounded_channel();
        let released = Arc::new(AtomicUsize::new(0));
        (
            Self {
                zones,
                src: Arc::new(AsyncMutex::new(rx)),
                forwarder: None,
                released: released.clone(),
            },
            FakeCaptureHandle { tx, released },
        )
    }
}

impl FakeCaptureHandle {
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
    /// How many times the module called `release()`.
    pub fn released(&self) -> usize {
        self.released.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl CaptureAdapter for FakeCapture {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        Ok(self.zones.clone())
    }
    async fn start(
        &mut self,
        _barriers: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        if let Some(h) = self.forwarder.take() {
            h.abort();
        }
        let (tx, rx) = mpsc::channel(4096);
        let src = self.src.clone();
        self.forwarder = Some(tokio::spawn(async move {
            let mut src = src.lock().await;
            while let Some(ev) = src.recv().await {
                if tx.send(ev).await.is_err() {
                    break;
                }
            }
        }));
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.released.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        if let Some(h) = self.forwarder.take() {
            h.abort();
        }
        Ok(())
    }
}

#[derive(Default)]
struct EmuState {
    started: bool,
    emitted: usize,
    /// (is_button, code)
    pressed: HashSet<(bool, u32)>,
    release_calls: usize,
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
        for ev in events {
            s.emitted += 1;
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
