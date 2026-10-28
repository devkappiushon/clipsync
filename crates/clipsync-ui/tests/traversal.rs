//! S8: estado de saída observado pela API pública, sem confundir permissão de entrada.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use clipsync_proto::PeerPerms;
use common::*;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn no_profile_and_input_disabled_explain_why_output_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "empty").await;
    assert_eq!(app_state(&ui).await["traversal"]["code"], "not_initialized");
    ui.shutdown().await;
    init_profile(dir.path(), "clip", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "clip").await;
    assert_eq!(app_state(&ui).await["traversal"]["code"], "input_off");
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn no_local_link_explains_output_without_demanding_a_link_to_receive() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "receiver", BOTH);
    let ui = start(dir.path(), "receiver").await;
    let t = app_state(&ui).await["traversal"].clone();
    assert_eq!(t["code"], "no_local_link");
    assert_eq!(t["local_links"], 0);
    assert!(t["detail"].as_str().unwrap().contains("mouse físico"));
    assert!(t["detail"].as_str().unwrap().contains("receber"));
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_outgoing_link_does_not_require_granting_control_of_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let (a, ida) = init_profile(dir.path(), "source", BOTH);
    let (b, idb) = init_profile(dir.path(), "dest", BOTH);
    pair_files(
        &a,
        &b,
        NONE,
        PeerPerms {
            input_in: true,
            ..NONE
        },
    );
    let ui_a = start(dir.path(), "source").await;
    let ui_b = start(dir.path(), "dest").await;
    wait_until("dest monitors", || async {
        let l = ok(&ui_a, json!({"cmd":"layout_get"})).await;
        l["devices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["peer_id"] == idb.to_hex() && d["monitors_source"] == "live")
    })
    .await;
    let endpoint = |id: String, edge: &str| json!({"peer_id":id,"monitor":"fake-0","edge":edge,"from":0.0,"to":1.0});
    ok(&ui_a, json!({"cmd":"layout_set_link","args":{"id":null,"a":endpoint(ida.to_hex(),"right"),"b":endpoint(idb.to_hex(),"left"),"base_rev":0}})).await;
    let t = app_state(&ui_a).await["traversal"].clone();
    assert_eq!(t["code"], "unverified");
    assert_eq!(t["local_links"], 1);
    assert_eq!(t["peers"][0]["code"], "unverified");
    assert_eq!(t["peers"][0]["protocol_warning"], Value::Null);
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_and_manual_lock_emit_updated_traversal_and_stopped_agent_is_distinct() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "source", BOTH);
    let ui = start(dir.path(), "source").await;
    let mut events = ui.subscribe();
    ok(&ui, json!({"cmd":"pause_set","args":{"paused":true}})).await;
    let ev = next_event(&mut events, "traversal paused", |ev| {
        let value = serde_json::to_value(ev).unwrap();
        value["event"] == "traversal_changed" && value["data"]["traversal"]["code"] == "paused"
    })
    .await;
    assert_eq!(
        serde_json::to_value(ev).unwrap()["data"]["traversal"]["code"],
        "paused"
    );
    ok(&ui, json!({"cmd":"pause_set","args":{"paused":false}})).await;
    ok(&ui, json!({"cmd":"lock_to_screen","args":{"locked":true}})).await;
    assert_eq!(app_state(&ui).await["traversal"]["code"], "screen_locked");
    ok(&ui, json!({"cmd":"agent_stop"})).await;
    assert_eq!(app_state(&ui).await["traversal"]["code"], "agent_stopped");
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_layout_is_not_reported_as_an_empty_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let (profile, _) = init_profile(dir.path(), "source", BOTH);
    let ui = start(dir.path(), "source").await;
    std::fs::write(profile.dir.join("layout.toml"), "not valid [ toml").unwrap();
    assert_eq!(app_state(&ui).await["traversal"]["code"], "layout_error");
    ui.shutdown().await;
}
