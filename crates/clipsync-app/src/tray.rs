//! Bandeja: Abrir, "Pausar tudo"/"Retomar" e Sair (§12.7 regra 5). No Linux o clique no ícone não
//! gera evento (limitação do Tauri): tudo vai pelo menu.

use serde_json::{Value, json};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::state::AppState;
use crate::ui::Ui;
use crate::{lifecycle, msg, window};

const ID_OPEN: &str = "open";
const ID_PAUSE: &str = "pause";
const ID_QUIT: &str = "quit";

/// O único item cujo texto muda; guardado para o laço de eventos trocar o rótulo.
struct PauseItem(MenuItem<Wry>);

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, ID_OPEN, msg::TRAY_OPEN, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, ID_PAUSE, msg::TRAY_PAUSE, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_QUIT, msg::TRAY_QUIT, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pause, &separator, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(msg::APP_NAME)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            ID_OPEN => window::show_main(app),
            ID_PAUSE => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { toggle_pause(&app).await });
            }
            ID_QUIT => {
                tauri::async_runtime::spawn(lifecycle::quit(app.clone()));
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    app.manage(PauseItem(pause));
    Ok(())
}

/// Guarda o estado e troca o rótulo para a ação que o item fará a seguir.
pub fn set_paused(app: &AppHandle, paused: bool) {
    app.state::<AppState>().set_paused(paused);
    let label = if paused {
        msg::TRAY_RESUME
    } else {
        msg::TRAY_PAUSE
    };
    if let Some(item) = app.try_state::<PauseItem>()
        && let Err(error) = item.0.set_text(label)
    {
        tracing::warn!(error = %error, "could not relabel the pause item");
    }
}

/// Estado inicial de pausa (`app_state.paused`), logo que a `Ui` sobe. O resto vem por
/// `pause_changed`, que pega também o `clipsync pause` do terminal.
pub async fn sync_paused(app: &AppHandle, ui: &Ui) {
    match ui.call(json!({ "cmd": "app_state" })).await {
        Ok(state) => {
            if let Some(paused) = state.get("paused").and_then(Value::as_bool) {
                set_paused(app, paused);
            }
        }
        Err(error) => tracing::debug!(code = %error.code, "app_state unavailable at boot"),
    }
}

async fn toggle_pause(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(ui) = state.ui_now() else {
        return;
    };
    let target = !state.paused();
    match ui
        .call(json!({ "cmd": "pause_set", "args": { "paused": target } }))
        .await
    {
        Ok(reply) => {
            let paused = reply
                .get("paused")
                .and_then(Value::as_bool)
                .unwrap_or(target);
            set_paused(app, paused);
        }
        Err(error) => tracing::warn!(code = %error.code, "pause toggle failed"),
    }
}
