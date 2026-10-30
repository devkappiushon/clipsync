//! §9 S2 item 1: module tests over MemHub + FakeClipboard.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_clipboard::{
    ClipMode, ClipSnapshot, ClipboardConfig, FakeClipboard, FakeClipboardHandle,
    build_with_adapter, msg,
};
use clipsync_proto::*;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

const ALL: PeerPerms = PeerPerms {
    clipboard_in: true,
    clipboard_out: true,
    input_in: false,
};
const BOTH: ModuleSet = ModuleSet {
    clipboard: true,
    input: false,
};

struct Clip {
    id: PeerId,
    fake: FakeClipboardHandle,
    tx: mpsc::Sender<ModuleInput>,
}

async fn spawn_clip(hub: &MemHub, name: &str, mode: ClipMode) -> Clip {
    let node = hub.add_node(name, BOTH);
    let (fake_ad, fake) = FakeClipboard::new();
    let cfg = ClipboardConfig {
        mode,
        ..ClipboardConfig::default()
    };
    let module = build_with_adapter(cfg, Box::new(fake_ad));
    let (host, rx) = (node.host.clone(), node.rx);
    tokio::spawn(module.run(host, rx));
    let c = Clip {
        id: node.id,
        fake,
        tx: node.tx,
    };
    // a Status round trip proves the module finished its startup read
    let ModuleReply::Status(_) = command(&c, ModuleCommand::Status).await else {
        panic!("status")
    };
    c
}

async fn until(what: &str, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !cond() {
        assert!(Instant::now() < end, "timeout waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(400)).await;
}

fn offer(lamport: u64, text: &str) -> ClipOffer {
    ClipOffer {
        id: EventId::random(),
        lamport,
        mime: MIME_TEXT_UTF8.into(),
        text: text.into(),
        created_unix_ms: 0,
        ttl_ms: CLIP_TTL_MS,
    }
}

async fn send_offer(from: &MemNode, to: PeerId, o: ClipOffer) {
    from.host
        .send_reliable(to, ModuleMsg::Clip(ClipMsg::Offer(o)))
        .await
        .unwrap();
}

async fn next_ack(n: &mut MemNode) -> ClipAck {
    loop {
        let i = tokio::time::timeout(Duration::from_secs(3), n.rx.recv())
            .await
            .unwrap()
            .unwrap();
        if let ModuleInput::Reliable {
            msg: ModuleMsg::Clip(ClipMsg::Ack(a)),
            ..
        } = i
        {
            return a;
        }
    }
}

async fn command(c: &Clip, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    c.tx.send(ModuleInput::Command { cmd, reply })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(8), rx)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn a_mesh_one_copy_two_offers_no_reoffer() {
    let hub = MemHub::new();
    let (a, b, c) = (
        spawn_clip(&hub, "a", ClipMode::Auto).await,
        spawn_clip(&hub, "b", ClipMode::Auto).await,
        spawn_clip(&hub, "c", ClipMode::Auto).await,
    );
    hub.link(a.id, b.id, ALL, ALL);
    hub.link(a.id, c.id, ALL, ALL);
    hub.link(b.id, c.id, ALL, ALL);
    a.fake.user_copy("texto sintético 1");
    until("b and c apply", || {
        b.fake.contents().is_some() && c.fake.contents().is_some()
    })
    .await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        (b.fake.writes(), c.fake.writes(), a.fake.writes()),
        (1, 1, 0)
    );
    assert_eq!(
        (hub.sent_count(a.id, b.id), hub.sent_count(a.id, c.id)),
        (1, 1),
        "2 offers in total"
    );
    assert_eq!(
        (hub.sent_count(b.id, c.id), hub.sent_count(c.id, b.id)),
        (0, 0),
        "no relay"
    );
    assert_eq!(
        (hub.sent_count(b.id, a.id), hub.sent_count(c.id, a.id)),
        (1, 1),
        "acks only"
    );
}

#[tokio::test]
async fn b_duplicate_event_id_applies_once_and_acks_duplicate() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    hub.link(a.id, b.id, ALL, ALL);
    let o = offer(1, "dup");
    send_offer(&a, b.id, o.clone()).await;
    send_offer(&a, b.id, o).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Applied);
    assert_eq!(
        next_ack(&mut a).await.state,
        DeliveryState::Rejected(RejectReason::Duplicate)
    );
    settle().await;
    assert_eq!(b.fake.writes(), 1);
}

