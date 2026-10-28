//! 3 · Pareamento (§12.9 item 2, fluxo 3): two `Ui` get `pair_code_ready` with the SAME code;
//! `pair_finish` before `pair_code_matches` is `wrong_step`; accepting on both sides saves
//! exactly the grants (everything false by default) and emits `pair_result` paired;
//! `pair_cancel` emits cancelled and saves nothing. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_ui::UiEvent;
use clipsync_ui::events::{PairCodeReady, PairOutcomeName, PairResult};
use common::*;
use serde_json::{Value, json};
use tokio::sync::broadcast;

async fn code_ready(rx: &mut broadcast::Receiver<UiEvent>, who: &str) -> PairCodeReady {
    match next_event(rx, who, |e| matches!(e, UiEvent::PairCodeReady(_))).await {
        UiEvent::PairCodeReady(c) => c,
        _ => unreachable!(),
    }
}

async fn result(rx: &mut broadcast::Receiver<UiEvent>, who: &str) -> PairResult {
    match next_event(rx, who, |e| matches!(e, UiEvent::PairResult(_))).await {
        UiEvent::PairResult(r) => r,
        _ => unreachable!(),
    }
}

fn attempt(cmd: &str, id: u64) -> Value {
    json!({ "cmd": cmd, "args": { "attempt_id": id } })
}

fn finish(id: u64, grants: Value, ack: bool) -> Value {
    json!({ "cmd": "pair_finish", "args": {
        "attempt_id": id, "grants": grants, "acknowledge_full_trust": ack,
    } })
}

