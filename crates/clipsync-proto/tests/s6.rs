//! Protocol 2 (S6, contract §12.6 and §12.9 item 1): Zones, LayoutOffer and EnterAt on the
//! wire, their validation, their authorization table and the version gate of the MemHub.

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
const INPUT_IN: PeerPerms = PeerPerms {
    input_in: true,
    ..NO
};

fn zone(index: u32) -> ZoneInfo {
    ZoneInfo {
        index,
        name: Some(format!("DP-{index}")),
        x: 1920 * i32::try_from(index).unwrap(),
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.25,
    }
}

fn zones(n: u32) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::Zones {
        zones: (0..n).map(zone).collect(),
    })
}

fn endpoint(peer: u8, from: f32, to: f32) -> OfferEndpoint {
    OfferEndpoint {
        peer: PeerId([peer; 32]),
        monitor: "HDMI-1".into(),
        edge: Edge::Right,
        from,
        to,
    }
}

fn layout_offer(from: f32, to: f32) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::LayoutOffer(LayoutOffer {
        rev: 7,
        devices: vec![
            OfferDevice {
                peer: PeerId([1; 32]),
                x: 0,
                y: 0,
            },
            OfferDevice {
                peer: PeerId([2; 32]),
                x: 1920,
                y: -40,
            },
        ],
        links: vec![OfferLink {
            a: endpoint(1, from, to),
            b: OfferEndpoint {
                edge: Edge::Left,
                ..endpoint(2, 0.0, 1.0)
            },
        }],
    }))
}

fn enter_at(pos: f32) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::EnterAt {
        session: 3,
        edge: Edge::Left,
        monitor: Some("eDP-1".into()),
        pos,
    })
}

fn roundtrip(m: ModuleMsg) -> ModuleMsg {
    let buf = encode_frame(&Frame::Module(m), MAX_FRAME_BYTES).unwrap();
    match decode_frame_body(&buf[4..]).unwrap() {
        Frame::Module(m) => m,
        other => panic!("not a module frame: {other:?}"),
    }
}

fn invalid(m: ModuleMsg) -> bool {
    matches!(
        encode_frame(&Frame::Module(m), MAX_FRAME_BYTES),
        Err(WireError::Invalid(_))
    )
}

// (a)
#[test]
fn zones_layout_offer_and_enter_at_roundtrip_through_postcard() {
    let Frame::Module(ModuleMsg::Input(InputMsg::Zones { zones: got })) =
        Frame::Module(roundtrip(zones(3)))
    else {
        panic!("not Zones");
    };
    assert_eq!(got, (0..3).map(zone).collect::<Vec<_>>());

    let ModuleMsg::Input(InputMsg::LayoutOffer(sent)) = layout_offer(0.5, 1.0) else {
        panic!()
    };
    let ModuleMsg::Input(InputMsg::LayoutOffer(got)) = roundtrip(layout_offer(0.5, 1.0)) else {
        panic!("not LayoutOffer");
    };
    assert_eq!(got, sent);

    match roundtrip(enter_at(0.25)) {
        ModuleMsg::Input(InputMsg::EnterAt {
            session: 3,
            edge: Edge::Left,
            monitor: Some(m),
            pos,
        }) => assert_eq!((m.as_str(), pos), ("eDP-1", 0.25)),
        other => panic!("not EnterAt: {other:?}"),
    }
}

/// The 0.1.0 decoder knows the first five InputMsg tags: the new variants must come after them,
/// or an old peer would read a Zones as something else instead of failing to decode.
#[test]
fn new_variants_take_new_postcard_tags() {
    let enter = postcard::to_allocvec(&InputMsg::Enter {
        session: 1,
        edge: Edge::Left,
        pos: 0.5,
    })
    .unwrap();
    let stats = postcard::to_allocvec(&InputMsg::Stats(LoopbackStats::default())).unwrap();
    assert_eq!((enter[0], stats[0]), (0, 4));
    for (m, tag) in [
        (zones(1), 5),
        (layout_offer(0.0, 1.0), 6),
        (enter_at(0.0), 7),
    ] {
        let ModuleMsg::Input(i) = m else { panic!() };
        assert_eq!(postcard::to_allocvec(&i).unwrap()[0], tag);
    }
}

