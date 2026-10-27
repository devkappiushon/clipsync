//! Janela `main`. Está no tauri.conf.json com `create: false` e nasce aqui, para ganhar o bloqueio
//! de navegação para fora do app (§12.7 regra 3). Invisível até o front terminar de carregar;
//! com `--hidden`, só aparece pela bandeja.

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
        .build()?;
    Ok(())
}

/// Só a origem do próprio app: `tauri://localhost` (Linux) e `http(s)://tauri.localhost` (Windows).
fn is_app_origin(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
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
        assert!(allowed("http://tauri.localhost/index.html"));
        assert!(allowed("https://tauri.localhost/"));
    }

    #[test]
    fn anything_else_is_blocked() {
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("http://tauri.localhost.evil.com/"));
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("javascript:alert(1)"));
    }
}
