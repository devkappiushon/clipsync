//! Janela `main`. Está no tauri.conf.json com `create: false` e nasce aqui, para ganhar o bloqueio
//! de navegação para fora do app e de janela nova (§12.7 regra 3). Invisível até o front terminar
//! de carregar; com `--hidden`, só aparece pela bandeja.

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Url, WebviewWindowBuilder};

use crate::state::AppState;

pub const LABEL: &str = "main";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let config = app.config();
    let window = config
        .app
        .windows
        .iter()
        .find(|w| w.label == LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;
    WebviewWindowBuilder::from_config(app, window)?
        .on_navigation(is_app_origin)
        // O front não abre janela (sem `window.open` nem link externo): nenhuma é permitida.
        .on_new_window(|_url, _features| NewWindowResponse::Deny)
        .build()?;
    Ok(())
}

/// Só a origem exata do app: `tauri://localhost` (Linux) e `http(s)://tauri.localhost` (Windows),
/// sem porta nem credencial na URL. Outro host, outra porta ou `user@` é navegação para fora.
fn is_app_origin(url: &Url) -> bool {
    if url.port().is_some() || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => url.host_str() == Some("tauri.localhost"),
        _ => false,
    }
}

/// Fim da carga do front: mostra a janela, salvo `--hidden`.
pub fn reveal(app: &AppHandle) {
    if !app.state::<AppState>().hidden() {
        show_main(app);
    }
}

/// Mostra e foca a janela principal (fechar só esconde, então ela sempre existe).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(s: &str) -> bool {
        is_app_origin(&Url::parse(s).expect("valid url"))
    }

    #[test]
    fn own_origin_is_allowed() {
        assert!(allowed("tauri://localhost/index.html"));
        assert!(allowed("tauri://localhost"));
        assert!(allowed("http://tauri.localhost/index.html"));
        assert!(allowed("https://tauri.localhost/"));
        // a porta padrão do esquema some na análise da URL: é a mesma origem
        assert!(allowed("https://tauri.localhost:443/"));
    }

    #[test]
    fn anything_else_is_blocked() {
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("http://tauri.localhost.evil.com/"));
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("javascript:alert(1)"));
    }

    #[test]
    fn host_port_and_credentials_must_match_exactly() {
        assert!(!allowed("tauri://outro/index.html"));
        assert!(!allowed("tauri://localhost.evil.com/"));
        assert!(!allowed("tauri://localhost:8080/"));
        assert!(!allowed("http://tauri.localhost:8080/"));
        assert!(!allowed("https://tauri.localhost:8443/"));
        assert!(!allowed("http://user@tauri.localhost/"));
        assert!(!allowed("http://user:senha@tauri.localhost/"));
        assert!(!allowed("tauri://user@localhost/"));
        assert!(!allowed("http://tauri.localhost@evil.com/"));
        assert!(!allowed("http://localhost/"));
        assert!(!allowed("ws://tauri.localhost/"));
    }
}
