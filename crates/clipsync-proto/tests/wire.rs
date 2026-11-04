// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clipsync_proto::*;

fn offer(text: &str) -> ClipOffer {
    ClipOffer {
        id: EventId::random(),
        lamport: 7,
        mime: MIME_TEXT_UTF8.to_string(),
        text: text.to_string(),
        created_unix_ms: 1,
        ttl_ms: CLIP_TTL_MS,
    }
}

fn hello(name: &str) -> Hello {
    Hello {
        version: PROTOCOL_VERSION,
        min_version: MIN_PROTOCOL_VERSION,
        device_name: name.to_string(),
        modules: ModuleSet {
            clipboard: true,
            input: false,
        },
        listen_port: 47800,
    }
}

fn datagram(n: usize) -> Datagram {
    Datagram::Input(InputDatagram {
        session: 1,
        seq: 2,
        t_mono_us: 3,
        events: vec![InputEvent::Motion { dx: 1.5, dy: -2.0 }; n],
    })
}

fn roundtrip(f: &Frame) -> Frame {
    let buf = encode_frame(f, MAX_FRAME_BYTES).unwrap();
    let len = u32::from_be_bytes(buf[..4].try_into().unwrap()) as usize;
    assert_eq!(len, buf.len() - 4);
    decode_frame_body(&buf[4..]).unwrap()
}

#[test]
fn every_frame_variant_roundtrips() {
    let frames = vec![
        Frame::Hello(hello("notebook-a")),
        Frame::Bye {
            code: CloseCode::Shutdown,
        },
        Frame::Ping { nonce: 9 },
        Frame::Pong { nonce: 9 },
        Frame::Pair(PairMsg::Commit { h_nb: [1; 32] }),
        Frame::Pair(PairMsg::Nonce { na: [2; 32] }),
        Frame::Pair(PairMsg::Reveal { nb: [3; 32] }),
        Frame::Pair(PairMsg::Confirm { accept: true }),
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(offer("olá ção")))),
        Frame::Module(ModuleMsg::Clip(ClipMsg::Ack(ClipAck {
            id: EventId::random(),
            state: DeliveryState::Rejected(RejectReason::Stale),
        }))),
        Frame::Module(ModuleMsg::Input(InputMsg::Enter {
            session: 1,
            edge: Edge::Right,
            pos: 0.25,
        })),
        Frame::Module(ModuleMsg::Input(InputMsg::Stats(LoopbackStats {
            received: 200,
            ..Default::default()
        }))),
        Frame::Rejected {
            kind: ModuleKind::Clipboard,
            ref_id: None,
            reason: RejectReason::Paused,
        },
    ];
    for f in &frames {
        let back = roundtrip(f);
        assert_eq!(format!("{f:?}"), format!("{back:?}"));
    }
    match roundtrip(&Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(offer(
        "olá ção",
    ))))) {
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(o))) => assert_eq!(o.text, "olá ção"),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn oversized_frame_is_refused_before_allocating() {
    // Header announces u32::MAX bytes; only 10 body bytes follow. If the reader allocated or
    // read the body first it would OOM or fail with Io(UnexpectedEof), never TooLarge.
    let mut input = u32::MAX.to_be_bytes().to_vec();
    input.extend_from_slice(&[0u8; 10]);
    let mut r: &[u8] = &input;
    match read_frame(&mut r, MAX_FRAME_BYTES).await {
        Err(WireError::TooLarge { len, max }) => {
            assert_eq!(len, u32::MAX as usize);
            assert_eq!(max, MAX_FRAME_BYTES);
        }
        other => panic!("expected TooLarge, got {other:?}"),
    }
    assert_eq!(r.len(), 10, "only the 4-byte header may be consumed");
}

