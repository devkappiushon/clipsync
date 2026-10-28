//! Revisão de segurança da S6 (docs/seguranca.md), itens da parte do Flux: gravação de `ui.toml` e
//! do `.desktop` sem seguir link e com modo certo, autostart que não sobra quando o salvar falha,
//! pacote de suporte igual à prévia.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

mod common;

use common::*;
use serde_json::json;

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// S6-07: `ui.toml` is private (0600) and the `.desktop` is readable (0644), whatever the umask,
/// and neither leaves a temp file behind.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn ui_toml_is_private_and_the_desktop_file_is_readable() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "a").await;
    ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": {
            "autostart": true,
            "appearance": { "theme": "dark", "reduce_motion": false },
        } }),
    )
    .await;
    let profile = dir.path().join("a");
    let autostart = dir.path().join("a-autostart");
    assert_eq!(mode_of(&profile.join("ui.toml")), 0o600);
    assert_eq!(
        mode_of(&autostart.join(clipsync_ui::autostart::DESKTOP_FILE)),
        0o644
    );
    let leftovers = |d: &std::path::Path| {
        std::fs::read_dir(d)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .count()
    };
    assert_eq!(leftovers(&profile), 0);
    assert_eq!(leftovers(&autostart), 0);
    ui.shutdown().await;
}
