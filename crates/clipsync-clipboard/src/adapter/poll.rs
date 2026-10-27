//! Change detector for platforms without clipboard events: polls the text on its own thread and
//! raises one `ChangeEvent` per change of content hash. Platform-neutral so it is tested on Linux.
use super::{ChangeEvent, ClipError};
use crate::ledger::{Hash, hash_text};
use clipsync_proto::mono_us;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// Longest the poll thread sleeps between checks of the stop flag.
const STOP_SLICE: Duration = Duration::from_millis(50);

pub struct PollWatcher {
    rx: Receiver<ChangeEvent>,
    stop: Arc<AtomicBool>,
}

impl PollWatcher {
    /// `source` returns the current text (`Ok(None)` = empty or not text). An `Err` (clipboard held
    /// by another app) skips that tick, so a transient failure never looks like a change.
    /// The content at spawn time is the baseline: it raises no event.
    pub fn spawn<F>(interval: Duration, mut source: F) -> Self
    where
        F: FnMut() -> Result<Option<String>, ClipError> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<()>(0);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        std::thread::spawn(move || {
            let hash_of = |t: Option<String>| -> Option<Hash> { t.map(|t| hash_text(&t)) };
            let mut last = source().ok().and_then(hash_of);
            let _ = ready_tx.send(());
            let mut seq = 0u64;
            while !flag.load(Ordering::Relaxed) {
                if !sleep_unless_stopped(&flag, interval) {
                    break;
                }
                let Ok(now) = source() else { continue };
                let now = hash_of(now);
                if now != last {
                    last = now;
                    seq += 1;
                    let ev = ChangeEvent {
                        seq,
                        t_mono_us: mono_us(),
                    };
                    if tx.send(ev).is_err() {
                        break;
                    }
                }
            }
        });
        let _ = ready_rx.recv(); // baseline taken before the caller can write anything
        Self { rx, stop }
    }

    pub fn wait(&self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError> {
        match self.rx.recv_timeout(timeout) {
            Ok(ev) => Ok(Some(ev)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(ClipError::Unavailable(
                "observador do clipboard parou".into(),
            )),
        }
    }
}

impl Drop for PollWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Sleeps `total` in slices; `false` when the stop flag was raised meanwhile.
fn sleep_unless_stopped(flag: &AtomicBool, total: Duration) -> bool {
    let mut left = total;
    while !left.is_zero() {
        if flag.load(Ordering::Relaxed) {
            return false;
        }
        let step = left.min(STOP_SLICE);
        std::thread::sleep(step);
        left -= step;
    }
    !flag.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const TICK: Duration = Duration::from_millis(10);
    const WAIT: Duration = Duration::from_secs(2);

    type Shared = Arc<Mutex<Result<Option<String>, ()>>>;

    fn watcher(initial: Option<&str>) -> (PollWatcher, Shared) {
        let cell: Shared = Arc::new(Mutex::new(Ok(initial.map(str::to_owned))));
        let c = Arc::clone(&cell);
        let w = PollWatcher::spawn(TICK, move || match &*c.lock().unwrap() {
            Ok(t) => Ok(t.clone()),
            Err(()) => Err(ClipError::Busy),
        });
        (w, cell)
    }

    fn set(cell: &Shared, v: Result<Option<&str>, ()>) {
        *cell.lock().unwrap() = v.map(|o| o.map(str::to_owned));
    }

    #[test]
    fn poll_baseline_and_same_content_raise_nothing() {
        let (w, cell) = watcher(Some("a"));
        assert!(w.wait(Duration::from_millis(100)).unwrap().is_none());
        set(&cell, Ok(Some("a")));
        assert!(w.wait(Duration::from_millis(100)).unwrap().is_none());
    }

    #[test]
    fn poll_change_raises_one_event_with_increasing_seq() {
        let (w, cell) = watcher(Some("a"));
        set(&cell, Ok(Some("b")));
        let e1 = w.wait(WAIT).unwrap().expect("event for b");
        assert!(w.wait(Duration::from_millis(100)).unwrap().is_none());
        set(&cell, Ok(Some("c")));
        let e2 = w.wait(WAIT).unwrap().expect("event for c");
        assert_eq!((e1.seq, e2.seq), (1, 2));
        assert!(e2.t_mono_us >= e1.t_mono_us);
    }

    #[test]
    fn poll_read_error_is_not_a_change() {
        let (w, cell) = watcher(Some("a"));
        set(&cell, Err(()));
        assert!(w.wait(Duration::from_millis(100)).unwrap().is_none());
        set(&cell, Ok(Some("a")));
        assert!(w.wait(Duration::from_millis(100)).unwrap().is_none());
    }

    #[test]
    fn poll_empty_to_text_raises_event() {
        let (w, cell) = watcher(None);
        set(&cell, Ok(Some("x")));
        assert!(w.wait(WAIT).unwrap().is_some());
    }

    #[test]
    fn poll_dropped_watcher_stops_the_thread() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c = Arc::clone(&calls);
        let w = PollWatcher::spawn(TICK, move || {
            c.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        });
        drop(w);
        std::thread::sleep(Duration::from_millis(150));
        let n = calls.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(n, calls.load(Ordering::Relaxed));
    }
}
