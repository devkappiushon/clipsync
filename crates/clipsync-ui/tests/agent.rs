//! Common commands (§12.5): `app_state`, `agent_start`, `agent_stop`, `pause_set`, the request
//! shape, and the in_app / external / failed modes (§12.9 item 2, "também"). Real agents in temp
//! profiles on 127.0.0.1 with fake adapters. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::UdpSocket;

use clipsync_core::agent;
use clipsync_proto::Module;
use clipsync_ui::UiEvent;
use clipsync_ui::events::{AgentStateData, PauseChanged};
use clipsync_ui::status::AgentMode;
use common::*;
use serde_json::json;
use tokio::sync::oneshot;

#[tokio::test(flavor = "multi_thread")]
async fn bad_json_is_bad_request_and_unknown_cmd_is_unknown_command() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "a").await;
    for bad in [
        json!("app_state"),
        json!({}),
        json!({ "cmd": 7 }),
        json!({ "cmd": "pause_set", "args": { "paused": "sim" } }),
        json!({ "cmd": "pause_set", "args": { "paused": true, "extra": 1 } }),
        json!({ "cmd": "app_state", "args": { "x": 1 } }),
    ] {
        let e = err(&ui, bad.clone()).await;
        assert_eq!(e.code, "bad_request", "{bad}: {e:?}");
        assert!(!e.message.is_empty());
    }
    let e = err(&ui, json!({ "cmd": "format_disk" })).await;
    assert_eq!(e.code, "unknown_command");
    // The serialized error has the four fields, none omitted.
    let v = serde_json::to_value(&e).unwrap();
    for k in ["code", "message", "field", "details"] {
        assert!(v.get(k).is_some(), "{k} missing in {v}");
    }
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_profile_nothing_starts_and_commands_say_not_initialized() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "novo").await;
    let s = app_state(&ui).await;
    assert_eq!(s["initialized"], json!(false));
    assert_eq!(s["agent"], json!("stopped"));
    assert_eq!(s["device"], json!(null));
    assert_eq!(s["agent_error"], json!(null), "null, never omitted");
    assert_eq!(s["features"]["input"], json!(false));
    assert_eq!(s["features"]["peer_pause"], json!(false));
    // S6-13: system notifications exist, so the preference switches are live.
    assert_eq!(s["features"]["os_notifications"], json!(true));
    for req in [
        json!({ "cmd": "agent_start" }),
        json!({ "cmd": "pause_set", "args": { "paused": true } }),
    ] {
        assert_eq!(err(&ui, req).await.code, "not_initialized");
    }
    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_stop" })).await,
        json!({ "agent": "stopped" })
    );
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_app_agent_starts_pauses_stops_and_starts_again() {
    let dir = tempfile::tempdir().unwrap();
    let (_, id) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let mut events = ui.subscribe();
    let s = app_state(&ui).await;
    assert_eq!(s["initialized"], json!(true));
    assert_eq!(s["agent"], json!("in_app"), "{s}");
    assert_eq!(s["device"]["peer_id"], json!(id.to_hex()));
    assert_eq!(s["device"]["short"], json!(id.short()));
    assert_eq!(s["device"]["name"], json!("a"));
    assert_eq!(s["modules"], json!({ "clipboard": true, "input": false }));
    assert_eq!(s["mesh"], json!({ "connected": 0, "total": 0 }));
    assert_eq!(s["paused"], json!(false));
    assert_eq!(s["version"], json!(env!("CARGO_PKG_VERSION")));
    let platform = if cfg!(windows) { "windows" } else { "linux" };
    assert_eq!(s["platform"], json!(platform));

    assert_eq!(
        ok(
            &ui,
            json!({ "cmd": "pause_set", "args": { "paused": true } })
        )
        .await,
        json!({ "paused": true })
    );
    next_event(&mut events, "pause_changed true", |e| {
        *e == UiEvent::PauseChanged(PauseChanged { paused: true })
    })
    .await;
    assert_eq!(app_state(&ui).await["paused"], json!(true));

    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_stop" })).await,
        json!({ "agent": "stopped" })
    );
    next_event(&mut events, "agent_state stopped", |e| {
        *e == UiEvent::AgentState(AgentStateData {
            agent: AgentMode::Stopped,
            error: None,
        })
    })
    .await;
    let s = app_state(&ui).await;
    assert_eq!(s["agent"], json!("stopped"));
    // Stopped: the pause comes from state.toml, the device from the profile.
    assert_eq!(s["paused"], json!(true));
    assert_eq!(s["device"]["peer_id"], json!(id.to_hex()));
    assert_eq!(
        err(
            &ui,
            json!({ "cmd": "pause_set", "args": { "paused": false } })
        )
        .await
        .code,
        "agent_not_running"
    );

    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_start" })).await,
        json!({ "agent": "in_app" })
    );
    // Starting twice is not an error.
    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_start" })).await,
        json!({ "agent": "in_app" })
    );
    next_event(
        &mut events,
        "agent_state in_app",
        |e| matches!(e, UiEvent::AgentState(d) if d.agent == AgentMode::InApp),
    )
    .await;
    assert_eq!(
        app_state(&ui).await["paused"],
        json!(true),
        "survives a restart"
    );
    ok(
        &ui,
        json!({ "cmd": "pause_set", "args": { "paused": false } }),
    )
    .await;
    next_event(&mut events, "pause_changed false", |e| {
        *e == UiEvent::PauseChanged(PauseChanged { paused: false })
    })
    .await;
    ui.shutdown().await;
    assert!(
        clipsync_core::control::request(
            &clipsync_core::Profile {
                dir: dir.path().join("a")
            },
            clipsync_proto::ControlRequest::Status
        )
        .await
        .is_err(),
        "shutdown stops the in_app agent"
    );
}