#[tokio::test]
async fn c_old_lamport_after_new_is_stale_and_goes_to_inbox() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    hub.link(a.id, b.id, ALL, ALL);
    send_offer(&a, b.id, offer(5, "novo")).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Applied);
    send_offer(&a, b.id, offer(3, "antigo")).await;
    assert_eq!(
        next_ack(&mut a).await.state,
        DeliveryState::Rejected(RejectReason::Stale)
    );
    assert_eq!(b.fake.contents().as_deref(), Some("novo"));
    let ModuleReply::ClipInbox(items) = command(&b, ModuleCommand::ClipInbox).await else {
        panic!("inbox")
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].state, DeliveryState::Rejected(RejectReason::Stale));
}

#[tokio::test]
async fn d_x_y_x_makes_three_offers_x_x_makes_one() {
    let hub = MemHub::new();
    let (a, b) = (
        spawn_clip(&hub, "a", ClipMode::Auto).await,
        spawn_clip(&hub, "b", ClipMode::Auto).await,
    );
    hub.link(a.id, b.id, ALL, ALL);
    for (i, t) in ["X", "Y", "X"].into_iter().enumerate() {
        a.fake.user_copy(t);
        until("offer delivered", || hub.sent_count(a.id, b.id) == i + 1).await;
        until("applied", || {
            b.fake.contents().as_deref() == Some(t) && b.fake.writes() == i + 1
        })
        .await;
    }
    assert_eq!(hub.sent_count(a.id, b.id), 3);
    let hub = MemHub::new();
    let (a, b) = (
        spawn_clip(&hub, "a", ClipMode::Auto).await,
        spawn_clip(&hub, "b", ClipMode::Auto).await,
    );
    hub.link(a.id, b.id, ALL, ALL);
    a.fake.user_copy("X");
    until("first", || hub.sent_count(a.id, b.id) == 1).await;
    a.fake.user_copy("X");
    settle().await;
    assert_eq!(hub.sent_count(a.id, b.id), 1);
}

#[tokio::test]
async fn e_own_write_echo_takeover_and_empty_do_not_offer() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    let c = hub.add_node("c", BOTH); // observer of B's outbound traffic
    hub.link(a.id, b.id, ALL, ALL);
    hub.link(b.id, c.id, ALL, ALL);
    send_offer(&a, b.id, offer(1, "eco")).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Applied);
    send_offer(&a, b.id, offer(2, "eco")).await; // identical re-write
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Applied);
    assert_eq!(b.fake.writes(), 2);
    // e2: the compositor takes ownership; e3: clear and empty copy
    b.fake.owner_takeover();
    b.fake.clear();
    b.fake.user_copy("");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(hub.sent_count(b.id, c.id), 0, "B never offered anything");
    // an empty clipboard does not reset the known hash: copying "eco" again by hand is unchanged content
    b.fake.user_copy("eco");
    settle().await;
    assert_eq!(hub.sent_count(b.id, c.id), 0);
}

#[tokio::test]
async fn f_remote_module_disabled_is_reported() {
    let hub = MemHub::new();
    let a = spawn_clip(&hub, "a", ClipMode::Auto).await;
    let b = hub.add_node(
        "b",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    hub.link(a.id, b.id, ALL, ALL);
    let ModuleReply::ClipSent(out) = command(
        &a,
        ModuleCommand::ClipSend {
            text: "oi".into(),
            to: None,
        },
    )
    .await
    else {
        panic!("sent")
    };
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0].state,
        DeliveryState::Rejected(RejectReason::ModuleDisabled)
    );
    // a Rejected frame from the wire resolves the pending send the same way
    a.fake.user_copy("auto");
    settle().await;
    assert_eq!(
        hub.sent_count(a.id, b.id),
        0,
        "no offers to a peer without the module"
    );
}

