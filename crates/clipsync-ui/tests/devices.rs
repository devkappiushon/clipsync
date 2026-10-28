//! 2 · Dispositivos (§12.9 item 2, fluxo 2): `input_in` without confirmation is
//! `needs_confirmation`; with it `peers.toml` is written and the running agent applies it (the
//! peer's clipboard now gets through); `device_revoke` with the wrong name is `name_mismatch`,
//! with the right one the peer leaves the status and the list on both sides: the revoked side,
//! told by `Bye{Revoked}` (v1.16), removes the revoker itself within 3 s, with one
//! `device_removed` notification. Also `peer_connected` within 3 s. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::time::Duration;

use clipsync_core::{Profile, control};
use clipsync_proto::{
    ControlRequest, ControlResponse, DeliveryState, ModuleCommand, ModuleKind, ModuleReply, PeerId,
    PeerPerms, PeerState,
};
use clipsync_ui::UiEvent;
use clipsync_ui::events::NotificationKind;
use common::*;
use serde_json::json;

async fn peer_state(p: &Profile, id: PeerId) -> Option<(PeerState, PeerPerms)> {
    match control::request(p, ControlRequest::Status).await.unwrap() {
        ControlResponse::Status(s) => s
            .peers
            .into_iter()
            .find(|x| x.id == id)
            .map(|x| (x.state, x.perms)),
        other => panic!("{other:?}"),
    }
}

