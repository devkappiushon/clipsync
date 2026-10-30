// Helpers outside #[test] are test code too.
#![allow(clippy::unwrap_used)]

use clipsync_proto::*;

fn inventory() -> MonitorInventory {
    MonitorInventory {
        rev: 7,
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
                x: -100,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: 1.0,
        }],
    }
}

#[test]
fn v4_is_announced_without_raising_the_connection_minimum() {
    assert_eq!(PROTOCOL_VERSION, 4);
    assert_eq!(MIN_PROTOCOL_VERSION, 1);
}

#[test]
fn all_eleven_legacy_input_variants_keep_v3_golden_bytes() {
    let cases = [
        (
            InputMsg::Enter {
                session: 1,
                edge: Edge::Right,
                pos: 0.5,
            },
            vec![0, 1, 1, 0, 0, 0, 63],
        ),
        (InputMsg::Leave { session: 1 }, vec![1, 1]),
        (InputMsg::ReleaseAll { session: 1 }, vec![2, 1]),
        (InputMsg::StatsRequest { session: 1 }, vec![3, 1]),
        (
            InputMsg::Stats(LoopbackStats::default()),
            vec![4, 0, 0, 0, 0, 0, 0, 0],
        ),
        (InputMsg::Zones { zones: vec![] }, vec![5, 0]),
        (
            InputMsg::LayoutOffer(LayoutOffer {
                rev: 1,
                devices: vec![],
                links: vec![],
            }),
            vec![6, 1, 0, 0],
        ),
        (
            InputMsg::EnterAt {
                session: 1,
                edge: Edge::Left,
                monitor: None,
                pos: 0.5,
            },
            vec![7, 1, 0, 0, 0, 0, 0, 63],
        ),
        (
            InputMsg::EnterSpan {
                session: 1,
                edge: Edge::Left,
                monitor: None,
                pos: 0.5,
                back_from: 0.0,
                back_to: 1.0,
            },
            vec![8, 1, 0, 0, 0, 0, 0, 63, 0, 0, 0, 0, 0, 0, 128, 63],
        ),
        (
            InputMsg::Returned {
                session: 1,
                pos: 0.5,
            },
            vec![9, 1, 0, 0, 0, 63],
        ),
        (
            InputMsg::Refused {
                session: 1,
                reason: RejectReason::Locked,
            },
            vec![10, 1, 9],
        ),
    ];
    for (msg, golden) in cases {
        assert_eq!(postcard::to_allocvec(&msg).unwrap(), golden);
        let back: InputMsg = postcard::from_bytes(&golden).unwrap();
        assert_eq!(postcard::to_allocvec(&back).unwrap(), golden);
    }
}

#[test]
fn inventory_rejects_seventeen_monitors_during_deserialization() {
    let mut inv = inventory();
    inv.monitors = (1..=17)
        .map(|n| MonitorInfo {
            uid: MonitorUid([n; 16]),
            ..inv.monitors[0].clone()
        })
        .collect();
    let raw = postcard::to_allocvec(&inv).unwrap();
    assert!(postcard::from_bytes::<MonitorInventory>(&raw).is_err());
}

fn route(id: u8) -> ExitRoute {
    ExitRoute {
        id: RouteId(id),
        monitor: MonitorUid([1; 16]),
        edge: Edge::Left,
        from: 0.1,
        to: 0.9,
    }
}
fn entry() -> EnterRoutes {
    EnterRoutes {
        session: 1,
        inventories: InventoryRevisions {
            source: 7,
            target: 8,
        },
        monitor: MonitorUid([1; 16]),
        edge: Edge::Left,
        pos: 0.5,
        routes: SessionRoutes {
            first: route(0),
            second: Some(ExitRoute {
                edge: Edge::Right,
                ..route(1)
            }),
        },
    }
}
fn offer_v4() -> LayoutOfferV4 {
    let a = MonitorKey {
        peer: PeerId([1; 32]),
        uid: MonitorUid([1; 16]),
    };
    let b = MonitorKey {
        peer: PeerId([2; 32]),
        uid: MonitorUid([2; 16]),
    };
    LayoutOfferV4 {
        rev: 1,
        devices: vec![
            OfferDevice {
                peer: a.peer,
                x: 0,
                y: 0,
            },
            OfferDevice {
                peer: b.peer,
                x: 1,
                y: 0,
            },
        ],
        positions: vec![
            MonitorPosition {
                key: a,
                x: -1920.0,
                y: 0.0,
            },
            MonitorPosition {
                key: b,
                x: 0.0,
                y: 0.0,
            },
        ],
        links: vec![MonitorLink {
            id: "passagem".into(),
            a: MonitorEndpoint {
                key: a,
                edge: Edge::Right,
                from: 0.1,
                to: 0.9,
            },
            b: MonitorEndpoint {
                key: b,
                edge: Edge::Left,
                from: 0.1,
                to: 0.9,
            },
        }],
        legacy_links: vec![],
    }
}
fn assert_invalid(msg: InputMsg) {
    assert!(msg.validate().is_err(), "{msg:?}");
    let frame = Frame::Module(ModuleMsg::Input(msg));
    assert!(encode_frame(&frame, MAX_FRAME_BYTES).is_err());
    let malicious = postcard::to_allocvec(&frame).unwrap();
    assert!(decode_frame_body(&malicious).is_err());
}

