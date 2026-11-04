//! §17 (v1.40), F1 tarefa 3: inventário, rotas, captura e entrada no log.
//!
//! RED primeiro: estes casos falham enquanto os eventos do vocabulário `inventory.*`,
//! `routes.*`, `capture.*` e `enter.*` não forem emitidos nos pontos reais do módulo de input.
//! O teste irmão lê pelo `Layer` de `tests/support/logs.rs`, com os dois nós do `MemHub`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/logs.rs"]
mod logs;

use clipsync_input::inventory::ObservedMonitors;
use clipsync_input::layout_document::{LayoutDocument, Organization};
use clipsync_input::monitor_identities::{Evidence, Observation};
use clipsync_input::{
    BarrierSpec, CaptureAdapter, CaptureEvent, FakeCapture, FakeEmulate, InputConfig, InputError,
    Layout, LayoutDevice, Zone, build_with_layout,
};
use clipsync_proto::monitors::*;
use clipsync_proto::*;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
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

// --------------------------------------------------------------- harness

struct Node {
    id: PeerId,
    host: Arc<dyn ModuleHost>,
    tx: mpsc::Sender<ModuleInput>,
    _dir: tempfile::TempDir,
}

fn node_id(name: &str) -> PeerId {
    PeerId::from_spki_der(format!("memhub:{name}").as_bytes())
}

fn device(peer: PeerId, name: &str) -> LayoutDevice {
    LayoutDevice {
        peer,
        name: name.into(),
        x: 0,
        y: 0,
        hotkey: None,
    }
}

/// The legacy document the module reads before an organization is applied.
fn write_devices(dir: &Path, peers: &[PeerId]) {
    let layout = Layout {
        rev: 1,
        options: Default::default(),
        devices: peers
            .iter()
            .enumerate()
            .map(|(i, p)| device(*p, &format!("p{i}")))
            .collect(),
        links: Vec::new(),
    };
    layout.write(&dir.join("layout.toml")).unwrap();
}

fn monitor(label: &str, path: &str, rect: (i32, i32, u32, u32)) -> Observation {
    Observation {
        evidence: Evidence::WindowsPath(path.into()),
        label: label.into(),
        rect: NativeRect {
            x: rect.0,
            y: rect.1,
            width: rect.2,
            height: rect.3,
        },
        scale: 1.0,
        legacy_aliases: vec![],
    }
}

/// Physical observed source: it can position its own return and read internal edges.
fn source_observation(label: &str, path: &str) -> ObservedMonitors {
    ObservedMonitors {
        observations: vec![monitor(label, path, (0, 0, 1000, 1000))],
        space: CoordinateSpace::Physical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: true,
            positioned_return: true,
            route_receiver: true,
        },
    }
}

/// The receiving notebook: one monitor, logical space, no return of its own.
fn receiver_observation(label: &str, path: &str) -> ObservedMonitors {
    ObservedMonitors {
        observations: vec![monitor(label, path, (0, 0, 1000, 1000))],
        space: CoordinateSpace::Logical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: false,
            positioned_return: false,
            route_receiver: true,
        },
    }
}

fn zones(observation: &ObservedMonitors) -> Vec<Zone> {
    observation
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
        .collect()
}

fn fake(observation: &ObservedMonitors) -> Box<dyn CaptureAdapter> {
    let (cap, handle) = FakeCapture::new(zones(observation));
    handle.set_monitor_observation(Some(observation.clone()));
    Box::new(cap)
}

fn spawn(hub: &MemHub, name: &str, dir: tempfile::TempDir, cap: Box<dyn CaptureAdapter>) -> Node {
    let node = hub.add_node(name, INPUT);
    let (emu, _emu) = FakeEmulate::new();
    let module = build_with_layout(InputConfig::default(), cap, Box::new(emu), dir.path()).unwrap();
    let host = node.host.clone();
    tokio::spawn(module.run(node.host, node.rx));
    Node {
        id: node.id,
        host,
        tx: node.tx,
        _dir: dir,
    }
}

