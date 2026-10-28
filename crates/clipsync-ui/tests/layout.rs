//! 4 · Layout de telas (§12.9 item 3, parte do clipsync-ui): as validações do `layout_set_link`
//! na ordem da §12.5, `stale_rev`, o `layout.toml` relido igual depois de gravado, e os demais
//! comandos da tela (dispositivo, opções, cópia, trava, troca, teste). Agentes reais em perfis
//! temporários, 127.0.0.1 e adaptadores fake: o monitor de cada lado é o `fake-0` de 1920x1080.
//! Owned by Vector.

// Input exists only on Linux in this version; `layout_windows.rs` covers the other side.
#![cfg(target_os = "linux")]
// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_input::Layout as FileLayout;
use clipsync_proto::{PeerId, PeerPerms};
use clipsync_ui::events::{LayoutChanged, LayoutReason};
use clipsync_ui::{Ui, UiEvent};
use common::*;
use serde_json::{Value, json};
use tempfile::TempDir;

/// What the peer grants: control of its keyboard and mouse, nothing else.
const CONTROL: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: true,
};

fn ep(peer: &PeerId, edge: &str, from: f64, to: f64) -> Value {
    json!({ "peer_id": peer.to_hex(), "monitor": "fake-0", "edge": edge, "from": from, "to": to })
}

fn set_link(id: Value, a: Value, b: Value, base_rev: u64) -> Value {
    json!({ "cmd": "layout_set_link", "args": { "id": id, "a": a, "b": b, "base_rev": base_rev } })
}

async fn get(ui: &Ui) -> Value {
    ok(ui, json!({ "cmd": "layout_get" })).await
}

async fn rev(ui: &Ui) -> u64 {
    get(ui).await["rev"].as_u64().unwrap()
}

fn device<'a>(layout: &'a Value, peer: &PeerId) -> &'a Value {
    layout["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["peer_id"] == json!(peer.to_hex()))
        .unwrap_or_else(|| panic!("{peer} is not in {layout}"))
}

/// The agent of `ui` knows the monitors `peer` announced.
async fn wait_live(ui: &Ui, peer: &PeerId) {
    wait_until("monitors of the peer, live", || async {
        let l = get(ui).await;
        let d = device(&l, peer);
        d["monitors_source"] == json!("live") && !d["monitors"].as_array().unwrap().is_empty()
    })
    .await;
}

struct Duo {
    dir: TempDir,
    a: Ui,
    b: Ui,
    ida: PeerId,
    idb: PeerId,
}

impl Duo {
    fn layout_file(&self, name: &str) -> std::path::PathBuf {
        self.dir.path().join(name).join("layout.toml")
    }
}

