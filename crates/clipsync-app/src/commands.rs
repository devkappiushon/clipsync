//! O único comando Tauri do app (§12.4). Fino de propósito: a lógica fica no `clipsync-ui`.

use serde_json::Value;
use tauri::State;

use crate::state::AppState;
use crate::ui::UiError;

/// `window.__TAURI__.core.invoke("clipsync", { req })`. Resposta: o objeto de retorno. Erro: a
/// promessa rejeita com `{ code, message, field, details }`.
#[tauri::command]
pub async fn clipsync(state: State<'_, AppState>, req: Value) -> Result<Value, UiError> {
    let ui = state.ui().await?;
    ui.call(req).await
}
