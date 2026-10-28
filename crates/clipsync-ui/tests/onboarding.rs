//! 1 · Onboarding (§12.9 item 2, fluxo 1): `onboarding_start` and `onboarding_create` give the
//! same fingerprint; an existing profile without `recreate` gives `needs_confirmation`. Owned by
//! Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_proto::{ControlRequest, ControlResponse};
use common::*;
use serde_json::json;

fn create(name: &str, recreate: bool) -> serde_json::Value {
    json!({ "cmd": "onboarding_create", "args": {
        "name": name,
        "modules": { "input": false, "clipboard": true },
        "port": 47800,
        "recreate": recreate,
    } })
}

#[tokio::test(flavor = "multi_thread")]
async fn create_writes_the_identity_shown_by_start_and_starts_the_agent() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "a").await;
    let s = ok(&ui, json!({ "cmd": "onboarding_start" })).await;
    assert_eq!(s["already_initialized"], json!(false));
    assert_eq!(s["defaults"]["clipboard"], json!(false));
    assert_eq!(s["defaults"]["input"], s["input_supported"]);
    assert!(!s["default_name"].as_str().unwrap().is_empty());
    let fp = s["fingerprint_short"].as_str().unwrap().to_owned();
    // Going back to step 1 keeps the same identity.
    let again = ok(&ui, json!({ "cmd": "onboarding_start" })).await;
    assert_eq!(again["fingerprint_short"], json!(fp));

    assert_eq!(
        ok(
            &ui,
            json!({ "cmd": "onboarding_check_name", "args": { "name": "pc-pessoal" } })
        )
        .await,
        json!({ "ok": true })
    );
    let e = err(
        &ui,
        json!({ "cmd": "onboarding_check_name", "args": { "name": "a\u{202e}b" } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("invalid_name", Some("name"))
    );

    let c = ok(&ui, create("pc-pessoal", false)).await;
    assert_eq!(c["device"]["short"], json!(fp));
    assert_eq!(c["device"]["name"], json!("pc-pessoal"));
    assert_eq!(c["modules"], json!({ "input": false, "clipboard": true }));
    assert!(c["listen"].as_str().unwrap().starts_with("127.0.0.1:"));

    let st = app_state(&ui).await;
    assert_eq!(st["initialized"], json!(true));
    assert_eq!(st["agent"], json!("in_app"), "{st}");
    assert_eq!(st["device"]["short"], json!(fp));
    assert_eq!(st["device"]["peer_id"], c["device"]["peer_id"]);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_existing_profile_needs_confirmation_and_recreate_changes_the_identity() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, old) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pb, _) = init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    let all = clipsync_proto::PeerPerms {
        clipboard_in: true,
        clipboard_out: true,
        input_in: true,
    };
    pair_files(&pa, &pb, all, all);
    let ui = start(dir.path(), "a").await;
    let s = ok(&ui, json!({ "cmd": "onboarding_start" })).await;
    assert_eq!(s["already_initialized"], json!(true));
    let fp = s["fingerprint_short"].as_str().unwrap().to_owned();
    assert_ne!(fp, old.short());

    let e = err(&ui, create("a", false)).await;
    assert_eq!(e.code, "needs_confirmation");
    assert!(e.message.contains("parear de novo"), "{}", e.message);
    assert!(e.message.contains("apaga a lista"), "{}", e.message);
    assert_eq!(
        pa.load_peers().unwrap().peer.len(),
        1,
        "nothing changes unconfirmed"
    );
    assert_eq!(app_state(&ui).await["device"]["short"], json!(old.short()));

    let c = ok(&ui, create("a", true)).await;
    assert_eq!(c["device"]["short"], json!(fp));
    let st = app_state(&ui).await;
    assert_eq!(st["agent"], json!("in_app"), "{st}");
    assert_eq!(
        st["device"]["short"],
        json!(fp),
        "the agent runs the new identity"
    );
    // S6-02: the old pairings (and their grants) do not carry over.
    assert!(pa.load_peers().unwrap().peer.is_empty());
    assert_eq!(
        ok(&ui, json!({ "cmd": "devices_list" })).await["devices"],
        json!([])
    );
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn create_refuses_bad_names_and_no_module() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "a").await;
    let e = err(&ui, create("", false)).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("invalid_name", Some("name"))
    );
    let e = err(
        &ui,
        json!({ "cmd": "onboarding_create", "args": {
            "name": "a", "modules": { "input": false, "clipboard": false }
        } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("no_module", Some("modules"))
    );
    assert_eq!(app_state(&ui).await["initialized"], json!(false));
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn self_test_runs_the_clipboard_and_leaves_the_agent_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let r = ok(&ui, json!({ "cmd": "onboarding_self_test" })).await;
    let results = r["results"].as_array().unwrap();
    let modules: Vec<_> = results.iter().map(|x| x["module"].clone()).collect();
    assert_eq!(modules, vec![json!("clipboard"), json!("input")]);
    let clip = &results[0];
    assert!(
        ["validated", "experimental"].contains(&clip["status"].as_str().unwrap()),
        "the fake clipboard passes: {clip}"
    );
    assert!(!clip["detail"].as_str().unwrap().is_empty());
    // Input is off in this profile (S7: Windows has input too, so the same answer there).
    let input = json!({ "module": "input", "status": "not_tested", "detail": "módulo desligado" });
    assert_eq!(results[1], input);
    // Paused during the test (too short for the poll to see), resumed after.
    assert_eq!(app_state(&ui).await["paused"], json!(false));

    // A pause the user chose stays.
    ok(
        &ui,
        json!({ "cmd": "pause_set", "args": { "paused": true } }),
    )
    .await;
    ok(&ui, json!({ "cmd": "onboarding_self_test" })).await;
    assert_eq!(app_state(&ui).await["paused"], json!(true));
    ui.shutdown().await;
}

/// Item 3 of the S6 fixes, and §13.11 item 4 on Windows: with input on, `clipsync_input::self_test`
/// is called (the portals on Linux, the hooks and monitors on Windows), so the input result is
/// never `not_tested`.
#[tokio::test(flavor = "multi_thread")]
async fn self_test_calls_the_input_module_when_it_is_on() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", BOTH);
    let ui = start(dir.path(), "a").await;
    let r = ok(&ui, json!({ "cmd": "onboarding_self_test" })).await;
    let input = &r["results"][1];
    assert_eq!(input["module"], json!("input"));
    assert!(
        ["validated", "experimental", "unavailable"].contains(&input["status"].as_str().unwrap()),
        "{input}"
    );
    assert!(!input["detail"].as_str().unwrap().is_empty());
    ui.shutdown().await;
}

