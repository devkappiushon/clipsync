//! Regressões da soltura por silêncio e falha de aplicação, sem plataforma física.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/logs.rs"]
mod logs;

use async_trait::async_trait;
use clipsync_input::{
    EmulateAdapter, FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle, InputConfig,
    InputError, Layout, LayoutDevice, LayoutOptions, Zone, build_with_layout,
};
use clipsync_proto::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const INPUT: ModuleSet = ModuleSet {
    clipboard: false,
    input: true,
};
const CONTROL: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: true,
};

fn id(name: &str) -> PeerId {
    PeerId::from_spki_der(format!("memhub:{name}").as_bytes())
}
fn zone() -> Zone {
    Zone {
        index: 0,
        name: Some("eDP-1".into()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}
fn layout() -> Layout {
    let endpoint = |peer, edge| clipsync_input::Endpoint {
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
            a: endpoint(id("a"), Edge::Right),
            b: endpoint(id("b"), Edge::Left),
        }],
    }
}
struct Node {
    host: Arc<dyn ModuleHost>,
    tx: mpsc::Sender<ModuleInput>,
    cap: FakeCaptureHandle,
    task: JoinHandle<()>,
    _dir: tempfile::TempDir,
}
impl Drop for Node {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn spawn(hub: &MemHub, name: &str, emu: Box<dyn EmulateAdapter>) -> Node {
    let dir = tempfile::tempdir().unwrap();
    layout()
        .write(&dir.path().join(clipsync_input::LAYOUT_FILE))
        .unwrap();
    let node = hub.add_node(name, INPUT);
    let (cap, handle) = FakeCapture::new(vec![zone()]);
    let module = build_with_layout(InputConfig::default(), Box::new(cap), emu, dir.path()).unwrap();
    let host = node.host.clone();
    let task = tokio::spawn(async move {
        let _ = module.run(node.host, node.rx).await;
    });
    Node {
        host,
        tx: node.tx,
        cap: handle,
        task,
        _dir: dir,
    }
}
async fn command(node: &Node, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    node.tx
        .send(ModuleInput::Command { cmd, reply })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), rx)
        .await
        .unwrap()
        .unwrap()
}
async fn until(what: &str, limit: Duration, predicate: impl Fn() -> bool) {
    let start = Instant::now();
    while !predicate() {
        assert!(start.elapsed() < limit, "prazo excedido: {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
async fn open(a: &Node) {
    until("barreira instalada", Duration::from_secs(2), || {
        !a.cap.barriers().is_empty()
    })
    .await;
    assert!(matches!(
        command(a, ModuleCommand::InputSwitch { to: id("b") }).await,
        ModuleReply::Done
    ));
    assert!(!a.cap.begins().is_empty());
}
fn closed(log: &logs::Captura, reason: &str) -> bool {
    log.todos("session.closed")
        .iter()
        .any(|e| e.campo("motivo") == Some(reason))
}

// Um receptor controlado deixa a conexão e a fila de entrada vivas ao calar somente Stats.
struct Responder {
    respond: Arc<AtomicBool>,
    requests: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}
impl Drop for Responder {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn responder(mut node: MemNode) -> Responder {
    let respond = Arc::new(AtomicBool::new(true));
    let requests = Arc::new(AtomicUsize::new(0));
    let on = respond.clone();
    let count = requests.clone();
    let task = tokio::spawn(async move {
        while let Some(input) = node.rx.recv().await {
            if let ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Input(InputMsg::StatsRequest { session }),
                ..
            } = input
            {
                count.fetch_add(1, Ordering::SeqCst);
                if on.load(Ordering::SeqCst) {
                    node.host
                        .send_reliable(
                            from,
                            ModuleMsg::Input(InputMsg::Stats(LoopbackStats {
                                session,
                                received: 0,
                                lost: 0,
                                out_of_order: 0,
                                p50_us: 0,
                                p95_us: 0,
                                max_us: 0,
                            })),
                        )
                        .await
                        .unwrap();
                }
            }
        }
    });
    Responder {
        respond,
        requests,
        task,
    }
}
async fn raw_pair(hub: &MemHub) -> (Node, Responder) {
    let (emu, _) = FakeEmulate::new();
    let a = spawn(hub, "a", Box::new(emu));
    let b = hub.add_node("b", INPUT);
    hub.set_remote_version(b.id, 1);
    hub.link(id("a"), id("b"), CONTROL, CONTROL);
    let b = responder(b);
    open(&a).await;
    (a, b)
}

#[tokio::test]
async fn silencio_de_stats_libera_captura_em_dois_segundos() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, b) = raw_pair(&hub).await;
    let released = a.cap.released();
    b.respond.store(false, Ordering::SeqCst);
    let start = Instant::now();
    // Entrada local contínua não pode renovar nem impedir o prazo.
    while !closed(&log, "peer_unresponsive") && start.elapsed() < Duration::from_millis(2500) {
        a.cap.push(InputEvent::Motion { dx: 1.0, dy: 0.0 });
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        closed(&log, "peer_unresponsive"),
        "sessão muda deve fechar em até 2,5 s"
    );
    assert!(a.cap.released() > released, "dispositivo local devolvido");
    until("nova geração", Duration::from_secs(1), || {
        a.cap.starts() > 1
    })
    .await;
    assert_eq!(log.todos("session.closed").len(), 1);
}

#[tokio::test]
async fn sessao_saudavel_ociosa_permanece_ativa_por_cinco_segundos() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, b) = raw_pair(&hub).await;
    let released = a.cap.released();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        b.requests.load(Ordering::SeqCst) >= 8,
        "vigia deve sondar a sessão mesmo sem movimento"
    );
    assert!(
        !log.tem("session.closed"),
        "respostas mantêm a sessão ativa"
    );
    assert_eq!(a.cap.released(), released);
}

