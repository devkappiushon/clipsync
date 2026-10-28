//! 4 · Layout de telas no Windows (§13.11 item 2h): o input existe nesta versão
//! (`clipsync_input::SUPPORTED`), então com o módulo ligado e adaptadores falsos os comandos da
//! tela funcionam, e `input_off` só aparece com o módulo desligado. Os monitores aqui são o
//! `fake-0` de 1920x1080 do adaptador falso; os reais saem do `known_zones()` do adaptador do
//! Windows. O resto da tela é testado em `layout.rs`. Owned by Vector.
#![cfg(windows)]
// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_proto::PeerId;
use common::*;
use serde_json::{Value, json};

fn set_device(peer: &PeerId, x: i64, y: i64, base_rev: u64) -> Value {
    json!({ "cmd": "layout_set_device", "args": { "peer_id": peer.to_hex(), "x": x, "y": y, "hotkey": null, "base_rev": base_rev } })
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_input_module_on_the_commands_of_the_screen_work() {
    let dir = tempfile::tempdir().unwrap();
    let (_, me) = init_profile(dir.path(), "a", BOTH);
    let ui = start(dir.path(), "a").await;

    let features = app_state(&ui).await["features"].clone();
    assert_eq!(features["input"], json!(true), "{features}");
    assert_eq!(features["layout_links"], json!(true), "{features}");

    // this machine is listed with the monitors the adapter reports, read without any dialog
    let l = ok(&ui, json!({ "cmd": "layout_get" })).await;
    assert_eq!(l["self_id"], json!(me.to_hex()));
    assert_eq!(l["snap_px"], json!(16));
    let mine = l["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["is_self"] == json!(true))
        .expect("this device");
    assert_eq!(mine["monitors"][0]["ref"], json!("fake-0"), "{mine}");

    // moving the device snaps to the grid and writes layout.toml
    let r = ok(&ui, set_device(&me, 37, -9, 0)).await;
    assert_eq!(r["layout_rev"], json!(1));
    assert_eq!(
        (r["device"]["x"].clone(), r["device"]["y"].clone()),
        (json!(32), json!(-16))
    );
    assert!(dir.path().join("a").join("layout.toml").exists());

    // options and the lock go through the agent
    let r = ok(
        &ui,
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 4, "base_rev": 1 } }),
    )
    .await;
    assert_eq!(r["layout_rev"], json!(2), "{r}");
    assert_eq!(
        ok(
            &ui,
            json!({ "cmd": "lock_to_screen", "args": { "locked": true } })
        )
        .await,
        json!({ "screen_locked": true })
    );
    assert_eq!(
        ok(&ui, json!({ "cmd": "layout_get" })).await["screen_locked"],
        json!(true)
    );
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn input_off_only_when_the_module_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let (_, other) = init_profile(dir.path(), "b", BOTH);
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let hex = other.to_hex();
    let ep = json!({ "peer_id": hex, "monitor": "m", "edge": "left", "from": 0.0, "to": 1.0 });
    for req in [
        json!({ "cmd": "layout_get" }),
        json!({ "cmd": "layout_set_link", "args": { "id": null, "a": ep, "b": ep, "base_rev": 0 } }),
        json!({ "cmd": "layout_remove_link", "args": { "id": "l1", "base_rev": 0 } }),
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 8, "base_rev": 0 } }),
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
