use clipsync_proto::*;

const SECRET: &str = "senha-secreta-123";

#[test]
fn clip_offer_debug_hides_text() {
    let o = ClipOffer {
        id: EventId::random(),
        lamport: 1,
        mime: MIME_TEXT_UTF8.into(),
        text: SECRET.into(),
        created_unix_ms: 0,
        ttl_ms: 1,
    };
    let s = format!("{o:?}");
    assert!(!s.contains(SECRET), "{s}");
    assert!(s.contains("<17 bytes>"), "{s}");
    let f = format!("{:?}", Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(o))));
    assert!(!f.contains(SECRET));
}

#[test]
fn key_and_button_debug_hide_code() {
    for ev in [
        InputEvent::Key {
            code: 30,
            pressed: true,
        },
        InputEvent::Button {
            code: 272,
            pressed: true,
        },
    ] {
        let s = format!("{ev:?}");
        assert!(!s.contains("30") && !s.contains("272"), "{s}");
        assert!(s.contains("<redacted>"), "{s}");
    }
    assert_eq!(
        format!(
            "{:?}",
            InputEvent::Key {
                code: 30,
                pressed: false
            }
        ),
        "Key(<redacted>)"
    );
    let d = InputDatagram {
        session: 5,
        seq: 6,
        t_mono_us: 7,
        events: vec![InputEvent::Key {
            code: 4242,
            pressed: true,
        }],
    };
    let s = format!("{d:?}");
    assert!(!s.contains("4242"), "{s}");
}

#[test]
fn control_request_debug_hides_clip_send_text() {
    let req = ControlRequest::Module {
        kind: ModuleKind::Clipboard,
        cmd: ModuleCommand::ClipSend {
            text: SECRET.into(),
            to: None,
        },
    };
    let s = format!("{req:?}");
    assert!(!s.contains(SECRET), "{s}");
    assert!(s.contains("ClipSend"), "{s}");
}

#[test]
fn inbox_item_debug_hides_preview() {
    let item = InboxItem {
        id: EventId::random(),
        from: "notebook-a".into(),
        received_unix_ms: 0,
        mime: MIME_TEXT_UTF8.into(),
        bytes: SECRET.len(),
        preview: SECRET.into(),
        state: DeliveryState::InInbox,
    };
    assert!(!format!("{item:?}").contains(SECRET));
    let reply = ModuleReply::ClipInbox(vec![item]);
    assert!(!format!("{reply:?}").contains(SECRET));
}

#[test]
fn control_messages_roundtrip_as_json() {
    let req = ControlRequest::Module {
        kind: ModuleKind::Input,
        cmd: ModuleCommand::InputLoopback {
            to: PeerId([7; 32]),
            seconds: 20,
        },
    };
    let json = serde_json::to_vec(&req).unwrap();
    let back: ControlRequest = serde_json::from_slice(&json).unwrap();
    assert_eq!(format!("{req:?}"), format!("{back:?}"));
    let resp = ControlResponse::Error {
        message: "x".into(),
    };
    let back: ControlResponse =
        serde_json::from_slice(&serde_json::to_vec(&resp).unwrap()).unwrap();
    assert!(matches!(back, ControlResponse::Error { .. }));
}

#[test]
fn v4_inventory_exposes_only_public_monitor_fields() {
    let inventory = MonitorInventory {
        rev: 1,
        space: CoordinateSpace::Logical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: false,
            positioned_return: false,
            route_receiver: true,
        },
        monitors: vec![MonitorInfo {
            uid: MonitorUid([1; 16]),
            label: "Tela".into(),
            rect: NativeRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: 1.0,
        }],
    };
    let message = InputMsg::InventoryV4(inventory);
    let json = serde_json::to_value(&message).unwrap();
    let fields: Vec<_> = json["InventoryV4"]["monitors"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(fields, ["label", "rect", "scale", "uid"]);
    let serialized = serde_json::to_string(&message).unwrap().to_lowercase();
    let debug = format!("{message:?}").to_lowercase();
    for private in [
        "path",
        "edid",
        "fingerprint",
        "aliases",
        "pressed",
        "code",
        "4242",
    ] {
        assert!(!serialized.contains(private), "{serialized}");
        assert!(!debug.contains(private), "{debug}");
    }
}
