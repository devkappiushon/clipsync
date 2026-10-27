//! Subida e encerramento. `Ui::start` roda depois que a janela existe; "Sair", SIGINT e SIGTERM
//! passam por `Ui::shutdown` (Bye aos pares) e só então o processo termina (§12.7 regra 6).

use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::time::timeout;

use crate::state::AppState;
use crate::ui::{Ui, UiOptions};
use crate::{events, tray};

/// Teto para o Bye aos pares; depois disso o app sai de qualquer jeito.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(8);

/// Sobe a `Ui` (e com ela o agente, se o perfil existe). Falha fica no estado: todo comando do
/// front recebe o erro e a tela mostra a mensagem; o app continua na bandeja.
pub async fn boot(app: AppHandle, options: UiOptions) {
    match Ui::start(options).await {
        Ok(ui) => {
            events::spawn(app.clone(), ui.clone());
            app.state::<AppState>().set_ready(ui.clone());
            tray::sync_paused(&app, &ui).await;
        }
        Err(error) => {
            tracing::error!(code = %error.code, "ui start failed");
            app.state::<AppState>().set_failed(error);
        }
    }
}

/// Encerramento limpo, uma vez só: Bye aos pares, depois `exit(0)`.
pub async fn quit(app: AppHandle) {
    let ui = {
        let state = app.state::<AppState>();
        if !state.begin_quit() {
            return;
        }
        state.ui_now()
    };
    if let Some(ui) = ui
        && timeout(SHUTDOWN_WAIT, ui.shutdown()).await.is_err()
    {
        tracing::warn!("ui did not shut down in time; exiting anyway");
    }
    app.exit(0);
}
