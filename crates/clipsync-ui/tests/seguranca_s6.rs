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

/// S6-08: when `ui.toml` cannot be saved, `prefs_set` fails and the autostart is left as it was.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_save_does_not_change_the_autostart() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "a").await;
    let desktop = dir
        .path()
        .join("a-autostart")
        .join(clipsync_ui::autostart::DESKTOP_FILE);
    let light = json!({ "theme": "light", "reduce_motion": false });
    let dark = json!({ "theme": "dark", "reduce_motion": false });

    // A directory where `ui.toml` goes: the rename of the new file over it fails.
    let ui_toml = dir.path().join("a").join("ui.toml");
    std::fs::create_dir_all(&ui_toml).unwrap();

    // Off -> on, with the save failing: the entry must not be created.
    let e = err(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "autostart": true, "appearance": dark } }),
    )
    .await;
    assert_eq!(e.code, "io");
    assert!(!desktop.exists(), "autostart left on after an error");
    assert_eq!(
        ok(&ui, json!({ "cmd": "prefs_get" })).await["autostart"],
        json!(false)
    );

    // On -> off, with the save failing: the entry must stay.
    ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "autostart": true } }),
    )
    .await;
    assert!(desktop.is_file());
    let e = err(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "autostart": false, "appearance": light } }),
    )
    .await;
    assert_eq!(e.code, "io");
    assert!(desktop.is_file(), "autostart turned off despite the error");

    // Only autostart asked: no save, so no error and the change stands.
    ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "autostart": false } }),
    )
    .await;
    assert!(!desktop.exists());
    ui.shutdown().await;
}
