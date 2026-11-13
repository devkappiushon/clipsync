//! Integração F1/F2: dois módulos sobre MemHub e o emissor PortalEmulate real sobre EI simulado.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/logs.rs"]
mod logs;
use async_trait::async_trait;
use clipsync_input::{
    EmulateAdapter, FakeCapture, FakeCaptureHandle, FakeEmulate, InputConfig, InputError, Layout,
    LayoutDevice, LayoutOptions, PortalEmulate, TokenStore, Zone, build_with_layout,
};
use clipsync_proto::*;
use reis::{
    PendingRequestResult, eis,
    handshake::EisHandshaker,
    request::{Connection, Device, DeviceCapability, EisRequest, EisRequestConverter, Seat},
};
use std::{
    future::{Future, poll_fn},
    os::unix::net::UnixStream,
    pin::pin,
    task::Poll,
    time::{Duration, Instant},
};
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

// Servidor independente e mínimo: não implementa a política de descarte que está sob teste.
// Apenas valida start antes dos pedidos e conta o que efetivamente chegou no fio EI.
struct Wire {
    ctx: eis::Context,
    handshake: Option<EisHandshaker>,
    conv: Option<EisRequestConverter>,
    conn: Option<Connection>,
    seat: Option<Seat>,
    pointer: Option<Device>,
    keyboard: Option<Device>,
    emulating: Vec<Device>,
    motions: usize,
    keys: usize,
}
impl Wire {
    fn new(stream: UnixStream) -> Self {
        let ctx = eis::Context::new(stream).unwrap();
        let handshake = Some(EisHandshaker::new(&ctx, 1));
        Self {
            ctx,
            handshake,
            conv: None,
            conn: None,
            seat: None,
            pointer: None,
            keyboard: None,
            emulating: Vec::new(),
            motions: 0,
            keys: 0,
        }
    }
    fn pump(&mut self) {
        match self.ctx.read() {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return,
            Err(_) => panic!("erro de transporte EI no servidor de teste"),
        }
        while let Some(pending) = self.ctx.pending_request() {
            let PendingRequestResult::Request(raw) = pending else {
                panic!("pedido EI inválido");
            };
            if let Some(h) = self.handshake.as_mut() {
                if let Some(response) = h.handle_request(raw).unwrap() {
                    let converter = EisRequestConverter::new(&self.ctx, response, 1);
                    let conn = converter.handle().clone();
                    self.seat = Some(conn.add_seat(
                        Some("teste"),
                        DeviceCapability::Pointer
                            | DeviceCapability::Button
                            | DeviceCapability::Scroll
                            | DeviceCapability::Keyboard,
                    ));
                    self.conn = Some(conn);
                    self.conv = Some(converter);
                    self.handshake = None;
                } else {
                    continue;
                }
            } else {
                let mut converter = self.conv.take().unwrap();
                converter.handle_request(raw).unwrap();
                while let Some(request) = converter.next_request() {
                    match request {
                        EisRequest::Bind(bind) => {
                            if self.pointer.is_none()
                                && bind.capabilities.contains(DeviceCapability::Pointer)
                            {
                                let d = bind.seat.add_device(
                                    Some("ponteiro"),
                                    eis::device::DeviceType::Virtual,
                                    DeviceCapability::Pointer
                                        | DeviceCapability::Button
                                        | DeviceCapability::Scroll,
                                    |_| {},
                                );
                                d.resumed();
                                self.pointer = Some(d);
                            }
                        }
                        EisRequest::DeviceStartEmulating(start) => {
                            assert!(!self.emulating.contains(&start.device), "start duplicado");
                            self.emulating.push(start.device);
                        }
                        EisRequest::DeviceStopEmulating(stop) => {
                            self.emulating.retain(|d| d != &stop.device)
                        }
                        EisRequest::PointerMotion(motion) => {
                            assert!(self.emulating.contains(&motion.device));
                            self.motions += 1;
                        }
                        EisRequest::KeyboardKey(key) => {
                            assert!(self.emulating.contains(&key.device));
                            self.keys += 1;
                        }
                        _ => {}
                    }
                }
                self.conv = Some(converter);
            }
            self.conn.as_ref().unwrap().flush().unwrap();
        }
    }
    fn add_keyboard(&mut self) {
        let d = self.seat.as_ref().unwrap().add_device(
            Some("teclado"),
            eis::device::DeviceType::Virtual,
            DeviceCapability::Keyboard.into(),
            |_| {},
        );
        d.resumed();
        self.keyboard = Some(d);
        self.conn.as_ref().unwrap().flush().unwrap();
    }
    fn pause_keyboard(&mut self) {
        let device = self.keyboard.as_ref().unwrap();
        device.paused();
        self.emulating.retain(|d| d != device);
        self.conn.as_ref().unwrap().flush().unwrap();
    }
    fn resume_keyboard(&mut self) {
        self.keyboard.as_ref().unwrap().resumed();
        self.conn.as_ref().unwrap().flush().unwrap();
    }
    fn remove_pointer(&mut self) {
        let d = self.pointer.take().unwrap();
        self.emulating.retain(|old| old != &d);
        d.remove();
        self.conn.as_ref().unwrap().flush().unwrap();
    }
}
async fn drive<T>(wire: &mut Wire, future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    loop {
        wire.pump();
        if let Poll::Ready(output) = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await {
            return output;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}
async fn observe(wire: &mut Wire, what: &str, limit: Duration, predicate: impl Fn(&Wire) -> bool) {
    let start = Instant::now();
    loop {
        wire.pump();
        if predicate(wire) {
            return;
        }
        assert!(start.elapsed() < limit, "prazo excedido: {what}");
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

// Só substitui o portal/D-Bus por um socket de teste. Prontidão, descarte e emissão
// continuam no código de produção PortalEmulate, sem reimplementar a política no fake.
struct TestEmulate {
    stream: Option<UnixStream>,
    portal: Option<PortalEmulate>,
    profile: tempfile::TempDir,
    delay: Duration,
}
#[async_trait]
impl EmulateAdapter for TestEmulate {
    fn name(&self) -> &'static str {
        "eis-sem-portal"
    }
    async fn start(&mut self) -> Result<(), InputError> {
        if self.portal.is_none() {
            tokio::time::sleep(self.delay).await;
            self.portal = Some(
                PortalEmulate::attach(
                    TokenStore::new(self.profile.path()),
                    self.stream.take().unwrap(),
                )
                .await?,
            );
        }
        Ok(())
    }
    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        self.portal.as_mut().unwrap().emit(events).await
    }
    fn block_reason(&self) -> Option<&'static str> {
        self.portal.as_ref().and_then(EmulateAdapter::block_reason)
    }
    async fn release_all(&mut self) -> Result<(), InputError> {
        if let Some(p) = &mut self.portal {
            p.release_all().await?;
        }
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        if let Some(p) = &mut self.portal {
            p.stop().await?;
        }
        Ok(())
    }
}
fn id(name: &str) -> PeerId {
    PeerId::from_spki_der(format!("memhub:{name}").as_bytes())
}
fn layout() -> Layout {
    let end = |peer, edge| clipsync_input::Endpoint {
        peer,
        monitor: "eDP-1".into(),
        edge,
        from: 0.0,
        to: 1.0,
    };
    Layout {
        rev: 1,
        options: LayoutOptions { dead_corner_px: 0 },
        devices: ["a", "b"]
            .into_iter()
            .map(|name| LayoutDevice {
                peer: id(name),
                name: name.into(),
                x: 0,
                y: 0,
                hotkey: None,
            })
            .collect(),
        links: vec![clipsync_input::Link {
            id: "ab".into(),
            a: end(id("a"), Edge::Right),
            b: end(id("b"), Edge::Left),
        }],
    }
}
struct Node {
    cap: FakeCaptureHandle,
    tx: mpsc::Sender<ModuleInput>,
    task: tokio::task::JoinHandle<()>,
    _profile: tempfile::TempDir,
}
impl Drop for Node {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn spawn(hub: &MemHub, name: &str, emu: Box<dyn EmulateAdapter>) -> Node {
    let node = hub.add_node(name, INPUT);
    let profile = tempfile::tempdir().unwrap();
    layout()
        .write(&profile.path().join(clipsync_input::LAYOUT_FILE))
        .unwrap();
    let (cap, handle) = FakeCapture::new(vec![Zone {
        index: 0,
        name: Some("eDP-1".into()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }]);
    let module =
        build_with_layout(InputConfig::default(), Box::new(cap), emu, profile.path()).unwrap();
    let task = tokio::spawn(async move {
        let _ = module.run(node.host, node.rx).await;
    });
    Node {
        cap: handle,
        tx: node.tx,
        task,
        _profile: profile,
    }
}
async fn open(wire: &mut Wire, a: &Node) {
    observe(wire, "barreira", Duration::from_secs(1), |_| {
        !a.cap.barriers().is_empty()
    })
    .await;
    let (reply, rx) = oneshot::channel();
    a.tx.send(ModuleInput::Command {
        cmd: ModuleCommand::InputSwitch { to: id("b") },
        reply,
    })
    .await
    .unwrap();
    let reply = drive(wire, tokio::time::timeout(Duration::from_secs(3), rx))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(reply, ModuleReply::Done));
}
async fn status(wire: &mut Wire, node: &Node) -> ModuleStatus {
    let (reply, rx) = oneshot::channel();
    node.tx
        .send(ModuleInput::Command {
            cmd: ModuleCommand::Status,
            reply,
        })
        .await
        .unwrap();
    let result = drive(wire, tokio::time::timeout(Duration::from_secs(1), rx))
        .await
        .unwrap()
        .unwrap();
    let ModuleReply::Status(status) = result else {
        panic!("resposta de status esperada");
    };
    status
}

fn pair(delay: Duration) -> (MemHub, Node, Node, Wire) {
    let hub = MemHub::new();
    let (a_emu, _) = FakeEmulate::new();
    let a = spawn(&hub, "a", Box::new(a_emu));
    let (client, server) = UnixStream::pair().unwrap();
    let b = spawn(
        &hub,
        "b",
        Box::new(TestEmulate {
            stream: Some(client),
            portal: None,
            profile: tempfile::tempdir().unwrap(),
            delay,
        }),
    );
    hub.set_remote_version(id("a"), 1);
    hub.set_remote_version(id("b"), 1);
    hub.link(id("a"), id("b"), CONTROL, CONTROL);
    (hub, a, b, Wire::new(server))
}
fn key(pressed: bool) -> InputEvent {
    InputEvent::Key { code: 30, pressed }
}

#[tokio::test]
async fn teclado_ausente_nao_fecha_sessao_mouse_continua_e_teclado_tardio_funciona() {
    let log = logs::capturar();
    let (_hub, a, mut b, mut wire) = pair(Duration::from_millis(1600));
    let start = Instant::now();
    open(&mut wire, &a).await;
    observe(
        &mut wire,
        "primeiro posicionamento",
        Duration::from_secs(3),
        |w| w.motions > 0,
    )
    .await;
    assert!(
        start.elapsed() < Duration::from_millis(1950),
        "primeira entrada não pode esperar 500 ms de teclado e perder o prazo F2"
    );
    assert!(!log.tem("session.closed"));
    assert!(
        status(&mut wire, &b).await.last_block.is_none(),
        "sem descarte ainda não há aviso de teclado"
    );
    let initial_motion = wire.motions;
    let releases = a.cap.released();
    a.cap.push(InputEvent::Motion { dx: 1.0, dy: 0.0 });
    a.cap.push(key(true));
    a.cap.push(key(false));
    a.cap.push(InputEvent::Motion { dx: 1.0, dy: 0.0 });
    observe(
        &mut wire,
        "mouse segue sem teclado",
        Duration::from_secs(1),
        |w| w.motions >= initial_motion + 2,
    )
    .await;
    drive(&mut wire, tokio::time::sleep(Duration::from_millis(650))).await;
    assert!(
        !log.tem("session.closed"),
        "tecla ausente não pode causar ApplyFailed ou peer_unresponsive"
    );
    assert_eq!(wire.keys, 0);
    assert_eq!(a.cap.released(), releases);
    const WARNING: &str =
        "Teclado indisponível neste computador: as teclas não estão sendo aplicadas";
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(WARNING),
        "ModuleStatus precisa informar descarte persistente"
    );
    drive(&mut wire, tokio::time::sleep(Duration::from_millis(150))).await;
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(WARNING),
        "aviso continua mesmo sem nova entrada"
    );
    wire.add_keyboard();
    observe(
        &mut wire,
        "teclado retomado no emissor",
        Duration::from_secs(1),
        |_| {
            log.todos("eis.device").iter().any(|e| {
                e.campo("acao") == Some("resumed") && e.campo("capacidades") == Some("keyboard")
            })
        },
    )
    .await;
    let deadline = Instant::now() + Duration::from_secs(1);
    while status(&mut wire, &b).await.last_block.is_some() {
        assert!(
            Instant::now() < deadline,
            "retomada limpa o aviso sem exigir outra tecla"
        );
        drive(&mut wire, tokio::time::sleep(Duration::from_millis(2))).await;
    }
    a.cap.push(key(true));
    a.cap.push(key(false));
    observe(
        &mut wire,
        "teclas no teclado tardio",
        Duration::from_secs(1),
        |w| w.keys == 2,
    )
    .await;
    assert!(!log.tem("session.closed"));
    b.tx.send(ModuleInput::Shutdown).await.unwrap();
    drive(
        &mut wire,
        tokio::time::timeout(Duration::from_secs(1), &mut b.task),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        log.primeiro("eis.loop").unwrap().campo("descartados"),
        Some("2")
    );
    assert_eq!(
        log.todos("eis.device")
            .iter()
            .filter(|e| e.campo("acao") == Some("discarding"))
            .count(),
        1,
        "um aviso no início do descarte, sem log por tecla"
    );
}