struct FalhaEmit {
    inner: FakeEmulate,
    fail: Arc<AtomicBool>,
}
#[async_trait]
impl EmulateAdapter for FalhaEmit {
    fn name(&self) -> &'static str {
        "fake"
    }
    async fn start(&mut self) -> Result<(), InputError> {
        self.inner.start().await
    }
    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(InputError::Other("falha simulada".into()));
        }
        self.inner.emit(events).await
    }
    async fn release_all(&mut self) -> Result<(), InputError> {
        self.inner.release_all().await
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.inner.stop().await
    }
}

#[tokio::test]
async fn falha_emit_legada_recusa_e_solta_tecla_e_botao() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (emu_a, _) = FakeEmulate::new();
    let a = spawn(&hub, "a", Box::new(emu_a));
    let (emu_b, handle): (_, FakeEmulateHandle) = FakeEmulate::new();
    let fail = Arc::new(AtomicBool::new(false));
    let _b = spawn(
        &hub,
        "b",
        Box::new(FalhaEmit {
            inner: emu_b,
            fail: fail.clone(),
        }),
    );
    // v1 força Rejected, sem Refused v3 ou inventário/rota v4.
    hub.set_remote_version(id("a"), 1);
    hub.set_remote_version(id("b"), 1);
    hub.link(id("a"), id("b"), CONTROL, CONTROL);
    open(&a).await;
    a.cap.push(InputEvent::Key {
        code: 30,
        pressed: true,
    });
    a.cap.push(InputEvent::Button {
        code: 272,
        pressed: true,
    });
    until(
        "tecla e botão pressionados no destino",
        Duration::from_secs(1),
        || handle.pressed_now() == 2,
    )
    .await;
    let released = a.cap.released();
    let releases_remote = handle.release_calls();
    fail.store(true, Ordering::SeqCst);
    a.cap.push(InputEvent::Motion { dx: 1.0, dy: 0.0 });
    until(
        "recusa por falha de aplicação",
        Duration::from_secs(1),
        || closed(&log, "refused"),
    )
    .await;
    assert!(a.cap.released() > released);
    assert_eq!(handle.pressed_now(), 0);
    assert!(handle.release_calls() > releases_remote);
    let refused = log.primeiro("session.refused").unwrap();
    assert_eq!(refused.campo("motivo"), Some("apply_failed"));
    assert!(
        log.todos("enter.refused").iter().any(
            |e| e.campo("motivo") == Some("apply_failed") && e.campo("enviada") == Some("true")
        )
    );
}

fn stats(session: u32) -> InputMsg {
    InputMsg::Stats(LoopbackStats {
        session,
        received: 0,
        lost: 0,
        out_of_order: 0,
        p50_us: 0,
        p95_us: 0,
        max_us: 0,
    })
}