#[test]
fn four_v4_messages_roundtrip_with_appended_tags() {
    let messages = [
        InputMsg::InventoryV4(inventory()),
        InputMsg::EnterRoutes(entry()),
        InputMsg::ReturnedRoute(ReturnedRoute {
            session: 1,
            inventories: InventoryRevisions {
                source: 7,
                target: 8,
            },
            route: RouteId(1),
            pos: 0.25,
        }),
        InputMsg::LayoutOfferV4(offer_v4()),
    ];
    for (msg, tag) in messages.into_iter().zip(11..=14) {
        let body = postcard::to_allocvec(&msg).unwrap();
        assert_eq!(body[0], tag);
        let frame = Frame::Module(ModuleMsg::Input(msg));
        let bytes = encode_frame(&frame, MAX_FRAME_BYTES).unwrap();
        let back = decode_frame_body(&bytes[4..]).unwrap();
        assert_eq!(postcard::to_allocvec(&back).unwrap(), bytes[4..]);
    }
}

#[test]
fn routes_are_fixed_one_or_two_and_reject_invalid_ids_spans_and_sessions() {
    for second in [None, Some(route(1))] {
        let routes = SessionRoutes {
            first: route(0),
            second,
        };
        let raw = postcard::to_allocvec(&routes).unwrap();
        assert_eq!(
            postcard::to_allocvec(&postcard::from_bytes::<SessionRoutes>(&raw).unwrap()).unwrap(),
            raw
        );
    }
    for bad in ["[]", "[{}, {}, {}]", "{}", "{\"second\":null}"] {
        assert!(serde_json::from_str::<SessionRoutes>(bad).is_err());
    }
    let frame = Frame::Module(ModuleMsg::Input(InputMsg::EnterRoutes(entry())));
    let mut bytes = postcard::to_allocvec(&frame).unwrap();
    bytes.extend(postcard::to_allocvec(&route(1)).unwrap());
    assert!(
        decode_frame_body(&bytes).is_err(),
        "a third route must not be ignored"
    );
    for mutate in [
        |e: &mut EnterRoutes| e.session = 0,
        |e: &mut EnterRoutes| e.inventories.source = 0,
        |e: &mut EnterRoutes| e.inventories.target = 0,
        |e: &mut EnterRoutes| e.monitor = MonitorUid([0; 16]),
        |e: &mut EnterRoutes| e.routes.first.monitor = MonitorUid([0; 16]),
        |e: &mut EnterRoutes| e.routes.first.id = RouteId(2),
        |e: &mut EnterRoutes| e.routes.second.as_mut().unwrap().id = RouteId(0),
        |e: &mut EnterRoutes| e.routes.first.to = e.routes.first.from,
        |e: &mut EnterRoutes| e.routes.first.from = f32::NAN,
        |e: &mut EnterRoutes| e.routes.first.to = f32::INFINITY,
        |e: &mut EnterRoutes| e.pos = f32::NAN,
        |e: &mut EnterRoutes| e.pos = 1.01,
    ] {
        let mut e = entry();
        mutate(&mut e);
        assert_invalid(InputMsg::EnterRoutes(e));
    }
    let mut e = entry();
    e.routes.second = None;
    let mut raw =
        postcard::to_allocvec(&Frame::Module(ModuleMsg::Input(InputMsg::EnterRoutes(e)))).unwrap();
    raw.truncate(raw.len() - 1);
    assert!(
        decode_frame_body(&raw).is_err(),
        "missing first/option cannot encode zero routes"
    );
}

#[test]
fn returned_route_rejects_zero_bindings_unknown_id_and_nonfinite_position() {
    let valid = ReturnedRoute {
        session: 1,
        inventories: InventoryRevisions {
            source: 7,
            target: 8,
        },
        route: RouteId(0),
        pos: 0.5,
    };
    for mutate in [
        |r: &mut ReturnedRoute| r.session = 0,
        |r: &mut ReturnedRoute| r.inventories.source = 0,
        |r: &mut ReturnedRoute| r.inventories.target = 0,
        |r: &mut ReturnedRoute| r.route = RouteId(2),
        |r: &mut ReturnedRoute| r.pos = f32::INFINITY,
        |r: &mut ReturnedRoute| r.pos = -0.1,
    ] {
        let mut r = valid.clone();
        mutate(&mut r);
        assert_invalid(InputMsg::ReturnedRoute(r));
    }
}

#[test]
fn inventory_validates_labels_geometry_revisions_and_unique_uids() {
    for mutate in [
        |i: &mut MonitorInventory| i.rev = 0,
        |i: &mut MonitorInventory| i.monitors[0].uid = MonitorUid([0; 16]),
        |i: &mut MonitorInventory| i.monitors.push(i.monitors[0].clone()),
        |i: &mut MonitorInventory| i.monitors[0].label = "a\nb".into(),
        |i: &mut MonitorInventory| i.monitors[0].label = "a\u{202e}".into(),
        |i: &mut MonitorInventory| i.monitors[0].rect.width = 0,
        |i: &mut MonitorInventory| i.monitors[0].scale = f64::INFINITY,
        |i: &mut MonitorInventory| i.monitors[0].scale = f64::NAN,
        |i: &mut MonitorInventory| i.monitors[0].scale = 0.0,
    ] {
        let mut i = inventory();
        mutate(&mut i);
        assert_invalid(InputMsg::InventoryV4(i));
    }
}

