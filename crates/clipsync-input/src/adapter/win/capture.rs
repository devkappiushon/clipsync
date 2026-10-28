//! `WinCapture` (name `"win-hooks"`): the low-level hooks of `clipsync-win` plus the decision core of
//! `wincore`. Contract §13.6, "Adaptador Windows" item 1.
//!
//! Three threads: the hook thread (the callback, which only decides and queues, never waits); a worker
//! that does what the callback must not (parks the cursor, calls `SendInput`, feeds the module's
//! channel); and the caller's async side (`release_at`, `set_barriers`, `stop`). The queue between the
//! first two is bounded; if it fills up the capture ends and the queue is dropped (Sentinel R4).
//!
//! Nothing here logs a key, a scan code or a character (§13.2 rule 2e).
use super::super::wincore::{
    ArmedBarrier, CaptureCore, Held, Out, Restart, RestartPolicy, repress_synths, resolve_barriers,
};
use super::super::winmath::{self, NEUTRAL_KEY, Rect};
use super::super::{BarrierSpec, CaptureAdapter, CaptureEvent, Hotkey, InputError, Zone};
use crate::keymap;
use async_trait::async_trait;
use clipsync_proto::{InputEvent, mono_us};
use clipsync_win::{
    CursorPark, Hooks, Modifier, Notice, RawEvent, ResumeHotkey, Synth, WinButton, WinError,
};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Events waiting between the hook thread and the worker (Sentinel R4).
const QUEUE: usize = 8000;
/// How often the idle worker looks at the stop flag.
const STOP_POLL: Duration = Duration::from_millis(100);
/// What the module's channel holds before the worker waits.
const CHANNEL: usize = 4096;

const MODIFIER_KEYS: [(Modifier, u32); 8] = [
    (Modifier::LeftCtrl, 29),
    (Modifier::LeftShift, 42),
    (Modifier::RightShift, 54),
    (Modifier::LeftAlt, 56),
    (Modifier::RightCtrl, 97),
    (Modifier::RightAlt, 100),
    (Modifier::LeftWin, 125),
    (Modifier::RightWin, 126),
];
const BUTTONS: [WinButton; 5] = [
    WinButton::Left,
    WinButton::Right,
    WinButton::Middle,
    WinButton::X1,
    WinButton::X2,
];

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Msg {
    Out(Out, u64),
    Notice(Notice),
}

/// What the three sides share. Everything the hook thread touches here is an atomic or a `try_lock`:
/// it never waits.
struct Shared {
    barriers: Mutex<Arc<[Option<ArmedBarrier>]>>,
    specs: Mutex<Vec<BarrierSpec>>,
    generation: AtomicU32,
    locked: AtomicBool,
    /// The async side or the worker ends the capture: the hook goes back to passing everything.
    end: AtomicBool,
    /// A session was opened by a switch: capture from now on.
    begin: AtomicBool,
    /// The cursor is parked at `park_point`: the core may start reading deltas.
    ready: AtomicBool,
    park_point: AtomicU64,
    park_dpi: AtomicU32,
    held: [AtomicU64; 5],
    overflow: AtomicBool,
    /// The adapter is stopping: the worker leaves by this flag, which does not depend on the queue
    /// (the worker keeps a sender of its own, so the queue never closes by itself; Sentinel S7-07).
    stop: AtomicBool,
    park: Mutex<Option<CursorPark>>,
    hooks: Mutex<Option<Hooks>>,
    /// Where the last capture began, to give the cursor back.
    last: Mutex<Option<(usize, f32)>>,
}

impl Shared {
    fn new() -> Shared {
        Shared {
            barriers: Mutex::new(Arc::from(Vec::new())),
            specs: Mutex::new(Vec::new()),
            generation: AtomicU32::new(0),
            locked: AtomicBool::new(false),
            end: AtomicBool::new(false),
            begin: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            park_point: AtomicU64::new(0),
            park_dpi: AtomicU32::new(96),
            held: Default::default(),
            overflow: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            park: Mutex::new(None),
            hooks: Mutex::new(None),
            last: Mutex::new(None),
        }
    }

