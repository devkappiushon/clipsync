use clipsync_proto::*;

const BOTH: ModuleSet = ModuleSet {
    clipboard: true,
    input: true,
};
const NONE: ModuleSet = ModuleSet {
    clipboard: false,
    input: false,
};
type InboundCase = (
    ModuleSet,
    PeerPerms,
    bool,
    ModuleMsg,
    Result<(), RejectReason>,
);
type OutboundCase = (
    ModuleSet,
    PeerPerms,
    ModuleSet,
    bool,
    ModuleMsg,
    Result<(), SendError>,
);

const ALL: PeerPerms = PeerPerms {
    clipboard_in: true,
    clipboard_out: true,
    input_in: true,
};
const NO: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: false,
};

fn offer() -> ModuleMsg {
    ModuleMsg::Clip(ClipMsg::Offer(ClipOffer {
        id: EventId::random(),
        lamport: 1,
        mime: MIME_TEXT_UTF8.into(),
        text: "x".into(),
        created_unix_ms: 0,
        ttl_ms: 1,
    }))
}

fn ack() -> ModuleMsg {
    ModuleMsg::Clip(ClipMsg::Ack(ClipAck {
        id: EventId::random(),
        state: DeliveryState::Applied,
    }))
}

fn enter() -> ModuleMsg {
    ModuleMsg::Input(InputMsg::Enter {
        session: 1,
        edge: Edge::Right,
        pos: 0.5,
    })
}

fn open_msgs() -> Vec<ModuleMsg> {
    vec![
        ack(),
        ModuleMsg::Input(InputMsg::Leave { session: 1 }),
        ModuleMsg::Input(InputMsg::ReleaseAll { session: 1 }),
        ModuleMsg::Input(InputMsg::StatsRequest { session: 1 }),
        ModuleMsg::Input(InputMsg::Stats(LoopbackStats::default())),
    ]
}

#[test]
fn inbound_table() {
    use RejectReason::*;
    let only_clip = ModuleSet {
        clipboard: true,
        input: false,
    };
    // (local, perms, paused, msg) -> expected
    let cases: Vec<InboundCase> = vec![
        (BOTH, ALL, false, offer(), Ok(())),
        (only_clip, ALL, false, enter(), Err(ModuleDisabled)),
        (NONE, NO, true, offer(), Err(ModuleDisabled)), // disabled wins over perm and pause
        (BOTH, NO, false, offer(), Err(NotAuthorized)),
        (BOTH, NO, true, offer(), Err(NotAuthorized)), // perm wins over pause
        (
            BOTH,
            PeerPerms {
                clipboard_in: true,
                ..NO
            },
            true,
            offer(),
            Err(Paused),
        ),
        (
            BOTH,
            PeerPerms {
                clipboard_in: true,
                ..NO
            },
            false,
            enter(),
            Err(NotAuthorized),
        ),
        (
            BOTH,
            PeerPerms {
                input_in: true,
                ..NO
            },
            false,
            enter(),
            Ok(()),
        ),
        (
            BOTH,
            PeerPerms {
                input_in: true,
                ..NO
            },
            true,
            enter(),
            Err(Paused),
        ),
        (
            BOTH,
            PeerPerms {
                clipboard_out: true,
                ..NO
            },
            false,
            offer(),
            Err(NotAuthorized),
        ),
        (NONE, ALL, false, ack(), Err(ModuleDisabled)),
    ];
    for (i, (local, perms, paused, msg, want)) in cases.into_iter().enumerate() {
        assert_eq!(
            authorize_inbound(local, &perms, paused, &msg),
            want,
            "case {i}"
        );
    }
    for msg in open_msgs() {
        assert_eq!(
            authorize_inbound(BOTH, &NO, true, &msg),
            Ok(()),
            "{msg:?} passes paused without perm"
        );
    }
}

#[test]
fn inbound_datagram_table() {
    use RejectReason::*;
    let only_clip = ModuleSet {
        clipboard: true,
        input: false,
    };
    let input = PeerPerms {
        input_in: true,
        ..NO
    };
    assert_eq!(authorize_inbound_datagram(BOTH, &input, false), Ok(()));
    assert_eq!(
        authorize_inbound_datagram(only_clip, &input, false),
        Err(ModuleDisabled)
    );
    assert_eq!(
        authorize_inbound_datagram(BOTH, &NO, true),
        Err(NotAuthorized)
    );
    assert_eq!(authorize_inbound_datagram(BOTH, &input, true), Err(Paused));
}

#[test]
fn outbound_table() {
    use SendError::*;
    let only_clip = ModuleSet {
        clipboard: true,
        input: false,
    };
    let out = PeerPerms {
        clipboard_out: true,
        ..NO
    };
    // (local, perms, remote, paused, msg) -> expected
    let cases: Vec<OutboundCase> = vec![
        (BOTH, out, BOTH, false, offer(), Ok(())),
        (NONE, ALL, BOTH, false, offer(), Err(LocalModuleDisabled)),
        (
            only_clip,
            ALL,
            BOTH,
            false,
            enter(),
            Err(LocalModuleDisabled),
        ),
        (
            BOTH,
            NO,
            only_clip,
            true,
            enter(),
            Err(RemoteModuleDisabled),
        ),
        (BOTH, NO, NONE, true, offer(), Err(RemoteModuleDisabled)), // remote wins over perm and pause
        (BOTH, NO, BOTH, true, offer(), Err(NotAuthorized)),        // perm wins over pause
        (
            BOTH,
            PeerPerms {
                clipboard_in: true,
                input_in: true,
                ..NO
            },
            BOTH,
            false,
            offer(),
            Err(NotAuthorized),
        ),
        (BOTH, out, BOTH, true, offer(), Err(Paused)),
        (BOTH, NO, BOTH, false, enter(), Ok(())), // control permission is checked at the destination
        (BOTH, NO, BOTH, true, enter(), Err(Paused)),
        (NONE, ALL, BOTH, false, ack(), Err(LocalModuleDisabled)),
    ];
    for (i, (local, perms, remote, paused, msg, want)) in cases.into_iter().enumerate() {
        assert_eq!(
            authorize_outbound(local, &perms, remote, paused, &msg),
            want,
            "case {i}"
        );
    }
    for msg in open_msgs() {
        assert_eq!(
            authorize_outbound(BOTH, &NO, NONE, true, &msg),
            Ok(()),
            "{msg:?} passes paused"
        );
    }
}

#[test]
fn outbound_datagram_table() {
    use SendError::*;
    let only_clip = ModuleSet {
        clipboard: true,
        input: false,
    };
    assert_eq!(authorize_outbound_datagram(BOTH, BOTH, false), Ok(()));
    assert_eq!(
        authorize_outbound_datagram(only_clip, BOTH, false),
        Err(LocalModuleDisabled)
    );
    assert_eq!(
        authorize_outbound_datagram(BOTH, only_clip, true),
        Err(RemoteModuleDisabled)
    );
    assert_eq!(authorize_outbound_datagram(BOTH, BOTH, true), Err(Paused));
}
