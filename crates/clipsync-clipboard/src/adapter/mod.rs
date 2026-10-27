//! Clipboard adapter contract (contrato §7). The module never talks to a platform API directly.
use std::fmt;
use std::time::Duration;

mod fake;
mod x11;

pub use fake::{FakeClipboard, FakeClipboardHandle};
pub use x11::X11Clipboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchMode {
    Events,
    Poll { interval_ms: u32 },
}

/// Change event. It does NOT say who changed the clipboard: our own write raises one too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeEvent {
    pub seq: u64,
    pub t_mono_us: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ClipSnapshot {
    pub mime: String,
    /// Never empty.
    pub text: String,
}

impl fmt::Debug for ClipSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ClipSnapshot {{ mime: {}, text: <{} bytes> }}",
            self.mime,
            self.text.len()
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClipError {
    #[error("no display")]
    NoDisplay,
    #[error("unavailable: {0}")]
    Unavailable(String),
    #[error("busy")]
    Busy,
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Other(String),
}

pub trait ClipboardAdapter: Send + 'static {
    /// "fake" | "x11-xfixes"
    fn name(&self) -> &'static str;
    fn watch_mode(&self) -> WatchMode;
    /// Blocks up to `timeout`. `Some(event)` for every SetSelectionOwner of CLIPBOARD, including the
    /// echo of our own write (~2 ms) and the compositor taking ownership when the source app dies.
    fn wait_change(&mut self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError>;
    /// `Ok(None)` = empty: no targets, only `application/x-zerosize`, or no UTF8_STRING. Empty never syncs.
    fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError>;
    /// Writes text to CLIPBOARD. Raises a `ChangeEvent`; the module drops it through the write ledger.
    fn write_text(&mut self, text: &str) -> Result<(), ClipError>;
}