#[tokio::test]
async fn control_frame_of_64k_plus_one_is_refused() {
    let len = (MAX_CONTROL_FRAME_BYTES + 1) as u32;
    let mut r: &[u8] = &len.to_be_bytes();
    assert!(matches!(
        read_frame(&mut r, MAX_CONTROL_FRAME_BYTES).await,
        Err(WireError::TooLarge {
            len: 65_537,
            max: 65_536
        })
    ));
}

#[test]
fn encode_refuses_body_over_max() {
    let f = Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(offer(&"x".repeat(2_000)))));
    assert!(matches!(
        encode_frame(&f, 1_000),
        Err(WireError::TooLarge { max: 1_000, .. })
    ));
}

#[tokio::test]
async fn write_then_read_over_a_duplex() {
    let (mut a, mut b) = tokio::io::duplex(4096);
    write_frame(&mut a, &Frame::Ping { nonce: 42 }, MAX_CONTROL_FRAME_BYTES)
        .await
        .unwrap();
    write_frame(&mut a, &Frame::Hello(hello("b")), MAX_CONTROL_FRAME_BYTES)
        .await
        .unwrap();
    assert!(matches!(
        read_frame(&mut b, MAX_CONTROL_FRAME_BYTES).await,
        Ok(Frame::Ping { nonce: 42 })
    ));
    assert!(matches!(
        read_frame(&mut b, MAX_CONTROL_FRAME_BYTES).await,
        Ok(Frame::Hello(_))
    ));
    drop(a);
    assert!(matches!(
        read_frame(&mut b, MAX_CONTROL_FRAME_BYTES).await,
        Err(WireError::Io(_))
    ));
}

#[test]
fn garbage_and_trailing_bytes_are_decode_errors() {
    assert!(matches!(
        decode_frame_body(&[0xff; 7]),
        Err(WireError::Decode)
    ));
    let mut buf = encode_frame(&Frame::Ping { nonce: 1 }, MAX_FRAME_BYTES).unwrap();
    buf.push(0);
    assert!(matches!(
        decode_frame_body(&buf[4..]),
        Err(WireError::Decode)
    ));
}

#[test]
fn datagram_over_1200_bytes_is_refused() {
    assert!(matches!(
        decode_datagram(&[0u8; MAX_DATAGRAM_BYTES + 1]),
        Err(WireError::TooLarge {
            len: 1_201,
            max: 1_200
        })
    ));
}

#[test]
fn datagram_with_32_events_fits_and_roundtrips() {
    let buf = encode_datagram(&datagram(MAX_EVENTS_PER_DATAGRAM)).unwrap();
    assert!(buf.len() <= MAX_DATAGRAM_BYTES);
    let Datagram::Input(d) = decode_datagram(&buf).unwrap();
    assert_eq!(d.events.len(), 32);
    assert_eq!(d.events[0], InputEvent::Motion { dx: 1.5, dy: -2.0 });
}

#[test]
fn validate_refuses_33_events() {
    assert!(matches!(
        encode_datagram(&datagram(33)),
        Err(WireError::Invalid(_))
    ));
    let raw = postcard::to_allocvec(&datagram(33)).unwrap();
    assert!(raw.len() <= MAX_DATAGRAM_BYTES);
    assert!(matches!(decode_datagram(&raw), Err(WireError::Invalid(_))));
}

#[test]
fn validate_refuses_nan_and_infinite_floats() {
    for ev in [
        InputEvent::Motion {
            dx: f32::NAN,
            dy: 0.0,
        },
        InputEvent::Scroll {
            dx: 0.0,
            dy: f32::INFINITY,
        },
    ] {
        let d = Datagram::Input(InputDatagram {
            session: 1,
            seq: 1,
            t_mono_us: 1,
            events: vec![ev],
        });
        assert!(matches!(d.validate(), Err(WireError::Invalid(_))));
        let raw = postcard::to_allocvec(&d).unwrap();
        assert!(matches!(decode_datagram(&raw), Err(WireError::Invalid(_))));
    }
}

