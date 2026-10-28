//! Laço de eventos: `Ui::subscribe` → `emit_to("main", "clipsync://event", ev)` (§12.7 regra 7).
//! `notification` vira notificação do sistema; `pause_changed` troca o texto da bandeja. A casca
//! lê o evento pela forma JSON da §12.5 (`{event, data}`), não pelas variantes do crate.

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::broadcast::error::RecvError;

use crate::notify_prefs::NotifyPrefs;
use crate::notify_text;
use crate::state::AppState;
use crate::ui::Ui;
use crate::{msg, tray, window};

/// Nome único de evento no front (§12.4). O Tauri aceita `:` e `/` em nomes de evento.
pub const EVENT: &str = "clipsync://event";

/// Lê as preferências de notificação (`prefs_get`) para a casca. Sem elas nenhuma notificação do
/// sistema sai (falha fechada). Roda antes de a `Ui` ser liberada ao front, então nenhuma
/// preferência muda no meio.
pub async fn load_notify_prefs(app: &AppHandle, ui: &Ui) {
    match ui.call(json!({ "cmd": "prefs_get" })).await {
        Ok(prefs) => match NotifyPrefs::from_prefs(&prefs) {
            Some(prefs) => app.state::<AppState>().set_notify_prefs(prefs),
            None => {
                tracing::warn!("prefs_get without notifications; system notifications stay off")
            }
        },
        Err(error) => {
            tracing::warn!(code = %error.code, "prefs_get failed; system notifications stay off");
        }
    }
}

/// Assina na hora (antes do `await`), para não perder evento entre a subida e o primeiro laço.
pub fn spawn(app: AppHandle, ui: Ui) {
    let mut events = ui.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => match serde_json::to_value(&event) {
                    Ok(value) => forward(&app, value),
                    Err(error) => tracing::warn!(error = %error, "event did not serialize"),
                },
                Err(RecvError::Lagged(skipped)) => {
                    // O front perdeu eventos: pede que ele busque tudo de novo (§12.5 evento 11).
                    tracing::warn!(skipped, "event receiver lagged");
                    forward(&app, json!({ "event": "resync", "data": {} }));
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn forward(app: &AppHandle, event: Value) {
    match event_name(&event) {
        Some("notification") => notify(app, &event),
        Some("prefs_changed") => {
            if let Some(prefs) = event
                .pointer("/data/prefs")
                .and_then(NotifyPrefs::from_prefs)
            {
                app.state::<AppState>().set_notify_prefs(prefs);
            }
        }
        Some("pause_changed") => {
            if let Some(paused) = paused_of(&event) {
                tray::set_paused(app, paused);
            }
        }
        _ => {}
    }
    if let Err(error) = app.emit_to(window::LABEL, EVENT, event) {
        tracing::warn!(error = %error, "could not emit to the window");
    }
}

fn event_name(event: &Value) -> Option<&str> {
    event.get("event").and_then(Value::as_str)
}

fn paused_of(event: &Value) -> Option<bool> {
    event.pointer("/data/paused").and_then(Value::as_bool)
}

/// Título e corpo de um evento `notification`; sem título, o nome do app.
fn notification_text(event: &Value) -> (&str, &str) {
    let title = event
        .pointer("/data/title")
        .and_then(Value::as_str)
        .unwrap_or(msg::APP_NAME);
    let body = event
        .pointer("/data/body")
        .and_then(Value::as_str)
        .unwrap_or_default();
    (title, body)
}

/// O `kind` de um evento `notification` (`item_received`, `device_connected` ou `block`).
fn notification_kind(event: &Value) -> Option<&str> {
    event.pointer("/data/kind").and_then(Value::as_str)
}

/// Notificação do sistema, só se o usuário ligou aquele tipo. O crate já filtra e nunca manda
/// conteúdo de clipboard (§12.2 regra 7); a casca confere de novo porque é ela que fala com o
/// sistema (S6-13).
fn notify(app: &AppHandle, event: &Value) {
    let wanted = notification_kind(event)
        .is_some_and(|kind| app.state::<AppState>().notify_prefs().allows(kind));
    if !wanted {
        tracing::debug!("system notification skipped by the preferences");
        return;
    }
    let (title, body) = notification_text(event);
    // O nome do par vem de fora: no Linux o daemon lê marcação, então `&`, `<` e `>` são escapados.
    let (title, body) = (
        notify_text::for_system(title),
        notify_text::for_system(body),
    );
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!(error = %error, "system notification failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_is_read_from_the_contract_shape() {
        let on = json!({ "event": "pause_changed", "data": { "paused": true } });
        let other = json!({ "event": "peer_connected", "data": { "peer_id": "ab" } });
        assert_eq!(event_name(&on), Some("pause_changed"));
        assert_eq!(paused_of(&on), Some(true));
        assert_eq!(paused_of(&other), None);
    }

    #[test]
    fn notification_kind_is_read_from_the_contract_shape() {
        let event = json!({ "event": "notification", "data": { "kind": "block", "title": "t", "body": "b" } });
        assert_eq!(notification_kind(&event), Some("block"));
        assert_eq!(notification_kind(&json!({ "event": "notification" })), None);
    }

    #[test]
    fn notification_falls_back_to_the_app_name() {
        let full = json!({ "event": "notification", "data": { "kind": "block", "title": "Bloqueio", "body": "x" } });
        let bare = json!({ "event": "notification", "data": { "kind": "block", "title": null, "body": null } });
        assert_eq!(notification_text(&full), ("Bloqueio", "x"));
        assert_eq!(notification_text(&bare), (msg::APP_NAME, ""));
    }
}
