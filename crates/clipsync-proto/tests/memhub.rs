// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use clipsync_proto::*;
use tokio::sync::mpsc::Receiver;
use tokio::time::timeout;

const BOTH: ModuleSet = ModuleSet {
    clipboard: true,
    input: true,
};
const NO: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: false,
};
const CLIP_OUT: PeerPerms = PeerPerms {
    clipboard_out: true,
    ..NO
};
const CLIP_IN: PeerPerms = PeerPerms {
    clipboard_in: true,
    ..NO
};

fn offer(text: &str) -> (EventId, ModuleMsg) {
    let id = EventId::random();
    let msg = ModuleMsg::Clip(ClipMsg::Offer(ClipOffer {
        id,
        lamport: 1,
        mime: MIME_TEXT_UTF8.into(),
        text: text.into(),
        created_unix_ms: 0,
        ttl_ms: CLIP_TTL_MS,
    }));
    (id, msg)
}

fn dg(n: usize) -> InputDatagram {
    InputDatagram {
        session: 1,
        seq: 1,
        t_mono_us: mono_us(),
        events: vec![InputEvent::Motion { dx: 1.0, dy: 1.0 }; n],
    }
}

async fn next(rx: &mut Receiver<ModuleInput>) -> ModuleInput {
    timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("closed")
}

fn assert_empty(rx: &mut Receiver<ModuleInput>) {
    assert!(rx.try_recv().is_err(), "unexpected input");
}

/// A grants clipboard_out to B; B grants clipboard_in to A. PeerUp already drained.
fn pair(
    hub: &MemHub,
    a_mods: ModuleSet,
    b_mods: ModuleSet,
    b_grants: PeerPerms,
) -> (MemNode, MemNode) {
    let mut a = hub.add_node("a", a_mods);
    let mut b = hub.add_node("b", b_mods);
    let (a_id, b_id) = (a.id, b.id);
    hub.link(a_id, b_id, CLIP_OUT, b_grants);
    for (node, other) in [(&mut a, b_id), (&mut b, a_id)] {
        match node.rx.try_recv() {
            Ok(ModuleInput::PeerUp(info)) => assert_eq!(info.id, other),
            other => panic!("expected PeerUp, got {other:?}"),
        }
    }
    (a, b)
}

#[tokio::test]
async fn link_delivers_peer_up_with_grants_and_remote_modules() {
    let hub = MemHub::new();
    let only_clip = ModuleSet {
        clipboard: true,
        input: false,
    };
    let mut a = hub.add_node("a", BOTH);
    let mut b = hub.add_node("b", only_clip);
    hub.link(a.id, b.id, CLIP_OUT, CLIP_IN);
    let ModuleInput::PeerUp(info) = next(&mut a.rx).await else {
        panic!()
    };
    assert_eq!(
        (info.id, info.name.as_str(), info.perms, info.remote_modules),
        (b.id, "b", CLIP_OUT, only_clip)
    );
    assert!(info.connected);
    let ModuleInput::PeerUp(info) = next(&mut b.rx).await else {
        panic!()
    };
    assert_eq!(
        (info.id, info.perms, info.remote_modules),
        (a.id, CLIP_IN, BOTH)
    );
    let peers = a.host.peers();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].id, b.id);
    assert_eq!(a.host.local_id(), a.id);
    assert_eq!(a.host.local_name(), "a");
}