/// S6-16: what goes wrong with the resume after the test shows up in the result, not only in
/// the log. An agent "outside the app" answers as the test wants.
#[tokio::test(flavor = "multi_thread")]
async fn a_failed_resume_after_the_test_is_in_the_result() {
    let not_saved = clipsync_core::msg::STATE_NOT_SAVED;
    let cases = [
        // Pause reached state.toml, resume did not: a restart comes back paused.
        (
            None,
            Some(not_saved),
            Some("depois de reiniciar ele volta pausado"),
        ),
        // Neither reached state.toml: memory and file agree again, nothing to say.
        (Some(not_saved), Some(not_saved), None),
        // The resume itself was refused: still paused.
        (
            None,
            Some("recusado"),
            Some("o agente ficou pausado depois do teste"),
        ),
    ];
    for (pause, resume, note) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (pa, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
        let agent = fake_agent(&pa, move |req| {
            let answer = |e: Option<&str>| match e {
                None => ControlResponse::Ok,
                Some(m) => ControlResponse::Error { message: m.into() },
            };
            match req {
                ControlRequest::Pause => answer(pause),
                ControlRequest::Resume => answer(resume),
                _ => ControlResponse::Error {
                    message: "não suportado".into(),
                },
            }
        })
        .await;
        let ui = start(dir.path(), "a").await;
        let r = ok(&ui, json!({ "cmd": "onboarding_self_test" })).await;
        let detail = r["results"][0]["detail"].as_str().unwrap();
        match note {
            Some(n) => assert!(detail.contains(n), "{detail}"),
            None => assert!(!detail.contains("agente"), "{detail}"),
        }
        ui.shutdown().await;
        agent.stop().await;
    }
}
