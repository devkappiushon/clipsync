#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::{
    inventory::{ObservedMonitors, windows_observation},
    *,
};
use clipsync_proto::*;
use std::time::Duration;
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
async fn command(tx: &mpsc::Sender<ModuleInput>, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    tx.send(ModuleInput::Command { cmd, reply }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .unwrap()
        .unwrap()
}
fn observed() -> ObservedMonitors {
    windows_observation(&[clipsync_win::Monitor {
        friendly: "A".into(),
        device: "d".into(),
        path: "private-a".into(),
        x: 0,
        y: 0,
        width: 1000,
        height: 1000,
        dpi: 96,
        primary: true,
    }])
    .unwrap()
}
struct Harness {
    dir: tempfile::TempDir,
    tx: mpsc::Sender<ModuleInput>,
    request: OrganizationRequest,
    cap: FakeCaptureHandle,
    remote: std::sync::Arc<dyn ModuleHost>,
    id: PeerId,
    _hub: MemHub,
    _rx: mpsc::Receiver<ModuleInput>,
}
async fn harness() -> Harness {
    harness_with(1).await
}
async fn harness_with(count: usize) -> Harness {
    let hub = MemHub::new();
    let a = hub.add_node("a", INPUT);
    let b = hub.add_node("b", INPUT);
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout {
        rev: 1,
        options: Default::default(),
        devices: vec![
            LayoutDevice {
                peer: a.id,
                name: "a".into(),
                x: 0,
                y: 0,
                hotkey: Some(1),
            },
            LayoutDevice {
                peer: b.id,
                name: "b".into(),
                x: 1000,
                y: 0,
                hotkey: Some(2),
            },
        ],
        links: vec![],
    };
    layout.write(&dir.path().join("layout.toml")).unwrap();
    let mut observation = observed();
    if count == 2 {
        let mut second = observation.observations[0].clone();
        second.evidence =
            clipsync_input::monitor_identities::Evidence::WindowsPath("private-b".into());
        second.label = "B".into();
        second.legacy_aliases = vec!["B".into()];
        second.rect.x = 1000;
        observation.observations.push(second);
    }
    let zones = observation
        .observations
        .iter()
        .enumerate()
        .map(|(index, m)| Zone {
            index: index as u32,
            name: Some(m.label.clone()),
            x: m.rect.x,
            y: m.rect.y,
            width: m.rect.width,
            height: m.rect.height,
            scale: m.scale,
        })
        .collect();
    let (cap, handle) = FakeCapture::new(zones);
    handle.set_monitor_observation(Some(observation));
    let (emu, _) = FakeEmulate::new();
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(cap),
        Box::new(emu),
        dir.path(),
    )
    .unwrap();
    tokio::spawn(module.run(a.host, a.rx));
    hub.link(a.id, b.id, CONTROL, CONTROL);
    let remote_inventory = MonitorInventory {
        rev: 3,
        space: CoordinateSpace::Logical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: false,
            positioned_return: false,
            route_receiver: true,
        },
        monitors: vec![MonitorInfo {
            uid: MonitorUid([3; 16]),
            label: "N".into(),
            rect: NativeRect {
                x: 0,
                y: 0,
                width: 1000,
                height: 1000,
            },
            scale: 1.0,
        }],
    };
    b.host
        .send_reliable(
            a.id,
            ModuleMsg::Input(InputMsg::InventoryV4(remote_inventory)),
        )
        .await
        .unwrap();
    let state = loop {
        let ModuleReply::InputState(s) = command(&a.tx, ModuleCommand::InputState).await else {
            panic!()
        };
        if s.inventories.len() == 2 {
            break s;
        }
        tokio::task::yield_now().await;
    };
    let positions = state
        .inventories
        .iter()
        .flat_map(|v| {
            v.inventory.monitors.iter().map(|m| MonitorPosition {
                key: MonitorKey {
                    peer: v.stamp.peer,
                    uid: m.uid,
                },
                x: if v.stamp.peer == a.id {
                    f64::from(m.rect.x) * 2.0
                } else {
                    1000.0
                },
                y: 0.0,
            })
        })
        .collect();
    let request = OrganizationRequest {
        base_rev: 1,
        inventories: state.inventories.iter().map(|v| v.stamp).collect(),
        positions,
        replace_link_ids: vec![],
    };
    Harness {
        dir,
        tx: a.tx,
        request,
        cap: handle,
        remote: b.host,
        id: a.id,
        _hub: hub,
        _rx: b.rx,
    }
}
#[tokio::test]
async fn preview_does_not_migrate_and_apply_publishes_once_with_backup() {
    let h = harness().await;
    let path = h.dir.path().join("layout.toml");
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputPreviewOrganization(h.request.clone())
        )
        .await,
        ModuleReply::OrganizationPreview(_)
    ));
    assert_eq!(before, std::fs::read(&path).unwrap());
    assert!(!h.dir.path().join("layout.v1.backup.toml").exists());
    let reply = command(
        &h.tx,
        ModuleCommand::InputApplyOrganization(h.request.clone()),
    )
    .await;
    let ModuleReply::OrganizationApplied(applied) = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(applied.layout_rev, 2);
    assert_eq!(applied.durability, DurabilityState::Confirmed);
    assert_eq!(
        std::fs::read(h.dir.path().join("layout.v1.backup.toml")).unwrap(),
        before
    );
    let after = std::fs::read(&path).unwrap();
    assert!(
        matches!(command(&h.tx,ModuleCommand::InputApplyOrganization(h.request)).await,ModuleReply::OrganizationFailure(OrganizationFailure {code,..}) if code=="stale_rev")
    );
    assert_eq!(after, std::fs::read(path).unwrap());
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}
#[tokio::test]
async fn changed_inventory_or_backup_conflict_never_writes_and_stays_suspended() {
    let h = harness().await;
    let path = h.dir.path().join("layout.toml");
    let before = std::fs::read(&path).unwrap();
    std::fs::write(
        h.dir.path().join("layout.v1.backup.toml"),
        b"conflicting backup",
    )
    .unwrap();
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputApplyOrganization(h.request.clone())
        )
        .await,
        ModuleReply::OrganizationFailure(_)
    ));
    assert_eq!(before, std::fs::read(&path).unwrap());
    assert!(!h.cap.started());
    h.cap.set_monitor_observation(None);
    assert!(matches!(
        command(&h.tx, ModuleCommand::InputApplyOrganization(h.request)).await,
        ModuleReply::OrganizationFailure(_)
    ));
    assert_eq!(before, std::fs::read(path).unwrap());
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}
#[tokio::test]
async fn suspension_does_not_try_to_take_layout_lock() {
    let h = harness().await;
    let storage = clipsync_input::layout_storage::LayoutStorage::new(h.dir.path());
    let _guard = storage.try_lock().unwrap();
    assert!(matches!(
        command(&h.tx, ModuleCommand::InputSuspendOrganization).await,
        ModuleReply::Done
    ));
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}
#[tokio::test]
async fn received_routes_do_not_require_inverse_inventory_or_a_saved_layout() {
    let hub = MemHub::new();
    let a = hub.add_node("origin", INPUT);
    let b = hub.add_node("receiver", INPUT);
    let dir = tempfile::tempdir().unwrap();
    let (cap, handle) = FakeCapture::new(vec![fake_zone()]);
    handle.set_monitor_observation(Some(observed()));
    let (emu, eh) = FakeEmulate::new();
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(cap),
        Box::new(emu),
        dir.path(),
    )
    .unwrap();
    tokio::spawn(module.run(b.host, b.rx));
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    let ModuleReply::InputState(state) = command(&b.tx, ModuleCommand::InputState).await else {
        panic!()
    };
    let inv = &state.inventories[0].inventory;
    let entry = EnterRoutes {
        session: 77,
        inventories: InventoryRevisions {
            source: 999,
            target: inv.rev,
        },
        monitor: inv.monitors[0].uid,
        edge: Edge::Left,
        pos: 0.5,
        routes: SessionRoutes {
            first: ExitRoute {
                id: RouteId(0),
                monitor: inv.monitors[0].uid,
                edge: Edge::Left,
                from: 0.1,
                to: 0.9,
            },
            second: None,
        },
    };
    a.host
        .send_reliable(b.id, ModuleMsg::Input(InputMsg::EnterRoutes(entry)))
        .await
        .unwrap();
    command(&b.tx, ModuleCommand::Status).await;
    assert!(eh.started());
    assert!(!dir.path().join("layout.toml").exists());
    b.tx.send(ModuleInput::Shutdown).await.unwrap();
}