#[test]
fn offers_validate_canvas_peers_uids_link_ids_and_aggregate_limit() {
    for mutate in [
        |o: &mut LayoutOfferV4| o.rev = 0,
        |o: &mut LayoutOfferV4| o.positions[0].x = 1_000_001.0,
        |o: &mut LayoutOfferV4| o.positions[0].y = f64::NAN,
        |o: &mut LayoutOfferV4| o.devices[0].x = -1_000_001,
        |o: &mut LayoutOfferV4| o.devices.push(o.devices[0].clone()),
        |o: &mut LayoutOfferV4| o.positions.push(o.positions[0].clone()),
        |o: &mut LayoutOfferV4| o.positions[0].key.uid = MonitorUid([0; 16]),
        |o: &mut LayoutOfferV4| o.positions[0].key.peer = PeerId([3; 32]),
        |o: &mut LayoutOfferV4| o.links[0].a.key.peer = PeerId([3; 32]),
        |o: &mut LayoutOfferV4| o.links[0].a.from = f32::NAN,
        |o: &mut LayoutOfferV4| o.links[0].b.to = o.links[0].b.from,
        |o: &mut LayoutOfferV4| o.links.push(o.links[0].clone()),
        |o: &mut LayoutOfferV4| o.links[0].id = "a\u{202e}".into(),
    ] {
        let mut o = offer_v4();
        mutate(&mut o);
        assert_invalid(InputMsg::LayoutOfferV4(o));
    }
    let mut o = offer_v4();
    o.legacy_links = (0..64)
        .map(|_| OfferLink {
            a: OfferEndpoint {
                peer: o.devices[0].peer,
                monitor: "Tela A".into(),
                edge: Edge::Right,
                from: 0.1,
                to: 0.9,
            },
            b: OfferEndpoint {
                peer: o.devices[1].peer,
                monitor: "Tela B".into(),
                edge: Edge::Left,
                from: 0.1,
                to: 0.9,
            },
        })
        .collect();
    assert_invalid(InputMsg::LayoutOfferV4(o.clone()));
    assert!(postcard::from_bytes::<LayoutOfferV4>(&postcard::to_allocvec(&o).unwrap()).is_err());
    o.legacy_links.pop();
    assert!(
        InputMsg::LayoutOfferV4(o).validate().is_ok(),
        "64 aggregate links remain representable"
    );
}

#[test]
fn new_vectors_reject_declared_huge_lengths_and_limits_without_elements() {
    let huge = postcard::to_allocvec(&u64::MAX).unwrap();
    let mut inv = inventory();
    inv.monitors.clear();
    let mut raw = postcard::to_allocvec(&inv).unwrap();
    raw.pop();
    raw.extend(&huge);
    assert!(postcard::from_bytes::<MonitorInventory>(&raw).is_err());
    for field in 0..4 {
        let mut o = offer_v4();
        o.devices.clear();
        o.positions.clear();
        o.links.clear();
        o.legacy_links.clear();
        let raw = postcard::to_allocvec(&o).unwrap();
        let prefix = 1 + field;
        let mut malicious = raw[..prefix].to_vec();
        malicious.extend(&huge);
        assert!(postcard::from_bytes::<LayoutOfferV4>(&malicious).is_err());
    }
    for (count, field) in [(17usize, 0), (257, 1), (65, 2), (65, 3)] {
        let mut o = offer_v4();
        o.devices.clear();
        o.positions.clear();
        o.links.clear();
        o.legacy_links.clear();
        let raw = postcard::to_allocvec(&o).unwrap();
        let mut malicious = raw[..1 + field].to_vec();
        malicious.extend(postcard::to_allocvec(&count).unwrap());
        assert!(postcard::from_bytes::<LayoutOfferV4>(&malicious).is_err());
    }
}

