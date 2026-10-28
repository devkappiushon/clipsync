//! §13.11 item 2 (a to g) and the Sentinel's R2 (the hook goes away on four triggers), over
//! MemHub with the fake adapters.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_input::{
    Endpoint, FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle, Hotkey, InputConfig,
    Layout, LayoutDevice, LayoutOptions, Link, Zone, build_with_adapters, build_with_layout,
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
const BTN_LEFT: u32 = 0x110;

fn zone(w: u32, h: u32) -> Zone {
    Zone {
        index: 0,
        name: Some("eDP-1".into()),
        x: 0,
        y: 0,
        width: w,
        height: h,
        scale: 1.0,
    }
}

fn node_id(name: &str) -> PeerId {
    PeerId::from_spki_der(format!("memhub:{name}").as_bytes())
}

fn device(peer: PeerId, name: &str, hotkey: Option<u8>) -> LayoutDevice {
    LayoutDevice {
        peer,
        name: name.into(),
        x: 0,
        y: 0,
        hotkey,
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

/// `a`'s right edge (0.5..1.0) into the left edge of `b` over `b_from..b_to`.
fn layout_ab(a: PeerId, b: PeerId, b_from: f32, b_to: f32, dead: u32) -> Layout {
    Layout {
        rev: 1,
        options: LayoutOptions {
            dead_corner_px: dead,
        },
        devices: vec![device(a, "a", Some(1)), device(b, "b", Some(2))],
        links: vec![Link {
            id: "l1".into(),
            a: ep(a, Edge::Right, 0.5, 1.0),
            b: ep(b, Edge::Left, b_from, b_to),
        }],
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

/// A node that runs the real module, with the layout in `dir`.
fn spawn_src(hub: &MemHub, name: &str, dir: &Path, hooks: bool) -> Run {
    let node = hub.add_node(name, INPUT);
    let (cap_ad, cap) = FakeCapture::new(vec![zone(1920, 1080)]);
    cap.set_hooks(hooks);
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

/// A destination without layout: it only receives.
fn spawn_dest(hub: &MemHub, name: &str) -> Run {
    let node = hub.add_node(name, INPUT);
    let (cap_ad, cap) = FakeCapture::new(vec![zone(1920, 1080)]);
    let (emu_ad, emu) = FakeEmulate::new();
    let module = build_with_adapters(InputConfig::default(), Box::new(cap_ad), Box::new(emu_ad));
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

async fn send(raw: &Raw, to: PeerId, m: InputMsg) {
    raw.host
        .send_reliable(to, ModuleMsg::Input(m))
        .await
        .unwrap();
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
async fn status(run: &Run) -> ModuleStatus {
    let ModuleReply::Status(s) = command(&run.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
    s
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

/// Reads until a message matches, skipping the rest (a peer sends `Zones` and others first).
async fn wait_input<T>(raw: &mut Raw, ms: u64, pick: impl Fn(InputMsg) -> Option<T>) -> Option<T> {
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        let left = end.saturating_duration_since(Instant::now());
        let m = next_input(raw, left.as_millis() as u64).await?;
        if let Some(t) = pick(m) {
            return Some(t);
        }
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

fn mv(dx: f32, dy: f32) -> InputEvent {
    InputEvent::Motion { dx, dy }
}

/// Source `a` (real module, fake adapters) and destination `b` (real module).
async fn pair(b_from: f32, b_to: f32) -> (MemHub, tempfile::TempDir, Run, Run) {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), b_from, b_to, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), false);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    status(&a).await;
    status(&b).await;
    (hub, d, a, b)
}

/// Raw destination `b` that records what the source sends.
async fn src_with_raw_dest(
    b_from: f32,
    b_to: f32,
    dead: u32,
) -> (MemHub, tempfile::TempDir, Run, Raw) {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), b_from, b_to, dead),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), false);
    let b = raw(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    status(&a).await;
    (hub, d, a, b)
}

// ---- (a) locked destination

#[tokio::test]
async fn a_locked_destination_refuses_the_span_and_the_source_says_why() {
    let (_hub, _d, a, b) = pair(0.0, 1.0).await;
    b.emu.set_locked(true);
    a.cap.activate(0, 0.75);
    until("source gave the cursor back", || a.cap.released() >= 1).await;
    // nothing was applied at the destination and its local session is closed
    assert_eq!(b.emu.emitted(), 0);
    assert!(b.emu.release_calls() >= 1);
    let s = status(&a).await;
    assert_eq!(
        s.last_block.as_deref(),
        Some("b está com a tela bloqueada: o controle voltou para esta máquina"),
        "{s:?}"
    );
    // a datagram of the refused session is dropped and counted
    a.cap.push(mv(5.0, 0.0));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(b.emu.emitted(), 0);
}

#[tokio::test]
async fn a_paused_destination_refuses_with_the_paused_text() {
    let (hub, _d, a, b) = pair(0.0, 1.0).await;
    // the destination module is paused but the hub still lets the Enter through
    b.tx.send(ModuleInput::Paused(true)).await.unwrap();
    status(&b).await;
    a.cap.activate(0, 0.75);
    until("source gave the cursor back", || a.cap.released() >= 1).await;
    assert_eq!(b.emu.emitted(), 0);
    let s = status(&a).await;
    assert_eq!(
        s.last_block.as_deref(),
        Some("b está pausado: o controle voltou para esta máquina"),
        "{s:?}"
    );
    drop(hub);
}

// ---- (b) the way back

#[tokio::test]
async fn pushing_out_through_the_span_returns_at_the_mapped_position() {
    // the way back is the middle half of b's left edge, linked to a's 0.5..1.0
    let (_hub, _d, a, mut b) = src_with_raw_dest(0.25, 0.75, 0).await;
    a.cap.activate(0, 0.75);
    let (session, pos, back) = wait_input(&mut b, 2000, |m| match m {
        InputMsg::EnterSpan {
            session,
            edge: Edge::Left,
            pos,
            back_from,
            back_to,
            ..
        } => Some((session, pos, (back_from, back_to))),
        _ => None,
    })
    .await
    .expect("EnterSpan");
    assert!((pos - 0.5).abs() < 1e-5, "{pos}");
    assert!(
        (back.0 - 0.25).abs() < 1e-5 && (back.1 - 0.75).abs() < 1e-5,
        "{back:?}"
    );
    // the destination says the cursor left at 0.5: halfway through the way back, so 0.75 on `a`
    send(&b, a.id, InputMsg::Returned { session, pos: 0.5 }).await;
    until("release_at", || !a.cap.released_at().is_empty()).await;
    let at = a.cap.released_at();
    assert_eq!(at.len(), 1);
    assert_eq!(at[0].0, 0);
    assert!((at[0].1 - 0.75).abs() < 1e-5, "{at:?}");
    // the source closes the session
    assert!(
        wait_input(&mut b, 2000, |m| matches!(m, InputMsg::Leave { .. })
            .then_some(()))
        .await
        .is_some()
    );
}

#[tokio::test]
async fn a_returned_of_another_session_is_ignored() {
    let (_hub, _d, a, mut b) = src_with_raw_dest(0.25, 0.75, 0).await;
    a.cap.activate(0, 0.75);
    let session = wait_input(&mut b, 2000, |m| match m {
        InputMsg::EnterSpan { session, .. } => Some(session),
        _ => None,
    })
    .await
    .expect("EnterSpan");
    send(
        &b,
        a.id,
        InputMsg::Returned {
            session: session.wrapping_add(1),
            pos: 0.5,
        },
    )
    .await;
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let s = status(&a).await;
        if s.last_block
            .as_deref()
            .is_some_and(|t| t.contains("ignorada"))
        {
            break;
        }
        assert!(Instant::now() < end, "never counted as ignored: {s:?}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(a.cap.released_at().is_empty());
}

#[tokio::test]
async fn with_the_lock_on_a_returned_is_ignored() {
    let (_hub, _d, a, mut b) = src_with_raw_dest(0.25, 0.75, 0).await;
    a.cap.activate(0, 0.75);
    let session = wait_input(&mut b, 2000, |m| match m {
        InputMsg::EnterSpan { session, .. } => Some(session),
        _ => None,
    })
    .await
    .expect("EnterSpan");
    let ModuleReply::Done = command(&a.tx, ModuleCommand::InputLock { locked: true }).await else {
        panic!("lock")
    };
    send(&b, a.id, InputMsg::Returned { session, pos: 0.5 }).await;
    status(&a).await;
    assert!(a.cap.released_at().is_empty(), "the lock keeps the cursor");
}

#[tokio::test]
async fn the_destination_returns_only_through_the_way_back() {
    // raw source `a` drives a real destination `b`; the way back is the top fifth of the left edge
    let hub = MemHub::new();
    let mut a = raw(&hub, "a");
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    status(&b).await;
    let session = 7;
    send(
        &a,
        b.id,
        InputMsg::EnterSpan {
            session,
            edge: Edge::Left,
            monitor: Some("eDP-1".into()),
            pos: 0.1,
            back_from: 0.0,
            back_to: 0.2,
        },
    )
    .await;
    until("destination started", || {
        b.emu.started() && b.emu.emitted() > 0
    })
    .await;
    let mut seq = 0;
    let mut dgram = |events: Vec<InputEvent>| {
        seq += 1;
        dg(session, seq, events)
    };
    // a wall: outside the way back, pushing out of every edge does nothing
    a.host
        .send_datagram(b.id, dgram(vec![mv(0.0, 600.0)]))
        .unwrap(); // down to 0.6 of the edge
    for _ in 0..5 {
        a.host
            .send_datagram(b.id, dgram(vec![mv(-50.0, 0.0)]))
            .unwrap();
    }
    // the top edge, the other edge: pushing up does not return either
    a.host
        .send_datagram(b.id, dgram(vec![mv(0.0, -2000.0)]))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        wait_input(&mut a, 100, |m| matches!(m, InputMsg::Returned { .. })
            .then_some(()))
        .await
        .is_none(),
        "nothing outside the way back"
    );
    // at the top, inside the way back, a push out of the left edge returns
    for _ in 0..5 {
        a.host
            .send_datagram(b.id, dgram(vec![mv(-20.0, 0.0)]))
            .unwrap();
    }
    let (got_session, pos) = wait_input(&mut a, 2000, |m| match m {
        InputMsg::Returned { session, pos } => Some((session, pos)),
        _ => None,
    })
    .await
    .expect("Returned");
    assert_eq!(got_session, session);
    assert!((0.0..=0.2).contains(&pos), "{pos}");
    // and the destination let go of everything
    assert_eq!(b.emu.pressed_now(), 0);
}

// ---- (c) button held across

#[tokio::test]
async fn a_held_button_arrives_after_the_positioning_and_is_gone_after_the_return() {
    let (_hub, _d, a, b) = pair(0.0, 1.0).await;
    a.cap.activate(0, 0.75);
    a.cap.push(InputEvent::Button {
        code: BTN_LEFT,
        pressed: true,
    });
    until("button applied", || b.emu.pressed_now() == 1).await;
    let events = b.emu.events();
    let at = events
        .iter()
        .position(|e| matches!(e, InputEvent::Button { .. }))
        .expect("button");
    assert!(at >= 1, "positioned first: {events:?}");
    assert!(
        events[..at]
            .iter()
            .all(|e| matches!(e, InputEvent::Motion { .. }))
    );
    // the way back
    a.cap.push(mv(-200.0, 0.0));
    until("returned", || !a.cap.released_at().is_empty()).await;
    until("nothing held at the destination", || {
        b.emu.pressed_now() == 0
    })
    .await;
}

// ---- (d) shortcuts

#[tokio::test]
async fn shortcuts_switch_lock_and_pause() {
    let (hub, _d, a, b) = pair(0.0, 1.0).await;
    // GoTo(2): the device with hotkey 2 is `b`
    a.cap.hotkey(Hotkey::GoTo(2));
    until("session on b", || b.emu.started() && b.emu.emitted() > 0).await;
    // GoTo(1): the hotkey of this very device gives the cursor back
    let before = a.cap.released();
    a.cap.hotkey(Hotkey::GoTo(1));
    until("cursor back", || a.cap.released() > before).await;
    // ToggleLock flips the lock
    async fn screen_locked(run: &Run) -> bool {
        match command(&run.tx, ModuleCommand::InputState).await {
            ModuleReply::InputState(s) => s.screen_locked,
            other => panic!("{other:?}"),
        }
    }
    async fn wait_lock(run: &Run, want: bool) {
        let end = Instant::now() + Duration::from_secs(3);
        while screen_locked(run).await != want {
            assert!(Instant::now() < end, "lock never became {want}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    a.cap.hotkey(Hotkey::ToggleLock);
    wait_lock(&a, true).await;
    a.cap.hotkey(Hotkey::ToggleLock);
    wait_lock(&a, false).await;
    // TogglePause asks the host
    assert_eq!(hub.pause_requests(a.id), 0);
    a.cap.hotkey(Hotkey::TogglePause);
    until("pause requested", || hub.pause_requests(a.id) == 1).await;
}

// ---- (e) zones in real time

#[tokio::test]
async fn a_new_resolution_republishes_zones_and_moves_the_barriers() {
    let d = tempfile::tempdir().unwrap();
    // the link covers the whole right edge, so the dead corner shows in the barrier
    let mut l = layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 16);
    l.links[0].a = ep(node_id("a"), Edge::Right, 0.0, 1.0);
    write_layout(d.path(), &l);
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), false);
    let mut b = raw(&hub, "b");
    hub.set_remote_version(b.id, 2);
    hub.link(a.id, b.id, CONTROL, PeerPerms::default());
    status(&a).await;
    until("barriers installed", || !a.cap.barriers().is_empty()).await;
    let before = a.cap.barriers()[0].from;
    assert!((before - 16.0 / 1080.0).abs() < 1e-5, "{before}");
    // PeerUp brought the first Zones; drain it
    let first = wait_input(&mut b, 2000, |m| match m {
        InputMsg::Zones { zones } => Some(zones),
        _ => None,
    })
    .await
    .expect("first Zones");
    assert_eq!(first[0].height, 1080);
    // the same number of monitors, another resolution
    a.cap.set_zones(vec![zone(1280, 720)]);
    let again = wait_input(&mut b, 3000, |m| match m {
        InputMsg::Zones { zones } => Some(zones),
        _ => None,
    })
    .await
    .expect("new Zones within 3 s");
    assert_eq!((again[0].width, again[0].height), (1280, 720));
    until("barrier follows the new geometry", || {
        (a.cap.barriers()[0].from - 16.0 / 720.0).abs() < 1e-5
    })
    .await;
    // nothing changed: nothing goes out
    a.cap.set_zones(vec![zone(1280, 720)]);
    assert!(
        wait_input(&mut b, 2500, |m| matches!(m, InputMsg::Zones { .. })
            .then_some(()))
        .await
        .is_none(),
        "no Zones without a change"
    );
}

// ---- (f) screen lock at the source

#[tokio::test]
async fn a_locked_source_closes_the_session_and_blocks_activation() {
    let (_hub, _d, a, mut b) = src_with_raw_dest(0.0, 1.0, 0).await;
    a.cap.activate(0, 0.75);
    let session = wait_input(&mut b, 2000, |m| match m {
        InputMsg::EnterSpan { session, .. } => Some(session),
        _ => None,
    })
    .await
    .expect("EnterSpan");
    let before = a.cap.released();
    a.cap.session_locked(true);
    until("cursor back", || a.cap.released() > before).await;
    let mut saw_release_all = false;
    let mut saw_leave = false;
    while let Some(m) = next_input(&mut b, 500).await {
        match m {
            InputMsg::ReleaseAll { session: s } if s == session => saw_release_all = true,
            InputMsg::Leave { session: s } if s == session => saw_leave = true,
            _ => {}
        }
    }
    assert!(saw_release_all && saw_leave);
    // crossing a barrier while locked opens nothing
    a.cap.activate(0, 0.75);
    assert!(
        wait_input(&mut b, 400, |m| matches!(m, InputMsg::EnterSpan { .. })
            .then_some(()))
        .await
        .is_none()
    );
    // and works again after the unlock
    a.cap.session_locked(false);
    status(&a).await;
    a.cap.activate(0, 0.75);
    assert!(
        wait_input(&mut b, 2000, |m| matches!(m, InputMsg::EnterSpan { .. })
            .then_some(()))
        .await
        .is_some()
    );
}

// ---- (g) a protocol 2 peer

#[tokio::test]
async fn a_v2_peer_gets_enter_at_and_the_source_decides_the_way_back() {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), false);
    let mut b = raw(&hub, "b");
    hub.set_remote_version(b.id, 2);
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    status(&a).await;
    a.cap.activate(0, 0.75);
    a.cap.push(mv(50.0, 0.0));
    a.cap.push(mv(-70.0, 0.0));
    until("virtual cursor brought it back", || a.cap.released() == 1).await;
    let mut kinds = Vec::new();
    while let Some(m) = next_input(&mut b, 300).await {
        kinds.push(m);
    }
    assert!(kinds.iter().any(|m| matches!(m, InputMsg::EnterAt { .. })));
    assert!(
        kinds.iter().all(|m| !matches!(
            m,
            InputMsg::EnterSpan { .. } | InputMsg::Returned { .. } | InputMsg::Refused { .. }
        )),
        "no v3 variant to a v2 peer: {kinds:?}"
    );
    assert!(a.cap.released_at().is_empty());
}

// ---- Sentinel R2: the hook goes away on four triggers

#[tokio::test]
async fn the_capture_stops_when_the_last_local_link_leaves_layout_toml() {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), true);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    until("capture started", || a.cap.started()).await;
    // the link goes away
    let mut l = layout_ab(a.id, b.id, 0.0, 1.0, 0);
    l.links.clear();
    write_layout(d.path(), &l);
    let ModuleReply::Done = command(&a.tx, ModuleCommand::InputReload).await else {
        panic!("reload")
    };
    until("hook gone", || !a.cap.started()).await;
}

