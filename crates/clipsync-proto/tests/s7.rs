//! Protocol 3 (S7, contract §13.4 and §13.11 item 1): EnterSpan, Returned and Refused on the
//! wire, their validation, their authorization, the version gate of the MemHub and
//! `request_pause` counted by the MemHub.

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

fn span(monitor: &str, pos: f32, back_from: f32, back_to: f32) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::EnterSpan {
        session: 9,
        edge: Edge::Top,
        monitor: Some(monitor.into()),
        pos,
        back_from,
        back_to,
    })
}

fn returned(pos: f32) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::Returned { session: 9, pos })
}

fn refused(reason: RejectReason) -> ModuleMsg {
    ModuleMsg::Input(InputMsg::Refused { session: 9, reason })
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
fn enter_span_returned_and_refused_roundtrip_through_postcard() {
    match roundtrip(span("eDP-1", 0.4, 0.25, 0.75)) {
        ModuleMsg::Input(InputMsg::EnterSpan {
            session: 9,
            edge: Edge::Top,
            monitor: Some(m),
            pos,
            back_from,
            back_to,
        }) => assert_eq!(
            (m.as_str(), pos, back_from, back_to),
            ("eDP-1", 0.4, 0.25, 0.75)
        ),
        other => panic!("not EnterSpan: {other:?}"),
    }
    let no_monitor = ModuleMsg::Input(InputMsg::EnterSpan {
        session: 1,
        edge: Edge::Left,
        monitor: None,
        pos: 0.0,
        back_from: 0.0,
        back_to: 1.0,
    });
    assert!(matches!(
        roundtrip(no_monitor),
        ModuleMsg::Input(InputMsg::EnterSpan { monitor: None, .. })
    ));
    match roundtrip(returned(1.0)) {
        ModuleMsg::Input(InputMsg::Returned { session: 9, pos }) => assert_eq!(pos, 1.0),
        other => panic!("not Returned: {other:?}"),
    }
    for reason in [
        RejectReason::Locked,
        RejectReason::Paused,
        RejectReason::ApplyFailed,
    ] {
        match roundtrip(refused(reason)) {
            ModuleMsg::Input(InputMsg::Refused {
                session: 9,
                reason: r,
            }) => {
                assert_eq!(r, reason)
            }
            other => panic!("not Refused: {other:?}"),
        }
    }
}

/// Older decoders know the first eight InputMsg tags: the new variants come after them, so a
/// version 2 peer fails to decode them instead of reading something else.
#[test]
fn new_variants_take_new_postcard_tags() {
    for (m, tag) in [
        (span("eDP-1", 0.5, 0.0, 1.0), 8),
        (returned(0.5), 9),
        (refused(RejectReason::Locked), 10),
    ] {
        let ModuleMsg::Input(i) = m else { panic!() };
        assert_eq!(postcard::to_allocvec(&i).unwrap()[0], tag);
    }
}

// (b)
#[test]
fn validate_refuses_pos_out_of_range_nan_empty_way_back_and_a_bad_monitor_name() {
    assert!(!invalid(span("eDP-1", 0.0, 0.0, 1.0)));
    assert!(!invalid(span("eDP-1", 1.0, 0.1, 0.2)));
    assert!(invalid(span("eDP-1", 1.5, 0.0, 1.0)));
    assert!(invalid(span("eDP-1", -0.1, 0.0, 1.0)));
    assert!(invalid(span("eDP-1", f32::NAN, 0.0, 1.0)));
    assert!(invalid(span("eDP-1", 0.5, f32::NAN, 1.0)));
    assert!(invalid(span("eDP-1", 0.5, 0.0, f32::INFINITY)));
    assert!(invalid(span("eDP-1", 0.5, 0.0, 1.5)));
    assert!(
        invalid(span("eDP-1", 0.5, 0.5, 0.5)),
        "back_from == back_to"
    );
    assert!(
        invalid(span("eDP-1", 0.5, 0.75, 0.25)),
        "back_from > back_to"
    );
    assert!(invalid(span("", 0.5, 0.0, 1.0)), "empty monitor name");
    assert!(
        invalid(span("eDP\u{1b}[31m", 0.5, 0.0, 1.0)),
        "control char"
    );
    assert!(invalid(span(
        &"m".repeat(MAX_NAME_BYTES + 1),
        0.5,
        0.0,
        1.0
    )));

    assert!(!invalid(returned(0.0)));
    assert!(!invalid(returned(1.0)));
    assert!(invalid(returned(1.5)));
    assert!(invalid(returned(f32::NAN)));
    assert!(!invalid(refused(RejectReason::Locked)));
}

#[test]
fn min_version_is_3_only_for_the_new_variants() {
    assert_eq!((PROTOCOL_VERSION, MIN_PROTOCOL_VERSION), (4, 1));
    for m in [
        span("eDP-1", 0.5, 0.0, 1.0),
        returned(0.5),
        refused(RejectReason::Paused),
    ] {
        assert_eq!(min_version(&m), 3);
    }
    let enter_at = ModuleMsg::Input(InputMsg::EnterAt {
        session: 1,
        edge: Edge::Left,
        monitor: None,
        pos: 0.5,
    });
    assert_eq!(min_version(&enter_at), 2);
    let leave = ModuleMsg::Input(InputMsg::Leave { session: 1 });
    assert_eq!(min_version(&leave), 1);
}

// (c)
#[test]
fn enter_span_needs_input_in_and_no_pause_returned_and_refused_need_neither() {
    let out = |perms: PeerPerms, paused: bool, m: &ModuleMsg| {
        authorize_outbound(BOTH, &perms, BOTH, paused, m)
    };
    let inb =
        |perms: PeerPerms, paused: bool, m: &ModuleMsg| authorize_inbound(BOTH, &perms, paused, m);
    let e = span("eDP-1", 0.5, 0.0, 1.0);

    // EnterSpan, like EnterAt: enters only with input_in, and pause blocks it on both sides.
    assert_eq!(inb(NO, false, &e), Err(RejectReason::NotAuthorized));
    assert_eq!(inb(INPUT_IN, false, &e), Ok(()));
    assert_eq!(inb(INPUT_IN, true, &e), Err(RejectReason::Paused));
    assert_eq!(out(NO, false, &e), Ok(()));
    assert_eq!(out(NO, true, &e), Err(SendError::Paused));

    // Returned and Refused, like Stats: no input_in, no pause, both ways.
    for m in [returned(0.5), refused(RejectReason::Locked)] {
        assert_eq!(inb(NO, false, &m), Ok(()));
        assert_eq!(inb(NO, true, &m), Ok(()));
        assert_eq!(out(NO, false, &m), Ok(()));
        assert_eq!(out(NO, true, &m), Ok(()));
    }

    // The local input module has to be on, both ways; like Stats, Returned and Refused do not
    // look at the peer's modules on the way out (only EnterSpan does).
    let clip_only = ModuleSet {
        clipboard: true,
        input: false,
    };
    for m in [&e, &returned(0.5), &refused(RejectReason::Paused)] {
        assert_eq!(
            authorize_outbound(clip_only, &INPUT_IN, BOTH, false, m),
            Err(SendError::LocalModuleDisabled)
        );
        assert_eq!(
            authorize_inbound(clip_only, &INPUT_IN, false, m),
            Err(RejectReason::ModuleDisabled)
        );
    }
    assert_eq!(
        authorize_outbound(BOTH, &INPUT_IN, clip_only, false, &e),
        Err(SendError::RemoteModuleDisabled)
    );
}

async fn next(rx: &mut Receiver<ModuleInput>) -> ModuleInput {
    timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed")
}

// (d), MemHub side; the real mesh is in clipsync-core/tests/mesh.rs.
#[tokio::test]
async fn memhub_refuses_the_v3_messages_to_a_version_2_peer_and_nothing_leaves() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let mut v2 = hub.add_node("v2", BOTH);
    hub.set_remote_version(v2.id, 2);
    hub.link(a.id, v2.id, INPUT_IN, INPUT_IN);
    let ModuleInput::PeerUp(info) = next(&mut a.rx).await else {
        panic!("PeerUp")
    };
    assert_eq!((info.id, info.remote_version), (v2.id, 2));
    next(&mut v2.rx).await;

    for m in [
        span("eDP-1", 0.5, 0.0, 1.0),
        returned(0.5),
        refused(RejectReason::Locked),
    ] {
        assert_eq!(
            a.host.send_reliable(v2.id, m).await,
            Err(SendError::RemoteTooOld)
        );
    }
    assert_eq!(hub.sent_count(a.id, v2.id), 0);
    assert!(v2.rx.try_recv().is_err(), "nothing reached the v2 peer");

    // EnterAt (protocol 2) still goes to it.
    let enter_at = ModuleMsg::Input(InputMsg::EnterAt {
        session: 1,
        edge: Edge::Top,
        monitor: Some("eDP-1".into()),
        pos: 0.5,
    });
    a.host.send_reliable(v2.id, enter_at).await.unwrap();
    assert_eq!(hub.sent_count(a.id, v2.id), 1);
}