#[tokio::test]
async fn ponteiro_ausente_no_meio_da_sessao_preserva_apply_failed_da_f2() {
    let log = logs::capturar();
    let (_hub, a, _b, mut wire) = pair(Duration::ZERO);
    open(&mut wire, &a).await;
    observe(
        &mut wire,
        "posicionamento inicial",
        Duration::from_secs(1),
        |w| w.motions > 0,
    )
    .await;
    wire.remove_pointer();
    observe(
        &mut wire,
        "remoção observada",
        Duration::from_secs(1),
        |_| {
            log.todos("eis.device").iter().any(|e| {
                e.campo("acao") == Some("removed")
                    && e.campo("capacidades")
                        .is_some_and(|c| c.contains("pointer"))
            })
        },
    )
    .await;
    let releases = a.cap.released();
    a.cap.push(InputEvent::Motion { dx: 1.0, dy: 0.0 });
    observe(
        &mut wire,
        "recusa ApplyFailed",
        Duration::from_secs(1),
        |_| log.tem("session.refused"),
    )
    .await;
    assert_eq!(
        log.primeiro("session.refused").unwrap().campo("motivo"),
        Some("apply_failed")
    );
    assert_eq!(
        log.primeiro("session.closed").unwrap().campo("motivo"),
        Some("refused")
    );
    assert!(a.cap.released() > releases);
}

