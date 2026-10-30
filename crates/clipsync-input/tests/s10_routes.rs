use clipsync_input::{
    route_session::{LocalReturn, RouteSession},
    route_track::RouteTrack,
};
use clipsync_proto::*;
fn inventory() -> MonitorInventory {
    MonitorInventory {
        rev: 9,
        space: CoordinateSpace::Logical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: false,
            positioned_return: false,
            route_receiver: true,
        },
        monitors: vec![MonitorInfo {
            uid: MonitorUid([1; 16]),
            label: "N".into(),
            rect: NativeRect {
                x: 0,
                y: 0,
                width: 1000,
                height: 1000,
            },
            scale: 1.5,
        }],
    }
}
fn entry(edge: Edge) -> EnterRoutes {
    EnterRoutes {
        session: 7,
        inventories: InventoryRevisions {
            source: 8,
            target: 9,
        },
        monitor: MonitorUid([1; 16]),
        edge,
        pos: 0.5,
        routes: SessionRoutes {
            first: ExitRoute {
                id: RouteId(0),
                monitor: MonitorUid([1; 16]),
                edge: Edge::Left,
                from: 0.1,
                to: 0.9,
            },
            second: Some(ExitRoute {
                id: RouteId(1),
                monitor: MonitorUid([1; 16]),
                edge: Edge::Right,
                from: 0.1,
                to: 0.9,
            }),
        },
    }
}
#[test]
fn both_entries_can_return_through_either_side_and_truncate_following_keys() {
    for edge in [Edge::Left, Edge::Right] {
        for (dx, id) in [(-2000.0, 0), (2000.0, 1)] {
            let mut track = RouteTrack::new(&inventory(), &entry(edge)).unwrap();
            let mut events = vec![
                InputEvent::Motion { dx, dy: 0.0 },
                InputEvent::Key {
                    code: 1,
                    pressed: true,
                },
            ];
            let (route, pos) = track.step(&mut events).unwrap();
            assert_eq!(route, RouteId(id));
            assert!((pos - 0.5).abs() < 0.001);
            assert_eq!(events.len(), 1);
        }
    }
}
#[test]
fn one_position_handles_diagonal_motion_dead_corners_and_jitter() {
    let mut track = RouteTrack::new(&inventory(), &entry(Edge::Left)).unwrap();
    track.sync(0, 500);
    assert!(
        track
            .step(&mut vec![InputEvent::Motion { dx: -8.0, dy: 0.0 }])
            .is_none()
    );
    assert!(
        track
            .step(&mut vec![InputEvent::Motion { dx: 1.0, dy: 0.0 }])
            .is_none()
    );
    track.sync(500, 500);
    assert!(
        track
            .step(&mut vec![InputEvent::Motion {
                dx: 100.0,
                dy: 100.0
            }])
            .is_none()
    );
    let (_, pos) = track
        .step(&mut vec![InputEvent::Motion {
            dx: 1000.0,
            dy: 0.0,
        }])
        .unwrap();
    assert!((pos - 600.0 / 999.0).abs() < 0.001);
    let mut track = RouteTrack::new(&inventory(), &entry(Edge::Left)).unwrap();
    track.sync(0, 0);
    assert!(
        track
            .step(&mut vec![InputEvent::Motion {
                dx: -2000.0,
                dy: 0.0
            }])
            .is_none()
    );
}
#[test]
fn inventory_mismatch_or_ambiguous_monitor_refuses_track() {
    let mut inv = inventory();
    inv.rev += 1;
    assert!(RouteTrack::new(&inv, &entry(Edge::Left)).is_err());
    inv = inventory();
    inv.monitors.push(inv.monitors[0].clone());
    assert!(RouteTrack::new(&inv, &entry(Edge::Left)).is_err());
}
#[test]
fn frozen_return_table_rejects_every_wrong_binding_and_maps_to_b() {
    let peer = PeerId([2; 32]);
    let connection = 3;
    let session = RouteSession {
        peer,
        connection,
        session: 7,
        inventories: InventoryRevisions {
            source: 8,
            target: 9,
        },
        returns: vec![LocalReturn {
            id: RouteId(1),
            key: MonitorKey {
                peer: PeerId([3; 32]),
                uid: MonitorUid([4; 16]),
            },
            barrier: 5,
            local: (0.2, 0.8),
            remote: (0.1, 0.9),
        }],
    };
    let good = ReturnedRoute {
        session: 7,
        inventories: session.inventories,
        route: RouteId(1),
        pos: 0.5,
    };
    let (barrier, pos) = session
        .resolve(peer, connection, &good, session.inventories)
        .unwrap();
    assert_eq!(barrier, 5);
    assert!((pos - 0.5).abs() < 0.000001);
    assert!(
        session
            .resolve(PeerId([5; 32]), connection, &good, session.inventories)
            .is_none()
    );
    assert!(
        session
            .resolve(peer, connection + 1, &good, session.inventories)
            .is_none()
    );
    for bad in [
        ReturnedRoute {
            session: 8,
            ..good.clone()
        },
        ReturnedRoute {
            route: RouteId(0),
            ..good.clone()
        },
        ReturnedRoute {
            pos: 0.99,
            ..good.clone()
        },
    ] {
        assert!(
            session
                .resolve(peer, connection, &bad, session.inventories)
                .is_none()
        );
    }
    assert!(
        session
            .resolve(
                peer,
                connection,
                &good,
                InventoryRevisions {
                    source: 10,
                    target: 9
                }
            )
            .is_none()
    );
}

#[test]
fn all_configured_fractions_and_explicit_physical_scale_use_the_same_position() {
    for pos in [0.1, 0.125, 0.25, 0.5, 0.75, 0.875, 0.9] {
        for edge in [Edge::Left, Edge::Right] {
            for dx in [-2000.0, 2000.0] {
                let mut e = entry(edge);
                e.pos = pos;
                let mut track = RouteTrack::new(&inventory(), &e).unwrap();
                let (_, returned) = track
                    .step(&mut vec![InputEvent::Motion { dx, dy: 0.0 }])
                    .unwrap();
                assert!((returned - pos).abs() < 0.00001);
            }
        }
    }
    let mut inv = inventory();
    inv.space = CoordinateSpace::Physical;
    let mut track = RouteTrack::new(&inv, &entry(Edge::Left)).unwrap();
    track.sync(500, 500);
    assert!(
        track
            .step(&mut vec![InputEvent::Motion {
                dx: 100.0,
                dy: 100.0
            }])
            .is_none()
    );
    let (_, pos) = track
        .step(&mut vec![InputEvent::Motion {
            dx: 1000.0,
            dy: 0.0,
        }])
        .unwrap();
    assert!((pos - 650.0 / 999.0).abs() < 0.00001);
}
