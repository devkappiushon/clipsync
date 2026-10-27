//! Windows adapter: arboard for reads and writes, text change detection by polling every 250 ms
//! (the clipboard-format-listener route needs unsafe Win32 calls, forbidden by the workspace lints).
//! Text only in this version. A fresh `arboard::Clipboard` per operation: Windows opens the native
//! clipboard only for the transfer, so nothing is held between calls.
use super::poll::PollWatcher;
use super::{ChangeEvent, ClipError, ClipSnapshot, ClipboardAdapter, WatchMode};
use clipsync_proto::MIME_TEXT_UTF8;
use std::time::Duration;

pub const POLL_INTERVAL_MS: u32 = 250;

pub struct WinClipboard {
    watcher: PollWatcher,
}

fn open() -> Result<arboard::Clipboard, ClipError> {
    arboard::Clipboard::new().map_err(map_err)
}

fn map_err(e: arboard::Error) -> ClipError {
    match e {
        arboard::Error::ClipboardOccupied => ClipError::Busy,
        other => ClipError::Other(other.to_string()),
    }
}

/// `Ok(None)` = empty or no text on the clipboard. Empty text counts as empty.
fn read() -> Result<Option<String>, ClipError> {
    match open()?.get_text() {
        Ok(t) if t.is_empty() => Ok(None),
        Ok(t) => Ok(Some(t)),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => Err(map_err(e)),
    }
}

impl WinClipboard {
    pub fn new() -> Result<Self, ClipError> {
        open()?; // fail now, not on the first poll
        let watcher = PollWatcher::spawn(Duration::from_millis(u64::from(POLL_INTERVAL_MS)), read);
        Ok(Self { watcher })
    }
}

impl ClipboardAdapter for WinClipboard {
    fn name(&self) -> &'static str {
        "win-poll"
    }
    fn watch_mode(&self) -> WatchMode {
        WatchMode::Poll {
            interval_ms: POLL_INTERVAL_MS,
        }
    }
    fn wait_change(&mut self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError> {
        self.watcher.wait(timeout)
    }
    fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError> {
        Ok(read()?.map(|text| ClipSnapshot {
            mime: MIME_TEXT_UTF8.into(),
            text,
        }))
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipError> {
        open()?.set_text(text).map_err(map_err)
    }
}
