//! §12.6 clipboard item 5 (L7): the self-test writes synthetic text, checks the read, checks the
//! change event and gives the previous text back, without ever putting content in its outcome.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_clipboard::{
    ClipError, ClipSnapshot, ClipboardAdapter, FakeClipboard, FakeClipboardHandle, SelfTestOutcome,
    self_test_with,
};
use clipsync_proto::MIME_TEXT_UTF8;
use std::time::Duration;

type Change = Option<clipsync_clipboard::adapter::ChangeEvent>;

/// A fake clipboard with switches for the failure modes the test wants to see.
struct Tweaked {
    inner: FakeClipboard,
    /// `wait_change` never reports anything.
    deaf: bool,
    /// `read_text` answers another text.
    liar: bool,
    /// Writes after the first one fail.
    fail_after_first_write: bool,
    writes: usize,
}

impl Tweaked {
    fn new() -> (Self, FakeClipboardHandle) {
        let (inner, handle) = FakeClipboard::new();
        let tweaked = Self {
            inner,
            deaf: false,
            liar: false,
            fail_after_first_write: false,
            writes: 0,
        };
        (tweaked, handle)
    }
}

impl ClipboardAdapter for Tweaked {
    fn name(&self) -> &'static str {
        self.inner.name()
    }
    fn watch_mode(&self) -> clipsync_clipboard::adapter::WatchMode {
        self.inner.watch_mode()
    }
    fn wait_change(&mut self, timeout: Duration) -> Result<Change, ClipError> {
        if self.deaf {
            std::thread::sleep(timeout.min(Duration::from_millis(20)));
            return Ok(None);
        }
        self.inner.wait_change(timeout)
    }
    fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError> {
        if self.liar {
            return Ok(Some(ClipSnapshot {
                mime: MIME_TEXT_UTF8.into(),
                text: "outro texto".into(),
            }));
        }
        self.inner.read_text()
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipError> {
        self.writes += 1;
        if self.fail_after_first_write && self.writes > 1 {
            return Err(ClipError::Busy);
        }
        self.inner.write_text(text)
    }
}

#[test]
fn gives_the_previous_text_back() {
    let (mut adapter, handle) = Tweaked::new();
    handle.user_copy("minha senha secreta 123");
    let outcome = self_test_with(&mut adapter);
    // the fake is not the validated adapter: it works, but is reported as experimental
    assert!(
        matches!(outcome, SelfTestOutcome::Experimental(_)),
        "{outcome:?}"
    );
    assert_eq!(
        handle.contents().as_deref(),
        Some("minha senha secreta 123")
    );
    assert_eq!(handle.writes(), 2, "one synthetic write and one restore");
    assert!(outcome.detail().contains("conteúdo anterior foi devolvido"));
}

#[test]
fn an_empty_clipboard_stays_empty() {
    let (mut adapter, handle) = Tweaked::new();
    let outcome = self_test_with(&mut adapter);
    assert!(
        matches!(outcome, SelfTestOutcome::Experimental(_)),
        "{outcome:?}"
    );
    assert_eq!(handle.contents(), None, "the synthetic text must not stay");
    assert!(outcome.detail().contains("voltou a ficar vazio"));
}

#[test]
fn stale_events_do_not_count_as_the_echo() {
    // events queued before the test are drained, so the verdict comes from the test's own write
    let (mut adapter, handle) = Tweaked::new();
    handle.user_copy("antes");
    handle.owner_takeover();
    let outcome = self_test_with(&mut adapter);
    assert!(!outcome.detail().contains("não chegou"), "{outcome:?}");
}

#[test]
fn a_missing_change_event_is_experimental_and_says_so() {
    let (mut adapter, handle) = Tweaked::new();
    adapter.deaf = true;
    handle.user_copy("texto");
    let outcome = self_test_with(&mut adapter);
    match &outcome {
        SelfTestOutcome::Experimental(detail) => {
            assert!(detail.contains("aviso de mudança não chegou"), "{detail}");
        }
        other => panic!("esperado Experimental, veio {other:?}"),
    }
    assert_eq!(handle.contents().as_deref(), Some("texto"));
}

#[test]
fn a_failing_write_is_unavailable() {
    let (mut adapter, handle) = Tweaked::new();
    handle.fail_writes(true);
    let outcome = self_test_with(&mut adapter);
    match &outcome {
        SelfTestOutcome::Unavailable(detail) => {
            assert!(detail.contains("escrita no clipboard falhou"), "{detail}");
        }
        other => panic!("esperado Unavailable, veio {other:?}"),
    }
}

#[test]
fn a_read_that_does_not_match_is_unavailable() {
    let (mut adapter, _handle) = Tweaked::new();
    adapter.liar = true;
    let outcome = self_test_with(&mut adapter);
    assert_eq!(
        outcome,
        SelfTestOutcome::Unavailable(clipsync_clipboard::msg::SELF_TEST_MISMATCH.into())
    );
}

#[test]
fn not_giving_the_text_back_is_reported() {
    let (mut adapter, handle) = Tweaked::new();
    adapter.fail_after_first_write = true;
    handle.user_copy("importante");
    let outcome = self_test_with(&mut adapter);
    assert_eq!(
        outcome,
        SelfTestOutcome::Experimental(clipsync_clipboard::msg::SELF_TEST_RESTORE_FAILED.into())
    );
}

#[test]
fn the_outcome_never_carries_clipboard_content() {
    let (mut adapter, handle) = Tweaked::new();
    handle.user_copy("conteudo-sigiloso-do-usuario");
    let outcome = self_test_with(&mut adapter);
    let shown = format!("{outcome:?} {}", outcome.detail());
    assert!(!shown.contains("conteudo-sigiloso-do-usuario"), "{shown}");
    assert!(!shown.contains("clipsync-autoteste"), "{shown}");
}

/// The real adapter of the platform: X11 bridge on Linux (validated), `WinClipboard` on Windows
/// (experimental, polling). Needs a graphical session, so CI skips it.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requer sessão gráfica"]
async fn platform_adapter_round_trip() {
    let before = clipsync_clipboard::self_test().await;
    let detail = before.detail().to_owned();
    #[cfg(target_os = "linux")]
    assert!(matches!(before, SelfTestOutcome::Validated(_)), "{detail}");
    #[cfg(windows)]
    assert!(
        matches!(before, SelfTestOutcome::Experimental(_)),
        "{detail}"
    );
}