/// `a` and `b`, input and clipboard on, each granting the other control; both know the other's
/// monitors before the test starts.
async fn duo() -> Duo {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", BOTH);
    pair_files(&pa, &pb, CONTROL, CONTROL);
    let a = start(dir.path(), "a").await;
    let b = start(dir.path(), "b").await;
    wait_live(&a, &idb).await;
    wait_live(&b, &ida).await;
    Duo {
        dir,
        a,
        b,
        ida,
        idb,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn get_lists_this_device_and_the_paired_ones_with_their_monitors() {
    let d = duo().await;
    let l = get(&d.a).await;
    assert_eq!(l["rev"], json!(0));
    assert_eq!(l["self_id"], json!(d.ida.to_hex()));
    assert_eq!(l["snap_px"], json!(16));
    assert_eq!(l["options"], json!({ "dead_corner_px": 8 }));
    assert_eq!(l["screen_locked"], json!(false));
    assert_eq!(l["active_peer_id"], json!(null), "null, never omitted");
    assert_eq!(l["pending_offer"], json!(null));
    assert_eq!(l["links"], json!([]));
    assert_eq!(l["locked_edges"], json!([]));

    let me = device(&l, &d.ida);
    assert_eq!(me["is_self"], json!(true));
    assert_eq!(me["name"], json!("a"));
    assert_eq!((me["x"].clone(), me["y"].clone()), (json!(0), json!(0)));
    assert_eq!(me["monitors_source"], json!("live"));
    let m = &me["monitors"][0];
    assert_eq!(
        (m["ref"].clone(), m["width"].clone(), m["height"].clone()),
        (json!("fake-0"), json!(1920), json!(1080))
    );

    let other = device(&l, &d.idb);
    assert_eq!(other["is_self"], json!(false));
    assert_eq!(other["name"], json!("b"));
    assert_eq!(other["state"], json!("connected"));
    assert_eq!(other["x"], json!(2048), "unplaced devices go to the right");
    assert_eq!(other["hotkey"], json!(null));
    assert_eq!(other["monitors"][0]["ref"], json!("fake-0"));
    assert!(
        !d.layout_file("a").exists(),
        "reading never writes layout.toml"
    );

    // The peer leaves: its monitors stay on screen, marked as remembered.
    d.b.shutdown().await;
    wait_until("peer offline with cached monitors", || async {
        let l = get(&d.a).await;
        let o = device(&l, &d.idb);
        o["state"] == json!("offline") && o["monitors_source"] == json!("cached")
    })
    .await;
    d.a.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn without_the_input_module_or_a_profile_every_command_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (_, other) = init_profile(dir.path(), "b", BOTH);
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let ui = start(dir.path(), "a").await;
    let hex = other.to_hex();
    let ep = json!({ "peer_id": hex, "monitor": "m", "edge": "left", "from": 0.0, "to": 1.0 });
    for req in [
        json!({ "cmd": "layout_get" }),
        set_link(json!(null), ep.clone(), ep.clone(), 0),
        json!({ "cmd": "layout_remove_link", "args": { "id": "l1", "base_rev": 0 } }),
        json!({ "cmd": "layout_set_device", "args": { "peer_id": hex, "x": 0, "y": 0, "hotkey": null, "base_rev": 0 } }),
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 8, "base_rev": 0 } }),
        json!({ "cmd": "layout_copy_to_peers", "args": { "peer_ids": null } }),
        json!({ "cmd": "layout_offer_answer", "args": { "accept": true } }),
        json!({ "cmd": "lock_to_screen", "args": { "locked": true } }),
        json!({ "cmd": "switch_to_device", "args": { "peer_id": hex } }),
        json!({ "cmd": "input_test", "args": { "peer_id": hex, "seconds": 5 } }),
    ] {
        let e = err(&ui, req.clone()).await;
        assert_eq!(e.code, "input_off", "{req}: {e:?}");
        assert!(!e.message.is_empty());
    }
    ui.shutdown().await;

    let empty = tempfile::tempdir().unwrap();
    let ui = start(empty.path(), "novo").await;
    let e = err(&ui, json!({ "cmd": "layout_get" })).await;
    assert_eq!(e.code, "not_initialized");
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_shapes_are_bad_request() {
    let d = duo().await;
    let (a, b) = (ep(&d.ida, "right", 0.0, 1.0), ep(&d.idb, "left", 0.0, 1.0));
    let mut diagonal = a.clone();
    diagonal["edge"] = json!("diagonal");
    for req in [
        set_link(json!(null), diagonal, b.clone(), 0),
        json!({ "cmd": "layout_set_link", "args": { "id": null, "a": a, "b": b } }),
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 8, "base_rev": 0, "x": 1 } }),
        json!({ "cmd": "lock_to_screen", "args": { "locked": "sim" } }),
    ] {
        assert_eq!(err(&d.a, req.clone()).await.code, "bad_request", "{req}");
    }
    d.a.shutdown().await;
    d.b.shutdown().await;
}