#[test]
fn v4_gates_preserve_one_way_control_and_versions() {
    let both = ModuleSet {
        clipboard: true,
        input: true,
    };
    let off = ModuleSet {
        clipboard: true,
        input: false,
    };
    let messages = [
        InputMsg::InventoryV4(inventory()),
        InputMsg::EnterRoutes(entry()),
        InputMsg::ReturnedRoute(ReturnedRoute {
            session: 1,
            inventories: InventoryRevisions {
                source: 7,
                target: 8,
            },
            route: RouteId(0),
            pos: 0.5,
        }),
        InputMsg::LayoutOfferV4(offer_v4()),
    ];
    for (index, input) in messages.into_iter().enumerate() {
        let msg = ModuleMsg::Input(input);
        assert_eq!(min_version(&msg), 4);
        for input_in in [false, true] {
            for paused in [false, true] {
                let perms = PeerPerms {
                    input_in,
                    ..PeerPerms::default()
                };
                let inbound = match index {
                    1 | 3 if !input_in => Err(RejectReason::NotAuthorized),
                    1 if paused => Err(RejectReason::Paused),
                    _ => Ok(()),
                };
                let outbound = match index {
                    0 if !input_in => Err(SendError::NotAuthorized),
                    1 if paused => Err(SendError::Paused),
                    _ => Ok(()),
                };
                assert_eq!(authorize_inbound(both, &perms, paused, &msg), inbound);
                assert_eq!(
                    authorize_outbound(both, &perms, both, paused, &msg),
                    outbound
                );
                assert_eq!(
                    authorize_inbound(off, &perms, paused, &msg),
                    Err(RejectReason::ModuleDisabled)
                );
                assert_eq!(
                    authorize_outbound(off, &perms, both, paused, &msg),
                    Err(SendError::LocalModuleDisabled)
                );
                assert_eq!(
                    authorize_outbound(both, &perms, off, paused, &msg),
                    if index == 2 {
                        Ok(())
                    } else {
                        Err(SendError::RemoteModuleDisabled)
                    }
                );
            }
        }
    }
    assert_eq!(
        min_version(&ModuleMsg::Input(InputMsg::Enter {
            session: 1,
            edge: Edge::Left,
            pos: 0.5
        })),
        1
    );
    assert_eq!(
        min_version(&ModuleMsg::Input(InputMsg::Zones { zones: vec![] })),
        2
    );
    assert_eq!(
        min_version(&ModuleMsg::Input(InputMsg::Returned {
            session: 1,
            pos: 0.5
        })),
        3
    );
    // Desktop grants no input_in; notebook grants it. No source inventory at the receiver.
    let enter = ModuleMsg::Input(InputMsg::EnterRoutes(entry()));
    let back = ModuleMsg::Input(InputMsg::ReturnedRoute(ReturnedRoute {
        session: 1,
        inventories: entry().inventories,
        route: RouteId(1),
        pos: 0.5,
    }));
    let desktop = PeerPerms::default();
    let notebook = PeerPerms {
        input_in: true,
        ..desktop
    };
    assert_eq!(
        authorize_outbound(both, &desktop, both, false, &enter),
        Ok(())
    );
    assert_eq!(authorize_inbound(both, &notebook, false, &enter), Ok(()));
    assert_eq!(
        authorize_outbound(both, &notebook, both, false, &back),
        Ok(())
    );
    assert_eq!(authorize_inbound(both, &desktop, false, &back), Ok(()));
}

#[test]
fn public_v4_payloads_contain_only_public_inventory_and_route_data() {
    let payloads = [
        InputMsg::InventoryV4(inventory()),
        InputMsg::EnterRoutes(entry()),
        InputMsg::ReturnedRoute(ReturnedRoute {
            session: 1,
            inventories: entry().inventories,
            route: RouteId(1),
            pos: 0.5,
        }),
        InputMsg::LayoutOfferV4(offer_v4()),
    ];
    for payload in payloads {
        let json = serde_json::to_string(&payload).unwrap();
        let debug = format!("{payload:?}");
        for forbidden in [
            "path",
            "edid",
            "aliases",
            "fingerprint",
            "pressed",
            "code",
            "4242",
        ] {
            assert!(!json.to_lowercase().contains(forbidden), "{json}");
            assert!(!debug.to_lowercase().contains(forbidden), "{debug}");
        }
    }
}

#[tokio::test]
async fn host_refuses_only_messages_newer_than_the_remote_version() {
    let modules = ModuleSet {
        clipboard: true,
        input: true,
    };
    let grants = PeerPerms {
        input_in: true,
        ..PeerPerms::default()
    };
    for version in 1..=4 {
        let hub = MemHub::new();
        let source = hub.add_node("origem", modules);
        let destination = hub.add_node("destino", modules);
        hub.set_remote_version(destination.id, version);
        hub.link(source.id, destination.id, grants, grants);
        let messages = [
            (
                InputMsg::Enter {
                    session: 1,
                    edge: Edge::Left,
                    pos: 0.5,
                },
                1,
            ),
            (InputMsg::Zones { zones: vec![] }, 2),
            (
                InputMsg::Returned {
                    session: 1,
                    pos: 0.5,
                },
                3,
            ),
            (InputMsg::InventoryV4(inventory()), 4),
            (InputMsg::EnterRoutes(entry()), 4),
            (
                InputMsg::ReturnedRoute(ReturnedRoute {
                    session: 1,
                    inventories: entry().inventories,
                    route: RouteId(0),
                    pos: 0.5,
                }),
                4,
            ),
            (InputMsg::LayoutOfferV4(offer_v4()), 4),
        ];
        for (input, required) in messages {
            let count = hub.sent_count(source.id, destination.id);
            assert_eq!(
                source
                    .host
                    .send_reliable(destination.id, ModuleMsg::Input(input))
                    .await,
                if version >= required {
                    Ok(())
                } else {
                    Err(SendError::RemoteTooOld)
                }
            );
            assert_eq!(
                hub.sent_count(source.id, destination.id),
                count + usize::from(version >= required)
            );
        }
    }
}