#[tokio::test]
async fn g_paused_neither_sends_nor_applies_and_resume_does_not_resend() {
    let hub = MemHub::new();
    let (a, b) = (
        spawn_clip(&hub, "a", ClipMode::Auto).await,
        spawn_clip(&hub, "b", ClipMode::Auto).await,
    );
    hub.link(a.id, b.id, ALL, ALL);
    hub.set_paused(a.id, true);
    settle().await;
    a.fake.user_copy("durante a pausa");
    settle().await;
    assert_eq!(hub.sent_count(a.id, b.id), 0);
    hub.set_paused(a.id, false);
    settle().await;
    assert_eq!(hub.sent_count(a.id, b.id), 0, "resume does not resend");
    assert_eq!(b.fake.writes(), 0);
    // destination paused: nothing applied
    hub.set_paused(b.id, true);
    settle().await;
    a.fake.user_copy("b pausado");
    settle().await;
    assert_eq!(b.fake.writes(), 0);
}

#[tokio::test]
async fn h_manual_mode_only_fills_the_inbox() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Manual).await;
    hub.link(a.id, b.id, ALL, ALL);
    send_offer(&a, b.id, offer(1, "só caixa")).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::InInbox);
    assert_eq!((b.fake.writes(), b.fake.contents()), (0, None));
    let ModuleReply::ClipInbox(items) = command(&b, ModuleCommand::ClipInbox).await else {
        panic!("inbox")
    };
    assert_eq!(
        (items.len(), items[0].bytes, items[0].preview.as_str()),
        (1, "só caixa".len(), "só caixa")
    );
}

#[test]
fn i_debug_never_shows_content() {
    let s = ClipSnapshot {
        mime: MIME_TEXT_UTF8.into(),
        text: "segredo-xyz".into(),
    };
    assert!(!format!("{s:?}").contains("segredo-xyz"));
    let item = InboxItem {
        id: EventId::random(),
        from: "a".into(),
        received_unix_ms: 0,
        mime: "m".into(),
        bytes: 11,
        preview: "segredo-xyz".into(),
        state: DeliveryState::Applied,
    };
    assert!(!format!("{item:?}").contains("segredo-xyz"));
}

async fn clip_sent(c: &Clip, text: &str, to: Option<PeerId>) -> Vec<SendOutcome> {
    let cmd = ModuleCommand::ClipSend {
        text: text.into(),
        to,
    };
    match command(c, cmd).await {
        ModuleReply::ClipSent(v) => v,
        other => panic!("expected ClipSent, got {other:?}"),
    }
}

#[tokio::test]
async fn j_clipsend_lists_every_target_and_no_ack_is_unreachable() {
    let hub = MemHub::new();
    let a = spawn_clip(&hub, "a", ClipMode::Auto).await;
    let _silent = hub.add_node("silent", BOTH); // authorized, but nobody acks
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    let off = hub.add_node("off", BOTH); // no clipboard_out toward it: not a target for `None`
    let silent_id = _silent.id;
    hub.link(a.id, silent_id, ALL, ALL);
    hub.link(a.id, b.id, ALL, ALL);
    hub.link(
        a.id,
        off.id,
        PeerPerms {
            clipboard_out: false,
            ..ALL
        },
        ALL,
    );
    let out = clip_sent(&a, "para todos", None).await;
    let state_of = |n: &str| out.iter().find(|o| o.peer == n).map(|o| o.state);
    assert_eq!(out.len(), 2, "off is not a target");
    assert_eq!(state_of("b"), Some(DeliveryState::Applied));
    assert_eq!(state_of("silent"), Some(DeliveryState::Unreachable));
    // an explicit peer without permission is listed with the refusal
    let out = clip_sent(&a, "só ele", Some(off.id)).await;
    assert_eq!(
        out[0].state,
        DeliveryState::Rejected(RejectReason::NotAuthorized)
    );
    // rule 12: explicit sends never touch the local clipboard
    assert_eq!((a.fake.writes(), a.fake.contents()), (0, None));
    // no target at all, or an unknown --to: Error
    let stranger = PeerId([0xEE; 32]);
    assert!(matches!(
        command(
            &a,
            ModuleCommand::ClipSend {
                text: "x".into(),
                to: Some(stranger)
            }
        )
        .await,
        ModuleReply::Error(_)
    ));
}