/// §12.5: `same_device`, `internal_edge_locked`, `empty_segment`, `segment_out_of_range`,
/// `segment_overlap`, `stale_rev`, in this order. Each case below is wrong in two ways at once,
/// and the earlier error must win. (`internal_edge_locked` needs two monitors on one device;
/// the fake has one, so it is covered by the unit tests of `layout.rs`.)
#[tokio::test(flavor = "multi_thread")]
async fn set_link_validates_in_the_documented_order() {
    let d = duo().await;
    let (ida, idb) = (&d.ida, &d.idb);
    let stale = 99;

    // The shape of the endpoints comes first.
    let e = err(
        &d.a,
        set_link(json!(null), json!({ "peer_id": "xyz", "monitor": "fake-0", "edge": "left", "from": 0.0, "to": 1.0 }), ep(idb, "left", 0.0, 1.0), 0),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_request", Some("a.peer_id"))
    );
    let nobody = PeerId([7; 32]);
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(&nobody, "right", 0.0, 1.0),
            ep(idb, "left", 0.0, 1.0),
            0,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("unknown_peer", Some("a.peer_id"))
    );
    let mut ghost = ep(idb, "left", 0.0, 1.0);
    ghost["monitor"] = json!("HDMI-9");
    let e = err(
        &d.a,
        set_link(json!(null), ep(ida, "right", 0.0, 1.0), ghost, 0),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_request", Some("b.monitor"))
    );

    // same_device beats an empty segment and a stale revision.
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.5, 0.5),
            ep(ida, "left", 0.0, 1.0),
            stale,
        ),
    )
    .await;
    assert_eq!(e.code, "same_device");

    // empty_segment beats an out-of-range segment on the other side, and a stale revision.
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.0, 1.5),
            ep(idb, "left", 0.5, 0.5),
            stale,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("empty_segment", Some("b"))
    );
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.8, 0.2),
            ep(idb, "left", 0.0, 1.0),
            0,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("empty_segment", Some("a"))
    );

    // segment_out_of_range beats a stale revision.
    for (from, to) in [(0.0, 1.5), (-0.1, 0.5)] {
        let e = err(
            &d.a,
            set_link(
                json!(null),
                ep(ida, "right", from, to),
                ep(idb, "left", 0.0, 1.0),
                stale,
            ),
        )
        .await;
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            ("segment_out_of_range", Some("a")),
            "{from}..{to}"
        );
    }

    // segment_overlap beats a stale revision; touching segments are fine.
    let first = ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.25, 0.75),
            ep(idb, "left", 0.25, 0.75),
            0,
        ),
    )
    .await;
    assert_eq!(first["link"]["id"], json!("l1"));
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.5, 0.9),
            ep(idb, "left", 0.8, 0.9),
            stale,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("segment_overlap", Some("a"))
    );
    assert_eq!(e.details.as_ref().unwrap()["link"], json!("l1"));
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.8, 0.9),
            ep(idb, "left", 0.5, 0.9),
            stale,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("segment_overlap", Some("b"))
    );

    // stale_rev last, and it says which revision is current.
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.75, 0.9),
            ep(idb, "left", 0.75, 0.9),
            stale,
        ),
    )
    .await;
    assert_eq!(e.code, "stale_rev");
    assert_eq!(e.details.as_ref().unwrap()["rev"], json!(1));
    let ok_touching = ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.75, 0.9),
            ep(idb, "left", 0.75, 0.9),
            1,
        ),
    )
    .await;
    assert_eq!(ok_touching["link"]["id"], json!("l2"));
    assert_eq!(ok_touching["layout_rev"], json!(2));

    // Nothing that failed was written.
    assert_eq!(rev(&d.a).await, 2);
    assert_eq!(
        FileLayout::read(&d.layout_file("a"))
            .unwrap()
            .unwrap()
            .links
            .len(),
        2
    );
    d.a.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_saved_link_reads_back_the_same_and_survives_a_restart() {
    let d = duo().await;
    let mut events = d.a.subscribe();
    let reply = ok(
        &d.a,
        set_link(
            json!(null),
            ep(&d.ida, "right", 0.25, 0.75),
            ep(&d.idb, "left", 0.1, 0.5),
            0,
        ),
    )
    .await;
    assert_eq!(reply["layout_rev"], json!(1));
    assert_eq!(reply["warnings"], json!([]));
    assert_eq!(reply["link"]["problem"], json!(null));
    assert_eq!(reply["link"]["a"], ep(&d.ida, "right", 0.25, 0.75));
    assert_eq!(
        reply["link"]["b"],
        ep(&d.idb, "left", 0.1, 0.5),
        "0.1 stays 0.1, not 0.10000000149"
    );
    next_event(&mut events, "layout_changed saved", |e| {
        *e == UiEvent::LayoutChanged(LayoutChanged {
            rev: 1,
            reason: LayoutReason::Saved,
        })
    })
    .await;

    // The file is what the screen showed, and writing it again gives the same file.
    let path = d.layout_file("a");
    let on_disk = FileLayout::read(&path).unwrap().unwrap();
    assert_eq!(on_disk.rev, 1);
    assert_eq!(on_disk.links.len(), 1);
    assert_eq!(on_disk.links[0].id, "l1");
    assert_eq!(on_disk.devices.len(), 2);
    assert_eq!(
        FileLayout::from_toml_str(&on_disk.to_toml_string().unwrap()).unwrap(),
        on_disk
    );
    assert!(
        !path.with_extension("toml.tmp").exists(),
        "tmp + rename leaves no tmp"
    );

    let before = get(&d.a).await;
    assert_eq!(before["links"], json!([reply["link"].clone()]));
    d.a.shutdown().await;

    // A new Ui on the same profile shows the same layout.
    let again = start(d.dir.path(), "a").await;
    let after = get(&again).await;
    for key in ["rev", "links", "options", "snap_px", "self_id"] {
        assert_eq!(after[key], before[key], "{key}");
    }
    assert_eq!(
        device(&after, &d.idb)["x"],
        device(&before, &d.idb)["x"],
        "positions come from the file"
    );
    again.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn links_can_be_edited_and_removed_and_a_stale_screen_is_refused() {
    let d = duo().await;
    let (ida, idb) = (&d.ida, &d.idb);
    ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.25, 0.75),
            ep(idb, "left", 0.25, 0.75),
            0,
        ),
    )
    .await;
    ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.75, 0.9),
            ep(idb, "left", 0.75, 0.9),
            1,
        ),
    )
    .await;

    // Editing l1 may reuse its own old span, and never takes the span of l2.
    let edited = ok(
        &d.a,
        set_link(
            json!("l1"),
            ep(ida, "right", 0.1, 0.5),
            ep(idb, "left", 0.1, 0.5),
            2,
        ),
    )
    .await;
    assert_eq!(
        (edited["link"]["id"].clone(), edited["layout_rev"].clone()),
        (json!("l1"), json!(3))
    );
    let l = get(&d.a).await;
    assert_eq!(l["links"].as_array().unwrap().len(), 2, "edit, not create");
    let e = err(
        &d.a,
        set_link(
            json!("l1"),
            ep(ida, "right", 0.1, 0.8),
            ep(idb, "left", 0.1, 0.5),
            3,
        ),
    )
    .await;
    assert_eq!(e.code, "segment_overlap");
    let e = err(
        &d.a,
        set_link(
            json!("l9"),
            ep(ida, "right", 0.0, 0.05),
            ep(idb, "left", 0.0, 0.05),
            3,
        ),
    )
    .await;
    assert_eq!(e.code, "link_not_found");

    // Remove: stale first, then the unknown id, then the real thing.
    let remove = |id: &str, base_rev: u64| json!({ "cmd": "layout_remove_link", "args": { "id": id, "base_rev": base_rev } });
    let e = err(&d.a, remove("l2", 1)).await;
    assert_eq!(e.code, "stale_rev");
    assert_eq!(e.details.as_ref().unwrap()["rev"], json!(3));
    assert_eq!(err(&d.a, remove("l9", 3)).await.code, "link_not_found");
    assert_eq!(ok(&d.a, remove("l2", 3)).await, json!({ "layout_rev": 4 }));
    let l = get(&d.a).await;
    assert_eq!(l["links"].as_array().unwrap().len(), 1);
    assert_eq!(
        FileLayout::read(&d.layout_file("a"))
            .unwrap()
            .unwrap()
            .links
            .len(),
        1
    );
    // The id l2 is free again.
    let again = ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.75, 0.9),
            ep(idb, "left", 0.75, 0.9),
            4,
        ),
    )
    .await;
    assert_eq!(again["link"]["id"], json!("l2"));
    d.a.shutdown().await;
    d.b.shutdown().await;
}