#[tokio::test]
async fn host_routes_entry_and_return_without_a_reciprocal_grant_or_source_inventory() {
    let modules = ModuleSet {
        clipboard: false,
        input: true,
    };
    let hub = MemHub::new();
    let mut desktop = hub.add_node("desktop", modules);
    let mut notebook = hub.add_node("notebook", modules);
    hub.link(
        desktop.id,
        notebook.id,
        PeerPerms::default(),
        PeerPerms {
            input_in: true,
            ..PeerPerms::default()
        },
    );
    desktop.rx.try_recv().unwrap();
    notebook.rx.try_recv().unwrap(); // PeerUp only; no inventory sent.
    desktop
        .host
        .send_reliable(
            notebook.id,
            ModuleMsg::Input(InputMsg::EnterRoutes(entry())),
        )
        .await
        .unwrap();
    let ModuleInput::Reliable {
        from,
        msg: ModuleMsg::Input(InputMsg::EnterRoutes(accepted)),
        context,
    } = notebook.rx.try_recv().unwrap()
    else {
        panic!("entry was not delivered")
    };
    assert_eq!(from, desktop.id);
    assert!(
        notebook
            .host
            .peer_input_context(desktop.id)
            .unwrap()
            .permits(context)
    );
    assert_eq!(accepted.inventories.source, 7);
    notebook
        .host
        .send_reliable(
            desktop.id,
            ModuleMsg::Input(InputMsg::ReturnedRoute(ReturnedRoute {
                session: accepted.session,
                inventories: accepted.inventories,
                route: RouteId(1),
                pos: 0.5,
            })),
        )
        .await
        .unwrap();
    assert!(
        matches!(desktop.rx.try_recv().unwrap(), ModuleInput::Reliable { from, msg: ModuleMsg::Input(InputMsg::ReturnedRoute(_)), .. } if from == notebook.id)
    );
    assert!(
        !desktop
            .host
            .peer_input_context(notebook.id)
            .unwrap()
            .input_in
    );
    assert_eq!(hub.sent_count(desktop.id, notebook.id), 1);
    assert_eq!(hub.sent_count(notebook.id, desktop.id), 1);
}

// A hostile sequence exposes an enormous size hint and fails if an element is requested.
// This proves the bound is checked before element decoding (and hence reservation), rather
// than merely failing later on a truncated postcard body.
struct UnreadSequence<'a> {
    reads: &'a std::cell::Cell<usize>,
    length: usize,
}
impl<'de> serde::de::SeqAccess<'de> for UnreadSequence<'_> {
    type Error = serde::de::value::Error;
    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        _: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        self.reads.set(self.reads.get() + 1);
        Err(serde::de::Error::custom("element read before length bound"))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.length)
    }
}
struct ProbeFields<'a> {
    prefix: &'a [&'a [u8]],
    index: usize,
    reads: &'a std::cell::Cell<usize>,
    length: usize,
}
impl<'a: 'de, 'de> serde::de::SeqAccess<'de> for ProbeFields<'a> {
    type Error = serde::de::value::Error;
    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if let Some(raw) = self.prefix.get(self.index) {
            self.index += 1;
            seed.deserialize(&mut postcard::Deserializer::from_bytes(raw))
                .map(Some)
                .map_err(serde::de::Error::custom)
        } else {
            seed.deserialize(serde::de::value::SeqAccessDeserializer::new(
                UnreadSequence {
                    reads: self.reads,
                    length: self.length,
                },
            ))
            .map(Some)
        }
    }
}
#[test]
fn bounded_vectors_reject_size_hints_before_reading_even_one_element() {
    use serde::Deserialize;
    for length in [17, usize::MAX] {
        let reads = std::cell::Cell::new(0);
        let error = MonitorInventory::deserialize(serde::de::value::SeqAccessDeserializer::new(
            ProbeFields {
                prefix: &[&[1], &[0], &[0], &[0, 0, 1]],
                index: 0,
                reads: &reads,
                length,
            },
        ))
        .unwrap_err();
        assert!(error.to_string().contains("too many elements"), "{error}");
        assert_eq!(reads.get(), 0);
    }
    for (prefix, limit) in [
        (&[&[1][..]][..], 16),
        (&[&[1][..], &[0]][..], 256),
        (&[&[1][..], &[0], &[0]][..], 64),
        (&[&[1][..], &[0], &[0], &[0]][..], 64),
    ] {
        for length in [limit + 1, usize::MAX] {
            let reads = std::cell::Cell::new(0);
            let error = LayoutOfferV4::deserialize(serde::de::value::SeqAccessDeserializer::new(
                ProbeFields {
                    prefix,
                    index: 0,
                    reads: &reads,
                    length,
                },
            ))
            .unwrap_err();
            assert!(error.to_string().contains("too many elements"), "{error}");
            assert_eq!(reads.get(), 0);
        }
    }
}

#[test]
fn json_value_offer_bounds_work_with_reversed_link_fields_and_known_lengths() {
    let offer = offer_v4();
    let json = serde_json::to_value(&offer).unwrap();
    assert_eq!(
        serde_json::from_value::<LayoutOfferV4>(json.clone()).unwrap(),
        offer
    );
    for (field, count) in [("devices", 17), ("positions", 257), ("links", 65)] {
        let mut value = json.clone();
        let element = value[field][0].clone();
        value[field] = serde_json::Value::Array(vec![element; count]);
        assert!(serde_json::from_value::<LayoutOfferV4>(value).is_err());
    }
    let endpoint = OfferEndpoint {
        peer: offer.devices[0].peer,
        monitor: "Tela".into(),
        edge: Edge::Left,
        from: 0.1,
        to: 0.9,
    };
    let legacy = serde_json::to_value(OfferLink {
        a: endpoint.clone(),
        b: endpoint,
    })
    .unwrap();
    // serde_json's sorted map sees legacy_links before links; both share one budget.
    let mut value = json;
    value["legacy_links"] = serde_json::Value::Array(vec![legacy; 64]);
    assert!(serde_json::from_value::<LayoutOfferV4>(value).is_err());
    let routes = SessionRoutes {
        first: route(0),
        second: Some(route(1)),
    };
    let mut three = serde_json::to_value(routes).unwrap();
    three["third"] = serde_json::to_value(route(0)).unwrap();
    assert!(serde_json::from_value::<SessionRoutes>(three).is_err());
}

