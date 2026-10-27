use super::{ChangeEvent, ClipError, ClipSnapshot, ClipboardAdapter, WatchMode};
use clipsync_proto::{MIME_TEXT_UTF8, mono_us};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Default)]
struct State {
    contents: Option<String>,
    seq: u64,
    events: VecDeque<ChangeEvent>,
    writes: usize,
    fail_writes: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    cv: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn emit(&self, st: &mut State) {
        st.seq += 1;
        st.events.push_back(ChangeEvent {
            seq: st.seq,
            t_mono_us: mono_us(),
        });
        self.cv.notify_all();
    }
}

/// In-memory clipboard. Like X11, `write_text` also emits a `ChangeEvent`.
pub struct FakeClipboard {
    shared: Arc<Shared>,
}

#[derive(Clone)]
pub struct FakeClipboardHandle {
    shared: Arc<Shared>,
}

impl FakeClipboard {
    pub fn new() -> (Self, FakeClipboardHandle) {
        let shared = Arc::new(Shared::default());
        (
            Self {
                shared: shared.clone(),
            },
            FakeClipboardHandle { shared },
        )
    }
}

impl FakeClipboardHandle {
    /// The user copies `text` in some app. An empty string leaves an empty clipboard.
    pub fn user_copy(&self, text: &str) {
        let mut st = self.shared.lock();
        st.contents = if text.is_empty() {
            None
        } else {
            Some(text.to_owned())
        };
        self.shared.emit(&mut st);
    }
    /// The compositor takes ownership (source app died): same content, new event.
    pub fn owner_takeover(&self) {
        let mut st = self.shared.lock();
        self.shared.emit(&mut st);
    }
    pub fn clear(&self) {
        let mut st = self.shared.lock();
        st.contents = None;
        self.shared.emit(&mut st);
    }
    pub fn contents(&self) -> Option<String> {
        self.shared.lock().contents.clone()
    }
    /// Number of successful `write_text` calls.
    pub fn writes(&self) -> usize {
        self.shared.lock().writes
    }
    /// Makes the next writes fail with `ClipError::Busy` (apply-failure tests).
    pub fn fail_writes(&self, fail: bool) {
        self.shared.lock().fail_writes = fail;
    }
}

impl ClipboardAdapter for FakeClipboard {
    fn name(&self) -> &'static str {
        "fake"
    }
    fn watch_mode(&self) -> WatchMode {
        WatchMode::Events
    }
    fn wait_change(&mut self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError> {
        let deadline = Instant::now() + timeout;
        let mut st = self.shared.lock();
        loop {
            if let Some(ev) = st.events.pop_front() {
                return Ok(Some(ev));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(None);
            }
            st = self
                .shared
                .cv
                .wait_timeout(st, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }
    fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError> {
        Ok(self
            .shared
            .lock()
            .contents
            .clone()
            .map(|text| ClipSnapshot {
                mime: MIME_TEXT_UTF8.into(),
                text,
            }))
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipError> {
        let mut st = self.shared.lock();
        if st.fail_writes {
            return Err(ClipError::Busy);
        }
        st.contents = if text.is_empty() {
            None
        } else {
            Some(text.to_owned())
        };
        st.writes += 1;
        self.shared.emit(&mut st);
        Ok(())
    }
}