/// Two screens saving from the same revision: one wins, the other is told to reload.
#[tokio::test(flavor = "multi_thread")]
async fn two_saves_from_the_same_revision_do_not_both_win() {
    let d = duo().await;
    let (ida, idb) = (&d.ida, &d.idb);
    let one = set_link(
        json!(null),
        ep(ida, "right", 0.1, 0.3),
        ep(idb, "left", 0.1, 0.3),
        0,
    );
    let two = set_link(
        json!(null),
        ep(ida, "right", 0.5, 0.7),
        ep(idb, "left", 0.5, 0.7),
        0,
    );
    let (r1, r2) = tokio::join!(d.a.call(one), d.a.call(two));
    let results = [r1, r2];
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    let loser = results.iter().find_map(|r| r.as_ref().err()).unwrap();
    assert_eq!(loser.code, "stale_rev");
    assert_eq!(rev(&d.a).await, 1);
    d.a.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dead_corners_trim_the_segment_and_warn_and_options_refuse_what_would_empty_a_link() {
    let d = duo().await;
    let (ida, idb) = (&d.ida, &d.idb);

    // 8 px on a 1080 px edge: the whole edge loses its ends, on both sides.
    let r = ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.0, 1.0),
            ep(idb, "left", 0.0, 1.0),
            0,
        ),
    )
    .await;
    assert_eq!(
        r["warnings"],
        json!([
            { "code": "trimmed_by_dead_corner", "side": "a" },
            { "code": "trimmed_by_dead_corner", "side": "b" }
        ])
    );
    let (from, to) = (
        r["link"]["a"]["from"].as_f64().unwrap(),
        r["link"]["a"]["to"].as_f64().unwrap(),
    );
    assert!(
        (from - 8.0 / 1080.0).abs() < 1e-6 && (to - (1.0 - 8.0 / 1080.0)).abs() < 1e-6,
        "{from}..{to}"
    );

    // A segment that sits entirely inside a corner is an empty segment, on its side.
    let e = err(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "top", 0.0, 0.2),
            ep(idb, "bottom", 0.0, 0.003),
            1,
        ),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("empty_segment", Some("b"))
    );

    // The option itself.
    let e = err(
        &d.a,
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 33, "base_rev": 1 } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("out_of_range", Some("dead_corner_px"))
    );
    let r = ok(
        &d.a,
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 0, "base_rev": 1 } }),
    )
    .await;
    assert_eq!(
        r,
        json!({ "options": { "dead_corner_px": 0 }, "layout_rev": 2 })
    );

    // With 0 px a thin link on the top edge is fine and untouched ...
    let thin = ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "top", 0.0, 0.01),
            ep(idb, "bottom", 0.0, 0.01),
            2,
        ),
    )
    .await;
    assert_eq!(thin["warnings"], json!([]));
    // ... but 32 px on a 1920 px edge would take 0.0167 of each end, so it would be left empty.
    let e = err(
        &d.a,
        json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 32, "base_rev": 3 } }),
    )
    .await;
    assert_eq!(e.code, "empty_segment");
    assert_eq!(e.details.as_ref().unwrap()["links"], json!(["l2"]));
    let l = get(&d.a).await;
    assert_eq!(
        (l["options"]["dead_corner_px"].clone(), l["rev"].clone()),
        (json!(0), json!(3)),
        "nothing applied"
    );
    assert_eq!(
        err(
            &d.a,
            json!({ "cmd": "layout_set_options", "args": { "dead_corner_px": 8, "base_rev": 1 } })
        )
        .await
        .code,
        "stale_rev"
    );
    d.a.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn devices_snap_to_the_grid_and_hotkeys_are_unique() {
    let d = duo().await;
    let set = |peer: &PeerId, x: i64, y: i64, hotkey: Value, base_rev: u64| json!({ "cmd": "layout_set_device", "args": { "peer_id": peer.to_hex(), "x": x, "y": y, "hotkey": hotkey, "base_rev": base_rev } });
    let r = ok(&d.a, set(&d.idb, 37, -9, json!(2), 0)).await;
    assert_eq!(r["layout_rev"], json!(1));
    let dev = &r["device"];
    assert_eq!(
        (dev["x"].clone(), dev["y"].clone(), dev["hotkey"].clone()),
        (json!(32), json!(-16), json!(2))
    );
    assert_eq!(dev["peer_id"], json!(d.idb.to_hex()));
    assert_eq!(dev["monitors"][0]["ref"], json!("fake-0"));
    let on_disk = FileLayout::read(&d.layout_file("a")).unwrap().unwrap();
    let saved = on_disk.device(d.idb).unwrap();
    assert_eq!((saved.x, saved.y, saved.hotkey), (32, -16, Some(2)));

    // The hotkey belongs to b; a cannot take it, and says who has it.
    let e = err(&d.a, set(&d.ida, 0, 0, json!(2), 1)).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("hotkey_taken", Some("hotkey"))
    );
    assert_eq!(
        e.details.as_ref().unwrap()["peer_id"],
        json!(d.idb.to_hex())
    );
    for bad in [0, 10] {
        let e = err(&d.a, set(&d.ida, 0, 0, json!(bad), 1)).await;
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            ("bad_request", Some("hotkey")),
            "{bad}"
        );
    }
    // Keeping its own hotkey, clearing it, and taking a free one are all fine.
    ok(&d.a, set(&d.idb, 32, -16, json!(2), 1)).await;
    let cleared = ok(&d.a, set(&d.idb, 32, -16, json!(null), 2)).await;
    assert_eq!(cleared["device"]["hotkey"], json!(null));
    let r = ok(&d.a, set(&d.ida, 8, 0, json!(2), 3)).await;
    assert_eq!(
        (r["device"]["x"].clone(), r["device"]["hotkey"].clone()),
        (json!(16), json!(2)),
        "8 rounds up"
    );

    assert_eq!(
        err(&d.a, set(&d.idb, 0, 0, json!(null), 0)).await.code,
        "stale_rev"
    );
    assert_eq!(
        err(&d.a, set(&PeerId([5; 32]), 0, 0, json!(null), 4))
            .await
            .code,
        "unknown_peer"
    );
    let e = err(&d.a, set(&d.idb, 5_000_000, 0, json!(null), 4)).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_request", Some("x"))
    );
    d.a.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manual_barrier_and_a_link_together_are_refused_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", BOTH);
    pair_files(&pa, &pb, CONTROL, CONTROL);
    let mut config = std::fs::read_to_string(pa.config_path()).unwrap();
    config.push_str("\n[[input.barrier]]\nmonitor = \"fake-0\"\nedge = \"right\"\npeer = \"b\"\npeer_edge = \"left\"\n");
    std::fs::write(pa.config_path(), config).unwrap();
    let ui = start(dir.path(), "a").await;

    let e = err(
        &ui,
        set_link(
            json!(null),
            ep(&ida, "right", 0.25, 0.75),
            ep(&idb, "left", 0.25, 0.75),
            0,
        ),
    )
    .await;
    assert_eq!(e.code, "config");
    assert!(e.message.contains("[[input.barrier]]"), "{}", e.message);
    assert!(!pa.dir.join("layout.toml").exists());
    // Devices alone are no clash: moving one is allowed.
    ok(&ui, json!({ "cmd": "layout_set_device", "args": { "peer_id": idb.to_hex(), "x": 0, "y": 0, "hotkey": null, "base_rev": 0 } })).await;
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_lock_and_the_switch_go_through_the_agent() {
    let d = duo().await;
    let (ida, idb) = (&d.ida, &d.idb);
    let lock = |locked: bool| json!({ "cmd": "lock_to_screen", "args": { "locked": locked } });
    let switch =
        |peer: &PeerId| json!({ "cmd": "switch_to_device", "args": { "peer_id": peer.to_hex() } });

    assert_eq!(ok(&d.a, lock(true)).await, json!({ "screen_locked": true }));
    assert_eq!(get(&d.a).await["screen_locked"], json!(true));
    assert_eq!(app_state(&d.a).await["screen_locked"], json!(true));
    assert_eq!(
        ok(&d.a, lock(false)).await,
        json!({ "screen_locked": false })
    );
    assert_eq!(get(&d.a).await["screen_locked"], json!(false));

    // Nobody to go to: not paired, then paired but with no link.
    assert_eq!(
        err(&d.a, switch(&PeerId([9; 32]))).await.code,
        "unknown_peer"
    );
    assert_eq!(err(&d.a, switch(idb)).await.code, "not_linked");

    ok(
        &d.a,
        set_link(
            json!(null),
            ep(ida, "right", 0.25, 0.75),
            ep(idb, "left", 0.25, 0.75),
            0,
        ),
    )
    .await;
    assert_eq!(
        ok(&d.a, switch(idb)).await,
        json!({ "active_peer_id": idb.to_hex() })
    );
    wait_until("active peer shown by layout_get", || async {
        get(&d.a).await["active_peer_id"] == json!(idb.to_hex())
    })
    .await;

    // Locked to this screen, the switch is refused.
    ok(&d.a, lock(true)).await;
    assert_eq!(err(&d.a, switch(idb)).await.code, "denied");
    ok(&d.a, lock(false)).await;

    // No agent, no lock.
    ok(&d.a, json!({ "cmd": "agent_stop" })).await;
    assert_eq!(err(&d.a, lock(true)).await.code, "agent_not_running");
    assert_eq!(err(&d.a, switch(idb)).await.code, "agent_not_running");
    // The file can still be edited, and the screen still shows it, without monitors.
    let l = get(&d.a).await;
    assert_eq!(l["links"].as_array().unwrap().len(), 1);
    assert_eq!(device(&l, ida)["monitors_source"], json!("none"));
    d.a.shutdown().await;
    d.b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_input_test_checks_its_arguments_and_its_peer() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, _) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", BOTH);
    pair_files(&pa, &pb, CONTROL, CONTROL);
    // b is paired but its agent never starts.
    let ui = start(dir.path(), "a").await;
    let test = |peer: &PeerId, seconds: u64| json!({ "cmd": "input_test", "args": { "peer_id": peer.to_hex(), "seconds": seconds } });
    for seconds in [0, 4, 61] {
        let e = err(&ui, test(&idb, seconds)).await;
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            ("bad_request", Some("seconds")),
            "{seconds}"
        );
    }
    assert_eq!(
        err(&ui, test(&PeerId([3; 32]), 5)).await.code,
        "unknown_peer"
    );
    assert_eq!(err(&ui, test(&idb, 5)).await.code, "offline");
    ui.shutdown().await;
}