#[test]
fn maximum_inventories_and_offers_remain_representable() {
    let mut inv = inventory();
    inv.monitors = (1..=16)
        .map(|id| MonitorInfo {
            uid: MonitorUid([id; 16]),
            ..inv.monitors[0].clone()
        })
        .collect();
    let frame = Frame::Module(ModuleMsg::Input(InputMsg::InventoryV4(inv)));
    assert!(decode_frame_body(&encode_frame(&frame, MAX_FRAME_BYTES).unwrap()[4..]).is_ok());
    let mut offer = offer_v4();
    offer.devices = (1..=16)
        .map(|id| OfferDevice {
            peer: PeerId([id; 32]),
            x: -1_000_000,
            y: 1_000_000,
        })
        .collect();
    offer.positions = offer
        .devices
        .iter()
        .flat_map(|device| {
            (1..=16).map(move |id| MonitorPosition {
                key: MonitorKey {
                    peer: device.peer,
                    uid: MonitorUid([id; 16]),
                },
                x: -1_000_000.0,
                y: 1_000_000.0,
            })
        })
        .collect();
    offer.links = (0..64)
        .map(|id| MonitorLink {
            id: format!("passagem-{id}"),
            ..offer.links[0].clone()
        })
        .collect();
    let frame = Frame::Module(ModuleMsg::Input(InputMsg::LayoutOfferV4(offer)));
    assert!(decode_frame_body(&encode_frame(&frame, MAX_FRAME_BYTES).unwrap()[4..]).is_ok());
}

#[test]
fn aggregate_link_budget_is_checked_before_legacy_elements_are_read() {
    use serde::Deserialize;
    let offer = offer_v4();
    let links = postcard::to_allocvec(&offer.links).unwrap();
    let reads = std::cell::Cell::new(0);
    let error =
        LayoutOfferV4::deserialize(serde::de::value::SeqAccessDeserializer::new(ProbeFields {
            prefix: &[&[1], &[0], &[0], &links],
            index: 0,
            reads: &reads,
            length: 64,
        }))
        .unwrap_err();
    assert!(error.to_string().contains("too many elements"), "{error}");
    assert_eq!(reads.get(), 0);
}

// Counts the actual entry into the DTO's Deserialize implementation, before its fields
// (including Strings) can be materialized. Merely requesting an element is not construction:
// the exhausted-budget seed may reject without touching this deserializer.
struct CountConstructed<'a> {
    raw: &'a [u8],
    constructed: &'a std::cell::Cell<usize>,
}
impl<'a: 'de, 'de> serde::Deserializer<'de> for CountConstructed<'a> {
    type Error = serde::de::value::Error;
    fn deserialize_any<V: serde::de::Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::custom("expected a public DTO struct"))
    }
    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.constructed.set(self.constructed.get() + 1);
        serde::Deserializer::deserialize_struct(
            &mut postcard::Deserializer::from_bytes(self.raw),
            name,
            fields,
            visitor,
        )
        .map_err(serde::de::Error::custom)
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map enum
        identifier ignored_any
    }
}
struct NoHintSequence<'a> {
    element: &'a [u8],
    remaining: usize,
    constructed: &'a std::cell::Cell<usize>,
}
impl<'a: 'de, 'de> serde::de::SeqAccess<'de> for NoHintSequence<'a> {
    type Error = serde::de::value::Error;
    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(CountConstructed {
            raw: self.element,
            constructed: self.constructed,
        })
        .map(Some)
    }
    // Deliberately no size_hint: the default is None.
}
struct NoHintFields<'a> {
    prefix: &'a [&'a [u8]],
    index: usize,
    element: &'a [u8],
    count: usize,
    constructed: &'a std::cell::Cell<usize>,
}
impl<'a: 'de, 'de> serde::de::SeqAccess<'de> for NoHintFields<'a> {
    type Error = serde::de::value::Error;
    fn next_element_seed<T: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if let Some(raw) = self.prefix.get(self.index) {
            self.index += 1;
            seed.deserialize(&mut postcard::Deserializer::from_bytes(raw))
                .map(Some)
                .map_err(serde::de::Error::custom)
        } else if self.index == self.prefix.len() {
            self.index += 1;
            seed.deserialize(serde::de::value::SeqAccessDeserializer::new(
                NoHintSequence {
                    element: self.element,
                    remaining: self.count,
                    constructed: self.constructed,
                },
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }
}
#[test]
fn no_hint_vectors_reject_before_constructing_the_excess_dto() {
    use serde::Deserialize;
    let inv = inventory();
    let monitor = postcard::to_allocvec(&inv.monitors[0]).unwrap();
    for count in [0, 16, 17] {
        let constructed = std::cell::Cell::new(0);
        let result = MonitorInventory::deserialize(serde::de::value::SeqAccessDeserializer::new(
            NoHintFields {
                prefix: &[&[1], &[0], &[0], &[0, 0, 1]],
                index: 0,
                element: &monitor,
                count,
                constructed: &constructed,
            },
        ));
        assert_eq!(result.is_ok(), count <= 16);
        assert_eq!(
            constructed.get(),
            count.min(16),
            "excess monitor must not deserialize its label"
        );
    }
    let offer = offer_v4();
    let device = postcard::to_allocvec(&offer.devices[0]).unwrap();
    let position = postcard::to_allocvec(&offer.positions[0]).unwrap();
    let link = postcard::to_allocvec(&offer.links[0]).unwrap();
    let endpoint = OfferEndpoint {
        peer: offer.devices[0].peer,
        monitor: "Tela legada".into(),
        edge: Edge::Left,
        from: 0.1,
        to: 0.9,
    };
    let legacy = postcard::to_allocvec(&OfferLink {
        a: endpoint.clone(),
        b: endpoint,
    })
    .unwrap();
    let all_links = postcard::to_allocvec(&vec![offer.links[0].clone(); 64]).unwrap();
    for (prefix, element, max) in [
        (&[&[1][..]][..], device.as_slice(), 16),
        (&[&[1][..], &[0]][..], position.as_slice(), 256),
        (&[&[1][..], &[0], &[0]][..], link.as_slice(), 64),
        (&[&[1][..], &[0], &[0], &[0]][..], legacy.as_slice(), 64),
        (
            &[&[1][..], &[0], &[0], all_links.as_slice()][..],
            legacy.as_slice(),
            0,
        ),
    ] {
        for count in [0, max, max + 1] {
            let constructed = std::cell::Cell::new(0);
            let result = LayoutOfferV4::deserialize(serde::de::value::SeqAccessDeserializer::new(
                NoHintFields {
                    prefix,
                    index: 0,
                    element,
                    count,
                    constructed: &constructed,
                },
            ));
            // For earlier fields, later required fields are absent in this probe. The decisive
            // assertion is the number of actual DTO constructions before any resulting error.
            assert_eq!(
                constructed.get(),
                count.min(max),
                "budget {max}, count {count}"
            );
            if count > max {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("too many elements")
                );
            }
        }
    }
}

