//! §12.9.3: continuous capture from `layout.toml`, `Zones`, `LayoutOffer`, `EnterAt`,
//! `InputState`, `InputReload`, `InputLock`, `InputSwitch` and `InputOffer` over MemHub with the
//! fake adapters.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_input::{
    BarrierConfig, Endpoint, FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle,
    InputConfig, Layout, LayoutDevice, LayoutOptions, Link, Zone, build_with_layout,
};
use clipsync_proto::*;
use std::path::Path;
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

/// Same derivation as `MemHub::add_node`.
fn node_id(name: &str) -> PeerId {
    PeerId::from_spki_der(format!("memhub:{name}").as_bytes())
}

fn device(peer: PeerId, name: &str) -> LayoutDevice {
    LayoutDevice {
        peer,
        name: name.into(),
        x: 0,
        y: 0,
        hotkey: None,
    }
}

fn ep(peer: PeerId, edge: Edge, from: f32, to: f32) -> Endpoint {
    Endpoint {
        peer,
        monitor: "eDP-1".into(),
        edge,
        from,
        to,
    }
}

/// `a`'s right edge, upper-half excluded (0.5..1.0), into the whole left edge of `b`.
fn layout_ab(a: PeerId, b: PeerId) -> Layout {
    Layout {
        rev: 3,
        options: LayoutOptions { dead_corner_px: 0 },
        devices: vec![device(a, "a"), device(b, "b")],
        links: vec![Link {
            id: "l1".into(),
            a: ep(a, Edge::Right, 0.5, 1.0),
            b: ep(b, Edge::Left, 0.0, 1.0),
        }],
    }
}

fn devices_only(a: PeerId, b: PeerId) -> Layout {
    Layout {
        links: Vec::new(),
        ..layout_ab(a, b)
    }
}

fn write_layout(dir: &Path, l: &Layout) {
    l.write(&dir.join("layout.toml")).unwrap();
}

struct Run {
    id: PeerId,
    tx: mpsc::Sender<ModuleInput>,
    cap: FakeCaptureHandle,
    emu: FakeEmulateHandle,
}

fn spawn_with_layout(hub: &MemHub, name: &str, dir: &Path) -> Run {
    let node = hub.add_node(name, INPUT);
    let (cap_ad, cap) = FakeCapture::new(vec![zone()]);
    let (emu_ad, emu) = FakeEmulate::new();
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(cap_ad),
        Box::new(emu_ad),
        dir,
    )
    .unwrap();
    tokio::spawn(module.run(node.host, node.rx));
    Run {
        id: node.id,
        tx: node.tx,
        cap,
        emu,
    }
}

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