#[tokio::test]
async fn k_ttl_counts_from_local_receipt_and_drops_the_text() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Manual).await;
    hub.link(a.id, b.id, ALL, ALL);
    // ttl 0 expires on arrival
    let mut zero = offer(1, "zero-ttl-texto");
    zero.ttl_ms = 0;
    send_offer(&a, b.id, zero).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Expired);
    // a short ttl: fine on arrival, Expired (and without preview) once older than the ttl
    let mut short = offer(2, "ttl-curto-texto");
    short.ttl_ms = 800;
    send_offer(&a, b.id, short).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::InInbox);
    let ModuleReply::ClipInbox(items) = command(&b, ModuleCommand::ClipInbox).await else {
        panic!("inbox")
    };
    assert_eq!(items[0].state, DeliveryState::InInbox);
    assert_eq!(items[0].preview, "ttl-curto-texto");
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let ModuleReply::ClipInbox(items) = command(&b, ModuleCommand::ClipInbox).await else {
        panic!("inbox")
    };
    assert!(
        items
            .iter()
            .all(|i| i.state == DeliveryState::Expired && i.preview.is_empty())
    );
    assert_eq!(items[0].bytes, "ttl-curto-texto".len());
}

#[tokio::test]
async fn l_pause_between_authorization_and_write_blocks_the_write() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    hub.link(a.id, b.id, ALL, ALL);
    b.fake.block_reads(true); // the worker stalls inside the read that precedes the write
    send_offer(&a, b.id, offer(1, "durante o bloqueio")).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    hub.set_paused(b.id, true);
    b.fake.block_reads(false);
    assert_eq!(
        next_ack(&mut a).await.state,
        DeliveryState::Rejected(RejectReason::Paused)
    );
    settle().await;
    assert_eq!((b.fake.writes(), b.fake.contents()), (0, None));
    let ModuleReply::ClipInbox(items) = command(&b, ModuleCommand::ClipInbox).await else {
        panic!("inbox")
    };
    assert_eq!(
        items[0].state,
        DeliveryState::Rejected(RejectReason::Paused)
    );
}

// ---- §12.9 item 4: ClipApply, ClipDiscard and ClipSendCurrent (L5) ----

async fn inbox_of(c: &Clip) -> Vec<InboxItem> {
    match command(c, ModuleCommand::ClipInbox).await {
        ModuleReply::ClipInbox(v) => v,
        other => panic!("expected ClipInbox, got {other:?}"),
    }
}

async fn apply(c: &Clip, id: EventId) -> ModuleReply {
    command(c, ModuleCommand::ClipApply { id }).await
}

fn is_error(reply: &ModuleReply, text: &str) -> bool {
    matches!(reply, ModuleReply::Error(e) if e == text)
}

