//! 5 · Clipboard (§12.9 item 2, fluxo 5): B in manual gets the item (`clip_received` without
//! preview); `clipboard_get` without preview; `clipboard_preview` gives the text;
//! `clipboard_copy_item` applies it on the fake; `clipboard_discard_item` removes it. Owned by
//! Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_proto::PeerPerms;
use clipsync_ui::UiEvent;
use clipsync_ui::events::NotificationKind;
use common::*;
use serde_json::json;

const TEXT: &str = "senha do wifi: correto cavalo bateria grampo";

#[tokio::test(flavor = "multi_thread")]
async fn a_manual_box_receives_previews_applies_and_discards() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pb, idb) = init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    // a may send its clipboard to b; b accepts clipboard from a.
    let a_grants = PeerPerms {
        clipboard_out: true,
        ..NONE
    };
    let b_grants = PeerPerms {
        clipboard_in: true,
        ..NONE
    };
    pair_files(&pa, &pb, a_grants, b_grants);
    let ui_a = start(dir.path(), "a").await;
    let ui_b = start(dir.path(), "b").await;

    let mode = ok(
        &ui_b,
        json!({ "cmd": "clipboard_set_mode", "args": { "mode": "manual" } }),
    )
    .await;
    assert_eq!(mode, json!({ "mode": "manual", "restart_needed": false }));
    assert!(
        std::fs::read_to_string(pb.config_path())
            .unwrap()
            .contains("mode = \"manual\""),
        "written to config.toml"
    );
    let mut events_b = ui_b.subscribe();
    wait_until("a and b connected", || async {
        ok(&ui_a, json!({ "cmd": "devices_list" })).await["devices"][0]["state"]
            == json!("connected")
    })
    .await;

    // The in_app agent restarted: the fake clipboard handle is the new one.
    ui_a.fake_clipboard().unwrap().user_copy(TEXT);
    let ev = next_event(&mut events_b, "clip_received", |e| {
        matches!(e, UiEvent::ClipReceived(_))
    })
    .await;
    let ev_json = serde_json::to_string(&ev).unwrap();
    assert!(
        !ev_json.contains(TEXT) && !ev_json.contains("preview"),
        "{ev_json}"
    );
    let UiEvent::ClipReceived(item) = ev else {
        unreachable!()
    };
    assert_eq!((item.from.as_str(), item.bytes), ("a", TEXT.len()));
    let note = next_event(
        &mut events_b,
        "notification",
        |e| matches!(e, UiEvent::Notification(n) if n.kind == NotificationKind::ItemReceived),
    )
    .await;
    assert!(!serde_json::to_string(&note).unwrap().contains(TEXT));

    let state = ok(&ui_b, json!({ "cmd": "clipboard_get" })).await;
    let state_json = state.to_string();
    assert!(!state_json.contains(TEXT) && !state_json.contains("preview"));
    assert_eq!(state["enabled"], json!(true));
    assert_eq!(state["mode"], json!("manual"));
    assert_eq!(state["adapter"], json!("fake"));
    assert_eq!(state["classification"], json!("test"));
    assert_eq!(
        state["retention"],
        json!({ "max_items": 50, "ttl_ms": 600000 })
    );
    let listed = &state["inbox"][0];
    assert_eq!(listed["id"], json!(item.id));
    assert_eq!(listed["state"], json!("in_inbox"));
    assert_eq!(listed["reason"], json!(null));
    assert_eq!(item.state, clipsync_ui::clipboard::ItemState::InInbox);

    let id = json!({ "id": item.id });
    assert_eq!(
        ok(&ui_b, json!({ "cmd": "clipboard_preview", "args": id })).await,
        json!({ "preview": TEXT })
    );
    let fake_b = ui_b.fake_clipboard().unwrap();
    assert_ne!(
        fake_b.contents().as_deref(),
        Some(TEXT),
        "manual: not applied yet"
    );
    assert_eq!(
        ok(&ui_b, json!({ "cmd": "clipboard_copy_item", "args": id })).await,
        json!({ "state": "applied" })
    );
    assert_eq!(fake_b.contents().as_deref(), Some(TEXT));
    let e = err(&ui_b, json!({ "cmd": "clipboard_copy_item", "args": id })).await;
    assert_eq!(e.code, "wrong_state", "applied items have no text left");

    assert_eq!(
        ok(
            &ui_b,
            json!({ "cmd": "clipboard_discard_item", "args": id })
        )
        .await,
        json!({ "removed": true })
    );
    assert_eq!(
        ok(&ui_b, json!({ "cmd": "clipboard_get" })).await["inbox"],
        json!([])
    );
    let e = err(
        &ui_b,
        json!({ "cmd": "clipboard_discard_item", "args": id }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("no_item", Some("id"))
    );
    let e = err(
        &ui_b,
        json!({ "cmd": "clipboard_preview", "args": { "id": "xyz" } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_request", Some("id"))
    );

    // "Enviar agora": what a holds now goes to b, by id.
    let sent = ok(
        &ui_a,
        json!({ "cmd": "clipboard_send_now", "args": { "peer_id": null } }),
    )
    .await;
    let o = &sent["outcomes"][0];
    assert_eq!(
        (o["peer_id"].clone(), o["name"].clone()),
        (json!(idb.to_hex()), json!("b"))
    );
    assert_eq!(o["state"], json!("in_inbox"));
    ui_a.fake_clipboard().unwrap().clear();
    let e = err(&ui_a, json!({ "cmd": "clipboard_send_now", "args": {} })).await;
    assert_eq!(e.code, "empty_clipboard");

    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn with_the_module_off_the_screen_still_answers() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", INPUT_ONLY);
    let ui = start(dir.path(), "a").await;
    let s = ok(&ui, json!({ "cmd": "clipboard_get" })).await;
    assert_eq!(s["enabled"], json!(false));
    assert_eq!(s["classification"], json!("unavailable"));
    assert_eq!(s["inbox"], json!([]));
    let e = err(
        &ui,
        json!({ "cmd": "clipboard_set_mode", "args": { "mode": "auto" } }),
    )
    .await;
    assert_eq!(e.code, "module_off");
    ui.shutdown().await;
}
