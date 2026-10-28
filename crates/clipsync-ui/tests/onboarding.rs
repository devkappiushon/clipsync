//! 1 · Onboarding (§12.9 item 2, fluxo 1): `onboarding_start` and `onboarding_create` give the
//! same fingerprint; an existing profile without `recreate` gives `needs_confirmation`. Owned by
//! Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

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
    let (_, old) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let s = ok(&ui, json!({ "cmd": "onboarding_start" })).await;
    assert_eq!(s["already_initialized"], json!(true));
    let fp = s["fingerprint_short"].as_str().unwrap().to_owned();
    assert_ne!(fp, old.short());

    let e = err(&ui, create("a", false)).await;
    assert_eq!(e.code, "needs_confirmation");
    assert!(e.message.contains("parear de novo"), "{}", e.message);
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
async fn self_test_reports_every_module() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let r = ok(&ui, json!({ "cmd": "onboarding_self_test" })).await;
    let results = r["results"].as_array().unwrap();
    let modules: Vec<_> = results.iter().map(|x| x["module"].clone()).collect();
    assert_eq!(modules, vec![json!("clipboard"), json!("input")]);
    for x in results {
        let status = x["status"].as_str().unwrap();
        assert!(
            ["validated", "experimental", "unavailable", "not_tested"].contains(&status),
            "{x}"
        );
    }
    ui.shutdown().await;
}
