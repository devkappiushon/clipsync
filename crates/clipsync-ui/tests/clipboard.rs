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
    assert_eq!(mode["mode"], "manual");
    assert_eq!(mode["restart_needed"], false);
    assert_eq!(mode["observed"]["paused"], true);
    // Retomar é uma ação explícita, separada da troca de modo.
    ok(&ui_b, json!({"cmd":"pause_set","args":{"paused":false}})).await;
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

#[tokio::test(flavor = "multi_thread")]
async fn b09_pausa_nao_persistida_bloqueia_modo_e_reparo_reinicia_pausado() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(dir.path(), "b09", BOTH);
    let ui = start(dir.path(), "b09").await;
    ok(&ui, json!({"cmd":"lock_to_screen","args":{"locked":true}})).await;
    p.set_paused(false).unwrap();
    let state_lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(p.dir.join("state.toml.lock"))
        .unwrap();
    state_lock.lock().unwrap();
    let before = std::fs::read(p.config_path()).unwrap();
    let pause = err(&ui, json!({"cmd":"pause_set","args":{"paused":true}})).await;
    assert_eq!(pause.code, "io");
    let mode = err(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    let details = mode.details.unwrap();
    assert_eq!(details["effects"]["config_saved"], false);
    assert_eq!(details["effects"]["stop"], "not_requested");
    assert_eq!(details["observed"]["paused"], true);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    assert!(!p.load_state().unwrap().paused);
    let state = app_state(&ui).await;
    assert!(state["pause_warning"].as_str().unwrap().contains("pausa"));
    state_lock.unlock().unwrap();
    let mode = ok(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    assert_eq!(mode["effects"]["start"], "owned_ready");
    assert_eq!(mode["observed"]["paused"], true);
    assert_eq!(mode["observed"]["screen_locked"], true);
    assert!(app_state(&ui).await["pause_warning"].is_null());
    assert!(p.load_state().unwrap().paused);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn b09_noop_nao_pausa_e_parado_nao_inicia() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(dir.path(), "b09", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "b09").await;
    let before = std::fs::read(p.config_path()).unwrap();
    let r = ok(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"auto"}}),
    )
    .await;
    assert_eq!(r["effects"]["pause"], "not_requested");
    assert_eq!(r["effects"]["config_saved"], false);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    assert!(!p.state_path().exists());
    ok(&ui, json!({"cmd":"agent_stop"})).await;
    let r = ok(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    assert_eq!(r["observed"]["agent"], "stopped");
    assert_eq!(r["effects"]["start"], "not_requested");
    assert!(!p.state_path().exists());
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn b09_erro_de_retomada_nao_afirma_pausa_e_poll_nao_limpa_aviso() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(dir.path(), "b09", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "b09").await;
    ok(&ui, json!({"cmd":"pause_set","args":{"paused":true}})).await;
    let state_lock = std::fs::OpenOptions::new()
        .write(true)
        .open(p.dir.join("state.toml.lock"))
        .unwrap();
    state_lock.lock().unwrap();
    err(&ui, json!({"cmd":"pause_set","args":{"paused":false}})).await;
    for _ in 0..2 {
        let s = app_state(&ui).await;
        assert_eq!(s["paused"], false);
        assert!(s["pause_warning"].as_str().unwrap().contains("retomada"));
    }
    state_lock.unlock().unwrap();
    ok(&ui, json!({"cmd":"pause_set","args":{"paused":false}})).await;
    assert!(app_state(&ui).await["pause_warning"].is_null());
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn b09_externo_so_salva_configuracao_sem_mutar_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(dir.path(), "b09", CLIPBOARD_ONLY);
    let mutations = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = mutations.clone();
    let other = fake_agent(&p, move |_| {
        seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        clipsync_proto::ControlResponse::Error {
            message: "comando inesperado".into(),
        }
    })
    .await;
    let ui = start(dir.path(), "b09").await;
    let reply = ok(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    assert_eq!(reply["restart_needed"], true);
    assert_eq!(reply["observed"]["agent"], "external");
    assert_eq!(reply["effects"]["config_saved"], true);
    assert_eq!(reply["effects"]["pause"], "not_requested");
    assert_eq!(reply["effects"]["start"], "not_requested");
    assert_eq!(mutations.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(!p.state_path().exists());
    ui.shutdown().await;
    other.stop().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn b09_desconhecido_nao_salva_modo() {
    use tokio::io::AsyncWriteExt;
    let dir = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(dir.path(), "b09", CLIPBOARD_ONLY);
    let listener = tokio::net::UnixListener::bind(p.socket_path()).unwrap();
    let server = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let _ = stream.write_all(&[0, 0, 0, 3, b'x', b'x', b'x']).await;
        }
    });
    let ui = start(dir.path(), "b09").await;
    let before = std::fs::read(p.config_path()).unwrap();
    let error = err(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    let details = error.details.unwrap();
    assert_eq!(details["observed"]["agent"], "unknown");
    assert_eq!(details["effects"]["config_saved"], false);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    ui.shutdown().await;
    server.abort();
    let _ = server.await;
}