#[tokio::test]
async fn delivers_authorized_offer() {
    let hub = MemHub::new();
    let (a, mut b) = pair(&hub, BOTH, BOTH, CLIP_IN);
    let (id, msg) = offer("olá");
    a.host.send_reliable(b.id, msg).await.unwrap();
    match next(&mut b.rx).await {
        ModuleInput::Reliable {
            from,
            msg: ModuleMsg::Clip(ClipMsg::Offer(o)),
        } => {
            assert_eq!(from, a.id);
            assert_eq!(o.id, id);
            assert_eq!(o.text, "olá");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(hub.sent_count(a.id, b.id), 1);
}

#[tokio::test]
async fn inbound_refusal_becomes_rejected_at_sender() {
    let hub = MemHub::new();
    let (mut a, mut b) = pair(&hub, BOTH, BOTH, NO); // B did not grant clipboard_in
    let (id, msg) = offer("x");
    a.host.send_reliable(b.id, msg).await.unwrap();
    match next(&mut a.rx).await {
        ModuleInput::Rejected {
            from,
            ref_id,
            reason,
        } => {
            assert_eq!(
                (from, ref_id, reason),
                (b.id, Some(id), RejectReason::NotAuthorized)
            );
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_empty(&mut b.rx);
    assert_eq!(hub.sent_count(a.id, b.id), 0);
}

#[tokio::test]
async fn paused_destination_rejects_offer_but_ack_passes() {
    let hub = MemHub::new();
    let (mut a, mut b) = pair(&hub, BOTH, BOTH, CLIP_IN);
    hub.set_paused(b.id, true);
    assert!(matches!(next(&mut b.rx).await, ModuleInput::Paused(true)));
    assert!(b.host.paused());
    let (_, msg) = offer("x");
    a.host.send_reliable(b.id, msg).await.unwrap();
    assert!(matches!(
        next(&mut a.rx).await,
        ModuleInput::Rejected {
            reason: RejectReason::Paused,
            ..
        }
    ));
    // B is paused and still acks; A is not paused and receives it.
    let ack = ModuleMsg::Clip(ClipMsg::Ack(ClipAck {
        id: EventId::random(),
        state: DeliveryState::Received,
    }));
    b.host.send_reliable(a.id, ack).await.unwrap();
    assert!(matches!(
        next(&mut a.rx).await,
        ModuleInput::Reliable {
            msg: ModuleMsg::Clip(ClipMsg::Ack(_)),
            ..
        }
    ));
    assert_eq!(hub.sent_count(b.id, a.id), 1);
}

#[tokio::test]
async fn outbound_rules_return_errors() {
    let hub = MemHub::new();
    let only_input = ModuleSet {
        clipboard: false,
        input: true,
    };
    let (a, b) = pair(&hub, BOTH, only_input, CLIP_IN);
    let (_, msg) = offer("x");
    assert_eq!(
        a.host.send_reliable(b.id, msg).await,
        Err(SendError::RemoteModuleDisabled)
    );
    let (_, msg) = offer("x");
    assert_eq!(
        b.host.send_reliable(a.id, msg).await,
        Err(SendError::LocalModuleDisabled)
    );
    hub.set_paused(a.id, true);
    let (_, msg) = offer("x");
    assert_eq!(
        a.host.send_reliable(b.id, msg).await,
        Err(SendError::RemoteModuleDisabled)
    );
    let c = hub.add_node("c", BOTH);
    let (_, msg) = offer("x");
    assert_eq!(
        a.host.send_reliable(c.id, msg).await,
        Err(SendError::NotConnected)
    );
    assert_eq!(
        a.host.send_datagram(c.id, dg(1)),
        Err(SendError::NotConnected)
    );
}

#[tokio::test]
async fn invalid_or_oversized_message_is_not_sent() {
    let hub = MemHub::new();
    let (a, mut b) = pair(&hub, BOTH, BOTH, CLIP_IN);
    let ModuleMsg::Clip(ClipMsg::Offer(mut o)) = offer("x").1 else {
        panic!()
    };
    o.text = "x".repeat(MAX_CLIP_TEXT_BYTES + 1);
    let res = a
        .host
        .send_reliable(b.id, ModuleMsg::Clip(ClipMsg::Offer(o)))
        .await;
    assert!(matches!(res, Err(SendError::Transport(_))), "{res:?}");
    assert_eq!(
        a.host.send_datagram(b.id, dg(33)),
        Err(SendError::Transport("invalid: too many events".into()))
    );
    assert_empty(&mut b.rx);
}

#[tokio::test]
async fn datagrams_follow_the_same_rules() {
    let hub = MemHub::new();
    let input_in = PeerPerms {
        input_in: true,
        ..NO
    };
    let (a, mut b) = pair(&hub, BOTH, BOTH, input_in);
    a.host.send_datagram(b.id, dg(3)).unwrap();
    match next(&mut b.rx).await {
        ModuleInput::Datagram { from, dg } => assert_eq!((from, dg.events.len()), (a.id, 3)),
        other => panic!("unexpected {other:?}"),
    }
    // Destination without input_in: silently dropped, no reply.
    let (mut c, d) = pair(&MemHub::new(), BOTH, BOTH, NO);
    c.host.send_datagram(d.id, dg(1)).unwrap();
    tokio::task::yield_now().await;
    let mut d = d;
    assert_empty(&mut d.rx);
    assert_empty(&mut c.rx);
    // Paused sender: refused locally.
    hub.set_paused(a.id, true);
    assert_eq!(a.host.send_datagram(b.id, dg(1)), Err(SendError::Paused));
}