#[tokio::test]
async fn memhub_delivers_the_v3_messages_to_a_v3_peer() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let mut b = hub.add_node("b", BOTH);
    // b may control a: a lets EnterSpan in from b. a gives b nothing, and Returned and Refused
    // still reach b.
    hub.link(a.id, b.id, INPUT_IN, NO);
    next(&mut a.rx).await;
    next(&mut b.rx).await;
    b.host
        .send_reliable(a.id, span("eDP-1", 0.5, 0.25, 0.75))
        .await
        .unwrap();
    assert!(matches!(
        next(&mut a.rx).await,
        ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::EnterSpan { session: 9, .. }),
            ..
        }
    ));
    for m in [returned(0.3), refused(RejectReason::Paused)] {
        a.host.send_reliable(b.id, m).await.unwrap();
    }
    assert!(matches!(
        next(&mut b.rx).await,
        ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::Returned { session: 9, .. }),
            ..
        }
    ));
    assert!(matches!(
        next(&mut b.rx).await,
        ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::Refused {
                session: 9,
                reason: RejectReason::Paused
            }),
            ..
        }
    ));
    // a would not let b's EnterSpan in while paused: Rejected comes back to b.
    hub.set_paused(a.id, true);
    next(&mut a.rx).await;
    b.host
        .send_reliable(a.id, span("eDP-1", 0.5, 0.25, 0.75))
        .await
        .unwrap();
    assert!(matches!(
        next(&mut b.rx).await,
        ModuleInput::Rejected {
            reason: RejectReason::Paused,
            ..
        }
    ));
}

