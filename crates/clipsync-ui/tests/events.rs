//! Events born from `InputState` in the poll (§12.5, events 12 and 13): `layout_changed` with
//! reason `zones` when a peer's monitors arrive, `layout_changed` with reason `offer` when a
//! layout offer is pending, and `screen_lock_changed` when the lock changes. Real agents with
//! the fake input adapter. Owned by Rivet.

// Input exists only on Linux in this version.
#![cfg(target_os = "linux")]
// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_proto::PeerPerms;
use clipsync_ui::UiEvent;
use clipsync_ui::events::{LayoutChanged, LayoutReason, ScreenLockChanged};
use common::*;
use serde_json::json;

/// Each side lets the other control it: `Zones` only goes to a peer granted `input_in`.
const CONTROL: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: true,
};

#[tokio::test(flavor = "multi_thread")]
async fn input_state_changes_become_layout_and_lock_events() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", BOTH);
    pair_files(&pa, &pb, CONTROL, CONTROL);
    // a looks at its agent before b exists: b's monitors arriving is a change.
    let ui_a = start(dir.path(), "a").await;
    let mut ev_a = ui_a.subscribe();
    let ui_b = start(dir.path(), "b").await;
    let mut ev_b = ui_b.subscribe();

    // 12 (zones): b's monitors reach a. No layout.toml yet: rev 0.
    let z = next_event(
        &mut ev_a,
        "layout_changed zones",
        |e| matches!(e, UiEvent::LayoutChanged(l) if l.reason == LayoutReason::Zones),
    )
    .await;
    assert_eq!(
        z,
        UiEvent::LayoutChanged(LayoutChanged {
            rev: 0,
            reason: LayoutReason::Zones
        })
    );

    // 13: the lock is agent state, so the poll sees it on and then off.
    let lock = |locked: bool| json!({ "cmd": "lock_to_screen", "args": { "locked": locked } });
    ok(&ui_a, lock(true)).await;
    let on = next_event(&mut ev_a, "screen_lock_changed on", |e| {
        matches!(e, UiEvent::ScreenLockChanged(_))
    })
    .await;
    assert_eq!(
        on,
        UiEvent::ScreenLockChanged(ScreenLockChanged {
            screen_locked: true,
            active_peer_id: None
        })
    );
    ok(&ui_a, lock(false)).await;
    next_event(&mut ev_a, "screen_lock_changed off", |e| {
        *e == UiEvent::ScreenLockChanged(ScreenLockChanged {
            screen_locked: false,
            active_peer_id: None,
        })
    })
    .await;

    // 12 (offer): a saves a link and offers its layout; b has it pending until it answers.
    let ep = |peer: &clipsync_proto::PeerId, edge: &str| json!({ "peer_id": peer.to_hex(), "monitor": "fake-0", "edge": edge, "from": 0.0, "to": 1.0 });
    ok(
        &ui_a,
        json!({ "cmd": "layout_set_link", "args": {
            "id": null, "a": ep(&ida, "right"), "b": ep(&idb, "left"), "base_rev": 0 } }),
    )
    .await;
    ok(
        &ui_a,
        json!({ "cmd": "layout_copy_to_peers", "args": { "peer_ids": null } }),
    )
    .await;
    let o = next_event(
        &mut ev_b,
        "layout_changed offer",
        |e| matches!(e, UiEvent::LayoutChanged(l) if l.reason == LayoutReason::Offer),
    )
    .await;
    // The rev is b's own layout.toml (none yet), not the offer's.
    assert_eq!(
        o,
        UiEvent::LayoutChanged(LayoutChanged {
            rev: 0,
            reason: LayoutReason::Offer
        })
    );
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}