#[test]
fn validate_refuses_pos_outside_unit_range() {
    for pos in [1.5, -0.1, f32::NAN] {
        let f = Frame::Module(ModuleMsg::Input(InputMsg::Enter {
            session: 1,
            edge: Edge::Left,
            pos,
        }));
        assert!(
            matches!(f.validate(), Err(WireError::Invalid(_))),
            "pos {pos}"
        );
        assert!(matches!(
            encode_frame(&f, MAX_FRAME_BYTES),
            Err(WireError::Invalid(_))
        ));
    }
    for pos in [0.0, 1.0] {
        let f = Frame::Module(ModuleMsg::Input(InputMsg::Enter {
            session: 1,
            edge: Edge::Left,
            pos,
        }));
        assert!(f.validate().is_ok());
    }
}

#[test]
fn validate_device_name_bounds() {
    assert!(
        Frame::Hello(hello(&"a".repeat(MAX_NAME_BYTES)))
            .validate()
            .is_ok()
    );
    assert!(matches!(
        Frame::Hello(hello(&"a".repeat(65))).validate(),
        Err(WireError::Invalid(_))
    ));
    assert!(matches!(
        Frame::Hello(hello("")).validate(),
        Err(WireError::Invalid(_))
    ));
    // 65 bytes but 33 chars: the limit is in bytes.
    let multibyte = format!("a{}", "ç".repeat(32));
    assert_eq!(multibyte.len(), 65);
    assert!(Frame::Hello(hello(&multibyte)).validate().is_err());
}

#[test]
fn validate_hello_min_version() {
    let mut h = hello("a");
    h.min_version = h.version + 1;
    assert!(matches!(
        Frame::Hello(h).validate(),
        Err(WireError::Invalid(_))
    ));
}

#[test]
fn validate_clip_offer_mime_and_size() {
    let mut o = offer("x");
    o.mime = "image/png".into();
    assert!(
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(o)))
            .validate()
            .is_err()
    );
    let mut o = offer("x");
    o.mime = "m".repeat(MAX_MIME_BYTES + 1);
    assert!(
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(o)))
            .validate()
            .is_err()
    );
    let big = offer(&"x".repeat(MAX_CLIP_TEXT_BYTES + 1));
    assert!(
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(big)))
            .validate()
            .is_err()
    );
    let max = offer(&"x".repeat(MAX_CLIP_TEXT_BYTES));
    let f = Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(max)));
    assert!(
        encode_frame(&f, MAX_FRAME_BYTES).is_ok(),
        "max clip text fits a module frame"
    );
}

#[test]
fn close_codes_are_stable_short_tokens() {
    let mut seen = std::collections::HashSet::new();
    for c in CloseCode::ALL {
        assert_eq!(CloseCode::from_code(u64::from(c.code())), Some(c));
        let r = c.reason();
        assert!(r.is_ascii() && !r.is_empty() && r.len() <= 16);
        assert!(seen.insert(r));
    }
    assert_eq!(CloseCode::KeyChanged.code(), 7);
    assert_eq!(CloseCode::UnknownPeer.reason(), b"unknown-peer");
    assert_eq!(CloseCode::Revoked.code(), 8);
    assert_eq!(CloseCode::Revoked.reason(), b"revoked");
    assert_eq!(CloseCode::from_code(9), None);
}

#[test]
fn module_msg_kind() {
    assert_eq!(
        ModuleMsg::Clip(ClipMsg::Offer(offer("x"))).kind(),
        ModuleKind::Clipboard
    );
    assert_eq!(
        ModuleMsg::Input(InputMsg::Leave { session: 1 }).kind(),
        ModuleKind::Input
    );
}

