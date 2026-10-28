//! §9 S3 item 1 (a to e): module tests over MemHub + FakeCapture + FakeEmulate.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_input::{
    FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle, InputConfig, Zone,
    build_with_adapters,
};
use clipsync_proto::*;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

const INPUT: ModuleSet = ModuleSet {
    clipboard: false,
    input: true,
};
const CONTROL: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: true,
};

fn zone() -> Zone {
    Zone {
        index: 0,
        name: Some("eDP-1".into()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}

/// A node that runs the real input module.
struct Run {
    id: PeerId,
    tx: mpsc::Sender<ModuleInput>,
    cap: FakeCaptureHandle,
    emu: FakeEmulateHandle,
}

fn spawn_module(hub: &MemHub, name: &str) -> Run {
    let node = hub.add_node(name, INPUT);
    let (cap_ad, cap) = FakeCapture::new(vec![zone()]);
    let (emu_ad, emu) = FakeEmulate::new();
    let module = build_with_adapters(InputConfig::default(), Box::new(cap_ad), Box::new(emu_ad));
    let (host, rx) = (node.host.clone(), node.rx);
    tokio::spawn(module.run(host, rx));
    Run {
        id: node.id,
        tx: node.tx,
        cap,
        emu,
    }
}

async fn command(tx: &mpsc::Sender<ModuleInput>, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    tx.send(ModuleInput::Command { cmd, reply }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .expect("command timeout")
        .unwrap()
}

async fn until(what: &str, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !cond() {
        assert!(Instant::now() < end, "timeout waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn dg(session: u32, seq: u32, events: Vec<InputEvent>) -> InputDatagram {
    InputDatagram {
        session,
        seq,
        t_mono_us: mono_us(),
        events,
    }
}

fn key(code: u32, pressed: bool) -> InputEvent {
    InputEvent::Key { code, pressed }
}

fn motion() -> InputEvent {
    InputEvent::Motion { dx: 1.0, dy: 0.0 }
}

/// Raw controller: a bare node (no module) driving the destination by hand.
struct Raw {
    id: PeerId,
    host: std::sync::Arc<dyn ModuleHost>,
    rx: mpsc::Receiver<ModuleInput>,
}

fn raw(hub: &MemHub, name: &str) -> Raw {
    let n = hub.add_node(name, INPUT);
    Raw {
        id: n.id,
        host: n.host,
        rx: n.rx,
    }
}

async fn send(raw: &Raw, to: PeerId, m: InputMsg) {
    raw.host
        .send_reliable(to, ModuleMsg::Input(m))
        .await
        .unwrap();
}

/// Destination `b` that grants input control to a raw controller `a` (or not).
async fn dest_with_raw(grant: bool) -> (MemHub, Raw, Run) {
    let hub = MemHub::new();
    let a = raw(&hub, "ctl");
    let b = spawn_module(&hub, "dest");
    let perms = if grant { CONTROL } else { PeerPerms::default() };
    hub.link(a.id, b.id, PeerPerms::default(), perms);
    // a Status round trip proves the module is running and saw PeerUp
    let ModuleReply::Status(_) = command(&b.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    (hub, a, b)
}

#[tokio::test]
async fn loopback_counts_every_event_and_orders_percentiles() {
    let hub = MemHub::new();
    let a = spawn_module(&hub, "source");
    let b = spawn_module(&hub, "dest");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    // let both modules see PeerUp
    for r in [&a, &b] {
        let ModuleReply::Status(_) = command(&r.tx, ModuleCommand::Status).await else {
            panic!("status")
        };
    }

    let pending = {
        let tx = a.tx.clone();
        let to = b.id;
        tokio::spawn(
            async move { command(&tx, ModuleCommand::InputLoopback { to, seconds: 10 }).await },
        )
    };
    a.cap.activate(0, 0.25);
    for i in 0..200u32 {
        match i % 4 {
            0 => a.cap.push(key(30, true)),
            1 => a.cap.push(key(30, false)),
            _ => a.cap.push(motion()),
        }
    }
    a.cap.deactivate();

    let ModuleReply::InputLoopback(stats) = pending.await.unwrap() else {
        panic!("expected InputLoopback")
    };
    assert_eq!(stats.received, 200);
    assert!(stats.p50_us <= stats.p95_us, "{stats:?}");
    assert!(stats.p95_us <= stats.max_us, "{stats:?}");
    assert_eq!(stats.lost, 0);
    assert!(b.emu.started());
    assert_eq!(b.emu.pressed_now(), 0);
    assert_eq!(a.cap.released(), 1);
}

#[tokio::test]
async fn loopback_without_activation_reports_it() {
    let hub = MemHub::new();
    let a = spawn_module(&hub, "source");
    let b = spawn_module(&hub, "dest");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    let ModuleReply::Status(_) = command(&a.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    let reply = command(
        &a.tx,
        ModuleCommand::InputLoopback {
            to: b.id,
            seconds: 1,
        },
    )
    .await;
    assert!(matches!(reply, ModuleReply::Error(_)), "{reply:?}");
    assert!(!b.emu.started());
}

#[tokio::test]
async fn destination_without_input_in_refuses_enter_and_never_starts_the_emulator() {
    let (_hub, mut a, b) = dest_with_raw(false).await;
    send(
        &a,
        b.id,
        InputMsg::Enter {
            session: 7,
            edge: Edge::Left,
            pos: 0.5,
        },
    )
    .await;
    let rejected = loop {
        match tokio::time::timeout(Duration::from_secs(2), a.rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ModuleInput::Rejected { reason, .. } => break reason,
            _ => continue,
        }
    };
    assert_eq!(rejected, RejectReason::NotAuthorized);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!b.emu.started());
    assert_eq!(b.emu.emitted(), 0);
}

#[tokio::test]
async fn leave_and_peer_down_release_everything() {
    let (_hub, a, b) = dest_with_raw(true).await;
    let enter = |s| InputMsg::Enter {
        session: s,
        edge: Edge::Left,
        pos: 0.5,
    };

    send(&a, b.id, enter(7)).await;
    until("emulator started", || b.emu.started()).await;
    a.host
        .send_datagram(b.id, dg(7, 0, vec![key(30, true), key(31, true)]))
        .unwrap();
    until("keys pressed", || b.emu.pressed_now() == 2).await;

    // Leave from a session the peer does not own is ignored
    send(&a, b.id, InputMsg::Leave { session: 999 }).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(b.emu.pressed_now(), 2);
    let ModuleReply::Status(s) = command(&b.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    assert!(s.last_block.is_some(), "ignored messages are counted");

    send(&a, b.id, InputMsg::Leave { session: 7 }).await;
    until("released by Leave", || b.emu.pressed_now() == 0).await;

    send(&a, b.id, enter(8)).await;
    a.host
        .send_datagram(b.id, dg(8, 0, vec![key(30, true)]))
        .unwrap();
    until("key pressed again", || b.emu.pressed_now() == 1).await;
    b.tx.send(ModuleInput::PeerDown(a.id)).await.unwrap();
    until("released by PeerDown", || b.emu.pressed_now() == 0).await;
}

#[tokio::test]
async fn release_all_only_from_the_session_owner() {
    let (_hub, a, b) = dest_with_raw(true).await;
    send(
        &a,
        b.id,
        InputMsg::Enter {
            session: 5,
            edge: Edge::Top,
            pos: 0.0,
        },
    )
    .await;
    a.host
        .send_datagram(b.id, dg(5, 0, vec![key(30, true)]))
        .unwrap();
    until("key pressed", || b.emu.pressed_now() == 1).await;
    send(&a, b.id, InputMsg::ReleaseAll { session: 6 }).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(b.emu.pressed_now(), 1);
    send(&a, b.id, InputMsg::ReleaseAll { session: 5 }).await;
    until("released", || b.emu.pressed_now() == 0).await;
}

#[tokio::test]
async fn stats_request_for_foreign_session_gets_no_answer() {
    let (_hub, mut a, b) = dest_with_raw(true).await;
    send(&a, b.id, InputMsg::StatsRequest { session: 42 }).await;
    // (the destination may announce its Zones: only Stats counts here)
    let got = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            if let Some(ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::Stats(_)),
                ..
            }) = a.rx.recv().await
            {
                return true;
            }
        }
    })
    .await;
    assert!(got.is_err(), "no Stats for a session the peer never opened");
}

#[tokio::test]
async fn rate_limit_drops_the_excess_and_counts_it() {
    let (_hub, mut a, b) = dest_with_raw(true).await;
    send(
        &a,
        b.id,
        InputMsg::Enter {
            session: 3,
            edge: Edge::Left,
            pos: 0.5,
        },
    )
    .await;
    until("started", || b.emu.started()).await;
    let total = MAX_INPUT_DATAGRAMS_PER_SEC + 500;
    let started = Instant::now();
    for seq in 0..total {
        a.host
            .send_datagram(b.id, dg(3, seq, vec![motion()]))
            .unwrap();
        if seq % 100 == 0 {
            tokio::task::yield_now().await;
        }
    }
    send(&a, b.id, InputMsg::StatsRequest { session: 3 }).await;
    let stats = loop {
        match tokio::time::timeout(Duration::from_secs(3), a.rx.recv())
            .await
            .unwrap()
            .unwrap()
        {
            ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::Stats(s)),
                ..
            } => break s,
            _ => continue,
        }
    };
    // Token bucket: the budget plus whatever refilled while the burst ran. The window runs to
    // the answer, which is later than the last datagram, so the ceiling is never too tight.
    let window = started.elapsed();
    assert!(
        stats.received <= bucket_ceiling(window) as u64,
        "{stats:?} in {window:?}, the bucket allows {}",
        bucket_ceiling(window)
    );
    assert!(stats.received > 0);
    let ModuleReply::Status(s) = command(&b.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    assert!(
        s.last_block.is_some_and(|t| t.contains("taxa")),
        "the drop is reported"
    );
}

#[tokio::test]
async fn paused_destination_drops_datagrams_and_needs_a_new_enter() {
    let (hub, a, b) = dest_with_raw(true).await;
    send(
        &a,
        b.id,
        InputMsg::Enter {
            session: 9,
            edge: Edge::Left,
            pos: 0.5,
        },
    )
    .await;
    until("started", || b.emu.started()).await;
    a.host
        .send_datagram(b.id, dg(9, 0, vec![key(30, true)]))
        .unwrap();
    until("key pressed", || b.emu.pressed_now() == 1).await;

    hub.set_paused(b.id, true);
    until("released on pause", || b.emu.pressed_now() == 0).await;
    let emitted = b.emu.emitted();
    a.host
        .send_datagram(b.id, dg(9, 1, vec![motion(), key(31, true)]))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(b.emu.emitted(), emitted, "paused: datagram dropped");

    // resuming does not revive the old session
    hub.set_paused(b.id, false);
    tokio::time::sleep(Duration::from_millis(50)).await;
    a.host
        .send_datagram(b.id, dg(9, 2, vec![motion()]))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(b.emu.emitted(), emitted);
}

/// Waits until the emulator stops moving. Returns its count and how long after `since` it last
/// moved: the window in which the module could have been refilling its bucket.
async fn settle(emu: &FakeEmulateHandle, since: Instant) -> (usize, Duration) {
    let mut count = emu.emitted();
    let mut moved_at = since.elapsed();
    let mut quiet_since = Instant::now();
    while quiet_since.elapsed() < Duration::from_millis(150) {
        tokio::time::sleep(Duration::from_millis(5)).await;
        let now = emu.emitted();
        if now != count {
            count = now;
            moved_at = since.elapsed();
            quiet_since = Instant::now();
        }
    }
    (count, moved_at)
}

/// The token bucket starts full (`budget`) and refills `MAX_INPUT_DATAGRAMS_PER_SEC` per second,
/// so in a window of `w` seconds it accepts at most `budget + rate * w`. Small slack for the
/// 5 ms sampling and float rounding.
fn bucket_ceiling(window: Duration) -> usize {
    let rate = f64::from(MAX_INPUT_DATAGRAMS_PER_SEC);
    MAX_INPUT_DATAGRAMS_PER_SEC as usize + (rate * window.as_secs_f64()).ceil() as usize + 20
}

#[tokio::test]
async fn rate_limit_survives_a_new_enter() {
    let (_hub, a, b) = dest_with_raw(true).await;
    let enter = |s| InputMsg::Enter {
        session: s,
        edge: Edge::Left,
        pos: 0.5,
    };
    send(&a, b.id, enter(1)).await;
    until("started", || b.emu.started()).await;
    let started = Instant::now();
    let mut sent = 0u32;
    for session in [1u32, 2] {
        if session == 2 {
            send(&a, b.id, enter(2)).await;
        }
        for seq in 0..3000u32 {
            a.host
                .send_datagram(b.id, dg(session, seq, vec![motion()]))
                .unwrap();
            sent += 1;
            if seq % 100 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }
    let (total, window) = settle(&b.emu, started).await;
    assert_eq!(sent, 6000);
    // each Enter emits 3 positioning events; the rest are datagram events
    let accepted = total - 6;
    let budget = MAX_INPUT_DATAGRAMS_PER_SEC as usize;
    // The ceiling follows the time the run really took, so a slow runner cannot fail it. It
    // still has to leave room under `sent`, or it would prove nothing.
    let ceiling = bucket_ceiling(window);
    println!("rajada: aceitou {accepted} de {sent} em {window:?}, teto pelo tempo {ceiling}");
    assert!(
        ceiling < sent as usize,
        "run took {window:?}: too slow to tell a limiter from none"
    );
    assert!(
        accepted <= ceiling,
        "accepted {accepted} of {sent} in {window:?}, the bucket allows {ceiling}"
    );
    assert!(accepted >= budget - 100, "accepted {accepted} of {sent}");
}

/// The case that broke the fixed margin of `budget + 60`: a sender that takes its time. A pause
/// of 100 ms refills about 200 tokens, which is the limiter working, not leaking.
#[tokio::test]
async fn rate_limit_refills_at_the_documented_rate_when_the_sender_is_slow() {
    let (_hub, a, b) = dest_with_raw(true).await;
    send(
        &a,
        b.id,
        InputMsg::Enter {
            session: 1,
            edge: Edge::Left,
            pos: 0.5,
        },
    )
    .await;
    until("started", || b.emu.started()).await;
    let started = Instant::now();
    for (burst, seqs) in [(0u32, 0..1500u32), (1, 1500..3000)] {
        if burst == 1 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        for seq in seqs {
            a.host
                .send_datagram(b.id, dg(1, seq, vec![motion()]))
                .unwrap();
            if seq % 100 == 0 {
                tokio::task::yield_now().await;
            }
        }
    }
    let (total, window) = settle(&b.emu, started).await;
    let accepted = total - 3;
    let budget = MAX_INPUT_DATAGRAMS_PER_SEC as usize;
    assert!(window >= Duration::from_millis(100), "{window:?}");
    println!(
        "sender lento: aceitou {accepted} em {window:?}, teto antigo {}, teto pelo tempo {}",
        budget + 60,
        bucket_ceiling(window)
    );
    // Past the old fixed margin, because the pause refilled the bucket ...
    assert!(
        accepted >= budget + 100,
        "accepted {accepted}: the pause of 100 ms should have refilled the bucket"
    );
    // ... and never past what the rate allows for the time that passed.
    assert!(
        accepted <= bucket_ceiling(window),
        "accepted {accepted} in {window:?}, the bucket allows {}",
        bucket_ceiling(window)
    );
}
