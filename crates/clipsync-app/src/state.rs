//! Estado compartilhado da casca. O Tauri já compartilha o estado gerenciado: sem `Arc` aqui.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::watch;

use crate::msg;
use crate::ui::{Ui, UiError};

/// Fase da `Ui::start`, que roda depois que a janela existe.
pub enum Startup {
    Pending,
    Ready(Ui),
    Failed(UiError),
}

pub struct AppState {
    hidden: bool,
    startup: watch::Sender<Startup>,
    paused: AtomicBool,
    quitting: AtomicBool,
}

impl AppState {
    pub fn new(hidden: bool) -> Self {
        let (startup, _) = watch::channel(Startup::Pending);
        Self {
            hidden,
            startup,
            paused: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
        }
    }

    /// `--hidden`: a janela não aparece sozinha (autostart, §12.7 regra 6).
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    pub fn set_ready(&self, ui: Ui) {
        self.startup.send_replace(Startup::Ready(ui));
    }

    pub fn set_failed(&self, error: UiError) {
        self.startup.send_replace(Startup::Failed(error));
    }

    /// A `Ui` de pé, ou o erro da subida. Espera enquanto `Ui::start` ainda roda, para o primeiro
    /// comando do front não falhar só por ter chegado cedo.
    pub async fn ui(&self) -> Result<Ui, UiError> {
        let mut startup = self.startup.subscribe();
        let ready = startup
            .wait_for(|phase| !matches!(phase, Startup::Pending))
            .await;
        match ready.as_deref() {
            Ok(Startup::Ready(ui)) => Ok(ui.clone()),
            Ok(Startup::Failed(error)) => Err(error.clone()),
            Ok(Startup::Pending) | Err(_) => Err(UiError {
                code: "internal".into(),
                message: msg::UI_NOT_READY.into(),
                field: None,
                details: None,
            }),
        }
    }

    /// Sem esperar: para a bandeja e o encerramento.
    pub fn ui_now(&self) -> Option<Ui> {
        match &*self.startup.borrow() {
            Startup::Ready(ui) => Some(ui.clone()),
            Startup::Pending | Startup::Failed(_) => None,
        }
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    /// Verdade só para o primeiro chamador: o encerramento roda uma vez.
    pub fn begin_quit(&self) -> bool {
        !self.quitting.swap(true, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::UiOptions;
    use std::sync::Arc;
    use std::time::Duration;

    async fn stub_ui() -> Ui {
        Ui::start(UiOptions {
            profile: None,
            poll: Duration::from_millis(10),
            listen: None,
            fake_adapters: true,
            autostart_dir: None,
            downloads_dir: None,
        })
        .await
        .expect("stub starts")
    }

    #[tokio::test]
    async fn command_waits_for_the_ui_to_come_up() {
        let state = Arc::new(AppState::new(false));
        assert!(state.ui_now().is_none());
        let waiter = {
            let state = Arc::clone(&state);
            tokio::spawn(async move { state.ui().await.is_ok() })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        state.set_ready(stub_ui().await);
        assert!(waiter.await.expect("task finishes"));
        assert!(state.ui_now().is_some());
    }

    #[tokio::test]
    async fn failed_start_is_reported_to_every_command() {
        let state = AppState::new(true);
        state.set_failed(UiError {
            code: "config".into(),
            message: "x".into(),
            field: None,
            details: None,
        });
        let code = state.ui().await.err().map(|error| error.code);
        assert_eq!(code.as_deref(), Some("config"));
        assert!(state.ui_now().is_none());
        assert!(state.hidden());
    }

    #[test]
    fn quit_begins_only_once_and_pause_roundtrips() {
        let state = AppState::new(false);
        assert!(state.begin_quit());
        assert!(!state.begin_quit());
        assert!(!state.paused());
        state.set_paused(true);
        assert!(state.paused());
    }
}