/// §12.9 item 2: with another `agent::run` on the same profile, `app_state` says `external` and
/// `agent_stop` says `agent_external`; the app never starts a second agent.
#[tokio::test(flavor = "multi_thread")]
async fn an_agent_already_on_the_profile_makes_the_app_a_client() {
    let dir = tempfile::tempdir().unwrap();
    let (profile, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (fake, _handle) = clipsync_clipboard::FakeClipboard::new();
    let modules: Vec<Box<dyn Module>> = vec![clipsync_clipboard::build_with_adapter(
        clipsync_clipboard::ClipboardConfig::default(),
        Box::new(fake),
    )];
    let (stop, stop_rx) = oneshot::channel::<()>();
    let p = profile.clone();
    let terminal = tokio::spawn(async move {
        agent::run(&p, modules, async move {
            let _ = stop_rx.await;
        })
        .await
    });
    wait_until("terminal agent up", || async {
        clipsync_core::control::request(&profile, clipsync_proto::ControlRequest::Status)
            .await
            .is_ok()
    })
    .await;

    let ui = start(dir.path(), "a").await;
    let mut events = ui.subscribe();
    assert_eq!(app_state(&ui).await["agent"], json!("external"));
    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_start" })).await,
        json!({ "agent": "external" })
    );
    assert_eq!(
        err(&ui, json!({ "cmd": "agent_stop" })).await.code,
        "agent_external"
    );
    // Commands still reach it: the UI is a client of the terminal's agent.
    ok(
        &ui,
        json!({ "cmd": "pause_set", "args": { "paused": true } }),
    )
    .await;
    next_event(&mut events, "pause_changed from the external agent", |e| {
        *e == UiEvent::PauseChanged(PauseChanged { paused: true })
    })
    .await;

    let _ = stop.send(());
    terminal.await.unwrap().unwrap();
    next_event(
        &mut events,
        "agent_state stopped",
        |e| matches!(e, UiEvent::AgentState(d) if d.agent == AgentMode::Stopped),
    )
    .await;
    ui.shutdown().await;
}

/// A start that fails (port taken) leaves the app up, with `agent: "failed"` and the reason.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_start_is_reported_in_app_state() {
    let dir = tempfile::tempdir().unwrap();
    let (profile, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let taken = UdpSocket::bind(profile.load_config().unwrap().listen).unwrap();
    let ui = start(dir.path(), "a").await;
    let s = app_state(&ui).await;
    assert_eq!(s["agent"], json!("failed"), "{s}");
    let why = s["agent_error"].as_str().unwrap();
    assert!(why.starts_with("o agente não subiu"), "{why}");
    let e = err(&ui, json!({ "cmd": "agent_start" })).await;
    assert_eq!(e.code, "io", "{e:?}");
    drop(taken);
    assert_eq!(
        ok(&ui, json!({ "cmd": "agent_start" })).await,
        json!({ "agent": "in_app" })
    );
    assert_eq!(app_state(&ui).await["agent_error"], json!(null));
    ui.shutdown().await;
}

/// §13.11 item 4: the shortcuts work on Windows with the input module on (feature
/// `global_shortcuts`), never on Linux in this version, and never without input.
#[tokio::test(flavor = "multi_thread")]
async fn global_shortcuts_need_windows_and_the_input_module() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", BOTH);
    init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    let ui_a = start(dir.path(), "a").await;
    let ui_b = start(dir.path(), "b").await;
    wait_until("input answers on a", || async {
        app_state(&ui_a).await["features"]["input"] == json!(true)
    })
    .await;
    assert_eq!(app_state(&ui_a).await["input_supported"], json!(true));
    assert_eq!(
        app_state(&ui_a).await["features"]["global_shortcuts"],
        json!(cfg!(windows))
    );
    assert_eq!(
        app_state(&ui_b).await["features"]["global_shortcuts"],
        json!(false)
    );
    // I-S7-2: nothing refused Win+Alt+P here (the fake adapters register nothing).
    assert_eq!(
        app_state(&ui_a).await["features"]["resume_hotkey_refused"],
        json!(false)
    );
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}
