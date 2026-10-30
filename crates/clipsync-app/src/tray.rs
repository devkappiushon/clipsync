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
const ID_TRAVERSAL: &str = "traversal";

/// O único item cujo texto muda; guardado para o laço de eventos trocar o rótulo.
struct PauseItem(MenuItem<Wry>);
struct TraversalItem(MenuItem<Wry>);

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let traversal = MenuItem::with_id(
        app,
        ID_TRAVERSAL,
        msg::TRAY_TRAVERSAL_CHECKING,
        false,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, ID_OPEN, msg::TRAY_OPEN, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, ID_PAUSE, msg::TRAY_PAUSE, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ID_QUIT, msg::TRAY_QUIT, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&traversal, &open, &pause, &separator, &quit])?;

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
    app.manage(TraversalItem(traversal));
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
    if let Some(item) = app.try_state::<PauseItem>() {
        if let Err(error) = item.0.set_text(label) {
            tracing::warn!(error = %error, "could not relabel the pause item");
        }
        let _ = item.0.set_enabled(true);
    }
}

fn pause_unknown(app: &AppHandle) {
    if let Some(item) = app.try_state::<PauseItem>() {
        let _ = item.0.set_text(msg::TRAY_PAUSE_UNKNOWN);
        let _ = item.0.set_enabled(false);
    }
}

/// Estado inicial de pausa (`app_state.paused`), logo que a `Ui` sobe. O resto vem por
/// `pause_changed`, que pega também o `clipsync pause` do terminal.
pub async fn sync_paused(app: &AppHandle, ui: &Ui) {
    match ui.call(json!({ "cmd": "app_state" })).await {
        Ok(state) => {
            set_traversal(app, state.get("traversal").unwrap_or(&Value::Null));
            if let Some(paused) = confirmed_pause(&state) {
                set_paused(app, paused);
            } else {
                pause_unknown(app);
            }
        }
        Err(error) => {
            pause_unknown(app);
            set_traversal(app, &Value::Null);
            tracing::debug!(code = %error.code, "app_state unavailable at boot");
        }
    }
}

fn confirmed_pause(state: &Value) -> Option<bool> {
    if !state.get("agent_error").is_none_or(Value::is_null) {
        return None;
    }
    state.get("paused").and_then(Value::as_bool)
}

/// O mesmo estado da janela, resumido para o menu da bandeja.
pub fn set_traversal(app: &AppHandle, traversal: &Value) {
    let label = traversal_label(traversal);
    if let Some(item) = app.try_state::<TraversalItem>()
        && let Err(error) = item.0.set_text(&label)
    {
        tracing::warn!(error = %error, "could not relabel traversal item");
    }
}

fn traversal_label(traversal: &Value) -> String {
    let Some(detail) = traversal
        .get("detail")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return msg::TRAY_TRAVERSAL_CHECKING.into();
    };
    let sentence = detail.split('.').next().unwrap_or(detail);
    let short: String = sentence.chars().take(100).collect();
    let suffix = if sentence.chars().count() > 100 {
        "…"
    } else {
        ""
    };
    format!("Mouse: {short}{suffix}")
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
        Ok(_) => sync_paused(app, &ui).await,
        Err(error) => {
            tracing::warn!(code = %error.code, "pause toggle failed");
            pause_unknown(app);
            sync_paused(app, &ui).await;
            // O aviso fica em app_state; não depende de preferência/daemon de notificação.
            window::show_main(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reconsulta_com_erro_nao_confirma_fallback_de_pausa() {
        for agent in ["in_app", "external", "failed"] {
            for paused in [true, false] {
                assert_eq!(
                    confirmed_pause(
                        &json!({"agent":agent,"agent_error":"estado desconhecido","paused":paused,"pause_warning":"retomada não confirmada"})
                    ),
                    None
                );
                // Aviso de persistência não invalida um Status atual que foi observado.
                assert_eq!(
                    confirmed_pause(
                        &json!({"agent":agent,"agent_error":null,"paused":paused,"pause_warning":"pausa aplicada em memória, mas não salva"})
                    ),
                    Some(paused)
                );
            }
        }
        assert_eq!(confirmed_pause(&Value::Null), None);
        assert_eq!(confirmed_pause(&json!({"agent_error":null})), None);
    }

    #[test]
    fn old_or_missing_state_never_claims_the_mouse_is_working() {
        for value in [Value::Null, json!({}), json!({"detail":""})] {
            assert_eq!(traversal_label(&value), msg::TRAY_TRAVERSAL_CHECKING);
        }
    }
    #[test]
    fn traversal_label_uses_the_shared_detail_and_keeps_the_menu_bounded() {
        assert_eq!(
            traversal_label(
                &json!({"code":"paused","detail":"O ClipSync está pausado. Retome para usar."})
            ),
            "Mouse: O ClipSync está pausado"
        );
        let label = traversal_label(&json!({"detail":"ç".repeat(200)}));
        assert_eq!(label.chars().count(), 108);
        assert!(label.ends_with('…'));
    }
}
