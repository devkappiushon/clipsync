//! Clipboard self-test (contrato §12.6, clipboard item 5): writes synthetic text, reads it back,
//! checks the change event and gives the previous text back. Never reads the content into a log or
//! into the outcome: the detail is fixed Portuguese text with counts only.
//!
//! The test opens its own adapter. With an agent running in the same profile the caller must pause
//! it first (§12.5 `onboarding_self_test` does), or the agent would offer the synthetic text to peers.
use crate::adapter::{ClipError, ClipboardAdapter};
use crate::msg;
use clipsync_proto::{ModuleError, mono_us};
use std::time::{Duration, Instant};

/// Longest wait for the change event of our own write. The Windows poll runs every 250 ms.
const ECHO_WAIT: Duration = Duration::from_millis(1500);
/// Stale events drained before the write; a bound so a noisy clipboard cannot hold the test.
const DRAIN_LIMIT: usize = 64;

/// Result of the self-test; the `String` is the Portuguese detail for the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfTestOutcome {
    /// Works and the adapter is validated on this platform (Linux, XFixes via XWayland).
    Validated(String),
    /// Works, but with a caveat (Windows polling, no change event, previous text not given back).
    Experimental(String),
    /// Does not work.
    Unavailable(String),
}

impl SelfTestOutcome {
    pub fn detail(&self) -> &str {
        match self {
            Self::Validated(d) | Self::Experimental(d) | Self::Unavailable(d) => d,
        }
    }
}

/// Self-test on the platform adapter (X11 bridge on Linux, `WinClipboard` on Windows). The adapter
/// is synchronous, so the whole test runs in `spawn_blocking`.
pub async fn self_test() -> SelfTestOutcome {
    tokio::task::spawn_blocking(|| match crate::native() {
        Ok(mut adapter) => self_test_with(adapter.as_mut()),
        Err(ModuleError::Adapter(detail)) => SelfTestOutcome::Unavailable(detail),
        Err(other) => SelfTestOutcome::Unavailable(other.to_string()),
    })
    .await
    .unwrap_or_else(|_| SelfTestOutcome::Unavailable(msg::SELF_TEST_PANIC.into()))
}

/// The test itself, on a given adapter (blocking). Only `x11-xfixes` is classified as validated.
pub fn self_test_with(adapter: &mut dyn ClipboardAdapter) -> SelfTestOutcome {
    use SelfTestOutcome::{Experimental, Unavailable, Validated};

    // Previous text. Non-text or too-large content (Unsupported) cannot be given back either way.
    let previous = match adapter.read_text() {
        Ok(snapshot) => snapshot.map(|s| s.text),
        Err(ClipError::Unsupported(_)) => None,
        Err(e) => return Unavailable(msg::self_test_read_failed(&e.to_string())),
    };
    let had_text = previous.is_some();

    for _ in 0..DRAIN_LIMIT {
        match adapter.wait_change(Duration::ZERO) {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(e) => return Unavailable(msg::self_test_read_failed(&e.to_string())),
        }
    }

    let synthetic = format!("clipsync-autoteste-ção-{}", mono_us());
    let started = Instant::now();
    if let Err(e) = adapter.write_text(&synthetic) {
        return Unavailable(msg::self_test_write_failed(&e.to_string()));
    }
    let event = match adapter.wait_change(ECHO_WAIT) {
        Ok(event) => event.is_some(),
        Err(e) => {
            give_back(adapter, previous);
            return Unavailable(msg::self_test_read_failed(&e.to_string()));
        }
    };
    let latency_ms = started.elapsed().as_millis();
    let read_back = adapter.read_text();

    let restored = give_back(adapter, previous);

    match read_back {
        Err(e) => return Unavailable(msg::self_test_read_failed(&e.to_string())),
        Ok(Some(s)) if s.text == synthetic => {}
        Ok(_) => return Unavailable(msg::SELF_TEST_MISMATCH.into()),
    }
    if !restored {
        return Experimental(msg::SELF_TEST_RESTORE_FAILED.into());
    }
    if !event {
        return Experimental(msg::self_test_no_event(latency_ms, had_text));
    }
    if adapter.name() == "x11-xfixes" {
        Validated(msg::self_test_validated(had_text, latency_ms))
    } else {
        Experimental(msg::self_test_experimental(
            adapter.name(),
            had_text,
            latency_ms,
        ))
    }
}

/// Puts the previous text back; with none, writes the empty text, which every adapter reads as an
/// empty clipboard. `false` = the write failed.
fn give_back(adapter: &mut dyn ClipboardAdapter, previous: Option<String>) -> bool {
    adapter
        .write_text(previous.as_deref().unwrap_or_default())
        .is_ok()
}