// (b)
#[test]
fn validate_refuses_17_zones_nan_scale_pos_out_of_range_and_empty_segment() {
    assert!(!invalid(zones(MAX_ZONES as u32)));
    assert!(invalid(zones(MAX_ZONES as u32 + 1)), "17 zones");
    for bad in [
        ZoneInfo {
            scale: f64::NAN,
            ..zone(0)
        },
        ZoneInfo {
            scale: 0.0,
            ..zone(0)
        },
        ZoneInfo {
            width: 0,
            ..zone(0)
        },
        ZoneInfo {
            name: Some("DP\u{1b}[2J".into()),
            ..zone(0)
        },
    ] {
        let m = ModuleMsg::Input(InputMsg::Zones { zones: vec![bad] });
        assert!(invalid(m));
    }

    assert!(!invalid(enter_at(1.0)));
    assert!(invalid(enter_at(1.5)), "pos 1.5");
    assert!(invalid(enter_at(f32::NAN)));
    let ModuleMsg::Input(InputMsg::EnterAt { session, edge, .. }) = enter_at(0.5) else {
        panic!()
    };
    let unnamed = ModuleMsg::Input(InputMsg::EnterAt {
        session,
        edge,
        monitor: None,
        pos: 0.5,
    });
    assert!(!invalid(unnamed));

    assert!(invalid(layout_offer(0.5, 0.5)), "from == to");
    assert!(invalid(layout_offer(0.8, 0.2)), "from > to");
    assert!(invalid(layout_offer(-0.1, 0.5)));
    let ModuleMsg::Input(InputMsg::LayoutOffer(mut o)) = layout_offer(0.0, 1.0) else {
        panic!()
    };
    o.links = vec![o.links[0].clone(); MAX_LAYOUT_LINKS + 1];
    assert!(invalid(ModuleMsg::Input(InputMsg::LayoutOffer(o.clone()))));
    o.links.truncate(1);
    o.devices = vec![o.devices[0].clone(); MAX_ZONES + 1];
    assert!(invalid(ModuleMsg::Input(InputMsg::LayoutOffer(o))));
}

#[test]
fn min_version_is_2_only_for_the_new_variants() {
    assert_eq!(min_version(&zones(1)), 2);
    assert_eq!(min_version(&layout_offer(0.0, 1.0)), 2);
    assert_eq!(min_version(&enter_at(0.5)), 2);
    let old = ModuleMsg::Input(InputMsg::Enter {
        session: 1,
        edge: Edge::Left,
        pos: 0.5,
    });
    assert_eq!(min_version(&old), 1);
    let ack = ModuleMsg::Clip(ClipMsg::Ack(ClipAck {
        id: EventId::random(),
        state: DeliveryState::Applied,
    }));
    assert_eq!(min_version(&ack), 1);
    assert_eq!(MIN_PROTOCOL_VERSION, 1);
}

