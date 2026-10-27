//! SIGINT e SIGTERM (Ctrl+C no Windows) seguem o mesmo caminho limpo de "Sair" (§12.7 regra 6).

use tauri::AppHandle;

use crate::lifecycle;

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        wait().await;
        lifecycle::quit(app).await;
    });
}

#[cfg(unix)]
async fn wait() {
    use tokio::signal::unix::{SignalKind, signal};
    match (
        signal(SignalKind::terminate()),
        signal(SignalKind::interrupt()),
    ) {
        (Ok(mut term), Ok(mut int)) => {
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
        }
        // Sem tratamento de sinal disponível: nunca dispara; a bandeja ainda encerra.
        _ => std::future::pending::<()>().await,
    }
}

#[cfg(not(unix))]
async fn wait() {
    let _ = tokio::signal::ctrl_c().await;
}