#[test]
fn unreachable_never_goes_on_the_wire() {
    let ack = |state| {
        Frame::Module(ModuleMsg::Clip(ClipMsg::Ack(ClipAck {
            id: EventId([1; 16]),
            state,
        })))
    };
    assert!(matches!(
        encode_frame(&ack(DeliveryState::Unreachable), MAX_FRAME_BYTES),
        Err(WireError::Invalid(_))
    ));
    let raw = postcard::to_allocvec(&ack(DeliveryState::Unreachable)).unwrap();
    assert!(matches!(
        decode_frame_body(&raw),
        Err(WireError::Invalid(_))
    ));
    assert!(encode_frame(&ack(DeliveryState::Applied), MAX_FRAME_BYTES).is_ok());
    // The local control socket (JSON) does carry it.
    let reply = ModuleReply::ClipSent(vec![SendOutcome {
        peer: "b".into(),
        state: DeliveryState::Unreachable,
    }]);
    let json = serde_json::to_vec(&reply).unwrap();
    assert!(matches!(
        serde_json::from_slice::<ModuleReply>(&json).unwrap(),
        ModuleReply::ClipSent(v) if v[0].state == DeliveryState::Unreachable
    ));
}

#[test]
fn validate_device_name_rejects_control_and_bidi() {
    for ok in [
        "notebook-a",
        "Notebook do João",
        "çãé ü 日本",
        &"a".repeat(MAX_NAME_BYTES),
    ] {
        assert!(validate_device_name(ok).is_ok(), "{ok:?}");
    }
    let bad = [
        "a\u{1b}[1A\r\u{1b}[2KCódigo: 123 456\n", // the S1-01 proof
        "a\nb",
        "a\rb",
        "a\tb",
        "\u{0}",
        "a\u{7f}",
        "a\u{85}",
        "a\u{9b}2J",
        "a\u{200b}b",
        "a\u{200e}",
        "a\u{200f}",
        "a\u{2028}",
        "a\u{2029}",
        "a\u{202a}",
        "\u{202e}cba",
        "a\u{2066}",
        "a\u{2069}",
        "\u{feff}a",
        "a\u{061c}",
        "a\u{2060}",
    ];
    for name in bad {
        assert!(
            matches!(validate_device_name(name), Err(WireError::Invalid(_))),
            "{name:?}"
        );
        let mut h = hello(name);
        h.device_name = name.into();
        let f = Frame::Hello(h);
        assert!(f.validate().is_err(), "{name:?}");
        assert!(encode_frame(&f, MAX_FRAME_BYTES).is_err(), "{name:?}");
        // A peer that skips validate() is caught on decode.
        let raw = postcard::to_allocvec(&f).unwrap();
        assert!(
            matches!(decode_frame_body(&raw), Err(WireError::Invalid(_))),
            "{name:?}"
        );
    }
}

/// v1.16: `Bye{Revoked}` round-trips, and a status from an agent without the newer rings
/// (0.1.0 had neither `recent_blocks` nor `recent_removals`) still reads, with both empty.
#[test]
fn revoked_bye_and_status_without_the_rings() {
    let bye = Frame::Bye {
        code: CloseCode::Revoked,
    };
    let bytes = encode_frame(&bye, MAX_CONTROL_FRAME_BYTES).unwrap();
    assert!(matches!(
        decode_frame_body(&bytes[4..]).unwrap(),
        Frame::Bye {
            code: CloseCode::Revoked
        }
    ));
    let status = AgentStatus {
        log_path: None,
        device_name: "a".into(),
        id: PeerId([7; 32]),
        listen: "127.0.0.1:47800".parse().unwrap(),
        paused: false,
        peers: vec![],
        modules: vec![],
        recent_blocks: vec![],
        recent_removals: vec![RemovalRecord {
            at_unix_ms: 1,
            peer: PeerId([9; 32]),
            peer_name: "b".into(),
        }],
    };
    let mut v = serde_json::to_value(&status).unwrap();
    let back: AgentStatus = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(back.recent_removals, status.recent_removals);
    let o = v.as_object_mut().unwrap();
    o.remove("recent_blocks");
    o.remove("recent_removals");
    let old: AgentStatus = serde_json::from_value(v).unwrap();
    assert!(old.recent_blocks.is_empty() && old.recent_removals.is_empty());
}
