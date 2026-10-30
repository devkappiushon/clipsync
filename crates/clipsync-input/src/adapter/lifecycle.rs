//! Último estado de captura; a geração sobrevive à troca dos receivers de dados.
use super::Zone;
#[derive(Debug, Clone, Default)]
pub struct CaptureLifecycle {
    pub revision: u64,
    pub invalidation_generation: u64,
    pub valid: bool,
    pub session_locked: Option<bool>,
    pub desktop_available: Option<bool>,
    pub zones: Vec<Zone>,
    pub terminal_error: Option<&'static str>,
}
impl CaptureLifecycle {
    pub(crate) fn allows_capture(&self) -> bool {
        self.valid
            && self.session_locked == Some(false)
            && self.desktop_available == Some(true)
            && self.terminal_error.is_none()
    }
}

/// Baseline pertence ao consumidor; subscribe nunca confirma uma limpeza.
pub(crate) struct LifecycleCursor {
    rx: Option<tokio::sync::watch::Receiver<CaptureLifecycle>>,
    pub snapshot: Option<CaptureLifecycle>,
    last_generation: Option<u64>,
    pub pending_invalidation: bool,
}
impl LifecycleCursor {
    pub fn new(cap: &dyn super::CaptureAdapter) -> Self {
        let mut cursor = Self {
            rx: cap.lifecycle_receiver(),
            snapshot: None,
            last_generation: None,
            pending_invalidation: false,
        };
        cursor.refresh();
        cursor.acknowledge_cleanup();
        cursor
    }
    pub fn refresh(&mut self) {
        let Some(rx) = self.rx.as_mut() else {
            return;
        };
        let mut snapshot = rx.borrow_and_update().clone();
        if rx.has_changed().is_err() {
            snapshot.valid = false;
            snapshot.terminal_error = Some("captura encerrada");
        }
        if self
            .last_generation
            .is_some_and(|g| g != snapshot.invalidation_generation)
        {
            self.pending_invalidation = true;
        }
        self.snapshot = Some(snapshot);
    }
    pub fn reacquire(&mut self, cap: &dyn super::CaptureAdapter) {
        self.rx = cap.lifecycle_receiver();
        self.refresh();
    }
    pub fn acknowledge_cleanup(&mut self) {
        self.refresh();
        self.last_generation = self.snapshot.as_ref().map(|s| s.invalidation_generation);
        self.pending_invalidation = false;
    }
    pub async fn changed(&mut self) {
        match self.rx.as_mut() {
            Some(rx) if rx.has_changed().is_ok() => {
                let _ = rx.changed().await;
            }
            _ => std::future::pending::<()>().await,
        }
    }
}