/// §13.4 item 4: the MemHub applies `request_pause` like `set_paused` and counts the calls.
#[tokio::test]
async fn memhub_counts_request_pause_and_applies_it() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    assert_eq!(hub.pause_requests(a.id), 0);
    a.host.request_pause(true);
    assert!(matches!(next(&mut a.rx).await, ModuleInput::Paused(true)));
    assert!(a.host.paused());
    a.host.request_pause(false);
    assert!(matches!(next(&mut a.rx).await, ModuleInput::Paused(false)));
    assert!(!a.host.paused());
    assert_eq!(hub.pause_requests(a.id), 2);
}

/// §13.7 item 3, I-S7-2 and §13.9: `keyboard_layout`, `resume_hotkey_refused` and
/// `switch_supported` close `InputState` with serde defaults, so the reply of an agent without
/// them (the control socket speaks JSON) still decodes, as None, false and true.
#[test]
fn input_state_without_the_s7_fields_decodes_with_their_defaults() {
    let old = serde_json::json!({
        "local": [], "peers": [], "screen_locked": false, "active_peer": null, "pending_offer": null
    });
    let s: InputState = serde_json::from_value(old).unwrap();
    assert_eq!(
        (
            s.keyboard_layout.as_deref(),
            s.resume_hotkey_refused,
            s.switch_supported
        ),
        (None, false, true)
    );
    let new = InputState {
        keyboard_layout: Some("00010416".into()),
        resume_hotkey_refused: true,
        switch_supported: false,
        ..s
    };
    let back: InputState = serde_json::from_value(serde_json::to_value(&new).unwrap()).unwrap();
    assert_eq!(back, new);
}

/// §13.4 item 6: `unlink` undoes `link` on both sides, like a revoked peer.
#[tokio::test]
async fn memhub_unlink_takes_the_peer_away_on_both_sides() {
    let hub = MemHub::new();
    let mut a = hub.add_node("a", BOTH);
    let mut b = hub.add_node("b", BOTH);
    hub.link(a.id, b.id, INPUT_IN, INPUT_IN);
    next(&mut a.rx).await;
    next(&mut b.rx).await;
    assert_eq!(a.host.peers().len(), 1);

    hub.unlink(a.id, b.id);
    assert!(matches!(next(&mut a.rx).await, ModuleInput::PeerDown(p) if p == b.id));
    assert!(matches!(next(&mut b.rx).await, ModuleInput::PeerDown(p) if p == a.id));
    assert!(a.host.peers().is_empty() && b.host.peers().is_empty());
    assert_eq!(
        a.host.send_reliable(b.id, returned(0.5)).await,
        Err(SendError::NotConnected)
    );
    assert_eq!(
        b.host.send_datagram(
            a.id,
            InputDatagram {
                session: 9,
                seq: 1,
                t_mono_us: 0,
                events: vec![],
            }
        ),
        Err(SendError::NotConnected)
    );

    // Unlinking again (or two nodes never linked) says nothing.
    hub.unlink(a.id, b.id);
    assert!(a.rx.try_recv().is_err() && b.rx.try_recv().is_err());
}