#[tokio::test]
async fn stats_de_outro_par_conexao_ou_sessao_nao_renovam_prazo() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, b) = raw_pair(&hub).await;
    b.respond.store(false, Ordering::SeqCst);
    let c = hub.add_node("c", INPUT);
    hub.link(id("a"), c.id, CONTROL, CONTROL);
    let session: u32 = log
        .primeiro("session.open")
        .unwrap()
        .campo("session")
        .unwrap()
        .parse()
        .unwrap();
    let connection = a.host.peer_input_context(id("b")).unwrap().connection;
    let other_connection = a.host.peer_input_context(c.id).unwrap().connection;
    let start = Instant::now();
    while !closed(&log, "peer_unresponsive") && start.elapsed() < Duration::from_millis(2500) {
        for (from, connection, session) in [
            (id("b"), connection, session.wrapping_add(1)),
            (id("b"), connection.wrapping_add(1), session),
            (c.id, other_connection, session),
        ] {
            a.tx.send(ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Input(stats(session)),
                context: ReceiveContext {
                    connection,
                    input_epoch: 0,
                    stream: Some(0),
                },
            })
            .await
            .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(closed(&log, "peer_unresponsive"));
}

#[tokio::test]
async fn silencio_conta_da_ultima_resposta_valida() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, b) = raw_pair(&hub).await;
    tokio::time::sleep(Duration::from_millis(2200)).await;
    assert!(!log.tem("session.closed"));
    assert!(b.requests.load(Ordering::SeqCst) >= 4);
    b.respond.store(false, Ordering::SeqCst);
    let start = Instant::now();
    until(
        "prazo após última resposta",
        Duration::from_millis(2500),
        || closed(&log, "peer_unresponsive"),
    )
    .await;
    assert!(
        start.elapsed() >= Duration::from_millis(1400),
        "não conta apenas da abertura"
    );
    assert!(a.cap.released() > 0);
}

#[tokio::test]
async fn destino_real_do_modulo_responde_stats_sem_movimento() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (emu_a, _) = FakeEmulate::new();
    let a = spawn(&hub, "a", Box::new(emu_a));
    let (emu_b, _) = FakeEmulate::new();
    let _b = spawn(&hub, "b", Box::new(emu_b));
    hub.set_remote_version(id("a"), 1);
    hub.set_remote_version(id("b"), 1);
    hub.link(id("a"), id("b"), CONTROL, CONTROL);
    open(&a).await;
    let before = hub.sent_count(id("b"), id("a"));
    let releases = a.cap.released();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(hub.sent_count(id("b"), id("a")) >= before + 8);
    assert!(!log.tem("session.closed"));
    assert_eq!(a.cap.released(), releases);
    // Respostas normais da vigia não viram avisos de mensagem ignorada na interface.
    let ModuleReply::Status(status) = command(&a, ModuleCommand::Status).await else {
        panic!("status");
    };
    assert!(status.last_block.is_none(), "{:?}", status.last_block);
}

// O proprietário da captura pode esperar o adaptador enquanto o módulo recebe Stats.
#[derive(Default)]
struct BeginGate {
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
struct CapturaComEspera {
    inner: FakeCapture,
    gate: Arc<BeginGate>,
}
#[async_trait]
impl clipsync_input::CaptureAdapter for CapturaComEspera {
    fn name(&self) -> &'static str {
        "fake-com-espera"
    }
    fn known_zones(&self) -> Vec<Zone> {
        self.inner.known_zones()
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        self.inner.zones().await
    }
    async fn start(
        &mut self,
        specs: &[clipsync_input::BarrierSpec],
    ) -> Result<mpsc::Receiver<clipsync_input::CaptureEvent>, InputError> {
        self.inner.start(specs).await
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.inner.release().await
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.inner.stop().await
    }
    async fn set_barriers(
        &mut self,
        specs: &[clipsync_input::BarrierSpec],
    ) -> Result<(), InputError> {
        self.inner.set_barriers(specs).await
    }
    async fn begin(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        self.gate.entered.notify_one();
        self.gate.resume.notified().await;
        self.inner.begin(barrier, pos).await
    }
}