#[cfg(any(windows, test))]
use std::sync::atomic::{AtomicU64, Ordering};
/// Um único word: último WTS, dirty bits e geração. ACK só limpa os bits da
/// versão lida; uma escrita concorrente impede o ACK. Não há fila de notices.
#[cfg(any(windows, test))]
#[derive(Default)]
pub(crate) struct LifecycleMailbox(AtomicU64);
#[cfg(any(windows, test))]
impl LifecycleMailbox {
    pub const DESKTOP: u64 = 4;
    pub const DISPLAYS: u64 = 8;
    pub const HOOKS: u64 = 16;
    pub const SESSION: u64 = 32;
    const SHIFT: u32 = 6;
    pub fn read(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
    pub fn generation(word: u64) -> u64 {
        word >> Self::SHIFT
    }
    pub fn session(word: u64) -> Option<bool> {
        match word & 3 {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }
    pub fn mark(&self, dirty: u64, session: Option<bool>) {
        let _ = self
            .0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |word| {
                let generation = Self::generation(word).checked_add(1)?;
                if generation > (u64::MAX >> Self::SHIFT) {
                    return None;
                }
                let mut state = (word & 63) | dirty;
                if let Some(locked) = session {
                    state = (state & !3) | if locked { 1 } else { 2 };
                }
                Some((generation << Self::SHIFT) | state)
            });
    }
    pub fn exhausted(word: u64) -> bool {
        Self::generation(word) == (u64::MAX >> Self::SHIFT)
    }
    pub fn acknowledge(&self, word: u64) -> bool {
        self.0
            .compare_exchange(word, word & !60, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::FakeCapture;

    fn unlocked(generation: u64) -> CaptureLifecycle {
        CaptureLifecycle {
            valid: true,
            session_locked: Some(false),
            desktop_available: Some(true),
            invalidation_generation: generation,
            ..Default::default()
        }
    }

    #[test]
    fn reacquiring_does_not_acknowledge_coalesced_invalidation() {
        let (cap, handle) = FakeCapture::new(vec![]);
        handle.enable_lifecycle(unlocked(4));
        let mut cursor = LifecycleCursor::new(&cap);
        assert!(cursor.snapshot.as_ref().unwrap().allows_capture());
        handle.session_locked(true);
        handle.session_locked(false);
        cursor.reacquire(&cap);
        assert!(cursor.pending_invalidation);
        assert!(cursor.snapshot.as_ref().unwrap().allows_capture());
        cursor.reacquire(&cap);
        assert!(
            cursor.pending_invalidation,
            "subscribe não confirma limpeza"
        );
        cursor.acknowledge_cleanup();
        cursor.refresh();
        assert!(!cursor.pending_invalidation);
        handle.publish_lifecycle(unlocked(6));
        cursor.reacquire(&cap);
        assert!(cursor.pending_invalidation);
    }

    #[test]
    fn capture_gate_requires_known_wts_desktop_and_nonterminal_snapshot() {
        for session in [None, Some(true), Some(false)] {
            for desktop in [None, Some(false), Some(true)] {
                let mut snapshot = unlocked(0);
                snapshot.session_locked = session;
                snapshot.desktop_available = desktop;
                assert_eq!(
                    snapshot.allows_capture(),
                    session == Some(false) && desktop == Some(true)
                );
                snapshot.valid = false;
                assert!(!snapshot.allows_capture());
                snapshot.valid = true;
                snapshot.terminal_error = Some("encerrado");
                assert!(!snapshot.allows_capture());
            }
        }
    }

    #[test]
    fn mailbox_retains_all_facts_and_rejects_stale_acknowledgement() {
        let mailbox = LifecycleMailbox::default();
        let before = mailbox.read();
        mailbox.mark(LifecycleMailbox::SESSION, Some(true));
        mailbox.mark(LifecycleMailbox::SESSION, Some(false));
        mailbox.mark(LifecycleMailbox::DESKTOP, None);
        mailbox.mark(LifecycleMailbox::DISPLAYS, None);
        mailbox.mark(LifecycleMailbox::HOOKS, None);
        mailbox.mark(0, None); // overflow invalida dados, nunca apaga notices
        assert!(!mailbox.acknowledge(before));
        let word = mailbox.read();
        assert_eq!(LifecycleMailbox::generation(word), 6);
        assert_eq!(LifecycleMailbox::session(word), Some(false));
        assert_eq!(word & 60, 60);
        assert!(mailbox.acknowledge(word));
        let after = mailbox.read();
        assert_eq!(LifecycleMailbox::generation(after), 6);
        assert_eq!(LifecycleMailbox::session(after), Some(false));
        assert_eq!(after & 60, 0);
    }

    #[test]
    fn mailbox_exhaustion_never_reuses_a_generation() {
        let mailbox = LifecycleMailbox(AtomicU64::new(u64::MAX - 64));
        mailbox.mark(LifecycleMailbox::SESSION, Some(false));
        assert!(LifecycleMailbox::exhausted(mailbox.read()));
        let word = mailbox.read();
        mailbox.mark(LifecycleMailbox::SESSION, Some(true));
        assert_eq!(mailbox.read(), word);
        assert!(LifecycleMailbox::exhausted(mailbox.read()));
    }
}
