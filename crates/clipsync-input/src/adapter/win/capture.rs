//! `WinCapture` (name `"win-hooks"`): the low-level hooks of `clipsync-win` plus the decision core of
//! `wincore`. Contract §13.6, "Adaptador Windows" item 1.
//!
//! Three threads: the hook thread (the callback, which only decides and queues, never waits); a worker
//! that does what the callback must not (parks the cursor, calls `SendInput`, feeds the module's
//! channel); and the caller's async side (`release_at`, `set_barriers`, `stop`). The queue between the
//! first two is bounded; if it fills up the capture ends and the queue is dropped (Sentinel R4).
//!
//! Nothing here logs a key, a scan code or a character (§13.2 rule 2e).
use super::super::lifecycle::{CaptureLifecycle, LifecycleMailbox};
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
    Out(Out, u64, u64),
}

/// What the three sides share. Everything the hook thread touches here is an atomic or a `try_lock`:
/// it never waits.
struct Shared {
    #[cfg(test)]
    hook_starts: std::sync::atomic::AtomicUsize,
    restart_policy: Mutex<RestartPolicy>,
    restart_clock: Instant,
    mailbox: LifecycleMailbox,
    published_generation: AtomicU64,
    lifecycle: tokio::sync::watch::Sender<CaptureLifecycle>,
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
            #[cfg(test)]
            hook_starts: std::sync::atomic::AtomicUsize::new(0),
            restart_policy: Mutex::new(RestartPolicy::new()),
            restart_clock: Instant::now(),
            mailbox: LifecycleMailbox::default(),
            published_generation: AtomicU64::new(u64::MAX),
            lifecycle: tokio::sync::watch::channel(CaptureLifecycle::default()).0,
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