struct RedeComEspera {
    inner: Arc<dyn ModuleHost>,
    block_enter: AtomicBool,
    blocked_enters: AtomicUsize,
    block_leave: AtomicBool,
    leaving: tokio::sync::Notify,
}
#[async_trait]
impl ModuleHost for RedeComEspera {
    fn local_id(&self) -> PeerId {
        self.inner.local_id()
    }
    fn local_name(&self) -> String {
        self.inner.local_name()
    }
    fn peers(&self) -> Vec<PeerInfo> {
        self.inner.peers()
    }
    fn paused(&self) -> bool {
        self.inner.paused()
    }
    fn lifecycle_receiver(&self) -> Option<tokio::sync::watch::Receiver<HostLifecycle>> {
        self.inner.lifecycle_receiver()
    }
    fn peer_input_context(&self, peer: PeerId) -> Option<PeerInputContext> {
        self.inner.peer_input_context(peer)
    }
    fn request_pause(&self, paused: bool) {
        self.inner.request_pause(paused);
    }
    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError> {
        self.inner.send_datagram(to, dg)
    }
    async fn reject_input_on(
        &self,
        to: PeerId,
        connection: u64,
        reason: RejectReason,
    ) -> Result<(), SendError> {
        self.inner.reject_input_on(to, connection, reason).await
    }
    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError> {
        let connection = self.inner.peer_input_context(to).unwrap().connection;
        self.send_reliable_on(to, connection, msg).await
    }
    async fn send_reliable_on(
        &self,
        to: PeerId,
        connection: u64,
        msg: ModuleMsg,
    ) -> Result<(), SendError> {
        let enter = matches!(
            msg,
            ModuleMsg::Input(
                InputMsg::Enter { .. }
                    | InputMsg::EnterAt { .. }
                    | InputMsg::EnterSpan { .. }
                    | InputMsg::EnterRoutes(_)
            )
        );
        let leave = matches!(msg, ModuleMsg::Input(InputMsg::Leave { .. }));
        self.inner.send_reliable_on(to, connection, msg).await?;
        if leave && self.block_leave.load(Ordering::SeqCst) {
            self.leaving.notify_one();
            return std::future::pending().await;
        }
        if enter && self.block_enter.load(Ordering::SeqCst) {
            self.blocked_enters.fetch_add(1, Ordering::SeqCst);
            return std::future::pending().await;
        }
        Ok(())
    }
}

fn source_with_gates(hub: &MemHub) -> (Node, Arc<BeginGate>, Arc<RedeComEspera>) {
    let dir = tempfile::tempdir().unwrap();
    layout()
        .write(&dir.path().join(clipsync_input::LAYOUT_FILE))
        .unwrap();
    let node = hub.add_node("a", INPUT);
    let (cap, handle) = FakeCapture::new(vec![zone()]);
    let gate = Arc::new(BeginGate::default());
    let network = Arc::new(RedeComEspera {
        inner: node.host,
        block_enter: AtomicBool::new(false),
        blocked_enters: AtomicUsize::new(0),
        block_leave: AtomicBool::new(false),
        leaving: tokio::sync::Notify::new(),
    });
    let (emu, _) = FakeEmulate::new();
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(CapturaComEspera {
            inner: cap,
            gate: gate.clone(),
        }),
        Box::new(emu),
        dir.path(),
    )
    .unwrap();
    let host = network.clone();
    let task = tokio::spawn(async move {
        let _ = module.run(host, node.rx).await;
    });
    (
        Node {
            host: network.clone(),
            tx: node.tx,
            cap: handle,
            task,
            _dir: dir,
        },
        gate,
        network,
    )
}