/// `from` sends a text to `to` through its agent, like `clipsync send`.
async fn send(from: &Profile, to: PeerId, text: &str) -> DeliveryState {
    let req = ControlRequest::Module {
        kind: ModuleKind::Clipboard,
        cmd: ModuleCommand::ClipSend {
            text: text.into(),
            to: Some(to),
        },
    };
    match control::request(from, req).await.unwrap() {
        ControlResponse::Module(ModuleReply::ClipSent(o)) => o[0].state,
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn perms_need_full_trust_for_input_and_revoke_needs_the_typed_name() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    // b may send its clipboard to a; a grants nothing yet.
    let b_grants = PeerPerms {
        clipboard_out: true,
        ..NONE
    };
    pair_files(&pa, &pb, NONE, b_grants);

    // a is up (and looked at its mesh) before b exists, so b's arrival is a change it reports.
    let ui_a = start(dir.path(), "a").await;
    let mut events_a = ui_a.subscribe();
    let began = tokio::time::Instant::now();
    let ui_b = start(dir.path(), "b").await;
    let mut events_b = ui_b.subscribe();
    let ev = next_event(
        &mut events_a,
        "peer_connected b",
        |e| matches!(e, UiEvent::PeerConnected(p) if p.peer_id == idb.to_hex()),
    )
    .await;
    assert!(
        began.elapsed() <= Duration::from_secs(3),
        "peer_connected took {:?}",
        began.elapsed()
    );
    let UiEvent::PeerConnected(p) = ev else {
        unreachable!()
    };
    assert_eq!((p.name.as_str(), p.short.clone()), ("b", idb.short()));

    let list = ok(&ui_a, json!({ "cmd": "devices_list" })).await;
    assert_eq!(list["agent_running"], json!(true));
    let d = &list["devices"][0];
    assert_eq!(d["peer_id"], json!(idb.to_hex()));
    assert_eq!(d["state"], json!("connected"));
    assert_eq!(
        d["perms"],
        json!({ "clipboard_in": false, "clipboard_out": false, "input_in": false })
    );
    assert_eq!(
        d["remote_modules"],
        json!({ "clipboard": true, "input": false })
    );
    assert_eq!(d["remote_version"], json!(2));
    assert_eq!(d["blocked_reason"], json!(null));
    assert!(d["addr"].as_str().unwrap().starts_with("127.0.0.1:"));

    // Before a grants clipboard_in, b's clipboard is refused by a.
    assert!(matches!(
        send(&pb, ida, "antes").await,
        DeliveryState::Rejected(_)
    ));

    let set = |ack: bool| {
        json!({ "cmd": "device_set_perms", "args": {
            "peer_id": idb.to_hex(),
            "perms": { "clipboard_in": true, "clipboard_out": false, "input_in": true },
            "acknowledge_full_trust": ack,
        } })
    };
    let e = err(&ui_a, set(false)).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("needs_confirmation", Some("acknowledge_full_trust"))
    );
    assert!(
        !pa.load_peers().unwrap().peer[0].perms.input_in,
        "nothing saved"
    );

    let r = ok(&ui_a, set(true)).await;
    assert_eq!(r["device"]["perms"]["input_in"], json!(true));
    let saved = pa.load_peers().unwrap().peer[0].perms;
    assert!(saved.clipboard_in && saved.input_in && !saved.clipboard_out);
    // The running agent applied it at once (ReloadPeers), and b's clipboard now gets through.
    let (_, live) = peer_state(&pa, idb).await.unwrap();
    assert!(live.clipboard_in && live.input_in);
    assert!(matches!(
        send(&pb, ida, "depois").await,
        DeliveryState::Applied | DeliveryState::InInbox
    ));

    let e = err(
        &ui_a,
        json!({ "cmd": "device_set_perms", "args": {
            "peer_id": "ab12-cd34",
            "perms": { "clipboard_in": false, "clipboard_out": false, "input_in": false },
        } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_request", Some("peer_id"))
    );
    let e = err(
        &ui_a,
        json!({ "cmd": "device_set_perms", "args": {
            "peer_id": "cd".repeat(32),
            "perms": { "clipboard_in": false, "clipboard_out": false, "input_in": false },
        } }),
    )
    .await;
    assert_eq!(e.code, "unknown_peer");
    let e = err(
        &ui_a,
        json!({ "cmd": "device_pause", "args": { "peer_id": idb.to_hex(), "paused": true } }),
    )
    .await;
    assert_eq!(e.code, "unavailable");

    // b's poll must have seen a connected, so that a leaving is a change it reports.
    wait_until("b sees a connected", || async {
        ok(&ui_b, json!({ "cmd": "devices_list" })).await["devices"][0]["state"]
            == json!("connected")
    })
    .await;
    tokio::time::sleep(POLL * 3).await;

    let revoke = |name: &str| json!({ "cmd": "device_revoke", "args": { "peer_id": idb.to_hex(), "typed_name": name } });
    let e = err(&ui_a, revoke("B")).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("name_mismatch", Some("typed_name"))
    );
    assert_eq!(pa.load_peers().unwrap().peer.len(), 1);
    assert_eq!(ok(&ui_a, revoke("b")).await, json!({ "removed": true }));
    assert!(pa.load_peers().unwrap().peer.is_empty());
    assert!(peer_state(&pa, idb).await.is_none(), "gone from a's status");
    assert_eq!(
        ok(&ui_a, json!({ "cmd": "devices_list" })).await["devices"],
        json!([])
    );
    next_event(
        &mut events_a,
        "peer_disconnected b",
        |e| matches!(e, UiEvent::PeerDisconnected(p) if p.peer_id == idb.to_hex()),
    )
    .await;
    // b was told: it drops a from its own peers.toml, status and list within 3 s.
    let told = tokio::time::Instant::now();
    wait_until("b drops a", || async {
        pb.load_peers().unwrap().get(&ida).is_none()
    })
    .await;
    assert!(
        told.elapsed() <= Duration::from_secs(3),
        "{:?}",
        told.elapsed()
    );
    assert!(peer_state(&pb, ida).await.is_none(), "gone from b's status");
    assert_eq!(
        ok(&ui_b, json!({ "cmd": "devices_list" })).await["devices"],
        json!([])
    );
    next_event(
        &mut events_b,
        "peer_disconnected a",
        |e| matches!(e, UiEvent::PeerDisconnected(p) if p.peer_id == ida.to_hex()),
    )
    .await;
    // One notification, and no peer_blocked with it.
    let mut removed = Vec::new();
    let quiet = tokio::time::Instant::now() + POLL * 10;
    while let Ok(Ok(ev)) = tokio::time::timeout_at(quiet, events_b.recv()).await {
        match ev {
            UiEvent::Notification(n) => removed.push(n),
            UiEvent::PeerBlocked(b) => panic!("removal also gave peer_blocked: {b:?}"),
            _ => {}
        }
    }
    assert_eq!(removed.len(), 1, "{removed:?}");
    assert_eq!(removed[0].kind, NotificationKind::DeviceRemoved);
    assert_eq!(removed[0].title, "Dispositivo removido");
    assert_eq!(removed[0].body, "a removeu este dispositivo");
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_agent_the_list_comes_from_peers_toml_as_offline() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pb, idb) = init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    pair_files(&pa, &pb, NONE, NONE);
    let ui = start(dir.path(), "a").await;
    ok(&ui, json!({ "cmd": "agent_stop" })).await;
    let list = ok(&ui, json!({ "cmd": "devices_list" })).await;
    assert_eq!(list["agent_running"], json!(false));
    let d = &list["devices"][0];
    assert_eq!(d["peer_id"], json!(idb.to_hex()));
    assert_eq!(d["state"], json!("offline"));
    assert_eq!(d["remote_version"], json!(null));
    assert_eq!(d["paired_at"], json!("2026-10-28T12:00:00Z"));
    // Permissions still change with the agent stopped: peers.toml is the source.
    ok(
        &ui,
        json!({ "cmd": "device_set_perms", "args": {
            "peer_id": idb.to_hex(),
            "perms": { "clipboard_in": true, "clipboard_out": true, "input_in": false },
        } }),
    )
    .await;
    assert!(pa.load_peers().unwrap().peer[0].perms.clipboard_out);
    ui.shutdown().await;
}