#[tokio::test(flavor = "multi_thread")]
async fn both_sides_see_the_same_code_and_save_exactly_their_grants() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pb, idb) = init_profile(dir.path(), "b", CLIPBOARD_ONLY);
    let ui_a = start(dir.path(), "a").await;
    let ui_b = start(dir.path(), "b").await;
    let (mut ev_a, mut ev_b) = (ui_a.subscribe(), ui_b.subscribe());

    assert_eq!(
        ok(&ui_a, json!({ "cmd": "pair_state" })).await,
        json!({ "step": "idle", "attempt_id": null, "port": null, "addresses": [],
                "expires_unix_ms": null, "code": null, "peer": null })
    );
    assert_eq!(
        err(&ui_b, attempt("pair_code_matches", 1)).await.code,
        "no_attempt"
    );
    let e = err(
        &ui_b,
        json!({ "cmd": "pair_join_start", "args": { "addr": "um-pc" } }),
    )
    .await;
    assert_eq!(
        (e.code.as_str(), e.field.as_deref()),
        ("bad_addr", Some("addr"))
    );

    let host = ok(
        &ui_a,
        json!({ "cmd": "pair_host_start", "args": { "port": 0 } }),
    )
    .await;
    let id_a = host["attempt_id"].as_u64().unwrap();
    let port = host["port"].as_u64().unwrap();
    assert_eq!(host["addresses"], json!([format!("127.0.0.1:{port}")]));
    assert!(host["expires_unix_ms"].as_u64().unwrap() > 0);
    assert_eq!(
        ok(&ui_a, json!({ "cmd": "pair_state" })).await["step"],
        json!("listening")
    );
    let e = err(&ui_a, json!({ "cmd": "pair_host_start", "args": {} })).await;
    assert_eq!(e.code, "pair_busy");

    let join = ok(
        &ui_b,
        json!({ "cmd": "pair_join_start", "args": { "addr": format!("127.0.0.1:{port}") } }),
    )
    .await;
    let id_b = join["attempt_id"].as_u64().unwrap();

    let (ca, cb) = (
        code_ready(&mut ev_a, "code on a").await,
        code_ready(&mut ev_b, "code on b").await,
    );
    assert_eq!(ca.code, cb.code, "the same code on both screens");
    assert_eq!(ca.code.len(), 7);
    assert_eq!((ca.attempt_id, cb.attempt_id), (id_a, id_b));
    assert_eq!(
        (ca.peer.name.as_str(), ca.peer.short.clone()),
        ("b", idb.short())
    );
    assert_eq!(
        (cb.peer.name.as_str(), cb.peer.short.clone()),
        ("a", ida.short())
    );
    let st = ok(&ui_a, json!({ "cmd": "pair_state" })).await;
    assert_eq!(
        (st["step"].clone(), st["code"].clone()),
        (json!("code"), json!(ca.code))
    );

    let none = json!({ "clipboard_in": false, "clipboard_out": false, "input_in": false });
    assert_eq!(
        err(&ui_a, finish(id_a, none.clone(), false)).await.code,
        "wrong_step"
    );
    assert_eq!(
        err(&ui_a, attempt("pair_code_matches", id_a + 7))
            .await
            .code,
        "no_attempt"
    );

    for (ui, id) in [(&ui_a, id_a), (&ui_b, id_b)] {
        let m = ok(ui, attempt("pair_code_matches", id)).await;
        assert_eq!(m["step"], json!("permissions"));
        assert_eq!(m["defaults"], none);
        assert_eq!(m["available"], json!({ "clipboard": true, "input": false }));
    }
    assert_eq!(
        err(&ui_a, attempt("pair_code_matches", id_a)).await.code,
        "wrong_step"
    );
    let e = err(
        &ui_b,
        finish(
            id_b,
            json!({ "clipboard_in": false, "clipboard_out": false, "input_in": true }),
            false,
        ),
    )
    .await;
    assert_eq!(e.code, "needs_confirmation");

    let a_grants = json!({ "clipboard_in": true, "clipboard_out": false, "input_in": false });
    assert_eq!(
        ok(&ui_a, finish(id_a, a_grants.clone(), false)).await,
        json!({ "step": "waiting_peer" })
    );
    ok(&ui_b, finish(id_b, none.clone(), false)).await;

    let (ra, rb) = (
        result(&mut ev_a, "result on a").await,
        result(&mut ev_b, "result on b").await,
    );
    assert_eq!(
        (ra.outcome, rb.outcome),
        (PairOutcomeName::Paired, PairOutcomeName::Paired)
    );
    let da = ra.device.unwrap();
    assert_eq!(
        (da.peer_id.as_str(), da.name.as_str()),
        (idb.to_hex().as_str(), "b")
    );
    assert_eq!(serde_json::to_value(da.perms).unwrap(), a_grants);
    assert_eq!(
        serde_json::to_value(rb.device.unwrap().perms).unwrap(),
        none
    );

    let saved_a = pa.load_peers().unwrap();
    assert_eq!(saved_a.peer.len(), 1);
    assert_eq!(saved_a.peer[0].id, idb);
    let p = saved_a.peer[0].perms;
    assert!(p.clipboard_in && !p.clipboard_out && !p.input_in);
    let saved_b = pb.load_peers().unwrap();
    let p = saved_b.peer[0].perms;
    assert!(
        !p.clipboard_in && !p.clipboard_out && !p.input_in,
        "default is all off"
    );
    assert_eq!(
        ok(&ui_a, json!({ "cmd": "pair_state" })).await["step"],
        json!("idle")
    );

    // The core sent ReloadPeers: the two running agents find each other (the event may come
    // before pair_result, so the list is what is checked).
    wait_until("a connected to b", || async {
        ok(&ui_a, json!({ "cmd": "devices_list" })).await["devices"][0]["state"]
            == json!("connected")
    })
    .await;
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_ends_the_attempt_and_saves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pc, _) = init_profile(dir.path(), "c", CLIPBOARD_ONLY);
    let ui_a = start(dir.path(), "a").await;
    let ui_c = start(dir.path(), "c").await;
    let (mut ev_a, mut ev_c) = (ui_a.subscribe(), ui_c.subscribe());

    // Cancelled while listening.
    let host = ok(
        &ui_a,
        json!({ "cmd": "pair_host_start", "args": { "port": 0 } }),
    )
    .await;
    let id = host["attempt_id"].as_u64().unwrap();
    assert_eq!(
        ok(&ui_a, attempt("pair_cancel", id)).await,
        json!({ "cancelled": true })
    );
    let r = result(&mut ev_a, "cancelled while listening").await;
    assert_eq!((r.attempt_id, r.outcome), (id, PairOutcomeName::Cancelled));
    assert_eq!(r.device, None);

    // Cancelled on the code screen ("São diferentes, cancelar").
    let host = ok(
        &ui_a,
        json!({ "cmd": "pair_host_start", "args": { "port": 0 } }),
    )
    .await;
    let id = host["attempt_id"].as_u64().unwrap();
    let port = host["port"].as_u64().unwrap();
    ok(
        &ui_c,
        json!({ "cmd": "pair_join_start", "args": { "addr": format!("127.0.0.1:{port}") } }),
    )
    .await;
    code_ready(&mut ev_a, "code on a").await;
    code_ready(&mut ev_c, "code on c").await;
    ok(&ui_a, attempt("pair_cancel", id)).await;
    let r = result(&mut ev_a, "cancelled on the code").await;
    assert_eq!(r.outcome, PairOutcomeName::Cancelled);
    let rc = result(&mut ev_c, "the other side ends too").await;
    assert_ne!(rc.outcome, PairOutcomeName::Paired);
    assert!(rc.reason.is_some());

    assert!(pa.load_peers().unwrap().peer.is_empty());
    assert!(pc.load_peers().unwrap().peer.is_empty());
    assert_eq!(
        err(&ui_a, attempt("pair_cancel", id)).await.code,
        "no_attempt"
    );
    ui_a.shutdown().await;
    ui_c.shutdown().await;
}