#[tokio::test]
async fn abertura_por_barreira_tem_prazo_mesmo_com_limpeza_e_envio_bloqueados() {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, _gate, network) = source_with_gates(&hub);
    let _b = hub.add_node("b", INPUT);
    hub.set_remote_version(id("b"), 1);
    hub.link(id("a"), id("b"), CONTROL, CONTROL);
    until("barreira", Duration::from_secs(1), || {
        !a.cap.barriers().is_empty()
    })
    .await;
    a.cap.activate(0, 0.5);
    until("primeira sessão ativa", Duration::from_secs(1), || {
        log.tem("session.active")
    })
    .await;
    network.block_leave.store(true, Ordering::SeqCst);
    a.cap.deactivate();
    tokio::time::timeout(Duration::from_secs(1), network.leaving.notified())
        .await
        .unwrap();
    assert!(a.cap.released() > 0, "primeiro fechamento não espera rede");
    until("geração nova", Duration::from_secs(1), || {
        a.cap.starts() > 1
    })
    .await;
    let releases = a.cap.released();
    // O orçamento de Leave começa antes: Enter terá de ser sondado e bloquear
    // antes do prazo absoluto desta segunda ativação, sem somar seus orçamentos.
    tokio::time::sleep(Duration::from_millis(100)).await;
    network.block_enter.store(true, Ordering::SeqCst);
    let start = Instant::now();
    a.cap.activate(0, 0.5);
    until("segunda abertura", Duration::from_secs(1), || {
        log.todos("session.open").len() == 2
    })
    .await;
    while a.cap.released() == releases && start.elapsed() < Duration::from_millis(2400) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        a.cap.released() > releases,
        "captura física em opening precisa voltar em cerca de 2 s, sem esperar limpeza + envio"
    );
    assert!(closed(&log, "peer_unresponsive"));
    assert_eq!(
        network.blocked_enters.load(Ordering::SeqCst),
        1,
        "o segundo Enter realmente bloqueou depois da limpeza"
    );
    assert!(
        a.cap.begins().is_empty(),
        "ativação física, sem InputSwitch"
    );
}

async fn stats_queued_during_begin(
    receive_ms: u64,
    consume_ms: u64,
) -> (logs::Captura, Node, MemNode, Instant) {
    let log = logs::capturar();
    let hub = MemHub::new();
    let (a, gate, _network) = source_with_gates(&hub);
    let b = hub.add_node("b", INPUT);
    hub.set_remote_version(b.id, 1);
    hub.link(id("a"), b.id, CONTROL, CONTROL);
    until("barreira", Duration::from_secs(1), || {
        !a.cap.barriers().is_empty()
    })
    .await;
    let (reply, _rx) = oneshot::channel();
    // Mantém o solicitante vivo enquanto begin segura somente o proprietário da captura.
    a.tx.send(ModuleInput::Command {
        cmd: ModuleCommand::InputSwitch { to: b.id },
        reply,
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(1), gate.entered.notified())
        .await
        .unwrap();
    let start = Instant::now();
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        start + Duration::from_millis(receive_ms),
    ))
    .await;
    let session = log
        .primeiro("session.open")
        .unwrap()
        .campo("session")
        .unwrap()
        .parse()
        .unwrap();
    b.host
        .send_reliable(id("a"), ModuleMsg::Input(stats(session)))
        .await
        .unwrap();
    // Status confirma que o módulo recebeu Stats antes de liberar begin.
    assert!(matches!(
        command(&a, ModuleCommand::Status).await,
        ModuleReply::Status(_)
    ));
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        start + Duration::from_millis(consume_ms),
    ))
    .await;
    gate.resume.notify_one();
    until("begin concluiu", Duration::from_millis(300), || {
        log.tem("session.active")
    })
    .await;
    // Não deixar cair o rx de Switch cedo: um cancelamento seria outro motivo de fechamento.
    // Esquece apenas a espera, mantendo o receptor até a tarefa de teste terminar.
    tokio::spawn(async move {
        let _ = _rx.await;
    });
    (log, a, b, start)
}

#[tokio::test]
async fn stats_recebido_antes_do_limite_sobrevive_ao_consumo_depois_do_timer() {
    let (log, a, _b, start) = stats_queued_during_begin(1000, 2100).await;
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        start + Duration::from_millis(2400),
    ))
    .await;
    assert!(
        !log.tem("session.closed"),
        "Stats recebido em t=1 s protege até t=3 s mesmo consumido após t=2 s"
    );
    assert_eq!(a.cap.released(), 0);
}

#[tokio::test]
async fn stats_atrasado_na_fila_nao_compra_tempo_no_instante_do_consumo() {
    let (log, a, _b, start) = stats_queued_during_begin(1000, 1900).await;
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        start + Duration::from_millis(2700),
    ))
    .await;
    assert!(
        !log.tem("session.closed"),
        "a resposta deve renovar a sessão"
    );
    tokio::time::sleep_until(tokio::time::Instant::from_std(
        start + Duration::from_millis(3250),
    ))
    .await;
    assert!(
        closed(&log, "peer_unresponsive"),
        "Stats de t=1 s vence em t=3 s, não t=3,9 s"
    );
    assert!(a.cap.released() > 0);
}