#[tokio::test]
async fn the_capture_stops_when_the_module_shuts_down() {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), true);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    until("capture started", || a.cap.started()).await;
    a.tx.send(ModuleInput::Shutdown).await.unwrap();
    until("hook gone", || !a.cap.started()).await;
}

#[tokio::test]
async fn pausing_removes_the_hook_and_the_resume_hotkey_brings_it_back() {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), true);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    until("capture started", || a.cap.started()).await;
    assert!(!a.cap.resume_watching());
    hub.set_paused(a.id, true);
    until("hook gone while paused", || !a.cap.started()).await;
    // only the system hotkey listens: it reads no keys
    until("watching the resume combination", || {
        a.cap.resume_watching()
    })
    .await;
    let starts = a.cap.starts();
    a.cap.resume_hotkey();
    until("resumed by the hotkey", || hub.pause_requests(a.id) == 1).await;
    until("hook back", || a.cap.started()).await;
    assert_eq!(a.cap.starts(), starts + 1);
    assert!(
        !a.cap.resume_watching(),
        "the registration goes away on resume"
    );
}

#[tokio::test]
async fn when_the_system_refuses_the_resume_hotkey_the_tray_stays_as_the_way_back() {
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), true);
    a.cap.refuse_resume_hotkey(true);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    until("capture started", || a.cap.started()).await;
    hub.set_paused(a.id, true);
    until("hook gone while paused", || !a.cap.started()).await;
    let s = status(&a).await;
    assert!(!a.cap.resume_watching());
    assert!(
        s.last_block
            .as_deref()
            .is_some_and(|t| t.contains("Win+Alt+P")),
        "{s:?}"
    );
    // the tray or the button resumes
    hub.set_paused(a.id, false);
    until("hook back", || a.cap.started()).await;
}

#[tokio::test]
async fn an_adapter_without_hooks_keeps_its_session_while_paused() {
    // the Wayland portal: a new capture session is a new dialog, so pausing does not drop it
    let d = tempfile::tempdir().unwrap();
    write_layout(
        d.path(),
        &layout_ab(node_id("a"), node_id("b"), 0.0, 1.0, 0),
    );
    let hub = MemHub::new();
    let a = spawn_src(&hub, "a", d.path(), false);
    let b = spawn_dest(&hub, "b");
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    until("capture started", || a.cap.started()).await;
    hub.set_paused(a.id, true);
    status(&a).await;
    assert!(a.cap.started());
    assert!(!a.cap.resume_watching());
    let _ = b;
}