// (c)
#[test]
fn authorize_table_for_zones_enter_at_and_layout_offer() {
    let out = |perms: PeerPerms, paused: bool, m: &ModuleMsg| {
        authorize_outbound(BOTH, &perms, BOTH, paused, m)
    };
    let inb =
        |perms: PeerPerms, paused: bool, m: &ModuleMsg| authorize_inbound(BOTH, &perms, paused, m);
    let (z, e, o) = (zones(1), enter_at(0.5), layout_offer(0.0, 1.0));

    // Zones only LEAVES to a peer I granted input_in; pause does not stop it.
    assert_eq!(out(NO, false, &z), Err(SendError::NotAuthorized));
    assert_eq!(out(INPUT_IN, false, &z), Ok(()));
    assert_eq!(out(INPUT_IN, true, &z), Ok(()));
    // ...and comes in from anyone, paused or not (it only describes the sender's monitors).
    assert_eq!(inb(NO, true, &z), Ok(()));

    // EnterAt and LayoutOffer only ENTER with input_in.
    assert_eq!(inb(NO, false, &e), Err(RejectReason::NotAuthorized));
    assert_eq!(inb(INPUT_IN, false, &e), Ok(()));
    assert_eq!(inb(NO, false, &o), Err(RejectReason::NotAuthorized));
    assert_eq!(inb(INPUT_IN, false, &o), Ok(()));
    // Pause blocks EnterAt (like Enter), not LayoutOffer.
    assert_eq!(inb(INPUT_IN, true, &e), Err(RejectReason::Paused));
    assert_eq!(inb(INPUT_IN, true, &o), Ok(()));
    assert_eq!(out(NO, true, &e), Err(SendError::Paused));
    assert_eq!(out(NO, false, &e), Ok(()));
    assert_eq!(out(NO, true, &o), Ok(()));

    // Module order still wins over permission and pause.
    let clip_only = ModuleSet {
        clipboard: true,
        input: false,
    };
    for m in [&z, &e, &o] {
        assert_eq!(
            authorize_outbound(clip_only, &INPUT_IN, BOTH, false, m),
            Err(SendError::LocalModuleDisabled)
        );
        assert_eq!(
            authorize_outbound(BOTH, &INPUT_IN, clip_only, false, m),
            Err(SendError::RemoteModuleDisabled)
        );
        assert_eq!(
            authorize_inbound(clip_only, &INPUT_IN, true, m),
            Err(RejectReason::ModuleDisabled)
        );
    }
}

async fn next(rx: &mut Receiver<ModuleInput>) -> ModuleInput {
    timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed")
}

// (d), MemHub side; the real mesh is in clipsync-core/tests/mesh.rs.
#[tokio::test]
async fn memhub_refuses_zones_to_a_version_1_peer_and_nothing_leaves() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let mut old = hub.add_node("old", BOTH);
    let mut new = hub.add_node("new", BOTH);
    hub.set_remote_version(old.id, 1);
    hub.link(a.id, old.id, INPUT_IN, INPUT_IN);
    hub.link(a.id, new.id, INPUT_IN, INPUT_IN);
    let ModuleInput::PeerUp(info) = next(&mut a.rx).await else {
        panic!("PeerUp")
    };
    assert_eq!((info.id, info.remote_version), (old.id, 1));
    let ModuleInput::PeerUp(info) = next(&mut a.rx).await else {
        panic!("PeerUp")
    };
    assert_eq!((info.id, info.remote_version), (new.id, PROTOCOL_VERSION));
    next(&mut old.rx).await;
    next(&mut new.rx).await;

    for m in [zones(1), layout_offer(0.0, 1.0), enter_at(0.5)] {
        assert_eq!(
            a.host.send_reliable(old.id, m).await,
            Err(SendError::RemoteTooOld)
        );
    }
    assert_eq!(hub.sent_count(a.id, old.id), 0);
    assert!(old.rx.try_recv().is_err(), "nothing reached the old peer");

    // A protocol-1 message still goes to the old peer; Zones goes to the new one.
    let enter = ModuleMsg::Input(InputMsg::Enter {
        session: 1,
        edge: Edge::Left,
        pos: 0.5,
    });
    a.host.send_reliable(old.id, enter).await.unwrap();
    a.host.send_reliable(new.id, zones(2)).await.unwrap();
    assert_eq!(
        (hub.sent_count(a.id, old.id), hub.sent_count(a.id, new.id)),
        (1, 1)
    );
    let ModuleInput::Reliable {
        msg: ModuleMsg::Input(InputMsg::Zones { zones: got }),
        ..
    } = next(&mut new.rx).await
    else {
        panic!("Zones")
    };
    assert_eq!(got.len(), 2);

    // Without input_in from A, Zones does not leave either (NotAuthorized, counted 0).
    let mut c = hub.add_node("c", BOTH);
    hub.link(a.id, c.id, NO, INPUT_IN);
    next(&mut c.rx).await;
    assert_eq!(
        a.host.send_reliable(c.id, zones(1)).await,
        Err(SendError::NotAuthorized)
    );
    assert_eq!(hub.sent_count(a.id, c.id), 0);
}