    fn install(&self, specs: &[BarrierSpec], zones: &[Zone]) {
        *lock(&self.specs) = specs.to_vec();
        *lock(&self.barriers) = resolve_barriers(specs, zones);
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// A new hook instance starts from nothing: the set of held keys of the old one, and the flags
    /// that belonged to its capture, must not leak into the next (Sentinel S7-05).
    fn reset_input(&self) {
        for slot in &self.held {
            slot.store(0, Ordering::Relaxed);
        }
        self.begin.store(false, Ordering::Release);
        self.ready.store(false, Ordering::Release);
    }

    fn held(&self) -> Held {
        let words = std::array::from_fn(|i| self.held[i].load(Ordering::Relaxed));
        Held::from_words(words)
    }
}

fn pack(x: i32, y: i32) -> u64 {
    (u64::from(x as u32) << 32) | u64::from(y as u32)
}

fn unpack(v: u64) -> (i32, i32) {
    ((v >> 32) as u32 as i32, v as u32 as i32)
}

fn io_err(call: &str, e: &WinError) -> InputError {
    InputError::Other(format!("{call}: {e}"))
}

fn zones_now() -> Vec<Zone> {
    clipsync_win::monitors()
        .map(|m| winmath::zones(&m))
        .unwrap_or_default()
}

fn virtual_rect() -> Rect {
    let (x, y, w, h) = clipsync_win::virtual_screen();
    Rect {
        x,
        y,
        width: w.max(0) as u32,
        height: h.max(0) as u32,
    }
}

// ---------------------------------------------------------------------------------------------
// The hook callback

/// The closure that runs inside the hook callback: it syncs with what the other sides said (atomics),
/// lets the core decide, and queues what the worker has to do. It waits for nothing.
fn hook_closure(
    shared: Arc<Shared>,
    queue: SyncSender<Msg>,
) -> Box<dyn FnMut(&RawEvent) -> clipsync_win::Verdict + Send> {
    let mut core = CaptureCore::new();
    let mut seen_generation = u32::MAX;
    let mut stored = Held::default();
    Box::new(move |event| {
        if shared.end.swap(false, Ordering::AcqRel) {
            core.end_capture();
        }
        let generation = shared.generation.load(Ordering::Acquire);
        if generation != seen_generation
            && let Ok(barriers) = shared.barriers.try_lock()
        {
            core.set_barriers(Arc::clone(&barriers));
            seen_generation = generation;
        }
        core.set_locked(shared.locked.load(Ordering::Relaxed));
        if shared.begin.swap(false, Ordering::AcqRel) {
            core.begin();
        }
        if shared.ready.swap(false, Ordering::AcqRel) {
            let point = unpack(shared.park_point.load(Ordering::Acquire));
            core.park_ready(point, shared.park_dpi.load(Ordering::Relaxed));
        }
        let mut overflowed = false;
        let verdict = core.on_event(event, &mut |out| {
            if queue.try_send(Msg::Out(out, mono_us())).is_err() {
                overflowed = true;
            }
        });
        if overflowed {
            // The queue is full: end the capture here, so the local desktop answers again, and let
            // the worker tell the module.
            shared.overflow.store(true, Ordering::Release);
            core.end_capture();
        }
        let held = core.held();
        if held != stored {
            for (slot, word) in shared.held.iter().zip(held.to_words()) {
                slot.store(word, Ordering::Relaxed);
            }
            stored = held;
        }
        verdict
    })
}

fn start_hooks(shared: &Arc<Shared>, queue: &SyncSender<Msg>) -> Result<Hooks, WinError> {
    // Every instance, the first or a recreated one, starts from nothing (Sentinel S7-05).
    shared.reset_input();
    let notices = queue.clone();
    let flag = Arc::clone(shared);
    Hooks::start(
        hook_closure(Arc::clone(shared), queue.clone()),
        Box::new(move |notice| {
            if notices.try_send(Msg::Notice(notice)).is_err() {
                flag.overflow.store(true, Ordering::Release);
            }
        }),
    )
}

// ---------------------------------------------------------------------------------------------
// What the worker does for the hook thread

fn press_batch(held: &Held) -> Vec<InputEvent> {
    let mut events = Vec::new();
    for code in held.modifiers() {
        events.push(InputEvent::Key {
            code,
            pressed: true,
        });
    }
    for button in held.buttons() {
        events.push(InputEvent::Button {
            code: keymap::button_from_windows(button),
            pressed: true,
        });
    }
    events
}

/// What the system says is down although the hook may have missed it (Sentinel I-S7-3).
fn reconcile(mut held: Held) -> Held {
    for (modifier, code) in MODIFIER_KEYS {
        if clipsync_win::modifier_down(modifier) {
            held.add_key(code);
        }
    }
    for button in BUTTONS {
        if clipsync_win::button_down(button) {
            held.add_button(button);
        }
    }
    held
}

/// Lets go locally of everything in `held` (the injected releases come back marked and the hook
/// ignores them).
fn release_locally(held: &Held) -> Result<usize, WinError> {
    let mut batch = Vec::new();
    for code in held.keys() {
        if let Some(key) = keymap::to_windows(code) {
            batch.push(Synth::Key { key, up: true });
        }
    }
    for button in held.buttons() {
        batch.push(Synth::Button {
            button,
            down: false,
        });
    }
    if batch.is_empty() {
        return Ok(0);
    }
    clipsync_win::send(&batch)
}

/// Parks the cursor at the middle of the barrier's monitor and carries the held modifiers and buttons
/// over. Returns what has to go to the peer to press them there.
fn park_and_carry(
    shared: &Shared,
    barrier: usize,
    held: Held,
) -> Result<Vec<InputEvent>, InputError> {
    let zone = lock(&shared.barriers)
        .get(barrier)
        .and_then(|b| b.as_ref().map(|b| b.zone.clone()))
        .ok_or_else(|| InputError::ZoneNotFound(format!("#{barrier}")))?;
    let x = zone.x + (zone.width / 2) as i32;
    let y = zone.y + (zone.height / 2) as i32;
    let park = CursorPark::new(x, y).map_err(|e| io_err("park", &e))?;
    *lock(&shared.park) = Some(park);

    let held = reconcile(held);
    release_locally(&held).map_err(|e| io_err("release", &e))?;

    shared.park_point.store(pack(x, y), Ordering::Release);
    shared
        .park_dpi
        .store((zone.scale * 96.0).round() as u32, Ordering::Relaxed);
    shared.ready.store(true, Ordering::Release);
    Ok(press_batch(&held))
}

/// The capture ends: the hook passes again, the cursor stops being parked, and (if asked) it goes back
/// to `cursor` with the modifiers and buttons that are still held pressed again locally.
fn end_locally(shared: &Shared, cursor: Option<(i32, i32)>) -> Result<(), InputError> {
    shared.end.store(true, Ordering::Release);
    shared.ready.store(false, Ordering::Release);
    let park = lock(&shared.park).take();
    drop(park);
    let mut batch = Vec::new();
    if let Some((x, y)) = cursor {
        let (nx, ny) = winmath::normalize_abs(virtual_rect(), x, y);
        batch.push(Synth::MoveAbs { nx, ny });
    }
    // Only what the system says is physically down now: never the set an old hook instance kept.
    batch.extend(repress_synths(&reconcile(Held::default())));
    if batch.is_empty() {
        return Ok(());
    }
    clipsync_win::send(&batch)
        .map(drop)
        .map_err(|e| io_err("send", &e))
}

struct Worker {
    shared: Arc<Shared>,
    queue: SyncSender<Msg>,
    inbox: Receiver<Msg>,
    out: mpsc::Sender<CaptureEvent>,
    session_locked: bool,
    secure_desktop: bool,
    restarts: RestartPolicy,
    clock: Instant,
}

impl Worker {
    /// Sends to the module's channel. A full channel is waited for in small steps, so a stop is
    /// seen even when the module no longer reads.
    fn emit(&self, mut event: CaptureEvent) -> bool {
        loop {
            match self.out.try_send(event) {
                Ok(()) => return true,
                Err(mpsc::error::TrySendError::Closed(_)) => return false,
                Err(mpsc::error::TrySendError::Full(back)) => {
                    if self.shared.stop.load(Ordering::Acquire) {
                        return false;
                    }
                    event = back;
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }

    fn sync_lock(&mut self) {
        self.shared.locked.store(
            self.session_locked || self.secure_desktop,
            Ordering::Relaxed,
        );
    }

    fn run(mut self) {
        loop {
            if self.shared.stop.load(Ordering::Acquire) {
                return;
            }
            let msg = match self.inbox.recv_timeout(STOP_POLL) {
                Ok(msg) => msg,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            if self.shared.overflow.swap(false, Ordering::AcqRel) {
                // Whatever is still queued is dropped: no key typed minutes ago reaches the peer.
                while self.inbox.try_recv().is_ok() {}
                let _ = end_locally(&self.shared, None);
                if !self.emit(CaptureEvent::Deactivated) {
                    return;
                }
                continue;
            }
            let alive = match msg {
                Msg::Out(out, t) => self.on_out(out, t),
                Msg::Notice(notice) => self.on_notice(notice),
            };
            if !alive {
                return;
            }
        }
    }

    fn on_out(&mut self, out: Out, t_mono_us: u64) -> bool {
        match out {
            Out::Activate { barrier, pos, held } => {
                match park_and_carry(&self.shared, barrier, held) {
                    Ok(presses) => {
                        *lock(&self.shared.last) = Some((barrier, pos));
                        if !self.emit(CaptureEvent::Activated {
                            barrier,
                            pos,
                            t_mono_us,
                        }) {
                            return false;
                        }
                        presses
                            .into_iter()
                            .all(|ev| self.emit(CaptureEvent::Input { t_mono_us, ev }))
                    }
                    Err(e) => {
                        let _ = end_locally(&self.shared, None);
                        self.emit(CaptureEvent::Error(e.to_string()))
                    }
                }
            }
            Out::Input(ev) => self.emit(CaptureEvent::Input { t_mono_us, ev }),
            Out::Hotkey(hotkey) => self.emit(CaptureEvent::Hotkey(hotkey)),
            Out::TapThenRelease(code) => {
                let mut batch = vec![
                    Synth::Key {
                        key: NEUTRAL_KEY,
                        up: false,
                    },
                    Synth::Key {
                        key: NEUTRAL_KEY,
                        up: true,
                    },
                ];
                if let Some(key) = keymap::to_windows(code) {
                    batch.push(Synth::Key { key, up: true });
                }
                if let Err(e) = clipsync_win::send(&batch) {
                    return self.emit(CaptureEvent::Error(format!("send: {e}")));
                }
                true
            }
        }
    }

    fn on_notice(&mut self, notice: Notice) -> bool {
        match notice {
            Notice::DisplayChanged => {
                let zones = zones_now();
                let specs = lock(&self.shared.specs).clone();
                self.shared.install(&specs, &zones);
                self.emit(CaptureEvent::ZonesChanged(zones))
            }
            Notice::SessionLocked => {
                self.session_locked = true;
                self.sync_lock();
                self.emit(CaptureEvent::SessionLocked(true))
            }
            Notice::SessionUnlocked => {
                self.session_locked = false;
                self.sync_lock();
                self.emit(CaptureEvent::SessionLocked(
                    self.session_locked || self.secure_desktop,
                ))
            }
            Notice::DesktopSwitched => {
                self.secure_desktop = !clipsync_win::input_desktop_is_default();
                self.sync_lock();
                self.emit(CaptureEvent::SessionLocked(
                    self.session_locked || self.secure_desktop,
                ))
            }
            Notice::HooksLost => self.recreate_hooks(),
        }
    }

    /// The hook died (a callback panicked, or the system took it away): the session ends (the module
    /// answers `Leave`, and the destination lets go of everything) and the hooks are made again, at
    /// most `RestartPolicy::MAX` times in `RestartPolicy::WINDOW_MS`. One more and the capture stops
    /// and the module registers a block with a fixed text; it only comes back when the module restarts
    /// (Sentinel S7-02, decided by the Maestro).
    fn recreate_hooks(&mut self) -> bool {
        // Stopping (or the pause, which stops the adapter) wins over a hook that was lost at the same
        // time: nothing is installed while the adapter is on its way out (Sentinel S7-08).
        if self.shared.stop.load(Ordering::Acquire) {
            return false;
        }
        let _ = end_locally(&self.shared, None);
        let dead = lock(&self.shared.hooks).take();
        drop(dead);
        if !self.emit(CaptureEvent::Deactivated) {
            return false;
        }
        let now_ms = u64::try_from(self.clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        match self.restarts.on_lost(now_ms) {
            Restart::GiveUp => {
                self.shared.locked.store(true, Ordering::Relaxed);
                self.emit(CaptureEvent::Error(super::HOOKS_GAVE_UP.to_owned()))
            }
            Restart::Recreate => match start_hooks(&self.shared, &self.queue) {
                Ok(hooks) => {
                    *lock(&self.shared.hooks) = Some(hooks);
                    true
                }
                Err(e) => self.emit(CaptureEvent::Error(format!("hooks: {e}"))),
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The adapter

pub struct WinCapture {
    shared: Arc<Shared>,
    queue: Option<SyncSender<Msg>>,
    worker: Option<JoinHandle<()>>,
    out: Option<mpsc::Sender<CaptureEvent>>,
    /// Win+Alt+P registered with the system while the module is paused. The system reports the
    /// combination also when a program injects it; the only effect is to toggle the pause.
    resume: Option<ResumeHotkey>,
}

impl Default for WinCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl WinCapture {
    pub fn new() -> WinCapture {
        WinCapture {
            shared: Arc::new(Shared::new()),
            queue: None,
            worker: None,
            out: None,
            resume: None,
        }
    }

    fn last_barrier(&self) -> Option<(usize, f32)> {
        *lock(&self.shared.last)
    }

    /// Where the cursor goes back to: one pixel inside the barrier's zone, at `pos` along its edge.
    fn return_point(&self, barrier: usize, pos: f32) -> Option<(i32, i32)> {
        let barriers = Arc::clone(&lock(&self.shared.barriers));
        let b = barriers.get(barrier)?.as_ref()?;
        Some(winmath::inside_point(&b.zone, b.edge, pos))
    }
}

#[async_trait]
impl CaptureAdapter for WinCapture {
    fn name(&self) -> &'static str {
        "win-hooks"
    }

    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        let monitors = clipsync_win::monitors().map_err(|e| io_err("monitors", &e))?;
        Ok(winmath::zones(&monitors))
    }

    fn known_zones(&self) -> Vec<Zone> {
        zones_now()
    }

    async fn start(
        &mut self,
        barriers: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        self.stop().await?;
        self.shared.stop.store(false, Ordering::Release);
        let (out_tx, out_rx) = mpsc::channel(CHANNEL);
        let (queue_tx, queue_rx) = sync_channel(QUEUE);
        self.shared.install(barriers, &zones_now());
        self.shared.end.store(false, Ordering::Relaxed);
        self.shared.overflow.store(false, Ordering::Relaxed);
        self.shared.locked.store(false, Ordering::Relaxed);

        let hooks = start_hooks(&self.shared, &queue_tx).map_err(|e| io_err("hooks", &e))?;
        *lock(&self.shared.hooks) = Some(hooks);

        let worker = Worker {
            shared: Arc::clone(&self.shared),
            queue: queue_tx.clone(),
            inbox: queue_rx,
            out: out_tx.clone(),
            session_locked: false,
            secure_desktop: false,
            restarts: RestartPolicy::new(),
            clock: Instant::now(),
        };
        self.worker = Some(std::thread::spawn(move || worker.run()));
        self.queue = Some(queue_tx);
        self.out = Some(out_tx);
        Ok(out_rx)
    }

    async fn release(&mut self) -> Result<(), InputError> {
        match self.last_barrier() {
            Some((barrier, pos)) => self.release_at(barrier, pos).await,
            None => Ok(()),
        }
    }

    async fn set_barriers(&mut self, barriers: &[BarrierSpec]) -> Result<(), InputError> {
        self.shared.install(barriers, &zones_now());
        Ok(())
    }

    async fn release_at(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        let cursor = self.return_point(barrier, pos);
        let shared = Arc::clone(&self.shared);
        tokio::task::spawn_blocking(move || end_locally(&shared, cursor))
            .await
            .map_err(|e| InputError::Other(e.to_string()))?
    }

    // A session was opened by a switch (shortcut or button) instead of a crossing: park the cursor at
    // the middle of the barrier's monitor and capture from now on. The modifiers and buttons held now
    // go to the peer pressed, as in a crossing, but no `Activated` is sent (the module already opened
    // the session).
    async fn begin(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        let (Some(out), true) = (self.out.clone(), self.queue.is_some()) else {
            return Err(InputError::Other("capture not started".into()));
        };
        let shared = Arc::clone(&self.shared);
        let held = shared.held();
        let presses = tokio::task::spawn_blocking(move || {
            shared.begin.store(true, Ordering::Release);
            let presses = park_and_carry(&shared, barrier, held);
            if presses.is_err() {
                shared.begin.store(false, Ordering::Release);
                let _ = end_locally(&shared, None);
            } else {
                *lock(&shared.last) = Some((barrier, pos));
            }
            presses
        })
        .await
        .map_err(|e| InputError::Other(e.to_string()))??;
        let t_mono_us = mono_us();
        for ev in presses {
            if out
                .send(CaptureEvent::Input { t_mono_us, ev })
                .await
                .is_err()
            {
                break;
            }
        }
        Ok(())
    }

    fn holds_hooks(&self) -> bool {
        true
    }

    async fn watch_resume(&mut self) -> Result<Option<mpsc::Receiver<CaptureEvent>>, InputError> {
        let (tx, rx) = mpsc::channel(8);
        let hotkey = ResumeHotkey::register(Box::new(move || {
            let _ = tx.try_send(CaptureEvent::Hotkey(Hotkey::TogglePause));
        }))
        .map_err(|e| io_err("RegisterHotKey", &e))?;
        self.resume = Some(hotkey);
        Ok(Some(rx))
    }

    async fn unwatch_resume(&mut self) {
        let hotkey = self.resume.take();
        let _ = tokio::task::spawn_blocking(move || drop(hotkey)).await;
    }

    async fn stop(&mut self) -> Result<(), InputError> {
        let shared = Arc::clone(&self.shared);
        let worker = self.worker.take();
        self.queue = None;
        self.out = None;
        tokio::task::spawn_blocking(move || {
            // The hooks first: nothing new reaches the queue; then the worker, by the flag.
            shared.stop.store(true, Ordering::Release);
            let hooks = lock(&shared.hooks).take();
            drop(hooks);
            if let Some(worker) = worker {
                let _ = worker.join();
            }
            // The worker may have installed hooks between the first take and its exit.
            let late = lock(&shared.hooks).take();
            drop(late);
            let _ = end_locally(&shared, None);
        })
        .await
        .map_err(|e| InputError::Other(e.to_string()))?;
        self.unwatch_resume().await;
        Ok(())
    }
}

impl Drop for WinCapture {
    fn drop(&mut self) {
        // Neither the hooks nor the worker outlive the adapter (§13.2 rule 2g, Sentinel S7-07). The
        // worker leaves by the flag within one poll, whatever the state of the queue.
        self.shared.stop.store(true, Ordering::Release);
        let hooks = lock(&self.shared.hooks).take();
        drop(hooks);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let late = lock(&self.shared.hooks).take();
        drop(late);
        let park = lock(&self.shared.park).take();
        drop(park);
    }
}

/// Runs on Windows only, with the rest of the file. The hook closure is called with events built by
/// hand: no real hook is installed.
#[cfg(test)]
mod tests {
    use super::*;
    use clipsync_proto::Edge;
    use clipsync_win::Verdict;

    use super::super::super::ZoneRef;

    fn zone() -> Zone {
        Zone {
            index: 0,
            name: Some("z".into()),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale: 1.0,
        }
    }

    fn shared_with_a_barrier() -> Arc<Shared> {
        let shared = Arc::new(Shared::new());
        let spec = BarrierSpec {
            zone: ZoneRef::Index(0),
            edge: Edge::Right,
            from: 0.0,
            to: 1.0,
        };
        shared.install(&[spec], &[zone()]);
        shared
    }

    fn key(scan: u32, extended: bool, up: bool) -> RawEvent {
        RawEvent::Key {
            vk: 0,
            scan,
            extended,
            up,
        }
    }

    /// Sentinel S7-05: pause with Win and Alt held, resume, cross with the mouse only and come back:
    /// no Win or Alt may travel with the crossing or be pressed again on the way back.
    #[test]
    fn a_new_hook_instance_forgets_the_keys_the_old_one_held() {
        let shared = shared_with_a_barrier();
        let (queue, inbox) = sync_channel(64);

        // the first instance sees Win and Alt go down, then the hooks are removed (the pause)
        let mut first = hook_closure(Arc::clone(&shared), queue.clone());
        assert_eq!(first(&key(0x5B, true, false)), Verdict::Pass);
        assert_eq!(first(&key(0x38, false, false)), Verdict::Pass);
        assert!(!shared.held().is_empty());
        drop(first);

        // resume: a new instance
        shared.reset_input();
        assert!(shared.held().is_empty());
        let mut second = hook_closure(Arc::clone(&shared), queue);
        // the user crosses with the mouse only
        assert_eq!(second(&RawEvent::Move { x: 1000, y: 500 }), Verdict::Pass);
        assert_eq!(second(&RawEvent::Move { x: 1919, y: 540 }), Verdict::Block);
        let Ok(Msg::Out(Out::Activate { held, .. }, _)) = inbox.try_recv() else {
            panic!("the crossing did not activate");
        };
        assert!(held.is_empty(), "nothing is held, nothing travels");
        assert!(shared.held().is_empty());
        // and on the way back nothing is pressed again unless the system says it is physically down
        assert!(repress_synths(&Held::default()).is_empty());
    }

    #[test]
    fn resetting_clears_the_flags_of_the_old_capture_too() {
        let shared = Shared::new();
        shared.begin.store(true, Ordering::SeqCst);
        shared.ready.store(true, Ordering::SeqCst);
        shared.reset_input();
        assert!(!shared.begin.load(Ordering::SeqCst));
        assert!(!shared.ready.load(Ordering::SeqCst));
    }

    /// Sentinel S7-07: the worker keeps a sender of its own, so the queue never closes by itself. It
    /// must still leave when told to, with the queue full and the module's channel full.
    #[test]
    fn the_worker_leaves_by_the_flag_even_with_everything_full() {
        let shared = Arc::new(Shared::new());
        let (queue, inbox) = sync_channel(1);
        let (out, _unread) = mpsc::channel(1);
        // the module's channel is full, and so is the queue
        out.try_send(CaptureEvent::Deactivated).unwrap();
        queue
            .try_send(Msg::Out(Out::Hotkey(Hotkey::ToggleLock), 0))
            .unwrap();
        let worker = Worker {
            shared: Arc::clone(&shared),
            queue: queue.clone(),
            inbox,
            out,
            session_locked: false,
            secure_desktop: false,
            restarts: RestartPolicy::new(),
            clock: Instant::now(),
        };
        let handle = std::thread::spawn(move || worker.run());
        std::thread::sleep(Duration::from_millis(150));
        assert!(
            !handle.is_finished(),
            "it is stuck on the full channel, as expected"
        );
        shared.stop.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !handle.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(handle.is_finished(), "the worker did not leave");
        drop(queue);
    }

    /// Sentinel S7-08: a hook lost while the adapter is stopping is not recreated.
    #[test]
    fn a_lost_hook_is_not_recreated_while_stopping() {
        let shared = Arc::new(Shared::new());
        let (queue, inbox) = sync_channel(4);
        let (out, _unread) = mpsc::channel(4);
        shared.stop.store(true, Ordering::Release);
        let mut worker = Worker {
            shared: Arc::clone(&shared),
            queue,
            inbox,
            out,
            session_locked: false,
            secure_desktop: false,
            restarts: RestartPolicy::new(),
            clock: Instant::now(),
        };
        assert!(!worker.recreate_hooks());
        assert!(lock(&shared.hooks).is_none(), "no hook may be installed");
    }
}