#[tokio::test]
async fn m_clipapply_writes_an_inbox_item_without_a_new_offer() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Manual).await;
    let c = hub.add_node("c", BOTH); // B may send to it: a new offer would show up here
    hub.link(a.id, b.id, ALL, ALL);
    hub.link(b.id, c.id, ALL, ALL);
    let o = offer(1, "da caixa para o clipboard");
    send_offer(&a, b.id, o.clone()).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::InInbox);
    assert_eq!((b.fake.writes(), b.fake.contents()), (0, None));

    assert!(matches!(
        apply(&b, o.id).await,
        ModuleReply::ClipApplied(DeliveryState::Applied)
    ));
    assert_eq!(
        b.fake.contents().as_deref(),
        Some("da caixa para o clipboard")
    );
    assert_eq!(b.fake.writes(), 1);
    let items = inbox_of(&b).await;
    assert_eq!((items.len(), items[0].state), (1, DeliveryState::Applied));
    settle().await;
    assert_eq!(
        hub.sent_count(b.id, c.id),
        0,
        "a manual apply is not a local copy: no offer leaves"
    );
    // applied: the text is gone and a second apply is refused; an unknown id is refused too
    assert!(is_error(&apply(&b, o.id).await, msg::WRONG_STATE));
    assert!(is_error(&apply(&b, EventId::random()).await, msg::NO_ITEM));
    assert_eq!(b.fake.writes(), 1);
}

#[tokio::test]
async fn n_clipapply_works_while_paused_and_a_stale_item_qualifies() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    hub.link(a.id, b.id, ALL, ALL);
    send_offer(&a, b.id, offer(5, "novo")).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::Applied);
    let old = offer(3, "antigo");
    send_offer(&a, b.id, old.clone()).await;
    assert_eq!(
        next_ack(&mut a).await.state,
        DeliveryState::Rejected(RejectReason::Stale)
    );
    hub.set_paused(b.id, true);
    settle().await;
    assert!(matches!(
        apply(&b, old.id).await,
        ModuleReply::ClipApplied(DeliveryState::Applied)
    ));
    assert_eq!(b.fake.contents().as_deref(), Some("antigo"));
    assert_eq!(b.fake.writes(), 2);
    // the applied (auto mode) item was never eligible: no text kept for it
    let applied_first = inbox_of(&b)
        .await
        .into_iter()
        .find(|i| i.preview == "novo")
        .expect("first item listed");
    assert!(is_error(
        &apply(&b, applied_first.id).await,
        msg::WRONG_STATE
    ));
}

#[tokio::test]
async fn o_clipapply_refuses_an_expired_item_because_the_ttl_dropped_the_text() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Manual).await;
    hub.link(a.id, b.id, ALL, ALL);
    let mut short = offer(1, "some-no-ttl");
    short.ttl_ms = 800;
    send_offer(&a, b.id, short.clone()).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::InInbox);
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert!(is_error(&apply(&b, short.id).await, msg::WRONG_STATE));
    assert_eq!(inbox_of(&b).await[0].state, DeliveryState::Expired);
    assert_eq!((b.fake.writes(), b.fake.contents()), (0, None));
}

#[tokio::test]
async fn p_clipdiscard_removes_the_item_and_its_text() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let b = spawn_clip(&hub, "b", ClipMode::Manual).await;
    hub.link(a.id, b.id, ALL, ALL);
    let o = offer(1, "descartável");
    send_offer(&a, b.id, o.clone()).await;
    assert_eq!(next_ack(&mut a).await.state, DeliveryState::InInbox);
    assert_eq!(inbox_of(&b).await.len(), 1);
    assert!(matches!(
        command(&b, ModuleCommand::ClipDiscard { id: o.id }).await,
        ModuleReply::Done
    ));
    assert!(inbox_of(&b).await.is_empty());
    assert!(is_error(
        &command(&b, ModuleCommand::ClipDiscard { id: o.id }).await,
        msg::NO_ITEM
    ));
    assert!(is_error(&apply(&b, o.id).await, msg::NO_ITEM));
    assert_eq!((b.fake.writes(), b.fake.contents()), (0, None));
}

