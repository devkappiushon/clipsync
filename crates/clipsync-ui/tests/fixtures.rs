//! §12.9 item 2 ("também"): every `tests/fixtures/*.json` deserializes into the crate's type, and
//! serializing it back gives the same JSON (so no `null` field is ever omitted). The fixtures are
//! the examples the front's mock follows (§12.7, Pixel item 3). Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::Path;

use clipsync_ui::{
    UiError, UiEvent, agent, clipboard, devices, diag, onboarding, pairing, prefs, status,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn same<T: DeserializeOwned + Serialize>(name: &str, original: &Value) {
    let parsed: T = serde_json::from_value(original.clone())
        .unwrap_or_else(|e| panic!("{name}.json does not fit its type: {e}"));
    assert_eq!(
        &serde_json::to_value(&parsed).unwrap(),
        original,
        "{name}.json: the type writes something else (a field omitted or renamed?)"
    );
}

/// The type each command answers with (Vector's layout and input commands have their own).
fn check(name: &str, v: &Value) {
    match name {
        "app_state" => same::<status::AppState>(name, v),
        "agent_start" | "agent_stop" => same::<agent::AgentReply>(name, v),
        "pause_set" => same::<status::PausedReply>(name, v),
        "onboarding_start" => same::<onboarding::OnboardingStart>(name, v),
        "onboarding_check_name" => same::<onboarding::NameOk>(name, v),
        "onboarding_create" => same::<onboarding::Created>(name, v),
        "onboarding_self_test" => same::<onboarding::SelfTest>(name, v),
        "devices_list" => same::<devices::DevicesList>(name, v),
        "device_set_perms" => same::<devices::DeviceReply>(name, v),
        "device_revoke" => same::<devices::Removed>(name, v),
        "pair_host_start" => same::<pairing::HostStarted>(name, v),
        "pair_join_start" => same::<pairing::JoinStarted>(name, v),
        "pair_code_matches" => same::<pairing::CodeMatched>(name, v),
        "pair_finish" => same::<pairing::Finished>(name, v),
        "pair_cancel" => same::<pairing::Cancelled>(name, v),
        "pair_state" => same::<pairing::PairState>(name, v),
        "clipboard_get" => same::<clipboard::ClipboardState>(name, v),
        "clipboard_preview" => same::<clipboard::Preview>(name, v),
        "clipboard_set_mode" => same::<clipboard::ModeReply>(name, v),
        "clipboard_copy_item" => same::<clipboard::Applied>(name, v),
        "clipboard_discard_item" => same::<clipboard::Removed>(name, v),
        "clipboard_send_now" => same::<clipboard::Sent>(name, v),
        "diagnostics_get" => same::<diag::Diagnostics>(name, v),
        "support_bundle_preview" => same::<diag::BundlePreview>(name, v),
        "support_bundle_export" => same::<diag::Exported>(name, v),
        "prefs_get" => same::<prefs::Prefs>(name, v),
        "prefs_set" => same::<prefs::PrefsReply>(name, v),
        n if n.starts_with("error") => same::<UiError>(name, v),
        n if n.starts_with("event_") => {
            same::<UiEvent>(name, v);
            assert_eq!(v["event"], Value::from(&n["event_".len()..]), "{name}.json");
        }
        other => panic!("{other}.json has no type: add it here"),
    }
}

#[test]
fn every_fixture_fits_its_type_and_round_trips() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut seen = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{name}.json is not JSON: {e}"));
        check(&name, &v);
        seen.insert(name);
    }
    // One example of each answer of screens 1, 2, 3, 5, 6, 7 and of each event.
    let commands = clipsync_ui::api::COMMANDS.iter().filter(|c| {
        !c.starts_with("layout_")
            && ![
                "lock_to_screen",
                "switch_to_device",
                "input_test",
                "device_pause",
            ]
            .contains(c)
    });
    let events = [
        "peer_connected",
        "peer_disconnected",
        "peer_blocked",
        "clip_received",
        "pause_changed",
        "agent_state",
        "pair_code_ready",
        "pair_result",
        "notification",
        "prefs_changed",
        "resync",
        "layout_changed",
        "screen_lock_changed",
    ];
    for want in commands
        .map(|c| c.to_string())
        .chain(events.iter().map(|e| format!("event_{e}")))
    {
        assert!(
            seen.contains(&want),
            "tests/fixtures/{want}.json is missing"
        );
    }
}
