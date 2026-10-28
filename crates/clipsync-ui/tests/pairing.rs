//! 3 · Pareamento (§12.9 item 2, fluxo 3): two `Ui` get `pair_code_ready` with the SAME code;
//! `pair_finish` before `pair_code_matches` is `wrong_step`; accepting on both sides saves
//! exactly the grants (everything false by default) and emits `pair_result` paired;
//! `pair_cancel` emits cancelled and saves nothing. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clipsync_core::pairing::{self, PairDecision, PairPrompt};
use clipsync_core::tls::KEEP_ALIVE;
use clipsync_ui::UiEvent;
use clipsync_ui::events::{PairCodeReady, PairOutcomeName, PairResult};
use common::*;
use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

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

/// A UDP relay between the joiner and the host that can lose what the host sends, like a
/// network dropping the one packet that matters.
struct LossyRelay {
    addr: SocketAddr,
    losing: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

impl LossyRelay {
    async fn start(host: SocketAddr) -> Self {
        let front = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let back = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        back.connect(host).await.unwrap();
        let addr = front.local_addr().unwrap();
        let losing = Arc::new(AtomicBool::new(false));
        let lose = losing.clone();
        let task = tokio::spawn(async move {
            let (mut up, mut down) = (vec![0u8; 65_536], vec![0u8; 65_536]);
            let mut joiner = None;
            loop {
                tokio::select! {
                    r = front.recv_from(&mut up) => {
                        let Ok((n, from)) = r else { continue };
                        joiner = Some(from);
                        let _ = back.send(&up[..n]).await;
                    }
                    r = back.recv(&mut down) => {
                        let Ok(n) = r else { continue };
                        if let Some(j) = joiner.filter(|_| !lose.load(Ordering::SeqCst)) {
                            let _ = front.send_to(&down[..n], j).await;
                        }
                    }
                }
            }
        });
        Self { addr, losing, task }
    }

    /// Everything the host sends in the next `d` is lost.
    fn lose_host_packets(&self, d: Duration) {
        self.losing.store(true, Ordering::SeqCst);
        let losing = self.losing.clone();
        tokio::spawn(async move {
            tokio::time::sleep(d).await;
            losing.store(false, Ordering::SeqCst);
        });
    }
}

/// Lets the joiner's first datagram reach the host and, once the host answers it, drops
/// everything: the host holds an attempt whose handshake never ends, before any code.
async fn stall_handshake(host: SocketAddr) -> (SocketAddr, oneshot::Receiver<()>, JoinHandle<()>) {
    let front = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let back = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    back.connect(host).await.unwrap();
    let addr = front.local_addr().unwrap();
    let (answered, stalled) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut buf = vec![0u8; 65_536];
        if let Ok((n, _)) = front.recv_from(&mut buf).await {
            let _ = back.send(&buf[..n]).await;
        }
        if back.recv(&mut buf).await.is_ok() {
            let _ = answered.send(());
        }
        // Holds both sockets: the joiner's resends go nowhere.
        std::future::pending::<()>().await;
    });
    (addr, stalled, task)
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
    // S6-11: the window is at most PAIR_WINDOW_SECS (120 s).
    for w in [0, 121, 600] {
        let e = err(
            &ui_a,
            json!({ "cmd": "pair_host_start", "args": { "port": 0, "window_secs": w } }),
        )
        .await;
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            ("bad_request", Some("window_secs"))
        );
    }

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
    // S6-10: a's decision already left; a cancel now could race the core saving peers.toml.
    let e = err(&ui_a, attempt("pair_cancel", id_a)).await;
    assert_eq!(e.code, "wrong_step");
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