#[tokio::test]
async fn q_clipsendcurrent_sends_the_clipboard_to_clipboard_out_peers() {
    let hub = MemHub::new();
    let a = spawn_clip(&hub, "a", ClipMode::Auto).await;
    // copied before any peer is linked: no automatic offer leaves
    a.fake.user_copy("conteúdo atual");
    settle().await;
    let b = spawn_clip(&hub, "b", ClipMode::Auto).await;
    let off = hub.add_node("off", BOTH);
    hub.link(a.id, b.id, ALL, ALL);
    hub.link(
        a.id,
        off.id,
        PeerPerms {
            clipboard_out: false,
            ..ALL
        },
        ALL,
    );
    let ModuleReply::ClipSent(out) = command(&a, ModuleCommand::ClipSendCurrent { to: None }).await
    else {
        panic!("sent")
    };
    assert_eq!(out.len(), 1, "off has no clipboard_out");
    assert_eq!(
        (out[0].peer.as_str(), out[0].state),
        ("b", DeliveryState::Applied)
    );
    assert_eq!(b.fake.contents().as_deref(), Some("conteúdo atual"));
    assert_eq!(hub.sent_count(a.id, b.id), 1);
    // an explicit peer without permission is listed with the refusal, like ClipSend
    let ModuleReply::ClipSent(out) =
        command(&a, ModuleCommand::ClipSendCurrent { to: Some(off.id) }).await
    else {
        panic!("sent")
    };
    assert_eq!(
        out[0].state,
        DeliveryState::Rejected(RejectReason::NotAuthorized)
    );
    // empty clipboard: error with the public message
    a.fake.clear();
    settle().await;
    assert!(is_error(
        &command(&a, ModuleCommand::ClipSendCurrent { to: None }).await,
        msg::EMPTY_CLIPBOARD
    ));
}

#[tokio::test]
async fn r_remote_clock_rejection_does_not_poison_the_next_copy() {
    let hub = MemHub::new();
    let mut sender = hub.add_node("origem-relógio", BOTH);
    let receiver = spawn_clip(&hub, "destino-relógio", ClipMode::Auto).await;
    hub.link(sender.id, receiver.id, ALL, ALL);
    for timestamp in [0, u64::MAX, 1_000_001] {
        send_offer(&sender, receiver.id, offer(timestamp, "recusado sintético")).await;
        assert_eq!(
            next_ack(&mut sender).await.state,
            DeliveryState::Rejected(RejectReason::Invalid)
        );
        assert!(inbox_of(&receiver).await.is_empty());
        assert_eq!(
            (receiver.fake.writes(), receiver.fake.contents()),
            (0, None)
        );
    }
    receiver.fake.user_copy("cópia após recusa");
    loop {
        let input = tokio::time::timeout(Duration::from_secs(3), sender.rx.recv())
            .await
            .unwrap()
            .unwrap();
        if let ModuleInput::Reliable {
            msg: ModuleMsg::Clip(ClipMsg::Offer(o)),
            ..
        } = input
        {
            assert_eq!((o.lamport, o.text.as_str()), (1, "cópia após recusa"));
            break;
        }
    }
    assert!(matches!(
        command(&receiver, ModuleCommand::Status).await,
        ModuleReply::Status(_)
    ));
}

#[tokio::test]
async fn s_explicit_sends_do_not_claim_local_application() {
    let hub = MemHub::new();
    let mut source = hub.add_node("origem-explícita", BOTH);
    let sender = spawn_clip(&hub, "emissor-explícito", ClipMode::Auto).await;
    let target = spawn_clip(&hub, "destino-explícito", ClipMode::Auto).await;
    hub.link(source.id, sender.id, ALL, ALL);
    hub.link(sender.id, target.id, ALL, ALL);
    for text in ["primeiro", "segundo", "terceiro"] {
        let out = clip_sent(&sender, text, Some(target.id)).await;
        assert_eq!(out[0].state, DeliveryState::Applied);
    }
    assert_eq!((sender.fake.writes(), sender.fake.contents()), (0, None));
    // The local clock is now 3, but no origin has been applied/copied locally yet.
    send_offer(&source, sender.id, offer(1, "recebido menor que envios")).await;
    assert_eq!(next_ack(&mut source).await.state, DeliveryState::Applied);
    assert_eq!(
        (sender.fake.writes(), sender.fake.contents().as_deref()),
        (1, Some("recebido menor que envios"))
    );
}
