//! 4 · Layout de telas fora do Linux: o input não existe nesta versão (`INPUT_SUPPORTED`), então
//! `layout_get` e todos os comandos da tela 4 respondem `input_off`, mesmo com o módulo ligado
//! no perfil. O resto da tela é testado em `layout.rs`, só no Linux. Owned by Vector.
#![cfg(not(target_os = "linux"))]
// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn every_command_of_the_layout_screen_says_input_off() {
    let dir = tempfile::tempdir().unwrap();
    let (_, other) = init_profile(dir.path(), "b", BOTH);
    // The profile asks for input; the platform cannot give it.
    init_profile(dir.path(), "a", BOTH);
    let ui = start(dir.path(), "a").await;
    let hex = other.to_hex();
    let ep = json!({ "peer_id": hex, "monitor": "m", "edge": "left", "from": 0.0, "to": 1.0 });
    for req in [
        json!({ "cmd": "layout_get" }),
        json!({ "cmd": "layout_set_link", "args": { "id": null, "a": ep, "b": ep, "base_rev": 0 } }),
        json!({ "cmd": "layout_remove_link", "args": { "id": "l1", "base_rev": 0 } }),
        json!({ "cmd": "layout_set_device", "args": { "peer_id": hex, "x": 0, "y": 0, "hotkey": null, "base_rev": 0 } }),
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 8, "base_rev": 0 } }),
        json!({ "cmd": "layout_copy_to_peers", "args": { "peer_ids": null } }),
        json!({ "cmd": "layout_offer_answer", "args": { "accept": true } }),
        json!({ "cmd": "lock_to_screen", "args": { "locked": true } }),
        json!({ "cmd": "switch_to_device", "args": { "peer_id": hex } }),
        json!({ "cmd": "input_test", "args": { "peer_id": hex, "seconds": 5 } }),
    ] {
        let e = err(&ui, req.clone()).await;
        assert_eq!(e.code, "input_off", "{req}: {e:?}");
        assert_eq!(
            e.message,
            "teclado e mouse estão desligados neste dispositivo"
        );
    }
    let features = app_state(&ui).await["features"].clone();
    assert_eq!(features["input"], json!(false), "{features}");
    assert_eq!(features["layout_links"], json!(false), "{features}");
    ui.shutdown().await;
}