// The first link vector consumes all 64 slots; the second vector has no size hint.
// Feed the map in either order to exercise the actual production map visitor.
struct NoHintOfferMap<'a> {
    fields: &'a [(&'static str, &'a [u8])],
    index: usize,
    target: &'static str,
    element: &'a [u8],
    count: usize,
    constructed: &'a std::cell::Cell<usize>,
}
impl<'a: 'de, 'de> serde::de::MapAccess<'de> for NoHintOfferMap<'a> {
    type Error = serde::de::value::Error;
    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.fields.get(self.index) {
            Some((name, _)) => seed
                .deserialize(serde::de::value::BorrowedStrDeserializer::new(name))
                .map(Some),
            None => Ok(None),
        }
    }
    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let (name, raw) = self.fields[self.index];
        self.index += 1;
        if name == self.target {
            seed.deserialize(serde::de::value::SeqAccessDeserializer::new(
                NoHintSequence {
                    element: self.element,
                    remaining: self.count,
                    constructed: self.constructed,
                },
            ))
        } else {
            seed.deserialize(&mut postcard::Deserializer::from_bytes(raw))
                .map_err(serde::de::Error::custom)
        }
    }
}
#[test]
fn no_hint_zero_link_budget_does_not_construct_a_dto_in_either_map_order() {
    use serde::Deserialize;
    let offer = offer_v4();
    let endpoint = OfferEndpoint {
        peer: offer.devices[0].peer,
        monitor: "Tela".into(),
        edge: Edge::Left,
        from: 0.1,
        to: 0.9,
    };
    let legacy = OfferLink {
        a: endpoint.clone(),
        b: endpoint,
    };
    let link = postcard::to_allocvec(&offer.links[0]).unwrap();
    let legacy_element = postcard::to_allocvec(&legacy).unwrap();
    let links = postcard::to_allocvec(&vec![offer.links[0].clone(); 64]).unwrap();
    let legacy_links = postcard::to_allocvec(&vec![legacy; 64]).unwrap();
    for (first, first_raw, target, element) in [
        (
            "links",
            links.as_slice(),
            "legacy_links",
            legacy_element.as_slice(),
        ),
        (
            "legacy_links",
            legacy_links.as_slice(),
            "links",
            link.as_slice(),
        ),
    ] {
        for count in [0, 1] {
            let fields = [
                ("rev", &[1][..]),
                ("devices", &[0][..]),
                ("positions", &[0][..]),
                (first, first_raw),
                (target, &[][..]),
            ];
            let constructed = std::cell::Cell::new(0);
            let result = LayoutOfferV4::deserialize(serde::de::value::MapAccessDeserializer::new(
                NoHintOfferMap {
                    fields: &fields,
                    index: 0,
                    target,
                    element,
                    count,
                    constructed: &constructed,
                },
            ));
            assert_eq!(result.is_ok(), count == 0, "{first} first");
            assert_eq!(
                constructed.get(),
                0,
                "zero remaining budget must reject before any T field"
            );
        }
    }
}

