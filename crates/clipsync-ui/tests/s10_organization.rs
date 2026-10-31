//! Regressões da fronteira real Ui::call e do documento v2.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn comandos_novos_validam_json_antes_do_agente() {
    let dir = tempfile::tempdir().unwrap();
    let ui = start(dir.path(), "vazio").await;
    for cmd in [
        "layout_preview_organization",
        "layout_apply_organization",
        "layout_restore_v1",
        "layout_copy_monitors",
        "layout_monitor_offer_answer",
    ] {
        let e = err(&ui, json!({"cmd":cmd,"args":{"campo_inexistente":true}})).await;
        assert_eq!(e.code, "bad_request", "{cmd}");
    }
    let e = err(&ui, json!({"cmd":"layout_prepare_monitors"})).await;
    assert_eq!(e.code, "not_initialized");
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn leitura_v2_preserva_posicao_ausente_sem_fabricar_inventario() {
    use clipsync_input::{
        layout_document::{LayoutDocument, Organization},
        layout_storage::LayoutStorage,
    };
    use clipsync_proto::{MonitorKey, MonitorPosition, MonitorUid};
    let dir = tempfile::tempdir().unwrap();
    let (profile, id) = init_profile(dir.path(), "a", INPUT_ONLY);
    let mut doc = LayoutDocument::default();
    doc.legacy.rev = 9;
    doc.legacy.devices.push(clipsync_input::LayoutDevice {
        peer: id,
        name: "a".into(),
        x: 0,
        y: 0,
        hotkey: None,
    });
    doc.organization = Some(Organization {
        positions: vec![MonitorPosition {
            key: MonitorKey {
                peer: id,
                uid: MonitorUid([1; 16]),
            },
            x: -20.,
            y: 30.,
        }],
        ..Default::default()
    });
    LayoutStorage::new(&profile.dir)
        .try_lock()
        .unwrap()
        .write(&doc)
        .unwrap();
    let before = std::fs::read(profile.dir.join("layout.toml")).unwrap();
    let ui = start(dir.path(), "a").await;
    let view = ok(&ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(view["rev"], 9);
    assert_eq!(
        view["organization"]["positions"][0]["key"]["uid"],
        "01010101010101010101010101010101"
    );
    assert_eq!(view["inventories"], json!([]));
    assert_eq!(view["v1_backup_available"], false);
    assert_eq!(
        std::fs::read(profile.dir.join("layout.toml")).unwrap(),
        before
    );
    ui.shutdown().await;
}

use clipsync_core::control;
use clipsync_proto::*;
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct Replies {
    preview: Option<ModuleReply>,
    apply: Option<ModuleReply>,
    state: Option<InputState>,
}
struct AgentBoundary {
    status: Arc<Mutex<AgentStatus>>,
    injected: Arc<Mutex<Replies>>,
    tx: tokio::sync::mpsc::Sender<ModuleInput>,
    calls: Arc<Mutex<Vec<ModuleCommand>>>,
}
impl control::ControlHandler for AgentBoundary {
    fn handle<'a, 'b>(
        &'a self,
        req: ControlRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ControlResponse> + Send + 'b>>
    where
        'a: 'b,
        Self: 'b,
    {
        Box::pin(async move {
            match req {
                ControlRequest::Status => {
                    ControlResponse::Status(self.status.lock().unwrap().clone())
                }
                ControlRequest::Module {
                    kind: ModuleKind::Input,
                    cmd,
                } => {
                    self.calls.lock().unwrap().push(cmd.clone());
                    let replacement = {
                        let mut replies = self.injected.lock().unwrap();
                        match &cmd {
                            ModuleCommand::InputPreviewOrganization(_) => replies.preview.take(),
                            ModuleCommand::InputApplyOrganization(_) => replies.apply.take(),
                            ModuleCommand::InputState => {
                                replies.state.clone().map(ModuleReply::InputState)
                            }
                            _ => None,
                        }
                    };
                    if let Some(reply) = replacement {
                        return ControlResponse::Module(reply);
                    }

                    let (reply, rx) = tokio::sync::oneshot::channel();
                    self.tx
                        .send(ModuleInput::Command { cmd, reply })
                        .await
                        .unwrap();
                    ControlResponse::Module(rx.await.unwrap())
                }
                _ => ControlResponse::Error {
                    message: "fora do harness".into(),
                },
            }
        })
    }
}
struct Real {
    dir: tempfile::TempDir,
    ui: clipsync_ui::Ui,
    request: serde_json::Value,
    tx: tokio::sync::mpsc::Sender<ModuleInput>,
    calls: Arc<Mutex<Vec<ModuleCommand>>>,
    stop: tokio::sync::oneshot::Sender<()>,
    server: tokio::task::JoinHandle<Result<(), clipsync_core::CoreError>>,
    _hub: MemHub,
    _remote: tokio::sync::mpsc::Receiver<ModuleInput>,
    a: PeerId,
    b: PeerId,
    remote: Arc<dyn ModuleHost>,
    local: Arc<dyn ModuleHost>,
    remote_tx: tokio::sync::mpsc::Sender<ModuleInput>,
    status: Arc<Mutex<AgentStatus>>,
    injected: Arc<Mutex<Replies>>,
}
impl Real {
    async fn new() -> Self {
        use clipsync_input::{
            FakeCapture, FakeEmulate, InputConfig, Zone, build_with_layout,
            inventory::ObservedMonitors,
            monitor_identities::{Evidence, Observation},
        };
        let hub = MemHub::new();
        let a = hub.add_node("A", INPUT_ONLY);
        let b = hub.add_node("notebook", INPUT_ONLY);
        let dir = tempfile::tempdir().unwrap();
        let (profile, _) = init_profile(dir.path(), "a", INPUT_ONLY);
        let mut peers = profile.load_peers().unwrap();
        for (id, name) in [(a.id, "desktop"), (b.id, "notebook")] {
            peers.upsert(clipsync_core::PeerEntry {
                id,
                name: name.into(),
                addr: None,
                paired_at: "2026-10-30".into(),
                perms: NONE,
            });
        }
        profile.save_peers(&peers).unwrap();
        let rect = NativeRect {
            x: 0,
            y: 0,
            width: 1000,
            height: 1000,
        };
        let mut observation = ObservedMonitors {
            space: CoordinateSpace::Physical,
            source: GeometrySource::Observed,
            capabilities: MonitorCapabilities {
                internal_edges: true,
                positioned_return: true,
                route_receiver: true,
            },
            observations: vec![Observation {
                evidence: Evidence::WindowsPath("harness-only".into()),
                label: "A".into(),
                rect,
                scale: 1.,
                legacy_aliases: vec!["A".into()],
            }],
        };
        let mut second = observation.observations[0].clone();
        second.evidence = Evidence::WindowsPath("harness-b".into());
        second.label = "B".into();
        second.rect.x = 1000;
        second.legacy_aliases = vec!["B".into()];
        observation.observations.push(second);
        let (cap, handle) = FakeCapture::new(vec![
            Zone {
                index: 0,
                name: Some("A".into()),
                x: 0,
                y: 0,
                width: 1000,
                height: 1000,
                scale: 1.,
            },
            Zone {
                index: 1,
                name: Some("B".into()),
                x: 1000,
                y: 0,
                width: 1000,
                height: 1000,
                scale: 1.,
            },
        ]);
        handle.set_monitor_observation(Some(observation));
        let (emu, _) = FakeEmulate::new();
        let module = build_with_layout(
            InputConfig::default(),
            Box::new(cap),
            Box::new(emu),
            &profile.dir,
        )
        .unwrap();
        let document = clipsync_input::Layout {
            rev: 1,
            options: Default::default(),
            links: vec![],
            devices: vec![
                clipsync_input::LayoutDevice {
                    peer: a.id,
                    name: "A".into(),
                    x: 0,
                    y: 0,
                    hotkey: None,
                },
                clipsync_input::LayoutDevice {
                    peer: b.id,
                    name: "N".into(),
                    x: 1000,
                    y: 0,
                    hotkey: None,
                },
            ],
        };
        document.write(&profile.dir.join("layout.toml")).unwrap();
        tokio::spawn(module.run(a.host.clone(), a.rx));
        let perms = PeerPerms {
            input_in: true,
            ..NONE
        };
        hub.link(a.id, b.id, perms, perms);
        b.host
            .send_reliable(
                a.id,
                ModuleMsg::Input(InputMsg::InventoryV4(MonitorInventory {
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
                        label: "notebook".into(),
                        rect,
                        scale: 1.,
                    }],
                })),
            )
            .await
            .unwrap();
        let state = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let (reply, rx) = tokio::sync::oneshot::channel();
                a.tx.send(ModuleInput::Command {
                    cmd: ModuleCommand::InputState,
                    reply,
                })
                .await
                .unwrap();
                if let ModuleReply::InputState(s) = rx.await.unwrap()
                    && s.inventories.len() == 2
                {
                    break s;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let request = json!({"base_rev":1,"inventories":state.inventories.iter().map(|v|json!({"peer":v.stamp.peer.to_hex(),"connection":v.stamp.connection,"rev":v.stamp.rev})).collect::<Vec<_>>(),"positions":state.inventories.iter().flat_map(|v|v.inventory.monitors.iter().map(|m|json!({"key":{"peer":v.stamp.peer.to_hex(),"uid":m.uid},"x":if v.stamp.peer==a.id{m.rect.x*2}else{1000},"y":0}))).collect::<Vec<_>>(),"replace_link_ids":[]});
        let calls = Arc::new(Mutex::new(vec![]));
        let status = AgentStatus {
            device_name: "A".into(),
            id: a.id,
            listen: profile.load_config().unwrap().listen,
            paused: false,
            peers: vec![
                PeerStatus {
                    id: a.id,
                    name: "desktop".into(),
                    addr: None,
                    state: PeerState::Connected,
                    perms,
                    remote_modules: INPUT_ONLY,
                    remote_version: 4,
                },
                PeerStatus {
                    id: b.id,
                    name: "notebook".into(),
                    addr: None,
                    state: PeerState::Connected,
                    perms,
                    remote_modules: INPUT_ONLY,
                    remote_version: 4,
                },
            ],
            modules: vec![],
            recent_blocks: vec![],
            recent_removals: vec![],
        };
        let status = Arc::new(Mutex::new(status));
        let injected = Arc::new(Mutex::new(Replies::default()));
        let boundary = Arc::new(AgentBoundary {
            status: status.clone(),
            injected: injected.clone(),
            tx: a.tx.clone(),
            calls: calls.clone(),
        });
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let socket = profile.socket_path();
        let server = tokio::spawn(async move {
            control::serve(&socket, boundary, async {
                let _ = stopped.await;
            })
            .await
        });
        wait_until("controle T4", || async {
            control::request(&profile, ControlRequest::Status)
                .await
                .is_ok()
        })
        .await;
        let ui = start(dir.path(), "a").await;
        Self {
            dir,
            ui,
            request,
            tx: a.tx,
            calls,
            stop,
            server,
            _hub: hub,
            _remote: b.rx,
            a: a.id,
            b: b.id,
            remote: b.host,
            local: a.host,
            remote_tx: b.tx,
            status,
            injected,
        }
    }
    fn file(&self) -> std::path::PathBuf {
        self.dir.path().join("a/layout.toml")
    }
    async fn end(self) {
        self.ui.shutdown().await;
        self.stop.send(()).unwrap();
        self.server.await.unwrap().unwrap();
        self.tx.send(ModuleInput::Shutdown).await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn previa_aplica_uma_revisao_e_legados_preservam_v2_sob_lock() {
    let h = Real::new().await;
    let before = std::fs::read(h.file()).unwrap();
    let preview = ok(
        &h.ui,
        json!({"cmd":"layout_preview_organization","args":h.request}),
    )
    .await;
    assert_eq!(preview["effective_links"].as_array().unwrap().len(), 2);
    assert!([json!("left"), json!("right")].contains(&preview["effective_links"][0]["a"]["edge"]));
    assert_eq!(std::fs::read(h.file()).unwrap(), before);
    assert!(
        !h.calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| matches!(c, ModuleCommand::InputApplyOrganization(_)))
    );
    let result = ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    assert_eq!(result["layout_rev"], 2);
    let after = std::fs::read(h.file()).unwrap();
    let e = err(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    assert_eq!(e.code, "stale_rev");
    assert_eq!(e.details.unwrap()["current_rev"], 2);
    assert_eq!(std::fs::read(h.file()).unwrap(), after);
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(view["v1_backup_available"], true);
    let positions = view["organization"]["positions"].clone();
    ok(&h.ui,json!({"cmd":"layout_set_device","args":{"peer_id":h.a.to_hex(),"x":320,"y":160,"hotkey":1,"base_rev":2}})).await;
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(view["organization"]["positions"], positions);
    ok(
        &h.ui,
        json!({"cmd":"layout_set_options","args":{"dead_corner_px":8,"base_rev":3}}),
    )
    .await;
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(view["organization"]["positions"], positions);
    let lock = clipsync_input::layout_storage::LayoutStorage::new(&h.dir.path().join("a"))
        .try_lock()
        .unwrap();
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_set_options","args":{"dead_corner_px":8,"base_rev":4}})
        )
        .await
        .code,
        "busy"
    );
    drop(lock);
    let link = &view["links"][0];
    assert_eq!(err(&h.ui,json!({"cmd":"layout_set_link","args":{"id":null,"a":link["a"],"b":link["b"],"base_rev":4}})).await.code,"segment_overlap");
    ok(&h.ui,json!({"cmd":"layout_set_link","args":{"id":link["id"],"a":link["a"],"b":link["b"],"base_rev":4}})).await;
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(view["organization"]["links"].as_array().unwrap().len(), 1);
    assert_eq!(view["links"].as_array().unwrap().len(), 2);
    let mut next = h.request.clone();
    next["base_rev"] = json!(5);
    next["replace_link_ids"] = json!(
        view["organization"]["links"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["id"].clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":next})
        )
        .await
        .code,
        "advanced_link_conflict"
    );
    ok(
        &h.ui,
        json!({"cmd":"layout_remove_link","args":{"id":link["id"],"base_rev":5}}),
    )
    .await;
    assert_eq!(
        ok(&h.ui, json!({"cmd":"layout_get"})).await["organization"]["positions"],
        positions
    );
    let restored = ok(
        &h.ui,
        json!({"cmd":"layout_restore_v1","args":{"base_rev":6}}),
    )
    .await;
    assert_eq!(restored["layout_rev"], 7);
    assert_eq!(
        ok(&h.ui, json!({"cmd":"layout_get"})).await["organization"],
        serde_json::Value::Null
    );
    h.end().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn revisao_inventario_backup_e_json_invalidos_nao_gravam() {
    let h = Real::new().await;
    let before = std::fs::read(h.file()).unwrap();
    let mut invalid = h.request.clone();
    invalid["positions"][0]["key"]["peer"] = json!("sem identidade");
    let count = h.calls.lock().unwrap().len();
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_apply_organization","args":invalid})
        )
        .await
        .code,
        "bad_request"
    );
    assert!(
        !h.calls.lock().unwrap()[count..]
            .iter()
            .any(|c| matches!(c, ModuleCommand::InputApplyOrganization(_)))
    );
    let mut stale = h.request.clone();
    stale["inventories"][0]["rev"] = json!(999);
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":stale})
        )
        .await
        .code,
        "stale_inventory"
    );
    std::fs::write(
        h.dir.path().join("a/layout.v1.backup.toml"),
        "backup conflitante",
    )
    .unwrap();
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_apply_organization","args":h.request})
        )
        .await
        .code,
        "backup_conflict"
    );
    assert_eq!(std::fs::read(h.file()).unwrap(), before);
    h.end().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fronteira_normaliza_causas_e_publicacao_nao_confirmada_sem_repetir() {
    let h = Real::new().await;
    for (internal, expected) in [
        ("identity_ambiguous", "identity_ambiguous"),
        ("inventory_changed", "stale_inventory"),
        ("inventory_missing", "inventory_required"),
        ("advanced_link_conflict", "advanced_link_conflict"),
        ("backup_conflict", "backup_conflict"),
    ] {
        h.injected.lock().unwrap().preview =
            Some(ModuleReply::OrganizationFailure(OrganizationFailure {
                code: internal.into(),
                current_rev: Some(5),
            }));
        let e = err(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":h.request}),
        )
        .await;
        assert_eq!(e.code, expected);
        assert_eq!(e.details.unwrap()["current_rev"], 5);
    }
    h.injected.lock().unwrap().preview = None;
    h.status
        .lock()
        .unwrap()
        .peers
        .iter_mut()
        .find(|p| p.id == h.b)
        .unwrap()
        .remote_version = 3;
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":h.request})
        )
        .await
        .code,
        "protocol_required"
    );
    h.status
        .lock()
        .unwrap()
        .peers
        .iter_mut()
        .find(|p| p.id == h.b)
        .unwrap()
        .remote_version = 4;
    let (reply, rx) = tokio::sync::oneshot::channel();
    h.tx.send(ModuleInput::Command {
        cmd: ModuleCommand::InputState,
        reply,
    })
    .await
    .unwrap();
    let ModuleReply::InputState(mut state) = rx.await.unwrap() else {
        panic!()
    };
    state.inventories[0].inventory.source = GeometrySource::Estimated;
    h.injected.lock().unwrap().state = Some(state.clone());
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":h.request})
        )
        .await
        .code,
        "inventory_required"
    );
    state.inventories.clear();
    h.injected.lock().unwrap().state = Some(state);
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_apply_organization","args":h.request})
        )
        .await
        .code,
        "inventory_required"
    );
    h.injected.lock().unwrap().state = None;
    h.injected.lock().unwrap().apply =
        Some(ModuleReply::OrganizationApplied(OrganizationApplied {
            layout_rev: 12,
            activation: ActivationState::Unconfirmed,
            durability: DurabilityState::Unconfirmed,
        }));
    let mut events = h.ui.subscribe();
    let r = ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    assert_eq!(
        r,
        json!({"layout_rev":12,"activation":"unconfirmed","durability":"unconfirmed"})
    );
    assert_eq!(
        h.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, ModuleCommand::InputApplyOrganization(_)))
            .count(),
        1
    );
    let event = events.recv().await.unwrap();
    assert!(matches!(event,clipsync_ui::UiEvent::LayoutChanged(v) if v.rev==12));
    h.end().await;
}
#[test]
fn json_de_agente_antigo_nao_inventa_inventario_nem_oferta() {
    let state:InputState=serde_json::from_value(json!({"local":[],"peers":[],"screen_locked":false,"active_peer":null,"pending_offer":null})).unwrap();
    assert!(state.inventories.is_empty());
    assert!(state.inventory_problem.is_none());
    assert!(state.pending_monitor_offer.is_none());
}
#[tokio::test(flavor = "multi_thread")]
async fn preparar_copiar_e_recusar_oferta_usam_comandos_v4_reais() {
    let h = Real::new().await;
    ok(&h.ui, json!({"cmd":"layout_prepare_monitors"})).await;
    ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    let copied = ok(
        &h.ui,
        json!({"cmd":"layout_copy_monitors","args":{"to_peer_ids":[h.b.to_hex()]}}),
    )
    .await;
    assert_eq!(copied["rev"], 2);
    assert_eq!(copied["results"][0]["peer_id"], h.b.to_hex());
    h._hub.set_remote_version(h.b, 3);
    let old = ok(
        &h.ui,
        json!({"cmd":"layout_copy_monitors","args":{"to_peer_ids":[h.b.to_hex()]}}),
    )
    .await;
    assert_eq!(old["results"][0]["status"], "unsupported");
    h._hub.set_remote_version(h.b, 4);
    let doc = clipsync_input::layout_document::LayoutDocument::read(&h.file())
        .unwrap()
        .unwrap();
    let org = doc.organization.unwrap();
    h.remote
        .send_reliable(
            h.a,
            ModuleMsg::Input(InputMsg::LayoutOfferV4(LayoutOfferV4 {
                rev: 19,
                devices: vec![
                    OfferDevice {
                        peer: h.a,
                        x: 0,
                        y: 0,
                    },
                    OfferDevice {
                        peer: h.b,
                        x: 1000,
                        y: 0,
                    },
                ],
                positions: org.positions,
                links: org.links,
                legacy_links: vec![],
            })),
        )
        .await
        .unwrap();
    wait_until("oferta v4 na UI", || async {
        ok(&h.ui, json!({"cmd":"layout_get"})).await["pending_monitor_offer"].is_object()
    })
    .await;
    let before = std::fs::read(h.file()).unwrap();
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    let offer = &view["pending_monitor_offer"];
    assert_eq!(offer["positions"].as_array().unwrap().len(), 3);
    let r=ok(&h.ui,json!({"cmd":"layout_monitor_offer_answer","args":{"from_peer_id":offer["from_peer_id"],"connection":offer["connection"],"rev":19,"base_rev":2,"accept":false}})).await;
    assert_eq!(r["layout_rev"], 2);
    assert_eq!(r["activation"], "unconfirmed");
    assert_eq!(std::fs::read(h.file()).unwrap(), before);
    assert!(!h.calls.lock().unwrap().iter().any(|c| matches!(
        c,
        ModuleCommand::InputOffer { .. } | ModuleCommand::InputOfferClear
    )));
    h.end().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn writer_legado_e_aplicacao_real_na_mesma_base_tem_um_vencedor() {
    let h = Real::new().await;
    let (a, b) = tokio::join!(
        h.ui.call(json!({"cmd":"layout_apply_organization","args":h.request})),
        h.ui.call(json!({"cmd":"layout_set_options","args":{"dead_corner_px":8,"base_rev":1}}))
    );
    assert_eq!(
        usize::from(a.is_ok()) + usize::from(b.is_ok()),
        1,
        "{a:?} {b:?}"
    );
    let e = a.err().or(b.err()).unwrap();
    assert!(["busy", "stale_rev"].contains(&e.code.as_str()), "{e:?}");
    let doc = clipsync_input::layout_document::LayoutDocument::read(&h.file())
        .unwrap()
        .unwrap();
    assert_eq!(doc.legacy.rev, 2);
    if let Some(org) = doc.organization {
        assert_eq!(org.positions.len(), 3);
        assert_eq!(org.links.len(), 2);
    }
    h.end().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn oferta_legada_nao_apaga_v2_e_mutacao_sem_agente_falha_fechada() {
    let h = Real::new().await;
    ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    let copied = ok(
        &h.ui,
        json!({"cmd":"layout_copy_to_peers","args":{"peer_ids":[h.b.to_hex()]}}),
    )
    .await;
    assert_eq!(copied["rev"], 2);
    h.remote
        .send_reliable(
            h.a,
            ModuleMsg::Input(InputMsg::LayoutOffer(LayoutOffer {
                rev: 20,
                devices: vec![
                    OfferDevice {
                        peer: h.a,
                        x: 0,
                        y: 0,
                    },
                    OfferDevice {
                        peer: h.b,
                        x: 0,
                        y: 0,
                    },
                ],
                links: vec![],
            })),
        )
        .await
        .unwrap();
    wait_until("oferta antiga", || async {
        ok(&h.ui, json!({"cmd":"layout_get"})).await["pending_offer"].is_object()
    })
    .await;
    let before = std::fs::read(h.file()).unwrap();
    assert_eq!(err(&h.ui,json!({"cmd":"layout_offer_answer","args":{"from_peer_id":h.b.to_hex(),"rev":20,"accept":true}})).await.code,"incompatible_offer");
    assert_eq!(std::fs::read(h.file()).unwrap(), before);
    ok(&h.ui,json!({"cmd":"layout_offer_answer","args":{"from_peer_id":h.b.to_hex(),"rev":20,"accept":false}})).await;
    assert_eq!(std::fs::read(h.file()).unwrap(), before);
    let layout_path = h.file();
    h.stop.send(()).unwrap();
    h.server.await.unwrap().unwrap();
    assert_eq!(ok(&h.ui, json!({"cmd":"layout_get"})).await["rev"], 2);
    assert_eq!(
        err(
            &h.ui,
            json!({"cmd":"layout_set_options","args":{"dead_corner_px":8,"base_rev":2}})
        )
        .await
        .code,
        "agent_not_running"
    );
    assert_eq!(std::fs::read(layout_path).unwrap(), before);
    h.ui.shutdown().await;
    h.tx.send(ModuleInput::Shutdown).await.unwrap();
}

#[test]
fn layout_json_antigo_recebe_defaults_conservadores() {
    let mut original: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/layout_get.json")).unwrap();
    for field in [
        "organization",
        "inventories",
        "inventory_problem",
        "pending_monitor_offer",
        "v1_backup_available",
    ] {
        original.as_object_mut().unwrap().remove(field);
    }
    let layout: clipsync_ui::layout::Layout = serde_json::from_value(original).unwrap();
    assert!(layout.organization.is_none());
    assert!(layout.inventories.is_empty());
    assert!(layout.inventory_problem.is_none());
    assert!(layout.pending_monitor_offer.is_none());
    assert!(!layout.v1_backup_available);
}

#[tokio::test(flavor = "multi_thread")]
async fn borda_interna_so_e_liberada_com_capacidade_windows_observada() {
    let h = Real::new().await;
    ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":h.request}),
    )
    .await;
    let view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert!(
        !view["locked_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["peer_id"] == h.a.to_hex())
    );
    let (reply, rx) = tokio::sync::oneshot::channel();
    h.tx.send(ModuleInput::Command {
        cmd: ModuleCommand::InputState,
        reply,
    })
    .await
    .unwrap();
    let ModuleReply::InputState(mut state) = rx.await.unwrap() else {
        panic!()
    };
    let local = state
        .inventories
        .iter_mut()
        .find(|v| v.stamp.peer == h.a)
        .unwrap();
    local.inventory.space = CoordinateSpace::Logical;
    local.inventory.capabilities.internal_edges = false;
    h.injected.lock().unwrap().state = Some(state);
    let linux = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    assert!(
        linux["locked_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["peer_id"] == h.a.to_hex())
    );
    h.end().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn aceite_positivo_pela_ui_persiste_v2_no_receptor_sem_conceder_permissoes() {
    use clipsync_input::{
        FakeCapture, FakeEmulate, InputConfig, Layout, LayoutDevice, LayoutOptions, Zone,
        build_with_layout,
        inventory::ObservedMonitors,
        layout_document::LayoutDocument,
        monitor_identities::{Evidence, Observation},
    };
    let mut h = Real::new().await;
    let (profile, _) = init_profile(h.dir.path(), "receptor", INPUT_ONLY);
    let mut peers = profile.load_peers().unwrap();
    for (id, name) in [(h.a, "desktop"), (h.b, "notebook")] {
        peers.upsert(clipsync_core::PeerEntry {
            id,
            name: name.into(),
            addr: None,
            paired_at: "2026-10-30".into(),
            perms: NONE,
        });
    }
    profile.save_peers(&peers).unwrap();
    let local = Layout {
        rev: 7,
        options: LayoutOptions { dead_corner_px: 24 },
        links: vec![],
        devices: vec![
            LayoutDevice {
                peer: h.a,
                name: "desktop local".into(),
                x: -100,
                y: 50,
                hotkey: Some(1),
            },
            LayoutDevice {
                peer: h.b,
                name: "notebook local".into(),
                x: 500,
                y: 50,
                hotkey: Some(2),
            },
        ],
    };
    let file = profile.dir.join("layout.toml");
    local.write(&file).unwrap();
    let before = std::fs::read(&file).unwrap();
    let (cap, handle) = FakeCapture::new(vec![Zone {
        index: 0,
        name: Some("N".into()),
        x: 0,
        y: 0,
        width: 1000,
        height: 1000,
        scale: 1.,
    }]);
    handle.set_monitor_observation(Some(ObservedMonitors {
        space: CoordinateSpace::Physical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: true,
            positioned_return: true,
            route_receiver: true,
        },
        observations: vec![Observation {
            evidence: Evidence::WindowsPath("receptor-harness".into()),
            label: "N".into(),
            rect: NativeRect {
                x: 0,
                y: 0,
                width: 1000,
                height: 1000,
            },
            scale: 1.,
            legacy_aliases: vec!["N".into()],
        }],
    }));
    let (emu, _) = FakeEmulate::new();
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(cap),
        Box::new(emu),
        &profile.dir,
    )
    .unwrap();
    let (_, empty_rx) = tokio::sync::mpsc::channel(1);
    let receiver_rx = std::mem::replace(&mut h._remote, empty_rx);
    let receiver_tx = h.remote_tx.clone();
    let module_task = tokio::spawn(module.run(h.remote.clone(), receiver_rx));
    let mut status = h.status.lock().unwrap().clone();
    status.id = h.b;
    status.device_name = "notebook".into();
    status.listen = profile.load_config().unwrap().listen;
    let injected = Arc::new(Mutex::new(Replies::default()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let boundary = Arc::new(AgentBoundary {
        status: Arc::new(Mutex::new(status)),
        injected: injected.clone(),
        tx: receiver_tx.clone(),
        calls: calls.clone(),
    });
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let socket = profile.socket_path();
    let server = tokio::spawn(async move {
        control::serve(&socket, boundary, async {
            let _ = stopped.await;
        })
        .await
    });
    wait_until("controle real do receptor T4", || async {
        control::request(&profile, ControlRequest::Status)
            .await
            .is_ok()
    })
    .await;
    let receiver_ui = start(h.dir.path(), "receptor").await;
    let source_profile = clipsync_core::Profile {
        dir: h.dir.path().join("a"),
    };
    let source_peers = std::fs::read(source_profile.dir.join("peers.toml")).unwrap();
    let receiver_peers = std::fs::read(profile.dir.join("peers.toml")).unwrap();
    let contexts = (
        h.local.peer_input_context(h.b),
        h.remote.peer_input_context(h.a),
    );
    let grants = (h.local.peers()[0].perms, h.remote.peers()[0].perms);
    // As permissões de input já existiam no MemHub; nada é concedido por esta jornada.
    assert!(grants.0.input_in && grants.1.input_in);
    assert!(!grants.0.clipboard_in && !grants.1.clipboard_in);
    ok(&h.ui, json!({"cmd":"layout_prepare_monitors"})).await;
    ok(&receiver_ui, json!({"cmd":"layout_prepare_monitors"})).await;
    let receiver_view = ok(&receiver_ui, json!({"cmd":"layout_get"})).await;
    let receiver_inventory = receiver_view["inventories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["stamp"]["peer"] == h.b.to_hex())
        .unwrap()["inventory"]
        .clone();
    wait_until("inventários reais nos dois módulos", || async {
        let source = ok(&h.ui, json!({"cmd":"layout_get"})).await;
        let receiver = ok(&receiver_ui, json!({"cmd":"layout_get"})).await;
        source["inventories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["stamp"]["peer"] == h.b.to_hex() && v["inventory"] == receiver_inventory)
            && receiver["inventories"].as_array().unwrap().len() == 2
    })
    .await;
    let source_view = ok(&h.ui, json!({"cmd":"layout_get"})).await;
    let inventories = source_view["inventories"].as_array().unwrap();
    let positions: Vec<_> = inventories.iter().flat_map(|v| {
        v["inventory"]["monitors"].as_array().unwrap().iter().map(|m| json!({
            "key":{"peer":v["stamp"]["peer"],"uid":m["uid"]},
            "x": if v["stamp"]["peer"] == h.a.to_hex() { m["rect"]["x"].as_i64().unwrap() * 2 } else { 1000 }, "y":0
        }))
    }).collect();
    let request = json!({"base_rev":1,"inventories":inventories.iter().map(|v|v["stamp"].clone()).collect::<Vec<_>>(),"positions":positions,"replace_link_ids":[]});
    let preview = ok(
        &h.ui,
        json!({"cmd":"layout_preview_organization","args":request}),
    )
    .await;
    assert_eq!(preview["effective_links"].as_array().unwrap().len(), 2);
    let applied = ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":request}),
    )
    .await;
    assert_eq!(
        applied,
        json!({"layout_rev":2,"activation":"confirmed","durability":"confirmed"})
    );
    let source_doc = LayoutDocument::read(&h.file()).unwrap().unwrap();
    let offered = source_doc.organization.as_ref().unwrap();
    let copied = ok(
        &h.ui,
        json!({"cmd":"layout_copy_monitors","args":{"to_peer_ids":[h.b.to_hex()]}}),
    )
    .await;
    assert_eq!(copied["rev"], 2);
    assert_eq!(copied["results"][0]["status"], "offered");
    wait_until("oferta enviada pelo módulo emissor", || async {
        ok(&receiver_ui, json!({"cmd":"layout_get"})).await["pending_monitor_offer"].is_object()
    })
    .await;
    assert_eq!(std::fs::read(&file).unwrap(), before);
    let view = ok(&receiver_ui, json!({"cmd":"layout_get"})).await;
    let offer = &view["pending_monitor_offer"];
    assert_eq!(offer["from_peer_id"], h.a.to_hex());
    assert_eq!(offer["rev"], 2);
    let mut offered_links = offer["links"].as_array().unwrap().clone();
    let mut preview_links = preview["effective_links"].as_array().unwrap().clone();
    offered_links.sort_by_key(|l| l["id"].as_str().unwrap().to_owned());
    preview_links.sort_by_key(|l| l["id"].as_str().unwrap().to_owned());
    assert_eq!(offered_links, preview_links);
    let accepted = ok(
        &receiver_ui,
        json!({"cmd":"layout_monitor_offer_answer","args":{
            "from_peer_id":offer["from_peer_id"],"connection":offer["connection"],
            "rev":offer["rev"],"base_rev":7,"accept":true
        }}),
    )
    .await;
    assert_eq!(
        accepted,
        json!({"layout_rev":8,"activation":"confirmed","durability":"confirmed"})
    );
    let persisted = LayoutDocument::read(&file).unwrap().unwrap();
    assert_eq!(persisted.legacy.rev, 8);
    assert_eq!(persisted.legacy.options, local.options);
    assert_eq!(persisted.legacy.links, local.links);
    for device in &local.devices {
        let saved = persisted
            .legacy
            .devices
            .iter()
            .find(|d| d.peer == device.peer)
            .unwrap();
        assert_eq!(saved.hotkey, device.hotkey);
        assert_eq!(saved.name, device.name);
    }
    let received = persisted.organization.unwrap();
    assert_eq!(received.positions, offered.positions);
    assert_eq!(received.links, offered.links);
    // O valor local 24 não recorta outra vez a oferta calculada com 8.
    for link in &received.links {
        for endpoint in [&link.a, &link.b] {
            assert!((endpoint.from - 0.008).abs() < 1e-6);
            assert!((endpoint.to - 0.992).abs() < 1e-6);
        }
    }
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("version = 2")
    );
    assert_eq!(
        std::fs::read(profile.dir.join("layout.v1.backup.toml")).unwrap(),
        before
    );
    let final_view = ok(&receiver_ui, json!({"cmd":"layout_get"})).await;
    assert_eq!(final_view["rev"], 8);
    assert!(final_view["pending_monitor_offer"].is_null());
    assert_eq!(
        std::fs::read(source_profile.dir.join("peers.toml")).unwrap(),
        source_peers
    );
    assert_eq!(
        std::fs::read(profile.dir.join("peers.toml")).unwrap(),
        receiver_peers
    );
    assert_eq!(
        (
            h.local.peer_input_context(h.b),
            h.remote.peer_input_context(h.a)
        ),
        contexts
    );
    assert_eq!(
        (h.local.peers()[0].perms, h.remote.peers()[0].perms),
        grants
    );
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|cmd| matches!(cmd, ModuleCommand::InputAnswerOfferV4 { accept: true, .. }))
            .count(),
        1
    );
    // Nenhuma resposta de inventário, prévia ou aplicação foi substituída no controle.
    for replacement in [&h.injected, &injected] {
        let reply = replacement.lock().unwrap();
        assert!(reply.state.is_none() && reply.preview.is_none() && reply.apply.is_none());
    }
    receiver_ui.shutdown().await;
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    receiver_tx.send(ModuleInput::Shutdown).await.unwrap();
    module_task.await.unwrap().unwrap();
    h.end().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn mudar_cantos_v2_preserva_links_e_so_altera_compilacao_futura() {
    use clipsync_input::layout_document::LayoutDocument;
    let h = Real::new().await;
    // O perfil temporário e o MemHub têm identidades distintas neste harness.
    // Normalizar o device do perfil pelo writer antes de medir a mudança geométrica.
    ok(
        &h.ui,
        json!({"cmd":"layout_set_options","args":{"dead_corner_px":8,"base_rev":1}}),
    )
    .await;
    let mut request = h.request.clone();
    request["base_rev"] = json!(2);
    ok(
        &h.ui,
        json!({"cmd":"layout_apply_organization","args":request}),
    )
    .await;
    let initial = LayoutDocument::read(&h.file()).unwrap().unwrap();
    assert_eq!(initial.legacy.options.dead_corner_px, 8);
    let original_links = &initial.organization.as_ref().unwrap().links;
    assert_eq!(original_links.len(), 2);
    for dead in [24, 0] {
        let current = LayoutDocument::read(&h.file()).unwrap().unwrap();
        let changed = ok(
            &h.ui,
            json!({"cmd":"layout_set_options","args":{
                "dead_corner_px":dead,"base_rev":current.legacy.rev
            }}),
        )
        .await;
        assert_eq!(changed["layout_rev"], current.legacy.rev + 1);
        assert_eq!(changed["options"]["dead_corner_px"], dead);
        let mut expected = current;
        expected.legacy.rev += 1;
        expected.legacy.options.dead_corner_px = dead;
        assert_eq!(LayoutDocument::read(&h.file()).unwrap().unwrap(), expected);
        assert_eq!(
            &expected.organization.as_ref().unwrap().links,
            original_links
        );
        let bytes = std::fs::read(h.file()).unwrap();
        let mut request = h.request.clone();
        request["base_rev"] = json!(expected.legacy.rev);
        request["replace_link_ids"] =
            json!(original_links.iter().map(|l| &l.id).collect::<Vec<_>>());
        let preview = ok(
            &h.ui,
            json!({"cmd":"layout_preview_organization","args":request}),
        )
        .await;
        let links = preview["effective_links"].as_array().unwrap();
        assert_eq!(links.len(), 2);
        for link in links {
            for side in ["a", "b"] {
                let from = link[side]["from"].as_f64().unwrap();
                let to = link[side]["to"].as_f64().unwrap();
                assert!((from - f64::from(dead) / 1000.).abs() < 1e-6);
                assert!((to - (1. - f64::from(dead) / 1000.)).abs() < 1e-6);
            }
        }
        assert_eq!(std::fs::read(h.file()).unwrap(), bytes);
    }
    h.end().await;
}