async fn command(tx: &mpsc::Sender<ModuleInput>, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    tx.send(ModuleInput::Command { cmd, reply }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .expect("command timeout")
        .unwrap()
}

/// A Status round trip proves the module is running and has processed everything before.
async fn settle(run: &Run) {
    let ModuleReply::Status(_) = command(&run.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
}

async fn until(what: &str, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !cond() {
        assert!(Instant::now() < end, "timeout waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Next reliable input message at a raw node, or `None` after `ms`.
async fn next_input(raw: &mut Raw, ms: u64) -> Option<InputMsg> {
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        let left = end.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, raw.rx.recv()).await {
            Ok(Some(ModuleInput::Reliable {
                msg: ModuleMsg::Input(m),
                ..
            })) => return Some(m),
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
}

async fn next_rejection(raw: &mut Raw, ms: u64) -> Option<RejectReason> {
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        let left = end.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, raw.rx.recv()).await {
            Ok(Some(ModuleInput::Rejected { reason, .. })) => return Some(reason),
            Ok(Some(_)) => continue,
            _ => return None,
        }
    }
}

fn motion(dx: f32) -> InputEvent {
    InputEvent::Motion { dx, dy: 0.0 }
}

fn state(reply: ModuleReply) -> InputState {
    let ModuleReply::InputState(s) = reply else {
        panic!("expected InputState, got {reply:?}")
    };
    s
}

#[test]
fn manual_barrier_and_link_together_refuse_to_build() {
    let d = tempfile::tempdir().unwrap();
    write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
    let cfg = InputConfig {
        barriers: vec![BarrierConfig {
            monitor: "eDP-1".into(),
            edge: Edge::Right,
            from: 0.0,
            to: 0.5,
            peer: "b".into(),
            peer_edge: Edge::Left,
            peer_monitor: None,
            peer_from: None,
            peer_to: None,
        }],
        ..InputConfig::default()
    };
    let (cap, _) = FakeCapture::new(vec![zone()]);
    let (emu, _) = FakeEmulate::new();
    let err = build_with_layout(cfg, Box::new(cap), Box::new(emu), d.path())
        .err()
        .expect("conflict");
    let text = err.to_string();
    assert!(matches!(err, ModuleError::Config(_)), "{err:?}");
    assert!(text.contains("[[input.barrier]]"), "{text}");
    assert!(text.contains("layout.toml"), "{text}");
}

#[tokio::test]
async fn continuous_capture_enters_the_linked_peer_and_returns_through_the_entry_edge() {
    let d = tempfile::tempdir().unwrap();
    write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let b = spawn_with_layout(&hub, "b", d.path());
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&a).await;
    settle(&b).await;

    // crossing the barrier opens the session: the destination starts emulating
    a.cap.activate(0, 0.75);
    until("destination started", || b.emu.started()).await;
    let entry = b.emu.emitted();
    for _ in 0..5 {
        a.cap.push(motion(10.0));
    }
    until("events forwarded", || b.emu.emitted() >= entry + 5).await;
    let ModuleReply::Status(s) = command(&a.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    assert!(
        s.last_event
            .as_deref()
            .is_some_and(|t| t.contains("iniciado")),
        "{s:?}"
    );

    // pushing back through the entry edge (50 px in, 70 px out) ends it on both sides
    let releases = b.emu.release_calls();
    a.cap.push(motion(-70.0));
    until("cursor released at the source", || a.cap.released() == 1).await;
    until("destination released", || b.emu.release_calls() > releases).await;
    let ModuleReply::Status(s) = command(&a.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    assert!(
        s.last_event
            .as_deref()
            .is_some_and(|t| t.contains("voltou")),
        "{s:?}"
    );
    // a small wobble at the edge does not count as leaving
    a.cap.activate(0, 0.9);
    until("second session", || {
        b.emu.started() && a.cap.released() == 1
    })
    .await;
    a.cap.push(motion(-5.0));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(a.cap.released(), 1, "5 px against the edge is not a return");
}

#[tokio::test]
async fn enter_carries_the_mapped_position_and_the_monitor_only_for_a_v2_peer() {
    for version in [2u16, 1] {
        let d = tempfile::tempdir().unwrap();
        write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
        let hub = MemHub::new();
        let a = spawn_with_layout(&hub, "a", d.path());
        let mut b = raw(&hub, "b");
        hub.set_remote_version(b.id, version);
        hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
        settle(&a).await;
        // local 0.5..1.0 -> remote 0.0..1.0: 0.75 lands at 0.5
        a.cap.activate(0, 0.75);
        let m = next_input(&mut b, 2000).await.expect("enter");
        match (version, m) {
            (
                2,
                InputMsg::EnterAt {
                    edge,
                    monitor,
                    pos,
                    session,
                },
            ) => {
                assert_eq!(edge, Edge::Left);
                assert_eq!(monitor.as_deref(), Some("eDP-1"));
                assert!((pos - 0.5).abs() < 1e-5, "{pos}");
                assert_ne!(session, 0);
            }
            (1, InputMsg::Enter { edge, pos, session }) => {
                assert_eq!(edge, Edge::Left);
                assert!((pos - 0.5).abs() < 1e-5, "{pos}");
                assert_ne!(session, 0);
            }
            (v, other) => panic!("version {v}: unexpected {other:?}"),
        }
    }
}

#[tokio::test]
async fn lock_ignores_an_activation_and_switch_opens_a_session_on_the_linked_peer() {
    let d = tempfile::tempdir().unwrap();
    write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&a).await;

    assert!(matches!(
        command(&a.tx, ModuleCommand::InputLock { locked: true }).await,
        ModuleReply::Done
    ));
    a.cap.activate(0, 0.75);
    assert!(
        next_input(&mut b, 300).await.is_none(),
        "locked: no Enter leaves"
    );
    until("cursor given back", || a.cap.released() == 1).await;
    assert!(state(command(&a.tx, ModuleCommand::InputState).await).screen_locked);

    assert!(matches!(
        command(&a.tx, ModuleCommand::InputLock { locked: false }).await,
        ModuleReply::Done
    ));
    let reply = command(&a.tx, ModuleCommand::InputSwitch { to: b.id }).await;
    assert!(matches!(reply, ModuleReply::Done), "{reply:?}");
    match next_input(&mut b, 2000).await.expect("enter at") {
        InputMsg::EnterSpan {
            edge, monitor, pos, ..
        } => {
            assert_eq!(edge, Edge::Left);
            assert_eq!(monitor.as_deref(), Some("eDP-1"));
            assert!((pos - 0.5).abs() < 1e-5, "middle of the segment, got {pos}");
        }
        other => panic!("unexpected {other:?}"),
    }
    let st = state(command(&a.tx, ModuleCommand::InputState).await);
    assert_eq!(st.active_peer, Some(b.id));

    // a peer without a link
    let reply = command(
        &a.tx,
        ModuleCommand::InputSwitch {
            to: node_id("nobody"),
        },
    )
    .await;
    let ModuleReply::Error(e) = reply else {
        panic!("expected error, got {reply:?}")
    };
    assert!(e.contains("ligação"), "{e}");
}

#[tokio::test]
async fn zones_go_only_to_v2_peers_that_may_control_me() {
    // granted + v2: one Zones with my monitor
    let d = tempfile::tempdir().unwrap();
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, CONTROL, PeerPerms::default());
    match next_input(&mut b, 2000).await.expect("zones") {
        InputMsg::Zones { zones } => {
            assert_eq!(zones.len(), 1);
            assert_eq!(zones[0].name.as_deref(), Some("eDP-1"));
            assert_eq!((zones[0].width, zones[0].height), (1920, 1080));
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(hub.sent_count(a.id, b.id), 1);

    // not granted: nothing leaves
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), PeerPerms::default());
    settle(&a).await;
    assert!(next_input(&mut b, 300).await.is_none());
    assert_eq!(hub.sent_count(a.id, b.id), 0);

    // granted but a 0.1.0 peer: nothing leaves
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.set_remote_version(b.id, 1);
    hub.link(a.id, b.id, CONTROL, PeerPerms::default());
    settle(&a).await;
    assert!(next_input(&mut b, 300).await.is_none());
    assert_eq!(hub.sent_count(a.id, b.id), 0);
}

fn offer(a: PeerId, b: PeerId) -> InputMsg {
    InputMsg::LayoutOffer(LayoutOffer {
        rev: 9,
        devices: vec![
            OfferDevice {
                peer: a,
                x: 0,
                y: 0,
            },
            OfferDevice {
                peer: b,
                x: 1920,
                y: 0,
            },
        ],
        links: vec![OfferLink {
            a: OfferEndpoint {
                peer: a,
                monitor: "eDP-1".into(),
                edge: Edge::Right,
                from: 0.0,
                to: 1.0,
            },
            b: OfferEndpoint {
                peer: b,
                monitor: "eDP-1".into(),
                edge: Edge::Left,
                from: 0.0,
                to: 1.0,
            },
        }],
    })
}

#[tokio::test]
async fn layout_offer_needs_input_in_and_waits_as_pending_until_cleared() {
    let d = tempfile::tempdir().unwrap();
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    // a did not let b control it: the offer is refused at the door
    hub.link(a.id, b.id, PeerPerms::default(), PeerPerms::default());
    settle(&a).await;
    b.host
        .send_reliable(a.id, ModuleMsg::Input(offer(b.id, a.id)))
        .await
        .unwrap();
    assert_eq!(
        next_rejection(&mut b, 2000).await,
        Some(RejectReason::NotAuthorized)
    );
    settle(&a).await;
    assert!(
        state(command(&a.tx, ModuleCommand::InputState).await)
            .pending_offer
            .is_none()
    );

    // with input_in the offer waits for the user's answer
    hub.link(a.id, b.id, CONTROL, PeerPerms::default());
    b.host
        .send_reliable(a.id, ModuleMsg::Input(offer(b.id, a.id)))
        .await
        .unwrap();
    settle(&a).await;
    let st = state(command(&a.tx, ModuleCommand::InputState).await);
    let pending = st.pending_offer.expect("pending offer");
    assert_eq!(pending.from, b.id);
    assert_eq!(pending.offer.rev, 9);
    assert_eq!(pending.offer.links.len(), 1);
    assert!(matches!(
        command(&a.tx, ModuleCommand::InputOfferClear).await,
        ModuleReply::Done
    ));
    assert!(
        state(command(&a.tx, ModuleCommand::InputState).await)
            .pending_offer
            .is_none()
    );
}

#[tokio::test]
async fn input_offer_reports_offered_unsupported_denied_and_offline() {
    let d = tempfile::tempdir().unwrap();
    let (a_id, b_id) = (node_id("a"), node_id("b"));
    write_layout(d.path(), &layout_ab(a_id, b_id));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b"); // v2, lets a control it -> offered
    let c = raw(&hub, "c"); // 0.1.0 -> unsupported
    let d_node = raw(&hub, "d"); // v2 but no input_in for a -> denied
    hub.set_remote_version(c.id, 1);
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    hub.link(a.id, c.id, PeerPerms::default(), CONTROL);
    hub.link(a.id, d_node.id, PeerPerms::default(), PeerPerms::default());
    settle(&a).await;

    let reply = command(
        &a.tx,
        ModuleCommand::InputOffer {
            to: vec![b.id, c.id, d_node.id, node_id("e")],
        },
    )
    .await;
    let ModuleReply::InputOffered(out) = reply else {
        panic!("expected InputOffered, got {reply:?}")
    };
    let of = |p: PeerId| out.iter().find(|o| o.peer == p).map(|o| o.state);
    assert_eq!(of(b.id), Some(OfferState::Offered));
    assert_eq!(of(c.id), Some(OfferState::Unsupported));
    assert_eq!(of(d_node.id), Some(OfferState::Denied));
    assert_eq!(of(node_id("e")), Some(OfferState::Offline));
    assert_eq!(
        hub.sent_count(a.id, c.id),
        0,
        "nothing leaves for a 0.1.0 peer"
    );
    match next_input(&mut b, 2000).await.expect("offer") {
        InputMsg::LayoutOffer(o) => {
            assert_eq!(o.rev, 3);
            assert_eq!(o.devices.len(), 2);
            assert_eq!(o.links.len(), 1);
            assert_eq!(o.links[0].a.peer, a_id);
            assert_eq!(o.links[0].b.monitor, "eDP-1");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn input_reload_installs_the_barriers_of_the_new_file_and_keeps_the_old_on_error() {
    let d = tempfile::tempdir().unwrap();
    let (a_id, b_id) = (node_id("a"), node_id("b"));
    write_layout(d.path(), &devices_only(a_id, b_id));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&a).await;

    // no link: nothing captures
    a.cap.activate(0, 0.75);
    assert!(next_input(&mut b, 300).await.is_none());
    assert_eq!(a.cap.released(), 0, "no barrier was installed");

    // a broken file is refused and changes nothing
    std::fs::write(d.path().join("layout.toml"), "version = 1\nfoo = 1\n").unwrap();
    let ModuleReply::Error(e) = command(&a.tx, ModuleCommand::InputReload).await else {
        panic!("expected error")
    };
    assert!(e.contains("layout.toml"), "{e}");

    // the link arrives: the barrier is live without a restart
    write_layout(d.path(), &layout_ab(a_id, b_id));
    assert!(matches!(
        command(&a.tx, ModuleCommand::InputReload).await,
        ModuleReply::Done
    ));
    a.cap.activate(0, 0.75);
    assert!(matches!(
        next_input(&mut b, 2000).await,
        Some(InputMsg::EnterSpan { .. })
    ));
}

#[tokio::test]
async fn input_state_reports_zones_lock_and_the_active_peer() {
    let d = tempfile::tempdir().unwrap();
    write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let b = raw(&hub, "b");
    hub.link(a.id, b.id, CONTROL, CONTROL);
    settle(&a).await;
    let st = state(command(&a.tx, ModuleCommand::InputState).await);
    assert_eq!(st.local.len(), 1);
    assert_eq!(st.local[0].name.as_deref(), Some("eDP-1"));
    assert!(!st.screen_locked);
    assert_eq!(st.active_peer, None);
    assert!(st.peers.is_empty(), "b never sent Zones");

    // b announces its monitors
    b.host
        .send_reliable(
            a.id,
            ModuleMsg::Input(InputMsg::Zones {
                zones: vec![ZoneInfo {
                    index: 0,
                    name: Some("HDMI-1".into()),
                    x: 0,
                    y: 0,
                    width: 2560,
                    height: 1440,
                    scale: 1.0,
                }],
            }),
        )
        .await
        .unwrap();
    settle(&a).await;
    let st = state(command(&a.tx, ModuleCommand::InputState).await);
    assert_eq!(st.peers.len(), 1);
    assert_eq!(st.peers[0].peer, b.id);
    assert_eq!(st.peers[0].zones[0].width, 2560);

    a.cap.activate(0, 0.75);
    until("active peer", || true).await;
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let st = state(command(&a.tx, ModuleCommand::InputState).await);
        if st.active_peer == Some(b.id) {
            break;
        }
        assert!(Instant::now() < end, "active peer never set");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(matches!(
        command(&a.tx, ModuleCommand::InputLock { locked: true }).await,
        ModuleReply::Done
    ));
    until("released by lock", || a.cap.released() == 1).await;
    let st = state(command(&a.tx, ModuleCommand::InputState).await);
    assert!(st.screen_locked);
    assert_eq!(st.active_peer, None);
}

#[tokio::test]
async fn loopback_uses_the_link_barrier_and_maps_the_position() {
    let d = tempfile::tempdir().unwrap();
    write_layout(d.path(), &layout_ab(node_id("a"), node_id("b")));
    let hub = MemHub::new();
    let a = spawn_with_layout(&hub, "a", d.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&a).await;
    let pending = {
        let tx = a.tx.clone();
        let to = b.id;
        tokio::spawn(
            async move { command(&tx, ModuleCommand::InputLoopback { to, seconds: 5 }).await },
        )
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    a.cap.activate(0, 0.5);
    match next_input(&mut b, 2000).await.expect("enter") {
        InputMsg::Enter { edge, pos, .. } => {
            assert_eq!(edge, Edge::Left);
            assert!(
                pos.abs() < 1e-5,
                "start of the local segment maps to 0, got {pos}"
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    a.cap.deactivate();
    // the raw destination never answers StatsRequest: the loopback ends with an error
    let reply = pending.await.unwrap();
    assert!(matches!(reply, ModuleReply::Error(_)), "{reply:?}");
    // and the continuous capture is back: a new activation opens a session again
    // (the raw destination still has the loopback's Leave and StatsRequest queued)
    settle(&a).await;
    a.cap.activate(0, 0.75);
    let mut entered = false;
    while let Some(m) = next_input(&mut b, 2000).await {
        if matches!(m, InputMsg::EnterSpan { .. }) {
            entered = true;
            break;
        }
    }
    assert!(entered, "no EnterSpan after the loopback");
}

#[tokio::test]
async fn paused_bootstrap_with_link_never_starts_capture_until_explicit_resume() {
    for hooks in [false, true] {
        let hub = MemHub::new();
        let node = hub.add_node("paused-source", INPUT);
        let remote = hub.add_node("destination", INPUT);
        hub.link(node.id, remote.id, CONTROL, CONTROL);
        hub.set_paused(node.id, true);
        let directory = tempfile::tempdir().unwrap();
        write_layout(directory.path(), &layout_ab(node.id, remote.id));
        let (cap_ad, cap) = FakeCapture::new(vec![zone()]);
        cap.set_hooks(hooks);
        let (emu_ad, _) = FakeEmulate::new();
        let module = build_with_layout(
            InputConfig::default(),
            Box::new(cap_ad),
            Box::new(emu_ad),
            directory.path(),
        )
        .unwrap();
        let task = tokio::spawn(module.run(node.host, node.rx));
        command(&node.tx, ModuleCommand::Status).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(
            cap.starts(),
            0,
            "paused bootstrap installed capture, hooks={hooks}"
        );
        let reply = command(
            &node.tx,
            ModuleCommand::InputLoopback {
                to: remote.id,
                seconds: 1,
            },
        )
        .await;
        assert!(matches!(reply, ModuleReply::Error(_)));
        assert_eq!(cap.starts(), 0, "paused explicit test installed capture");
        hub.set_paused(node.id, false);
        until("capture starts only after resume", || cap.started()).await;
        node.tx.send(ModuleInput::Shutdown).await.unwrap();
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn revoking_local_input_in_preserves_the_independent_outgoing_session() {
    let hub = MemHub::new();
    let dir = tempfile::tempdir().unwrap();
    write_layout(dir.path(), &layout_ab(node_id("a"), node_id("b")));
    let a = spawn_with_layout(&hub, "a", dir.path());
    let mut b = raw(&hub, "b");
    hub.link(a.id, b.id, CONTROL, CONTROL);
    until("capture installed", || a.cap.started()).await;
    a.cap.activate(0, 0.7);
    loop {
        if matches!(
            next_input(&mut b, 2000).await,
            Some(InputMsg::EnterSpan { .. } | InputMsg::EnterAt { .. } | InputMsg::Enter { .. })
        ) {
            break;
        }
    }
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&a).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let ModuleReply::InputState(state) = command(&a.tx, ModuleCommand::InputState).await else {
        panic!("state");
    };
    assert_eq!(state.active_peer, Some(b.id));
    assert_eq!(a.cap.released(), 0);
    a.tx.send(ModuleInput::Shutdown).await.unwrap();
}