#[tokio::test]
async fn aviso_do_teclado_pausado_persiste_e_some_ao_retomar_sem_nova_tecla() {
    let log = logs::capturar();
    let (_hub, a, mut b, mut wire) = pair(Duration::ZERO);
    open(&mut wire, &a).await;
    observe(&mut wire, "entrada inicial", Duration::from_secs(1), |w| {
        w.motions > 0
    })
    .await;
    wire.add_keyboard();
    observe(
        &mut wire,
        "primeiro resumed",
        Duration::from_secs(1),
        |_| {
            log.todos("eis.device").iter().any(|e| {
                e.campo("acao") == Some("resumed") && e.campo("capacidades") == Some("keyboard")
            })
        },
    )
    .await;
    a.cap.push(key(true));
    a.cap.push(key(false));
    observe(
        &mut wire,
        "teclado inicialmente funcional",
        Duration::from_secs(1),
        |w| w.keys == 2,
    )
    .await;
    assert!(status(&mut wire, &b).await.last_block.is_none());
    wire.pause_keyboard();
    observe(&mut wire, "pausa recebida", Duration::from_secs(1), |_| {
        log.todos("eis.device").iter().any(|e| {
            e.campo("acao") == Some("paused") && e.campo("capacidades") == Some("keyboard")
        })
    })
    .await;
    a.cap.push(key(true));
    a.cap.push(key(false));
    observe(
        &mut wire,
        "descarte após pausa",
        Duration::from_secs(1),
        |_| {
            log.todos("eis.device")
                .iter()
                .any(|e| e.campo("acao") == Some("discarding"))
        },
    )
    .await;
    const WARNING: &str =
        "Teclado indisponível neste computador: as teclas não estão sendo aplicadas";
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(WARNING)
    );
    drive(&mut wire, tokio::time::sleep(Duration::from_millis(150))).await;
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(WARNING)
    );
    assert_eq!(
        wire.keys, 2,
        "o servidor não recebe teclas enquanto pausado"
    );
    wire.resume_keyboard();
    let deadline = Instant::now() + Duration::from_secs(1);
    while status(&mut wire, &b).await.last_block.is_some() {
        assert!(
            Instant::now() < deadline,
            "aviso precisa sumir pela retomada, sem outra tecla"
        );
        drive(&mut wire, tokio::time::sleep(Duration::from_millis(2))).await;
    }
    assert!(!log.tem("session.closed"));
    a.cap.push(key(true));
    a.cap.push(key(false));
    observe(
        &mut wire,
        "teclas após retomada",
        Duration::from_secs(1),
        |w| w.keys == 4,
    )
    .await;
    b.tx.send(ModuleInput::Shutdown).await.unwrap();
    drive(
        &mut wire,
        tokio::time::timeout(Duration::from_secs(1), &mut b.task),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        log.primeiro("eis.loop").unwrap().campo("descartados"),
        Some("2")
    );
}