async fn command(tx: &mpsc::Sender<ModuleInput>, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    tx.send(ModuleInput::Command { cmd, reply }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .expect("command timeout")
        .unwrap()
}

fn state(reply: ModuleReply) -> InputState {
    let ModuleReply::InputState(s) = reply else {
        panic!("expected InputState, got {reply:?}")
    };
    s
}

/// A Status round trip proves the module processed everything enqueued before it.
async fn settle(node: &Node) {
    let ModuleReply::Status(_) = command(&node.tx, ModuleCommand::Status).await else {
        panic!("status")
    };
}

async fn until(what: &str, cond: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(Instant::now() < end, "timeout waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn two_inventories(node: &Node, what: &str) -> InputState {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let s = state(command(&node.tx, ModuleCommand::InputState).await);
        if s.inventories.len() == 2 {
            return s;
        }
        assert!(Instant::now() < end, "timeout waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn numero(ev: &logs::Evento, campo: &str) -> u64 {
    ev.campo(campo)
        .unwrap_or_else(|| panic!("evento {} sem {campo}: {:?}", ev.evento, ev.campos))
        .parse()
        .unwrap_or_else(|_| panic!("{campo} de {} não é número", ev.evento))
}

fn lista(ev: &logs::Evento, campo: &str) -> serde_json::Value {
    let texto = ev
        .campo(campo)
        .unwrap_or_else(|| panic!("evento {} sem {campo}: {:?}", ev.evento, ev.campos));
    serde_json::from_str(texto).unwrap_or_else(|_| panic!("{campo} de {} não é JSON", ev.evento))
}

fn com_peer<'a>(eventos: &'a [logs::Evento], peer: &str) -> &'a logs::Evento {
    eventos
        .iter()
        .find(|e| e.campo("peer") == Some(peer))
        .unwrap_or_else(|| panic!("nenhum evento com peer {peer}: {eventos:?}"))
}

// --------------------------------------------------------------- casos

/// Dois nós v4: inventário preparado, trocado, rotas montadas e a captura pronta com barreira.
#[tokio::test]
async fn inventario_rotas_e_captura_aparecem() {
    let captura = logs::capturar();
    let hub = MemHub::new();
    let a_id = node_id("a");
    let b_id = node_id("b");
    let obs_a = source_observation("A", "ia");
    let obs_b = receiver_observation("N", "ib");
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    write_devices(dir_a.path(), &[a_id, b_id]);
    write_devices(dir_b.path(), &[a_id, b_id]);
    let a = spawn(&hub, "a", dir_a, fake(&obs_a));
    let b = spawn(&hub, "b", dir_b, fake(&obs_b));
    hub.link(a.id, b.id, CONTROL, CONTROL);
    settle(&a).await;
    settle(&b).await;
    two_inventories(&a, "o inventário de b chega em a").await;
    two_inventories(&b, "o inventário de a chega em b").await;

    assert!(matches!(
        command(&a.tx, ModuleCommand::InputPrepareInventory).await,
        ModuleReply::Done
    ));
    let s = two_inventories(&a, "o inventário local é preparado de novo").await;
    let positions: Vec<MonitorPosition> = s
        .inventories
        .iter()
        .flat_map(|v| {
            v.inventory.monitors.iter().map(|m| MonitorPosition {
                key: MonitorKey {
                    peer: v.stamp.peer,
                    uid: m.uid,
                },
                x: if v.stamp.peer == a.id {
                    f64::from(m.rect.x)
                } else {
                    1000.0
                },
                y: 0.0,
            })
        })
        .collect();
    let request = OrganizationRequest {
        base_rev: 1,
        inventories: s.inventories.iter().map(|v| v.stamp).collect(),
        positions,
        replace_link_ids: vec![],
    };
    let applied = command(&a.tx, ModuleCommand::InputApplyOrganization(request)).await;
    assert!(
        matches!(applied, ModuleReply::OrganizationApplied(_)),
        "{applied:?}"
    );
    until("capture.ready", || captura.tem("capture.ready")).await;

    let preparado = captura
        .primeiro("inventory.prepared")
        .expect("inventory.prepared");
    assert_eq!(numero(&preparado, "monitores"), 1, "{preparado:?}");
    let b_curto = b.id.short();
    let a_curto = a.id.short();
    let enviados = captura.todos("inventory.sent");
    assert_eq!(numero(com_peer(&enviados, &b_curto), "monitores"), 1);
    let recebidos = captura.todos("inventory.received");
    assert_eq!(numero(com_peer(&recebidos, &a_curto), "monitores"), 1);

    let rotas = captura.primeiro("routes.built").expect("routes.built");
    assert!(numero(&rotas, "rotas") >= 1, "{rotas:?}");
    assert!(numero(&rotas, "barreiras") >= 1, "{rotas:?}");
    assert!(
        lista(&rotas, "pares")
            .as_array()
            .is_some_and(|p| p.iter().any(|v| v.as_str() == Some(b_curto.as_str()))),
        "o par sai curto em pares: {rotas:?}"
    );

    let pronto = captura.primeiro("capture.ready").expect("capture.ready");
    assert_eq!(pronto.campo("adaptador"), Some("fake"), "{pronto:?}");
    let barreiras = lista(&pronto, "barreiras");
    let barreiras = barreiras.as_array().expect("barreiras é lista");
    assert!(!barreiras.is_empty(), "{pronto:?}");
    assert_eq!(
        barreiras[0]["peer"].as_str(),
        Some(b_curto.as_str()),
        "{pronto:?}"
    );
}

/// Par v4 autorizado, mas o destino não tem módulo de entrada: sem inventário remoto.
#[tokio::test]
async fn sem_inventario_remoto_nao_monta_rotas() {
    let captura = logs::capturar();
    let hub = MemHub::new();
    let a_id = node_id("a");
    let b_id = node_id("b");
    let obs_a = source_observation("A", "ia");
    let dir = tempfile::tempdir().unwrap();
    let uid_a = MonitorUid([1; 16]);
    let uid_b = MonitorUid([2; 16]);
    let doc = LayoutDocument {
        legacy: Layout {
            rev: 1,
            options: Default::default(),
            devices: vec![device(a_id, "a"), device(b_id, "b")],
            links: Vec::new(),
        },
        organization: Some(Organization {
            positions: vec![
                MonitorPosition {
                    key: MonitorKey {
                        peer: a_id,
                        uid: uid_a,
                    },
                    x: 0.0,
                    y: 0.0,
                },
                MonitorPosition {
                    key: MonitorKey {
                        peer: b_id,
                        uid: uid_b,
                    },
                    x: 1000.0,
                    y: 0.0,
                },
            ],
            links: vec![MonitorLink {
                id: "l1".into(),
                a: MonitorEndpoint {
                    key: MonitorKey {
                        peer: a_id,
                        uid: uid_a,
                    },
                    edge: Edge::Right,
                    from: 0.0,
                    to: 1.0,
                },
                b: MonitorEndpoint {
                    key: MonitorKey {
                        peer: b_id,
                        uid: uid_b,
                    },
                    edge: Edge::Left,
                    from: 0.0,
                    to: 1.0,
                },
            }],
            legacy_bindings: vec![],
        }),
    };
    std::fs::write(
        dir.path().join(clipsync_input::LAYOUT_FILE),
        doc.to_toml_string().unwrap(),
    )
    .unwrap();
    let a = spawn(&hub, "a", dir, fake(&obs_a));
    let b = hub.add_node("b", INPUT);
    hub.link(a.id, b.id, CONTROL, CONTROL);
    settle(&a).await;
    until("inventory.sent", || captura.tem("inventory.sent")).await;
    until("routes.empty", || captura.tem("routes.empty")).await;

    com_peer(&captura.todos("inventory.sent"), &b.id.short());
    assert!(
        !captura.tem("inventory.received"),
        "sem módulo no destino, nada é recebido: {:?}",
        captura.todos("inventory.received")
    );
    let vazio = captura.primeiro("routes.empty").expect("routes.empty");
    assert_eq!(
        vazio.campo("motivo"),
        Some("no_remote_inventory"),
        "{vazio:?}"
    );
    assert!(
        !captura.tem("capture.ready"),
        "sem rota não há captura pronta: {:?}",
        captura.todos("capture.ready")
    );
}

/// A entrada chega ao destino, é registrada e a recusa sai com motivo; nada leva tecla.
#[tokio::test]
async fn entrada_registra_recebida_e_recusada() {
    let captura = logs::capturar();
    let hub = MemHub::new();
    let a_id = node_id("a");
    let a = hub.add_node("a", INPUT);
    let b_id = node_id("b");
    let obs_b = receiver_observation("N", "ib");
    let dir = tempfile::tempdir().unwrap();
    write_devices(dir.path(), &[a_id, b_id]);
    let b = spawn(&hub, "b", dir, fake(&obs_b));
    // The destination sees a legacy peer, so the refusal goes back as `Rejected`.
    hub.set_remote_version(a.id, 1);
    hub.link(a.id, b.id, PeerPerms::default(), CONTROL);
    settle(&b).await;
    // The destination refuses entries while its startup inventory preparation runs: wait for
    // the local inventory to be published before the first entry.
    until("inventory.prepared", || captura.tem("inventory.prepared")).await;
    settle(&b).await;

    a.host
        .send_reliable(
            b.id,
            ModuleMsg::Input(InputMsg::Enter {
                session: 41,
                edge: Edge::Left,
                pos: 0.5,
            }),
        )
        .await
        .unwrap();
    until("enter.received", || {
        captura
            .todos("enter.received")
            .iter()
            .any(|e| e.campo("session") == Some("41"))
    })
    .await;
    let recebida = captura
        .todos("enter.received")
        .into_iter()
        .find(|e| e.campo("session") == Some("41"))
        .expect("enter.received da sessão 41");
    assert_eq!(
        recebida.campo("peer"),
        Some(a.id.short().as_str()),
        "{recebida:?}"
    );
    assert_eq!(recebida.campo("mensagem"), Some("enter"), "{recebida:?}");
    settle(&b).await;
    let recusas_41: Vec<_> = captura
        .todos("enter.refused")
        .into_iter()
        .filter(|e| e.campo("session") == Some("41"))
        .collect();
    assert!(
        recusas_41.is_empty(),
        "a sessão 41 é aceita antes da pausa: {recusas_41:?}"
    );

    // Paused at the destination (the host is the authority on pause): the entry is injected
    // with the destination's own view of the connection, so the module refuses by its own path
    // instead of the hub gate.
    hub.set_paused(b.id, true);
    settle(&b).await;
    let context = b
        .host
        .peer_input_context(a.id)
        .expect("contexto de entrada visto pelo destino");
    let mut rx = a.rx;
    b.tx.send(ModuleInput::Reliable {
        from: a.id,
        msg: ModuleMsg::Input(InputMsg::Enter {
            session: 42,
            edge: Edge::Left,
            pos: 0.5,
        }),
        context: ReceiveContext {
            connection: context.connection,
            input_epoch: context.input_epoch,
            stream: Some(9),
        },
    })
    .await
    .unwrap();
    until("enter.refused", || {
        captura
            .todos("enter.refused")
            .iter()
            .any(|e| e.campo("session") == Some("42"))
    })
    .await;
    let recusada = captura
        .todos("enter.refused")
        .into_iter()
        .find(|e| e.campo("session") == Some("42"))
        .expect("enter.refused da sessão 42");
    assert_eq!(recusada.campo("motivo"), Some("paused"), "{recusada:?}");
    assert_eq!(
        recusada.campo("peer"),
        Some(a.id.short().as_str()),
        "{recusada:?}"
    );

    // Pausing invalidates the session 41 still open at the destination: its own refusal
    // (`reconcile_incoming`) may reach the origin before the one for session 42.
    let end = Instant::now() + Duration::from_secs(2);
    let mut rejeitadas = Vec::new();
    while Instant::now() < end {
        if let Ok(Some(ModuleInput::Rejected { reason, .. })) =
            tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
        {
            rejeitadas.push(reason);
            if reason == RejectReason::Paused {
                break;
            }
        }
    }
    assert!(
        rejeitadas.contains(&RejectReason::Paused),
        "a origem recebe Rejected(Paused): {rejeitadas:?}"
    );

    for ev in captura
        .eventos
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
    {
        for campo in ev.campos.keys() {
            assert!(
                !matches!(
                    campo.as_str(),
                    "tecla" | "key" | "keycode" | "code" | "texto"
                ),
                "evento {} leva o campo proibido {campo}",
                ev.evento
            );
        }
    }
}

/// Falha ao abrir o portal vira `capture.failed` com motivo próprio, nunca um segundo
/// `capture.start` (decisão do Maestro para a v0.5.1).
#[tokio::test]
async fn falha_da_captura_registra_motivo_proprio() {
    struct FalhaAoAbrir {
        inner: Box<dyn CaptureAdapter>,
    }
    #[async_trait::async_trait]
    impl CaptureAdapter for FalhaAoAbrir {
        fn name(&self) -> &'static str {
            self.inner.name()
        }
        async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
            self.inner.zones().await
        }
        fn known_zones(&self) -> Vec<Zone> {
            self.inner.known_zones()
        }
        async fn start(
            &mut self,
            _barriers: &[BarrierSpec],
        ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
            Err(InputError::PermissionDenied)
        }
        async fn release(&mut self) -> Result<(), InputError> {
            self.inner.release().await
        }
        async fn stop(&mut self) -> Result<(), InputError> {
            self.inner.stop().await
        }
    }

    let captura = logs::capturar();
    let hub = MemHub::new();
    let a_id = node_id("a");
    let b_id = node_id("b");
    let obs_a = source_observation("A", "ia");
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout {
        rev: 1,
        options: Default::default(),
        devices: vec![device(a_id, "a"), device(b_id, "b")],
        links: vec![clipsync_input::Link {
            id: "l1".into(),
            a: clipsync_input::Endpoint {
                peer: a_id,
                monitor: "A".into(),
                edge: Edge::Right,
                from: 0.0,
                to: 1.0,
            },
            b: clipsync_input::Endpoint {
                peer: b_id,
                monitor: "N".into(),
                edge: Edge::Left,
                from: 0.0,
                to: 1.0,
            },
        }],
    };
    layout
        .write(&dir.path().join(clipsync_input::LAYOUT_FILE))
        .unwrap();
    let adapter = FalhaAoAbrir {
        inner: fake(&obs_a),
    };
    let a = spawn(&hub, "a", dir, Box::new(adapter));
    let b = hub.add_node("b", INPUT);
    hub.link(a.id, b.id, CONTROL, CONTROL);
    until("capture.failed", || captura.tem("capture.failed")).await;

    let falha = captura.primeiro("capture.failed").expect("capture.failed");
    assert_eq!(falha.campo("motivo"), Some("portal_denied"), "{falha:?}");
    let inicios = captura.todos("capture.start");
    assert_eq!(inicios.len(), 1, "um único capture.start: {inicios:?}");
    assert!(
        !captura.tem("capture.ready"),
        "sem captura não há capture.ready: {:?}",
        captura.todos("capture.ready")
    );
}
