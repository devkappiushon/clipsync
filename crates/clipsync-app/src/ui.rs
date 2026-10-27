//! STUB do `clipsync-ui` (Rivet). TODO(contrato §12.3): apagar este arquivo e trocar `mod ui;` em
//! `main.rs` por `use clipsync_ui as ui;` (mais a dependência no Cargo.toml) quando o crate
//! existir. A casca só usa a superfície da §12.3 (`Ui`, `UiOptions`, `UiError`, `UiEvent`) e lê
//! os eventos pela forma JSON da §12.5, então a troca não toca os outros módulos.
//!
//! Enquanto isso: `start` sobe sem agente, todo comando bem formado responde `unavailable`, forma
//! errada responde `bad_request`, e nenhum evento sai.
#![allow(dead_code)] // superfície completa do contrato; a casca ainda não usa cada campo

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::msg;

pub struct UiOptions {
    /// `None` = `Profile::resolve(None)`, igual à CLI.
    pub profile: Option<PathBuf>,
    /// Período da consulta ao agente (1 s no app; testes usam menos).
    pub poll: Duration,
    pub listen: Option<SocketAddr>,
    pub fake_adapters: bool,
    pub autostart_dir: Option<PathBuf>,
    pub downloads_dir: Option<PathBuf>,
}

/// Erro de comando (§12.4): a promessa do front rejeita com este objeto, `message` já em português.
#[derive(Serialize, Debug, Clone)]
pub struct UiError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub details: Option<Value>,
}

/// Evento da §12.5, serializado como `{"event": "<nome>", "data": {...}}`.
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum UiEvent {
    Resync {},
}

#[derive(Clone)]
pub struct Ui {
    events: broadcast::Sender<UiEvent>,
}

impl Ui {
    /// Exige runtime tokio multi-thread no contexto (a casca usa o do Tauri).
    pub async fn start(_options: UiOptions) -> Result<Ui, UiError> {
        let (events, _) = broadcast::channel(64);
        Ok(Ui { events })
    }

    /// Entrada única: o JSON de `UiRequest`. Forma errada = `bad_request`.
    pub async fn call(&self, req: Value) -> Result<Value, UiError> {
        let well_formed = req.get("cmd").and_then(Value::as_str).is_some();
        let (code, message) = if well_formed {
            ("unavailable", msg::UI_STUB)
        } else {
            ("bad_request", msg::BAD_REQUEST)
        };
        Err(UiError {
            code: code.into(),
            message: message.into(),
            field: None,
            details: None,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<UiEvent> {
        self.events.subscribe()
    }

    /// Para a consulta, a tentativa de pareamento e o agente in_app (os pares recebem Bye).
    pub async fn shutdown(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn options() -> UiOptions {
        UiOptions {
            profile: None,
            poll: Duration::from_millis(10),
            listen: None,
            fake_adapters: true,
            autostart_dir: None,
            downloads_dir: None,
        }
    }

    #[tokio::test]
    async fn well_formed_command_is_unavailable_in_the_stub() {
        let ui = Ui::start(options()).await.expect("stub starts");
        let error = ui.call(json!({ "cmd": "app_state" })).await.unwrap_err();
        assert_eq!(error.code, "unavailable");
    }

    #[tokio::test]
    async fn malformed_request_is_bad_request() {
        let ui = Ui::start(options()).await.expect("stub starts");
        let error = ui.call(json!("Status")).await.unwrap_err();
        assert_eq!(error.code, "bad_request");
    }

    #[test]
    fn event_json_has_the_contract_shape() {
        let value = serde_json::to_value(UiEvent::Resync {}).expect("serializes");
        assert_eq!(value, json!({ "event": "resync", "data": {} }));
    }
}