#[test]
fn textual_json_checks_budget_before_parsing_an_excess_value() {
    let mut inv = inventory();
    inv.monitors = (1..=16)
        .map(|n| MonitorInfo {
            uid: MonitorUid([n; 16]),
            ..inv.monitors[0].clone()
        })
        .collect();
    let inv_json = serde_json::to_string(&inv).unwrap();
    assert_eq!(
        serde_json::from_str::<MonitorInventory>(&inv_json).unwrap(),
        inv
    );
    let mut over = serde_json::to_value(&inv).unwrap();
    over["monitors"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::Value::Null);
    let error = serde_json::from_str::<MonitorInventory>(&serde_json::to_string(&over).unwrap())
        .unwrap_err();
    assert!(error.to_string().contains("too many elements"), "{error}");

    let offer = offer_v4();
    for (field, element, limit) in [
        (
            "devices",
            serde_json::to_value(&offer.devices[0]).unwrap(),
            16,
        ),
        (
            "positions",
            serde_json::to_value(&offer.positions[0]).unwrap(),
            256,
        ),
        ("links", serde_json::to_value(&offer.links[0]).unwrap(), 64),
    ] {
        let mut json = serde_json::to_value(&offer).unwrap();
        json[field] = serde_json::Value::Array(vec![element; limit]);
        let text = serde_json::to_string(&json).unwrap();
        assert!(
            serde_json::from_str::<LayoutOfferV4>(&text).is_ok(),
            "{field} exact limit"
        );
        json[field]
            .as_array_mut()
            .unwrap()
            .push(serde_json::Value::Null);
        let error = serde_json::from_str::<LayoutOfferV4>(&serde_json::to_string(&json).unwrap())
            .unwrap_err();
        assert!(
            error.to_string().contains("too many elements"),
            "{field}: {error}"
        );
    }
}

#[test]
fn textual_json_shares_exact_and_zero_link_budgets_in_both_field_orders() {
    let offer = offer_v4();
    let endpoint = OfferEndpoint {
        peer: offer.devices[0].peer,
        monitor: "Tela legada".into(),
        edge: Edge::Left,
        from: 0.1,
        to: 0.9,
    };
    let legacy = OfferLink {
        a: endpoint.clone(),
        b: endpoint,
    };
    for (new_count, legacy_count) in [(64, 0), (63, 1), (1, 63), (0, 64)] {
        let links = serde_json::to_string(&vec![offer.links[0].clone(); new_count]).unwrap();
        let legacy_links = serde_json::to_string(&vec![legacy.clone(); legacy_count]).unwrap();
        for legacy_first in [false, true] {
            let (first, second) = if legacy_first {
                (("legacy_links", &legacy_links), ("links", &links))
            } else {
                (("links", &links), ("legacy_links", &legacy_links))
            };
            let text = format!(
                "{{\"rev\":1,\"devices\":[],\"positions\":[],\"{}\":{},\"{}\":{}}}",
                first.0, first.1, second.0, second.1
            );
            let decoded: LayoutOfferV4 = serde_json::from_str(&text).unwrap();
            assert_eq!(
                (decoded.links.len(), decoded.legacy_links.len()),
                (new_count, legacy_count)
            );
            // Add an invalid object to the LAST vector: its type error must never be reached.
            let mut extra = second.1[..second.1.len() - 1].to_string();
            if extra.len() > 1 {
                extra.push(',');
            }
            extra.push_str("null]");
            let text = format!(
                "{{\"rev\":1,\"devices\":[],\"positions\":[],\"{}\":{},\"{}\":{extra}}}",
                first.0, first.1, second.0
            );
            let error = serde_json::from_str::<LayoutOfferV4>(&text).unwrap_err();
            assert!(error.to_string().contains("too many elements"), "{error}");
        }
    }
}

// Characterize postcard's actual fallback: a declaration larger than remaining bytes has
// no size hint. The following production decodes then exercise that real truncated path.
struct SequenceHasHint(bool);
impl<'de> serde::Deserialize<'de> for SequenceHasHint {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct HintVisitor;
        impl<'de> serde::de::Visitor<'de> for HintVisitor {
            type Value = SequenceHasHint;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a sequence")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                seq: A,
            ) -> Result<Self::Value, A::Error> {
                Ok(SequenceHasHint(seq.size_hint().is_some()))
            }
        }
        d.deserialize_seq(HintVisitor)
    }
}
#[test]
fn truncated_postcard_without_a_size_hint_still_rejects_declared_excess() {
    let huge = postcard::to_allocvec(&usize::MAX).unwrap();
    let monitor = postcard::to_allocvec(&inventory().monitors[0]).unwrap();
    for count in [16, 17] {
        let mut vector = huge.clone();
        for _ in 0..count {
            vector.extend(&monitor);
        }
        assert!(!postcard::from_bytes::<SequenceHasHint>(&vector).unwrap().0);
        let mut inv = inventory();
        inv.monitors.clear();
        let mut raw = postcard::to_allocvec(&inv).unwrap();
        raw.pop();
        raw.extend(vector);
        assert!(postcard::from_bytes::<MonitorInventory>(&raw).is_err());
    }
    let mut offer = offer_v4();
    offer.links = vec![offer.links[0].clone(); 64];
    let mut raw = postcard::to_allocvec(&offer).unwrap();
    raw.pop();
    raw.extend(&huge);
    assert!(!postcard::from_bytes::<SequenceHasHint>(&huge).unwrap().0);
    assert!(
        postcard::from_bytes::<LayoutOfferV4>(&raw).is_err(),
        "zero budget cannot accept a missing declared element"
    );
}