async fn next_input(rx: &mut mpsc::Receiver<ModuleInput>) -> InputMsg {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(ModuleInput::Reliable {
                msg: ModuleMsg::Input(m),
                ..
            }) = rx.recv().await
            {
                return m;
            }
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn runtime_two_exits_return_to_b_and_ignore_wrong_or_duplicate_returns() {
    let mut h = harness_with(2).await;
    let reply = command(
        &h.tx,
        ModuleCommand::InputApplyOrganization(h.request.clone()),
    )
    .await;
    assert!(
        matches!(
            reply,
            ModuleReply::OrganizationApplied(OrganizationApplied {
                activation: ActivationState::Confirmed,
                ..
            })
        ),
        "{reply:?}"
    );
    for _ in 0..100 {
        if h.cap.barriers().len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(h.cap.barriers().len(), 2);
    h.cap.activate(0, 0.5);
    let enter = loop {
        if let InputMsg::EnterRoutes(e) = next_input(&mut h._rx).await {
            break e;
        }
    };
    assert!(enter.routes.second.is_some());
    let returned = ReturnedRoute {
        session: enter.session,
        inventories: enter.inventories,
        route: RouteId(1),
        pos: 0.5,
    };
    for wrong in [
        ReturnedRoute {
            session: enter.session + 1,
            ..returned.clone()
        },
        ReturnedRoute {
            inventories: InventoryRevisions {
                source: 9999,
                ..enter.inventories
            },
            ..returned.clone()
        },
        ReturnedRoute {
            pos: 0.0,
            ..returned.clone()
        },
    ] {
        h.remote
            .send_reliable(h.id, ModuleMsg::Input(InputMsg::ReturnedRoute(wrong)))
            .await
            .unwrap();
    }
    command(&h.tx, ModuleCommand::Status).await;
    assert!(h.cap.released_at().is_empty());
    h.remote
        .send_reliable(
            h.id,
            ModuleMsg::Input(InputMsg::ReturnedRoute(returned.clone())),
        )
        .await
        .unwrap();
    for _ in 0..100 {
        if !h.cap.released_at().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(h.cap.released_at().len(), 1);
    assert_eq!(h.cap.released_at()[0].0, 1);
    h.remote
        .send_reliable(h.id, ModuleMsg::Input(InputMsg::ReturnedRoute(returned)))
        .await
        .unwrap();
    command(&h.tx, ModuleCommand::Status).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(h.cap.released_at().len(), 1);
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}
#[tokio::test]
async fn copy_offer_is_bound_to_seen_connection_revision_and_kept_on_missing_inventory() {
    let h = harness().await;
    let offer = LayoutOfferV4 {
        rev: 12,
        devices: vec![
            OfferDevice {
                peer: h.id,
                x: 0,
                y: 0,
            },
            OfferDevice {
                peer: h
                    .request
                    .positions
                    .iter()
                    .find(|p| p.key.peer != h.id)
                    .unwrap()
                    .key
                    .peer,
                x: 1000,
                y: 0,
            },
        ],
        positions: h.request.positions.clone(),
        links: vec![],
        legacy_links: vec![],
    };
    h.remote
        .send_reliable(h.id, ModuleMsg::Input(InputMsg::LayoutOfferV4(offer)))
        .await
        .unwrap();
    let ModuleReply::InputState(state) = command(&h.tx, ModuleCommand::InputState).await else {
        panic!()
    };
    let pending = state.pending_monitor_offer.unwrap();
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputAnswerOfferV4 {
                from: pending.from,
                connection: pending.connection,
                rev: 11,
                base_rev: 1,
                accept: false
            }
        )
        .await,
        ModuleReply::OrganizationFailure(_)
    ));
    h.cap.set_monitor_observation(None);
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputAnswerOfferV4 {
                from: pending.from,
                connection: pending.connection,
                rev: 12,
                base_rev: 1,
                accept: true
            }
        )
        .await,
        ModuleReply::OrganizationFailure(_)
    ));
    let ModuleReply::InputState(state) = command(&h.tx, ModuleCommand::InputState).await else {
        panic!()
    };
    assert!(state.pending_monitor_offer.is_some());
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputAnswerOfferV4 {
                from: pending.from,
                connection: pending.connection,
                rev: 12,
                base_rev: 1,
                accept: false
            }
        )
        .await,
        ModuleReply::Done
    ));
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}

#[tokio::test]
async fn s10_fix1_suspension_attempts_leave_for_the_active_session_after_local_release() {
    let mut h = harness_with(2).await;
    assert!(matches!(
        command(
            &h.tx,
            ModuleCommand::InputApplyOrganization(h.request.clone())
        )
        .await,
        ModuleReply::OrganizationApplied(_)
    ));
    h.cap.activate(0, 0.5);
    let entry = loop {
        if let InputMsg::EnterRoutes(e) = next_input(&mut h._rx).await {
            break e;
        }
    };
    let released = h.cap.released();
    assert!(matches!(
        command(&h.tx, ModuleCommand::InputSuspendOrganization).await,
        ModuleReply::Done
    ));
    assert!(h.cap.released() > released);
    let leave = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if let InputMsg::Leave { session } = next_input(&mut h._rx).await {
                break session;
            }
        }
    })
    .await;
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
    assert_eq!(leave.unwrap(), entry.session);
}
