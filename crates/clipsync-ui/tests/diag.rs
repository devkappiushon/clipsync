//! 6 · Diagnóstico (§12.9 item 2, fluxo 6): `support_bundle_preview` has no name, id (64 hex or
//! short) or IP of the peers, and not this device's name; the export writes the same kind of
//! text to the downloads folder. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_core::control;
use clipsync_proto::{ControlRequest, ModuleCommand, ModuleKind, PeerPerms};
use common::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn the_support_bundle_carries_no_identifier() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "pc-da-ana", CLIPBOARD_ONLY);
    let (pb, idb) = init_profile(dir.path(), "notebook-do-bruno", CLIPBOARD_ONLY);
    // b may send, a does not accept: a records a block naming b.
    let b_grants = PeerPerms {
        clipboard_out: true,
        ..NONE
    };
    pair_files(&pa, &pb, NONE, b_grants);
    let ui_a = start(dir.path(), "pc-da-ana").await;
    let ui_b = start(dir.path(), "notebook-do-bruno").await;
    wait_until("a and b connected", || async {
        ok(&ui_a, json!({ "cmd": "devices_list" })).await["devices"][0]["state"]
            == json!("connected")
    })
    .await;
    let send = ControlRequest::Module {
        kind: ModuleKind::Clipboard,
        cmd: ModuleCommand::ClipSend {
            text: "conteúdo que nunca entra no pacote".into(),
            to: Some(ida),
        },
    };
    control::request(&pb, send).await.unwrap();
    wait_until("a has a block", || async {
        !ok(&ui_a, json!({ "cmd": "diagnostics_get" })).await["blocks"]
            .as_array()
            .unwrap()
            .is_empty()
    })
    .await;

    let d = ok(&ui_a, json!({ "cmd": "diagnostics_get" })).await;
    assert!(!d["items"].as_array().unwrap().is_empty());
    assert!(
        d["network"]["listen"]
            .as_str()
            .unwrap()
            .starts_with("127.0.0.1:")
    );
    assert_eq!(d["network"]["peers"][0]["peer_id"], json!(idb.to_hex()));
    assert_eq!(d["network"]["peers"][0]["state"], json!("connected"));
    let b = &d["blocks"][0];
    assert!(b["reason"].as_str().is_some() && b["at_unix_ms"].as_u64().is_some());

    let p = ok(&ui_a, json!({ "cmd": "support_bundle_preview" })).await;
    let text = p["text"].as_str().unwrap();
    let secrets = [
        "notebook-do-bruno".to_string(),
        idb.to_hex(),
        idb.short(),
        "127.0.0.1".to_string(),
        "pc-da-ana".to_string(),
        ida.to_hex(),
        "conteúdo que nunca entra".to_string(),
    ];
    for s in &secrets {
        assert!(!text.contains(s.as_str()), "{s} leaked:\n{text}");
    }
    assert!(text.contains("<par-1>") && text.contains("<ip>"), "{text}");
    let count = |kind: &str| {
        p["redactions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == json!(kind))
            .map_or(0, |r| r["count"].as_u64().unwrap())
    };
    // This device's name is never written; its count may be 0, the others are not.
    for kind in ["peer_id", "peer_name", "ip"] {
        assert!(count(kind) > 0, "{kind}: {}", p["redactions"]);
    }

    let x = ok(&ui_a, json!({ "cmd": "support_bundle_export" })).await;
    let path = std::path::PathBuf::from(x["path"].as_str().unwrap());
    assert_eq!(
        path.parent().unwrap(),
        dir.path().join("pc-da-ana-downloads")
    );
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("clipsync-suporte-") && name.ends_with(".txt") && name.len() == 36,
        "{name}"
    );
    let file = std::fs::read_to_string(&path).unwrap();
    assert_eq!(x["bytes"].as_u64().unwrap() as usize, file.len());
    for s in &secrets {
        assert!(!file.contains(s.as_str()), "{s} leaked in the file");
    }
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

/// S7 (§13.9): "layout do teclado" comes from the input module's `InputState`, and says
/// "agente parado" with the agent stopped; on Windows "atalhos" too, inactive without a link of
/// this machine in layout.toml (the hooks only exist then, §13.2 rule 2a). Exact texts and the
/// active case: the unit tests of `diag.rs`.
#[tokio::test(flavor = "multi_thread")]
async fn diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", BOTH);
    let ui = start(dir.path(), "a").await;
    let item = |d: &serde_json::Value, name: &str| {
        d["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["area"] == json!("input") && i["name"] == json!(name))
            .cloned()
    };
    wait_until("input answers", || async {
        app_state(&ui).await["features"]["input"] == json!(true)
    })
    .await;
    let d = ok(&ui, json!({ "cmd": "diagnostics_get" })).await;
    let keyboard = item(&d, "layout do teclado").expect("keyboard layout item");
    assert_ne!(keyboard["detail"], json!("agente parado"), "{keyboard}");
    let shortcuts = item(&d, "atalhos");
    if cfg!(windows) {
        assert_eq!(
            shortcuts,
            Some(json!({
                "area": "input", "name": "atalhos", "status": "warn",
                "detail": "inativos: nenhuma ligação desta máquina no layout"
            }))
        );
    } else {
        assert_eq!(shortcuts, None, "no shortcuts on Linux in this version");
    }

    ok(&ui, json!({ "cmd": "agent_stop" })).await;
    let d = ok(&ui, json!({ "cmd": "diagnostics_get" })).await;
    assert_eq!(
        item(&d, "layout do teclado").unwrap()["detail"],
        json!("agente parado")
    );
    if cfg!(windows) {
        assert_eq!(
            item(&d, "atalhos").unwrap()["detail"],
            json!("inativos: agente parado")
        );
    }
    ui.shutdown().await;
}