/// a offers its layout to b (grants control), c (connected, grants nothing) and d (offline).
#[tokio::test(flavor = "multi_thread")]
async fn a_layout_is_offered_to_the_peers_and_each_answer_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", BOTH);
    let (pb, idb) = init_profile(dir.path(), "b", BOTH);
    let (pc, idc) = init_profile(dir.path(), "c", BOTH);
    let (pd, idd) = init_profile(dir.path(), "d", BOTH);
    pair_files(&pa, &pb, CONTROL, CONTROL);
    pair_files(&pa, &pc, CONTROL, common::NONE);
    pair_files(&pa, &pd, CONTROL, CONTROL);
    let a = start(dir.path(), "a").await;
    let b = start(dir.path(), "b").await;
    let c = start(dir.path(), "c").await;
    wait_live(&a, &idb).await;
    wait_until("c connected to a", || async {
        device(&get(&a).await, &idc)["state"] == json!("connected")
    })
    .await;
    drop((pb, pc, pd));

    let copy = |ids: Value| json!({ "cmd": "layout_copy_to_peers", "args": { "peer_ids": ids } });
    // Nothing saved yet, nothing to offer.
    assert_eq!(err(&a, copy(json!(null))).await.code, "no_layout");

    ok(
        &a,
        set_link(
            json!(null),
            ep(&ida, "right", 0.25, 0.75),
            ep(&idb, "left", 0.25, 0.75),
            0,
        ),
    )
    .await;
    let r = ok(&a, copy(json!(null))).await;
    assert_eq!(r["rev"], json!(1));
    let results = r["results"].as_array().unwrap();
    let status_of = |id: &PeerId| {
        let x = results
            .iter()
            .find(|x| x["peer_id"] == json!(id.to_hex()))
            .unwrap();
        (
            x["name"].clone(),
            x["status"].clone(),
            x["reason"].is_null(),
        )
    };
    assert_eq!(results.len(), 3, "{r}");
    assert_eq!(status_of(&idb), (json!("b"), json!("offered"), true));
    assert_eq!(status_of(&idc), (json!("c"), json!("denied"), false));
    assert_eq!(status_of(&idd), (json!("d"), json!("offline"), false));

    // Some peers only; a stranger or this device is not a peer.
    let r = ok(&a, copy(json!([idd.to_hex()]))).await;
    assert_eq!(r["results"].as_array().unwrap().len(), 1);
    assert_eq!(ok(&a, copy(json!([]))).await["results"], json!([]));
    let e = err(&a, copy(json!([PeerId([4; 32]).to_hex()]))).await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("unknown_peer", Some("peer_ids"))
    );
    assert_eq!(
        err(&a, copy(json!([ida.to_hex()]))).await.code,
        "unknown_peer"
    );

    // b has the offer waiting, with a's links and without any file written.
    wait_until("offer shown on b", || async {
        get(&b).await["pending_offer"] != json!(null)
    })
    .await;
    let l = get(&b).await;
    let offer = &l["pending_offer"];
    assert_eq!(
        (
            offer["from_peer_id"].clone(),
            offer["from_name"].clone(),
            offer["rev"].clone()
        ),
        (json!(ida.to_hex()), json!("a"), json!(1))
    );
    assert_eq!(offer["links"][0]["id"], json!("o1"));
    assert_eq!(offer["links"][0]["a"], ep(&ida, "right", 0.25, 0.75));
    assert_eq!(l["links"], json!([]));
    assert!(!dir.path().join("b").join("layout.toml").exists());

    // Declining clears it and changes nothing.
    let answer =
        |accept: bool| json!({ "cmd": "layout_offer_answer", "args": { "accept": accept } });
    assert_eq!(ok(&b, answer(false)).await, json!({ "layout_rev": 0 }));
    assert_eq!(get(&b).await["pending_offer"], json!(null));
    assert!(!dir.path().join("b").join("layout.toml").exists());
    assert_eq!(err(&b, answer(true)).await.code, "no_offer");

    // Accepting writes a's links into b's own layout.toml.
    ok(&a, copy(json!([idb.to_hex()]))).await;
    wait_until("second offer shown on b", || async {
        get(&b).await["pending_offer"] != json!(null)
    })
    .await;
    let accepted = ok(&b, answer(true)).await;
    assert_eq!(accepted, json!({ "layout_rev": 1 }));
    let l = get(&b).await;
    assert_eq!(
        (l["rev"].clone(), l["pending_offer"].clone()),
        (json!(1), json!(null))
    );
    assert_eq!(l["links"].as_array().unwrap().len(), 1);
    assert_eq!(l["links"][0]["a"], ep(&ida, "right", 0.25, 0.75));
    assert_eq!(l["links"][0]["b"], ep(&idb, "left", 0.25, 0.75));
    let on_disk = FileLayout::read(&dir.path().join("b").join("layout.toml"))
        .unwrap()
        .unwrap();
    assert_eq!((on_disk.rev, on_disk.links.len()), (1, 1));

    for ui in [&a, &b, &c] {
        ui.shutdown().await;
    }
}
