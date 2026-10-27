//! Laço de eventos: `Ui::subscribe` → `emit_to("main", "clipsync://event", ev)` (§12.7 regra 7).
//! `notification` vira notificação do sistema; `pause_changed` troca o texto da bandeja. A casca
//! lê o evento pela forma JSON da §12.5 (`{event, data}`), não pelas variantes do crate.

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::broadcast::error::RecvError;

use crate::ui::Ui;
use crate::{msg, tray, window};

/// Nome único de evento no front (§12.4). O Tauri aceita `:` e `/` em nomes de evento.
pub const EVENT: &str = "clipsync://event";

pub fn spawn(app: AppHandle, ui: Ui) {
    tauri::async_runtime::spawn(async move {
        let mut events = ui.subscribe();
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

/// Notificação do sistema. O crate já filtrou pelas preferências e nunca manda conteúdo de
/// clipboard (§12.2 regra 7).
fn notify(app: &AppHandle, event: &Value) {
    let (title, body) = notification_text(event);
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
    fn notification_falls_back_to_the_app_name() {
        let full = json!({ "event": "notification", "data": { "kind": "block", "title": "Bloqueio", "body": "x" } });
        let bare = json!({ "event": "notification", "data": { "kind": "block", "title": null, "body": null } });
        assert_eq!(notification_text(&full), ("Bloqueio", "x"));
        assert_eq!(notification_text(&bare), (msg::APP_NAME, ""));
    }
}