#[tokio::test]
async fn status_preserva_bloqueio_do_modulo_antes_do_aviso_do_teclado() {
    let log = logs::capturar();
    let (_hub, a, mut b, mut wire) = pair(Duration::ZERO);
    open(&mut wire, &a).await;
    observe(&mut wire, "entrada inicial", Duration::from_secs(1), |w| {
        w.motions > 0
    })
    .await;
    let reason = RejectReason::NotAuthorized;
    b.tx.send(ModuleInput::Rejected {
        from: id("a"),
        ref_id: None,
        reason,
    })
    .await
    .unwrap();
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(reason.pt()),
        "bloqueio do módulo sem aviso permanece visível"
    );
    a.cap.push(key(true));
    a.cap.push(key(false));
    observe(
        &mut wire,
        "episódio de descarte",
        Duration::from_secs(1),
        |_| {
            log.todos("eis.device")
                .iter()
                .any(|e| e.campo("acao") == Some("discarding"))
        },
    )
    .await;
    let expected = format!(
        "{} · {}",
        reason.pt(),
        "Teclado indisponível neste computador: as teclas não estão sendo aplicadas"
    );
    assert_eq!(
        status(&mut wire, &b).await.last_block.as_deref(),
        Some(expected.as_str()),
        "motivo do módulo deve vir primeiro e nenhum aviso pode sumir"
    );
    wire.add_keyboard();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let current = status(&mut wire, &b).await;
        if current.last_block.as_deref() == Some(reason.pt()) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "retomada tira só o aviso do teclado, preservando o bloqueio do módulo"
        );
        drive(&mut wire, tokio::time::sleep(Duration::from_millis(2))).await;
    }
    assert!(!log.tem("session.closed"));
    b.tx.send(ModuleInput::Shutdown).await.unwrap();
    drive(
        &mut wire,
        tokio::time::timeout(Duration::from_secs(1), &mut b.task),
    )
    .await
    .unwrap()
    .unwrap();
}