    fn capture_allowed(&self) -> bool {
        !self.stop.load(Ordering::Acquire)
            && !self.locked.load(Ordering::Acquire)
            && LifecycleMailbox::generation(self.mailbox.read())
                == self.published_generation.load(Ordering::Acquire)
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
    let mut seen_lifecycle = LifecycleMailbox::generation(shared.mailbox.read());
    Box::new(move |event| {
        let lifecycle = LifecycleMailbox::generation(shared.mailbox.read());
        if lifecycle != seen_lifecycle {
            core = CaptureCore::new();
            seen_generation = u32::MAX;
            stored = Held::default();
            shared.reset_input();
            seen_lifecycle = lifecycle;
        }
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
        let allowed = shared.capture_allowed();
        core.set_locked(!allowed);
        if !allowed {
            core.end_capture();
        }
        if shared.begin.swap(false, Ordering::AcqRel) && allowed {
            core.begin();
        }
        if shared.ready.swap(false, Ordering::AcqRel) && allowed {
            let point = unpack(shared.park_point.load(Ordering::Acquire));
            core.park_ready(point, shared.park_dpi.load(Ordering::Relaxed));
        }
        let mut overflowed = false;
        let verdict = core.on_event(event, &mut |out| {
            if queue.try_send(Msg::Out(out, mono_us(), lifecycle)).is_err() {
                overflowed = true;
            }
        });
        if overflowed {
            // The queue is full: end the capture here, so the local desktop answers again, and let
            // the worker tell the module.
            shared.overflow.store(true, Ordering::Release);
            shared.mailbox.mark(0, None);
            shared.locked.store(true, Ordering::Release);
            shared.end.store(true, Ordering::Release);
            shared.begin.store(false, Ordering::Release);
            shared.ready.store(false, Ordering::Release);
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

fn record_notice(flag: &Shared, notice: Notice) {
    let (dirty, session) = match notice {
        Notice::SessionLocked => (LifecycleMailbox::SESSION, Some(true)),
        Notice::SessionUnlocked => (LifecycleMailbox::SESSION, Some(false)),
        Notice::DesktopSwitched => (LifecycleMailbox::DESKTOP, None),
        Notice::DisplayChanged => (LifecycleMailbox::DISPLAYS, None),
        Notice::HooksLost => (LifecycleMailbox::HOOKS, None),
    };
    flag.locked.store(true, Ordering::Release);
    flag.end.store(true, Ordering::Release);
    flag.begin.store(false, Ordering::Release);
    flag.ready.store(false, Ordering::Release);
    flag.mailbox.mark(dirty, session);
}

fn start_hooks(shared: &Arc<Shared>, queue: &SyncSender<Msg>) -> Result<Hooks, WinError> {
    #[cfg(test)]
    shared.hook_starts.fetch_add(1, Ordering::SeqCst);
    // Every instance, the first or a recreated one, starts from nothing (Sentinel S7-05).
    shared.reset_input();
    let flag = Arc::clone(shared);
    Hooks::start(
        hook_closure(Arc::clone(shared), queue.clone()),
        Box::new(move |notice| {
            record_notice(&flag, notice);
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

#[cfg(test)]
type TestLifecycleQuery =
    Box<dyn FnMut(Option<bool>) -> (Option<bool>, bool, Result<Vec<Zone>, WinError>) + Send>;

#[cfg(test)]
#[derive(Default)]
struct WorkerTestHooks {
    query: Option<TestLifecycleQuery>,
    output_full: Option<SyncSender<()>>,
}

struct Worker {
    #[cfg(test)]
    test: WorkerTestHooks,
    shared: Arc<Shared>,
    queue: SyncSender<Msg>,
    inbox: Receiver<Msg>,
    out: mpsc::Sender<CaptureEvent>,
    session_locked: bool,
    secure_desktop: bool,
}

/// Obsoleto descarta apenas este evento/lote; somente Stopped encerra o worker.
#[derive(Debug, PartialEq, Eq)]
enum EmitOutcome {
    Sent,
    Obsolete,
    Stopped,
}

impl Worker {
    /// Sends to the module's channel. A full channel is waited for in small steps, so a stop is
    /// seen even when the module no longer reads.
    fn emit(&mut self, mut event: CaptureEvent, generation: u64) -> EmitOutcome {
        loop {
            if self.shared.stop.load(Ordering::Acquire) {
                return EmitOutcome::Stopped;
            }
            if LifecycleMailbox::generation(self.shared.mailbox.read()) != generation {
                // O laço principal reconcilia até obter um ACK atual. Não confundir
                // descartar este dado com parar: uma consulta pode perder o CAS.
                return EmitOutcome::Obsolete;
            }
            match self.out.try_send(event) {
                Ok(()) => return EmitOutcome::Sent,
                Err(mpsc::error::TrySendError::Closed(_)) => return EmitOutcome::Stopped,
                Err(mpsc::error::TrySendError::Full(back)) => {
                    #[cfg(test)]
                    if let Some(signal) = &self.test.output_full {
                        let _ = signal.try_send(());
                    }
                    if self.shared.stop.load(Ordering::Acquire) {
                        return EmitOutcome::Stopped;
                    }
                    event = back;
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }

    fn publish_terminal(&self, error: &'static str) {
        self.shared.locked.store(true, Ordering::Release);
        self.shared.lifecycle.send_modify(|s| {
            s.valid = false;
            s.terminal_error = Some(error);
            s.revision = s.revision.saturating_add(1);
        });
    }

    fn reconcile_lifecycle(&mut self) -> bool {
        #[cfg(test)]
        if let Some(mut query) = self.test.query.take() {
            let result = self.reconcile_with(query.as_mut());
            self.test.query = Some(query);
            return result;
        }
        self.reconcile_with(|known_session| {
            let session = known_session.or_else(|| clipsync_win::session_locked().ok());
            (
                session,
                clipsync_win::input_desktop_is_default(),
                clipsync_win::monitors().map(|m| winmath::zones(&m)),
            )
        })
    }

    fn reconcile_with(
        &mut self,
        query: impl FnOnce(Option<bool>) -> (Option<bool>, bool, Result<Vec<Zone>, WinError>),
    ) -> bool {
        let word = self.shared.mailbox.read();
        let generation = LifecycleMailbox::generation(word);
        if LifecycleMailbox::exhausted(word) {
            self.publish_terminal("geração de captura esgotada");
            return false;
        }
        let current = self.shared.lifecycle.borrow().clone();
        if current.terminal_error.is_some() {
            return false;
        }
        if current.valid && current.invalidation_generation == generation {
            return true;
        }
        self.shared.locked.store(true, Ordering::Release);
        if generation != current.invalidation_generation {
            while self.inbox.try_recv().is_ok() {}
            self.shared.reset_input();
            let _ = end_locally(&self.shared, None);
        }
        if word & LifecycleMailbox::HOOKS != 0 && !self.recreate_hooks() {
            return false;
        }
        if self.shared.stop.load(Ordering::Acquire) {
            return false;
        }
        let known_session = if word & LifecycleMailbox::SESSION != 0 {
            LifecycleMailbox::session(word)
        } else {
            None
        };
        let (session, desktop, zones) = query(known_session);
        // Consulta velha nunca confirma uma versão que recebeu outro callback.
        if !self.shared.mailbox.acknowledge(word) {
            return true;
        }
        let Some(revision) = current.revision.checked_add(1) else {
            self.publish_terminal("revisão de captura esgotada");
            return false;
        };
        self.session_locked = session.unwrap_or(true);
        self.secure_desktop = !desktop;
        let snapshot = CaptureLifecycle {
            revision,
            invalidation_generation: generation,
            valid: session.is_some() && zones.is_ok(),
            session_locked: session,
            desktop_available: Some(desktop),
            zones: zones.unwrap_or_else(|_| current.zones.clone()),
            terminal_error: None,
        };
        let allowed = snapshot.allows_capture();
        if snapshot.valid {
            let specs = lock(&self.shared.specs).clone();
            self.shared.install(&specs, &snapshot.zones);
        }
        self.shared.lifecycle.send_replace(snapshot);
        // Não basta locked: um notice pode fechar o gate entre ACK e esta publicação.
        // O hook só aceita a geração publicada que ainda coincide com a mailbox.
        self.shared
            .published_generation
            .store(generation, Ordering::Release);
        self.shared.locked.store(!allowed, Ordering::Release);
        if LifecycleMailbox::generation(self.shared.mailbox.read()) != generation {
            self.shared.locked.store(true, Ordering::Release);
        }
        true
    }

    fn run(mut self) {
        loop {
            if self.shared.stop.load(Ordering::Acquire) {
                return;
            }
            if !self.reconcile_lifecycle() {
                return;
            }
            let generation = self.shared.lifecycle.borrow().invalidation_generation;
            let msg = match self.inbox.recv_timeout(STOP_POLL) {
                Ok(msg) => msg,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            if self.shared.overflow.swap(false, Ordering::AcqRel)
                || LifecycleMailbox::generation(self.shared.mailbox.read()) != generation
            {
                continue;
            }
            let Msg::Out(out, t, produced_generation) = msg;
            if produced_generation != generation {
                continue;
            }
            if self.on_out(out, t, produced_generation) == EmitOutcome::Stopped {
                return;
            }
        }
    }

    fn on_out(&mut self, out: Out, t_mono_us: u64, generation: u64) -> EmitOutcome {
        if LifecycleMailbox::generation(self.shared.mailbox.read()) != generation {
            return EmitOutcome::Obsolete;
        }
        match out {
            Out::Activate { barrier, pos, held } => {
                match park_and_carry(&self.shared, barrier, held) {
                    Ok(presses) => {
                        *lock(&self.shared.last) = Some((barrier, pos));
                        let activated = self.emit(
                            CaptureEvent::Activated {
                                barrier,
                                pos,
                                t_mono_us,
                            },
                            generation,
                        );
                        if activated != EmitOutcome::Sent {
                            return activated;
                        }
                        for ev in presses {
                            let sent = self.emit(CaptureEvent::Input { t_mono_us, ev }, generation);
                            if sent != EmitOutcome::Sent {
                                return sent;
                            }
                        }
                        EmitOutcome::Sent
                    }
                    Err(e) => {
                        let _ = end_locally(&self.shared, None);
                        self.emit(CaptureEvent::Error(e.to_string()), generation)
                    }
                }
            }
            Out::Input(ev) => self.emit(CaptureEvent::Input { t_mono_us, ev }, generation),
            Out::Hotkey(hotkey) => self.emit(CaptureEvent::Hotkey(hotkey), generation),
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
                    return self.emit(CaptureEvent::Error(format!("send: {e}")), generation);
                }
                EmitOutcome::Sent
            }
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
        let now_ms =
            u64::try_from(self.shared.restart_clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        let restart = lock(&self.shared.restart_policy).on_lost(now_ms);
        match restart {
            Restart::GiveUp => {
                self.shared.locked.store(true, Ordering::Relaxed);
                self.publish_terminal(super::HOOKS_GAVE_UP);
                false
            }
            Restart::Recreate => match start_hooks(&self.shared, &self.queue) {
                Ok(hooks) => {
                    if self.shared.stop.load(Ordering::Acquire) {
                        drop(hooks);
                        return false;
                    }
                    *lock(&self.shared.hooks) = Some(hooks);
                    true
                }
                Err(_) => {
                    self.publish_terminal("falha ao reinstalar hooks");
                    false
                }
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
    fn known_monitor_observation(&self) -> Result<crate::inventory::ObservedMonitors, InputError> {
        let monitors = clipsync_win::monitors().map_err(|e| io_err("monitors", &e))?;
        crate::inventory::windows_observation(&monitors)
    }
    async fn prepare_monitor_observation(
        &mut self,
    ) -> Result<crate::inventory::ObservedMonitors, InputError> {
        self.known_monitor_observation()
    }

    fn lifecycle_receiver(&self) -> Option<tokio::sync::watch::Receiver<CaptureLifecycle>> {
        Some(self.shared.lifecycle.subscribe())
    }
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
        if let Some(error) = self.shared.lifecycle.borrow().terminal_error {
            return Err(InputError::Other(error.to_owned()));
        }
        self.shared.stop.store(false, Ordering::Release);
        let (out_tx, out_rx) = mpsc::channel(CHANNEL);
        let (queue_tx, queue_rx) = sync_channel(QUEUE);
        self.shared.install(barriers, &zones_now());
        self.shared.end.store(false, Ordering::Relaxed);
        self.shared.overflow.store(false, Ordering::Relaxed);
        // Register session/desktop notices before querying, so changes during the query are
        // queued. The hook's gate stays closed until the explicit snapshot is published.
        self.shared.locked.store(true, Ordering::Relaxed);
        let hooks = start_hooks(&self.shared, &queue_tx).map_err(|e| io_err("hooks", &e))?;
        *lock(&self.shared.hooks) = Some(hooks);
        // O worker faz o snapshot após registrar os notices. O watch permanece fechado
        // até reconciliar a mesma versão da mailbox antes/depois das consultas.
        let secure_desktop = true;
        let session_locked = true;
        let worker = Worker {
            #[cfg(test)]
            test: WorkerTestHooks::default(),
            shared: Arc::clone(&self.shared),
            queue: queue_tx.clone(),
            inbox: queue_rx,
            out: out_tx.clone(),
            session_locked,
            secure_desktop,
        };
        let mut initial = self.shared.lifecycle.subscribe();
        self.worker = Some(std::thread::spawn(move || worker.run()));
        let _ = initial.changed().await;
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
        self.shared.lifecycle.send_modify(|s| {
            s.valid = false;
            s.revision = s.revision.saturating_add(1);
            if s.revision == u64::MAX {
                s.terminal_error = Some("revisão de captura esgotada");
            }
        });
        self.unwatch_resume().await;
        Ok(())
    }
}

// WTS state and input desktop are independent. A known Default desktop never substitutes
// for a missing WTS snapshot. The worker retains both states and updates them independently.
#[cfg(test)]
async fn initial_lock_snapshot(
    out: &mpsc::Sender<CaptureEvent>,
    desktop_default: bool,
    session: Option<bool>,
) -> bool {
    let session_locked = session.unwrap_or(true);
    let locked = session_locked || !desktop_default;
    let _ = out.send(CaptureEvent::SessionLocked(locked)).await;
    if session.is_none() || !desktop_default {
        let _ = out
            .send(CaptureEvent::Error(
                crate::msg::SCREEN_STATE_UNAVAILABLE.to_owned(),
            ))
            .await;
    }
    session_locked
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
        shared.published_generation.store(0, Ordering::Release);
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
        let Ok(Msg::Out(Out::Activate { held, .. }, _, _)) = inbox.try_recv() else {
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
            .try_send(Msg::Out(Out::Hotkey(Hotkey::ToggleLock), 0, 0))
            .unwrap();
        let worker = Worker {
            #[cfg(test)]
            test: WorkerTestHooks::default(),
            shared: Arc::clone(&shared),
            queue: queue.clone(),
            inbox,
            out,
            session_locked: false,
            secure_desktop: false,
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
            #[cfg(test)]
            test: WorkerTestHooks::default(),
            shared: Arc::clone(&shared),
            queue,
            inbox,
            out,
            session_locked: false,
            secure_desktop: false,
        };
        assert!(!worker.recreate_hooks());
        assert!(lock(&shared.hooks).is_none(), "no hook may be installed");
    }
    #[tokio::test]
    async fn initial_snapshot_requires_both_wts_unlock_and_default_desktop() {
        for desktop_default in [false, true] {
            for session in [None, Some(true), Some(false)] {
                let (out, mut rx) = mpsc::channel(4);
                let session_locked = initial_lock_snapshot(&out, desktop_default, session).await;
                assert_eq!(session_locked, session.unwrap_or(true));
                assert!(
                    matches!(rx.recv().await, Some(CaptureEvent::SessionLocked(locked)) if locked == (session.unwrap_or(true) || !desktop_default))
                );
                if session.is_none() || !desktop_default {
                    assert!(matches!(rx.recv().await, Some(CaptureEvent::Error(_))));
                }
            }
        }
    }
    #[test]
    fn notice_between_ack_and_publication_cannot_reopen_hook_or_keep_held() {
        let shared = shared_with_a_barrier();
        let (queue, inbox) = sync_channel(QUEUE);
        let mut hook = hook_closure(shared.clone(), queue);
        hook(&key(0x38, false, false));
        assert!(!shared.held().is_empty());
        // O worker já confirmou uma consulta velha. O callback chega antes de locked=false.
        let old = shared.mailbox.read();
        assert!(shared.mailbox.acknowledge(old));
        shared.mailbox.mark(LifecycleMailbox::SESSION, Some(true));
        shared.mailbox.mark(LifecycleMailbox::SESSION, Some(false));
        shared.end.store(true, Ordering::Release);
        shared.locked.store(false, Ordering::Release); // publicação velha termina depois
        assert_eq!(hook(&RawEvent::Move { x: 1000, y: 500 }), Verdict::Pass);
        assert_eq!(hook(&RawEvent::Move { x: 1919, y: 540 }), Verdict::Pass);
        assert!(inbox.try_recv().is_err(), "a geração velha voltou a ativar");
        assert!(
            shared.held().is_empty(),
            "held da geração invalidada reapareceu"
        );
    }

    fn test_worker(shared: Arc<Shared>) -> (Worker, mpsc::Receiver<CaptureEvent>) {
        let (queue, inbox) = sync_channel(QUEUE);
        let (out, rx) = mpsc::channel(CHANNEL);
        (
            Worker {
                test: WorkerTestHooks::default(),
                shared,
                queue,
                inbox,
                out,
                session_locked: true,
                secure_desktop: true,
            },
            rx,
        )
    }

    #[test]
    fn every_notice_survives_full_data_queues_and_closes_flags() {
        let shared = shared_with_a_barrier();
        let (mut worker, _rx) = test_worker(shared.clone());
        for _ in 0..QUEUE {
            worker
                .queue
                .try_send(Msg::Out(Out::Hotkey(Hotkey::ToggleLock), 0, 0))
                .unwrap();
        }
        for _ in 0..CHANNEL {
            worker.out.try_send(CaptureEvent::Deactivated).unwrap();
        }
        for notice in [
            Notice::SessionLocked,
            Notice::SessionUnlocked,
            Notice::DesktopSwitched,
            Notice::DisplayChanged,
            Notice::HooksLost,
        ] {
            shared.begin.store(true, Ordering::Release);
            shared.ready.store(true, Ordering::Release);
            record_notice(&shared, notice);
            assert!(!shared.capture_allowed());
            assert!(!shared.begin.load(Ordering::Acquire));
            assert!(!shared.ready.load(Ordering::Acquire));
        }
        assert_eq!(worker.out.capacity(), 0);
        assert_eq!(shared.mailbox.read() & 60, 60);
        assert_eq!(
            LifecycleMailbox::session(shared.mailbox.read()),
            Some(false)
        );
        assert_eq!(LifecycleMailbox::generation(shared.mailbox.read()), 5);
        // Stop+HooksLost não pode instalar hooks, mesmo com ambas as filas cheias.
        shared.stop.store(true, Ordering::Release);
        assert!(!worker.reconcile_lifecycle());
        assert!(lock(&shared.hooks).is_none());
        assert_eq!(
            shared.mailbox.read() & LifecycleMailbox::HOOKS,
            LifecycleMailbox::HOOKS
        );
    }

    #[test]
    fn overflow_preserves_desktop_display_and_hook_facts() {
        let shared = shared_with_a_barrier();
        let (queue, inbox) = sync_channel(QUEUE);
        for _ in 0..QUEUE {
            queue
                .try_send(Msg::Out(Out::Hotkey(Hotkey::ToggleLock), 0, 0))
                .unwrap();
        }
        let mut hook = hook_closure(shared.clone(), queue);
        hook(&RawEvent::Move { x: 1000, y: 500 });
        hook(&RawEvent::Move { x: 1919, y: 540 });
        assert!(shared.overflow.load(Ordering::Acquire));
        record_notice(&shared, Notice::DesktopSwitched);
        record_notice(&shared, Notice::DisplayChanged);
        record_notice(&shared, Notice::HooksLost);
        while inbox.try_recv().is_ok() {}
        assert_eq!(shared.mailbox.read() & 28, 28);
        assert_eq!(LifecycleMailbox::generation(shared.mailbox.read()), 4);
        assert!(!shared.capture_allowed());
    }

    #[test]
    fn initial_queries_cannot_overwrite_a_newer_notice() {
        let shared = Arc::new(Shared::new());
        let (mut worker, _rx) = test_worker(shared.clone());
        assert!(worker.reconcile_with(|_| {
            record_notice(&shared, Notice::SessionLocked);
            record_notice(&shared, Notice::DisplayChanged);
            (Some(false), true, Ok(vec![zone()]))
        }));
        assert!(!shared.lifecycle.borrow().valid);
        assert!(!shared.capture_allowed());
        assert!(worker.reconcile_with(|session| {
            assert_eq!(session, Some(true));
            (session, true, Ok(vec![zone()]))
        }));
        assert!(shared.lifecycle.borrow().valid);
        assert_eq!(shared.lifecycle.borrow().session_locked, Some(true));
        assert_eq!(shared.lifecycle.borrow().invalidation_generation, 2);
        assert!(!shared.capture_allowed());
    }

    #[test]
    fn unknown_snapshot_is_retryable_but_terminal_failure_is_sticky() {
        let shared = Arc::new(Shared::new());
        let (mut worker, _rx) = test_worker(shared.clone());
        assert!(worker.reconcile_with(|_| (None, true, Ok(vec![zone()]))));
        assert!(!shared.lifecycle.borrow().valid);
        assert!(shared.lifecycle.borrow().terminal_error.is_none());
        assert_eq!(shared.lifecycle.borrow().desktop_available, Some(true));
        assert!(!shared.capture_allowed());
        assert!(worker.reconcile_with(|_| (Some(false), true, Ok(vec![zone()]))));
        assert!(shared.capture_allowed());
        worker.publish_terminal("teste terminal");
        assert!(!worker.reconcile_with(|_| panic!("terminal não consulta nem reabre")));
        assert!(!shared.capture_allowed());
    }

    #[tokio::test]
    async fn worker_publishes_lifecycle_while_output_is_really_full() {
        let shared = Arc::new(Shared::new());
        let (mut worker, _rx) = test_worker(shared.clone());
        assert!(worker.reconcile_with(|_| (Some(false), true, Ok(vec![zone()]))));
        for _ in 0..CHANNEL {
            worker.out.try_send(CaptureEvent::Deactivated).unwrap();
        }
        assert_eq!(worker.out.capacity(), 0);
        let (full, waiting) = sync_channel(1);
        worker.test.output_full = Some(full);
        worker.test.query = Some(Box::new(|session| (session, true, Ok(vec![zone()]))));
        let mut snapshots = shared.lifecycle.subscribe();
        worker
            .queue
            .try_send(Msg::Out(Out::Hotkey(Hotkey::ToggleLock), 0, 0))
            .unwrap();
        let thread = std::thread::spawn(move || worker.run());
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(!thread.is_finished());
        record_notice(&shared, Notice::SessionLocked);
        record_notice(&shared, Notice::SessionUnlocked);
        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            while snapshots.borrow_and_update().invalidation_generation < 2 {
                snapshots.changed().await.unwrap();
            }
        })
        .await;
        shared.stop.store(true, Ordering::Release);
        thread.join().unwrap();
        assert!(observed.is_ok());
        assert_eq!(shared.lifecycle.borrow().invalidation_generation, 2);
        assert_eq!(shared.lifecycle.borrow().session_locked, Some(false));
    }

    #[test]
    fn stop_start_publications_preserve_generation_and_pending_facts() {
        let shared = Arc::new(Shared::new());
        let (mut worker, _rx) = test_worker(shared.clone());
        assert!(worker.reconcile_with(|_| (Some(false), true, Ok(vec![zone()]))));
        record_notice(&shared, Notice::SessionLocked);
        let word = shared.mailbox.read();
        let revision = shared.lifecycle.borrow().revision;
        shared.stop.store(true, Ordering::Release);
        shared.lifecycle.send_modify(|s| {
            s.valid = false;
            s.revision += 1;
        });
        shared.reset_input();
        assert_eq!(shared.mailbox.read(), word);
        shared.stop.store(false, Ordering::Release);
        assert!(worker.reconcile_with(|session| (session, true, Ok(vec![zone()]))));
        assert_eq!(shared.lifecycle.borrow().invalidation_generation, 1);
        assert!(shared.lifecycle.borrow().revision > revision);
        assert_eq!(shared.lifecycle.borrow().session_locked, Some(true));
        assert!(!shared.capture_allowed());
    }

    #[tokio::test]
    async fn full_output_and_notice_during_query_keep_real_worker_reconciling() {
        use crate::adapter::lifecycle::LifecycleCursor;
        let cap = WinCapture::new();
        let shared = cap.shared.clone();
        let (mut worker, rx) = test_worker(shared.clone());
        assert!(worker.reconcile_with(|_| (Some(false), true, Ok(vec![zone()]))));
        let mut cursor = LifecycleCursor::new(&cap);
        let mut snapshots = shared.lifecycle.subscribe();
        // Retenha também o Sender como WinCapture faz, para não confundir EOF com invalidação.
        let out = worker.out.clone();
        for _ in 0..CHANNEL {
            out.try_send(CaptureEvent::Deactivated).unwrap();
        }
        assert_eq!(out.capacity(), 0);
        let (full, waiting) = sync_channel(1);
        worker.test.output_full = Some(full);
        let queried = shared.clone();
        worker.test.query = Some(Box::new(move |session| {
            if session == Some(true) {
                // Segunda mudança precisamente entre leitura da mailbox e seu ACK.
                record_notice(&queried, Notice::SessionUnlocked);
                assert_eq!(queried.lifecycle.borrow().invalidation_generation, 0);
            }
            (session, true, Ok(vec![zone()]))
        }));
        worker
            .queue
            .try_send(Msg::Out(
                Out::Input(InputEvent::Motion { dx: 1.0, dy: 0.0 }),
                0,
                0,
            ))
            .unwrap();
        let thread = std::thread::spawn(move || worker.run());
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        record_notice(&shared, Notice::SessionLocked);
        let observed = tokio::time::timeout(Duration::from_secs(2), snapshots.changed()).await;
        let snapshot = snapshots.borrow().clone();
        cursor.refresh();
        let alive = !thread.is_finished();
        // Sempre libere e faça join antes de avaliar o resultado, inclusive no caso de falha.
        shared.stop.store(true, Ordering::Release);
        thread.join().unwrap();
        assert!(
            matches!(observed, Ok(Ok(()))),
            "worker morreu sem publicar a geração nova"
        );
        assert!(
            alive,
            "descartar um dado obsoleto não deve encerrar o worker"
        );
        assert_eq!(snapshot.invalidation_generation, 2);
        assert_eq!(snapshot.session_locked, Some(false));
        assert!(
            cursor.pending_invalidation,
            "consumidor não recebeu a invalidação coalescida"
        );
        assert_eq!(
            out.capacity(),
            0,
            "não houve espaço na saída que explicasse a publicação"
        );
        drop(rx); // geração antiga descartada; watch foi observado independentemente dela
        assert!(out.is_closed());
    }

    #[tokio::test]
    async fn terminal_stop_start_returns_error_without_installing_hooks() {
        let mut cap = WinCapture::new();
        let (worker, _rx) = test_worker(cap.shared.clone());
        worker.publish_terminal("teste terminal persistente");
        cap.stop().await.unwrap();
        let hooks_before = cap.shared.hook_starts.load(Ordering::SeqCst);
        let generation = cap.shared.mailbox.read();
        let outcome = tokio::time::timeout(Duration::from_secs(2), cap.start(&[])).await;
        assert!(
            matches!(outcome, Ok(Err(InputError::Other(ref error))) if error == "teste terminal persistente")
        );
        assert_eq!(cap.shared.hook_starts.load(Ordering::SeqCst), hooks_before);
        assert!(lock(&cap.shared.hooks).is_none());
        assert!(cap.worker.is_none());
        assert!(cap.queue.is_none());
        assert!(cap.out.is_none());
        assert_eq!(cap.shared.mailbox.read(), generation);
        assert_eq!(
            cap.shared.lifecycle.borrow().terminal_error,
            Some("teste terminal persistente")
        );
        assert!(!cap.shared.capture_allowed());
        tokio::time::timeout(Duration::from_secs(2), cap.stop())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cap.shared.hook_starts.load(Ordering::SeqCst), hooks_before);
    }
}
