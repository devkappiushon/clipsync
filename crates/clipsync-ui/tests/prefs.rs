//! 7 · Preferências (§12.9 item 2, fluxo 7): `prefs_set` writes `ui.toml` and survives a new
//! `Ui`; an unknown field is `bad_request`. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_ui::UiEvent;
use common::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn prefs_set_writes_ui_toml_and_a_new_ui_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let mut events = ui.subscribe();
    let p = ok(&ui, json!({ "cmd": "prefs_get" })).await;
    assert_eq!(p["autostart"], json!(false), "off by default");
    assert_eq!(
        p["notifications"],
        json!({ "item_received": true, "device_connected": false, "block": true })
    );
    assert_eq!(
        p["appearance"],
        json!({ "theme": "system", "reduce_motion": false })
    );
    assert_eq!(p["shortcuts"]["open"], json!("Win+Alt+C"));
    assert_eq!(p["language"], json!("pt-BR"));
    assert_eq!(p["license"], json!("a definir"));

    let set = ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": {
            "autostart": true,
            "notifications": { "item_received": false, "device_connected": true, "block": true },
            "appearance": { "theme": "dark", "reduce_motion": true },
        } }),
    )
    .await;
    assert_eq!(set["prefs"]["autostart"], json!(true));
    assert_eq!(set["prefs"]["appearance"]["theme"], json!("dark"));
    let ev = next_event(&mut events, "prefs_changed", |e| {
        matches!(e, UiEvent::PrefsChanged(_))
    })
    .await;
    assert_eq!(serde_json::to_value(&ev).unwrap()["data"], set);
    assert!(dir.path().join("a").join("ui.toml").is_file());
    assert!(
        dir.path()
            .join("a-autostart")
            .join(clipsync_ui::autostart::DESKTOP_FILE)
            .is_file()
    );
    ui.shutdown().await;

    let ui = start(dir.path(), "a").await;
    assert_eq!(ok(&ui, json!({ "cmd": "prefs_get" })).await, set["prefs"]);
    // Only what is sent changes.
    let off = ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "autostart": false } }),
    )
    .await;
    assert_eq!(off["prefs"]["autostart"], json!(false));
    assert_eq!(off["prefs"]["appearance"], set["prefs"]["appearance"]);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn prefs_set_with_an_unknown_field_is_bad_request_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "novo").await;
    for args in [
        json!({ "tema": "dark" }),
        json!({ "appearance": { "theme": "dark", "reduce_motion": false, "x": 1 } }),
        json!({ "appearance": { "theme": "roxo", "reduce_motion": false } }),
    ] {
        let e = err(&ui, json!({ "cmd": "prefs_set", "args": args })).await;
        assert_eq!(e.code, "bad_request", "{args}");
    }
    assert!(!dir.path().join("novo").join("ui.toml").exists());
    // Before the onboarding the preferences already work.
    ok(
        &ui,
        json!({ "cmd": "prefs_set", "args": { "appearance": { "theme": "light", "reduce_motion": false } } }),
    )
    .await;
    assert!(dir.path().join("novo").join("ui.toml").is_file());
    ui.shutdown().await;
}