/// CI 40680756334 (e40849c): the cancel reached the other side only as the FIN and the
/// CONNECTION_CLOSE of a dropped connection, packets QUIC never sends twice. With them lost, c
/// ended on a stateless reset, if a's endpoint was still there to answer, or on the idle timeout
/// (keep-alive 5 s + idle 15 s, the 20 s the test waits), and its user kept waiting. The decline
/// now goes as Confirm{false} on the control stream, which QUIC retransmits.
#[tokio::test(flavor = "multi_thread")]
async fn cancel_reaches_the_other_side_even_when_packets_are_lost() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, _) = init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pc, _) = init_profile(dir.path(), "c", CLIPBOARD_ONLY);
    let ui_a = start(dir.path(), "a").await;
    let ui_c = start(dir.path(), "c").await;
    let (mut ev_a, mut ev_c) = (ui_a.subscribe(), ui_c.subscribe());

    let host = ok(
        &ui_a,
        json!({ "cmd": "pair_host_start", "args": { "port": 0 } }),
    )
    .await;
    let id = host["attempt_id"].as_u64().unwrap();
    let port = u16::try_from(host["port"].as_u64().unwrap()).unwrap();
    let relay = LossyRelay::start(SocketAddr::from(([127, 0, 0, 1], port))).await;
    ok(
        &ui_c,
        json!({ "cmd": "pair_join_start", "args": { "addr": relay.addr.to_string() } }),
    )
    .await;
    code_ready(&mut ev_a, "code on a").await;
    code_ready(&mut ev_c, "code on c").await;

    relay.lose_host_packets(Duration::from_millis(300));
    let t0 = tokio::time::Instant::now();
    ok(&ui_a, attempt("pair_cancel", id)).await;
    let rc = result(&mut ev_c, "the other side ends despite the loss").await;
    let took = t0.elapsed();
    assert!(
        took < KEEP_ALIVE,
        "c ended {took:?} after the cancel: it learned from the keep-alive, not from a"
    );
    // c read a's decision; a broken connection ("failed") is what the lost close used to leave.
    assert_eq!(rc.outcome, PairOutcomeName::DeclinedPeer);
    assert!(rc.reason.is_some());
    let r = result(&mut ev_a, "cancelled on a").await;
    assert_eq!((r.attempt_id, r.outcome), (id, PairOutcomeName::Cancelled));

    assert!(pa.load_peers().unwrap().peer.is_empty());
    assert!(pc.load_peers().unwrap().peer.is_empty());
    relay.task.abort();
    ui_a.shutdown().await;
    ui_c.shutdown().await;
}

/// A cancel before the code (here a joiner whose handshake never ends) used to leave the
/// refuser task holding the pairing endpoint and its port until the app quit.
#[tokio::test(flavor = "multi_thread")]
async fn cancel_before_the_code_frees_the_pairing_port() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", CLIPBOARD_ONLY);
    let (pc, _) = init_profile(dir.path(), "c", CLIPBOARD_ONLY);
    let ui_a = start(dir.path(), "a").await;
    let mut ev_a = ui_a.subscribe();

    let host = ok(
        &ui_a,
        json!({ "cmd": "pair_host_start", "args": { "port": 0 } }),
    )
    .await;
    let id = host["attempt_id"].as_u64().unwrap();
    let port = u16::try_from(host["port"].as_u64().unwrap()).unwrap();
    let pairing_addr = SocketAddr::from(([127, 0, 0, 1], port));
    let (relay, stalled, relay_task) = stall_handshake(pairing_addr).await;
    let joiner = tokio::spawn(async move {
        pairing::join(&pc, relay, |_: &PairPrompt| PairDecision {
            accept: false,
            grants: NONE,
        })
        .await
    });
    // The host answered: it took the connection and refuses every other one now.
    stalled.await.unwrap();
    assert_eq!(
        ok(&ui_a, json!({ "cmd": "pair_state" })).await["step"],
        json!("listening")
    );

    ok(&ui_a, attempt("pair_cancel", id)).await;
    let r = result(&mut ev_a, "cancelled before the code").await;
    assert_eq!((r.attempt_id, r.outcome), (id, PairOutcomeName::Cancelled));
    wait_until("the pairing port is free again", || async {
        std::net::UdpSocket::bind(pairing_addr).is_ok()
    })
    .await;
    joiner.abort();
    relay_task.abort();
    ui_a.shutdown().await;
}
