//! F1 do incidente de entrada de 11/11/2026: a primeira sessão depois de o emulador EIS nascer
//! fica sem teclado.
//!
//! O servidor de teste usa reis sobre UnixStream e verifica a ordem dos pedidos e as
//! transições de dispositivo. Não é o libeis nem prova de aplicação no GNOME real.
//! A referência 1.5.0 aceita teclas em EMULATING, ignora PAUSED e trata RESUMED como erro.
//! `cargo test -j2 -p clipsync-input --test eis_devices`
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "support/logs.rs"]
mod logs;

use clipsync_input::{EmulateAdapter, PortalEmulate, TokenStore};
use clipsync_proto::InputEvent;
use reis::PendingRequestResult;
use reis::eis;
use reis::handshake::EisHandshaker;
use reis::request::{Connection, Device, DeviceCapability, EisRequest, EisRequestConverter, Seat};
use std::future::{Future, poll_fn};
use std::os::unix::net::UnixStream;
use std::pin::pin;
use std::task::Poll;
use std::time::{Duration, Instant};

/// evdev KEY_A. A keycode lives only here, in the test; nothing writes it to a log.
const KEY_A: u32 = 30;

const SETTLE: u64 = 150;

fn key(pressed: bool) -> InputEvent {
    InputEvent::Key {
        code: KEY_A,
        pressed,
    }
}

/// Which devices the fake EIS creates when the client binds the seat capabilities.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Plan {
    /// Pointer and keyboard at once, as mutter does on a warm session.
    Both,
    /// Only the pointer; the test adds the keyboard later, as on the first session.
    PointerOnly,
    KeyboardOnly,
}

struct DevState {
    id: u64,
    dev: Device,
    keyboard: bool,
    /// What the EIS implementation believes: only EMULATING takes a key (libeis 1.5.0).
    emulating: bool,
    paused: bool,
}

/// Servidor de teste do protocolo EI: verifica start antes de tecla e detecta start
/// duplicado ou tecla em RESUMED. Não executa o estado interno do libeis nem o mutter.
struct FakeEis {
    ctx: eis::Context,
    hs: Option<EisHandshaker>,
    conv: Option<EisRequestConverter>,
    conn: Option<Connection>,
    seat: Option<Seat>,
    plan: Plan,
    devs: Vec<DevState>,
    /// Device number and key code of every key the server took as emulation.
    keys: Vec<(u64, u32)>,
    transitions: Vec<(u64, u32, bool)>,
    held: std::collections::HashSet<(u64, u32)>,
    /// Keys the server discarded because the device was not EMULATING.
    dropped: usize,
    /// Keys aimed at a device the server had already removed.
    stale: usize,
    /// Device numbers that asked for `start_emulating`.
    starts: Vec<u64>,
    motions: usize,
    next_id: u64,
}

impl FakeEis {
    fn new(stream: UnixStream, plan: Plan) -> Self {
        let ctx = eis::Context::new(stream).unwrap();
        let hs = EisHandshaker::new(&ctx, 1);
        Self {
            ctx,
            hs: Some(hs),
            conv: None,
            conn: None,
            seat: None,
            plan,
            devs: Vec::new(),
            keys: Vec::new(),
            transitions: Vec::new(),
            held: std::collections::HashSet::new(),
            dropped: 0,
            stale: 0,
            starts: Vec::new(),
            motions: 0,
            next_id: 0,
        }
    }

    fn pump(&mut self) {
        self.ctx.read().unwrap();
        while let Some(pending) = self.ctx.pending_request() {
            let request = match pending {
                PendingRequestResult::Request(request) => request,
                other => panic!("the client sent a broken request: {other:?}"),
            };
            if let Some(hs) = self.hs.as_mut() {
                let Some(resp) = hs.handle_request(request).unwrap() else {
                    continue;
                };
                let conv = EisRequestConverter::new(&self.ctx, resp, 1);
                let conn = conv.handle().clone();
                let seat = conn.add_seat(
                    Some("fake"),
                    DeviceCapability::Pointer
                        | DeviceCapability::Keyboard
                        | DeviceCapability::Button
                        | DeviceCapability::Scroll,
                );
                let _ = conn.flush();
                self.seat = Some(seat);
                self.conn = Some(conn);
                self.conv = Some(conv);
                self.hs = None;
            } else {
                let mut conv = self.conv.take().unwrap();
                conv.handle_request(request).unwrap();
                while let Some(high) = conv.next_request() {
                    self.handle(high);
                }
                self.conv = Some(conv);
                let _ = self.conn.as_ref().unwrap().flush();
            }
        }
    }

    fn handle(&mut self, request: EisRequest) {
        match request {
            EisRequest::Bind(bind) => {
                let seat = bind.seat.clone();
                if bind.capabilities.contains(DeviceCapability::Pointer)
                    && self.plan != Plan::KeyboardOnly
                {
                    let dev = seat.add_device(
                        Some("pointer"),
                        eis::device::DeviceType::Virtual,
                        DeviceCapability::Pointer
                            | DeviceCapability::Button
                            | DeviceCapability::Scroll,
                        |_| {},
                    );
                    dev.resumed();
                    self.push(dev, false);
                }
                if bind.capabilities.contains(DeviceCapability::Keyboard)
                    && self.plan != Plan::PointerOnly
                {
                    self.add_keyboard(&seat);
                }
                let _ = self.conn.as_ref().unwrap().flush();
            }
            EisRequest::DeviceStartEmulating(start) => {
                if let Some(id) = self.id_of(&start.device) {
                    if let Some(dev) = self.devs.iter_mut().find(|d| d.id == id) {
                        assert!(
                            !dev.emulating,
                            "start_emulating duplicado derrubaria a conexão no libeis"
                        );
                        if dev.paused {
                            return;
                        }
                        dev.emulating = true;
                    }
                    self.starts.push(id);
                } else {
                    self.stale += 1;
                }
            }
            EisRequest::DeviceStopEmulating(stop) => {
                if let Some(id) = self.id_of(&stop.device)
                    && let Some(dev) = self.devs.iter_mut().find(|d| d.id == id)
                {
                    dev.emulating = false;
                }
            }
            EisRequest::KeyboardKey(k) => match self.id_of(&k.device) {
                None => self.stale += 1,
                Some(id) => {
                    if self.devs.iter().any(|d| d.id == id && d.emulating) {
                        self.keys.push((id, k.key));
                        let pressed = k.state == eis::keyboard::KeyState::Press;
                        self.transitions.push((id, k.key, pressed));
                        if pressed {
                            self.held.insert((id, k.key));
                        } else {
                            self.held.remove(&(id, k.key));
                        }
                    } else if self.devs.iter().any(|d| d.id == id && d.paused) {
                        self.dropped += 1;
                    } else {
                        panic!("tecla em RESUMED seria erro de protocolo no libeis 1.5.0");
                    }
                }
            },
            EisRequest::PointerMotion(_) => self.motions += 1,
            _ => {}
        }
    }

    fn push(&mut self, dev: Device, keyboard: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.devs.push(DevState {
            id,
            dev,
            keyboard,
            emulating: false,
            paused: false,
        });
        id
    }

    fn add_keyboard(&mut self, seat: &Seat) {
        let dev = seat.add_device(
            Some("keyboard"),
            eis::device::DeviceType::Virtual,
            DeviceCapability::Keyboard.into(),
            |_| {},
        );
        dev.resumed();
        self.push(dev, true);
        let _ = self.conn.as_ref().unwrap().flush();
    }

    /// The test adds the keyboard on its own, after the pointer is already up.
    fn add_keyboard_now(&mut self) {
        let seat = self.seat.clone().unwrap();
        self.add_keyboard(&seat);
    }

    /// mutter on keymap-changed: the virtual keyboard is removed and a new one takes its place.
    fn remove_keyboard(&mut self) {
        let Some(pos) = self.devs.iter().position(|d| d.keyboard) else {
            return;
        };
        let dev = self.devs.remove(pos);
        self.held.retain(|(id, _)| *id != dev.id);
        dev.dev.remove();
        let _ = self.conn.as_ref().unwrap().flush();
    }

    /// Pausa no fio EI. O modelo conserva held de propósito para testar reconciliação
    /// conservadora, sem pressupor neutralização remota. Não simula o GNOME físico.
    fn pause_keyboard(&mut self) {
        let d = self.devs.iter_mut().find(|d| d.keyboard).unwrap();
        d.dev.paused();
        d.emulating = false;
        d.paused = true;
        self.conn.as_ref().unwrap().flush().unwrap();
    }

    fn resume_keyboard(&mut self) {
        let d = self.devs.iter_mut().find(|d| d.keyboard).unwrap();
        d.dev.resumed();
        d.paused = false;
        self.conn.as_ref().unwrap().flush().unwrap();
    }

    fn id_of(&self, dev: &Device) -> Option<u64> {
        self.devs.iter().find(|d| &d.dev == dev).map(|d| d.id)
    }

    fn keyboard_id(&self) -> Option<u64> {
        self.devs.iter().find(|d| d.keyboard).map(|d| d.id)
    }

    fn has_pointer(&self) -> bool {
        self.devs.iter().any(|d| !d.keyboard)
    }

    fn pump_until(&mut self, what: &str, pred: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            self.pump();
            if pred(self) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Pumps for `ms`, so the client can take what the server just sent.
    fn settle(&mut self, ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < deadline {
            self.pump();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

/// Polls `fut` while pumping the fake server: both sides live on this one thread.
async fn drive<T>(srv: &mut FakeEis, fut: impl Future<Output = T>) -> T {
    let mut fut = pin!(fut);
    loop {
        srv.pump();
        if let Poll::Ready(out) = poll_fn(|cx| Poll::Ready(fut.as_mut().poll(cx))).await {
            return out;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// Teclado tardio ou recriado recebe start antes das teclas, sem impedir a entrada do mouse.
#[test]
fn teclado_tardio_e_recriado_recebe_start_antes_das_teclas() {
    let (client, server) = UnixStream::pair().unwrap();
    let rt = runtime();
    rt.block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::PointerOnly);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        srv.add_keyboard_now();
        srv.settle(SETTLE);

        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("the first key on the keyboard", |s| s.keys.len() == 2);
        assert_eq!(srv.dropped, 0);
        assert_eq!(srv.stale, 0);
        let first = srv.keyboard_id().expect("keyboard");
        assert!(srv.starts.contains(&first), "the key needs start_emulating");

        // mutter on keymap-changed: remove and recreate the keyboard.
        srv.remove_keyboard();
        srv.settle(SETTLE);
        srv.add_keyboard_now();
        srv.settle(SETTLE);
        let second = srv.keyboard_id().expect("new keyboard");
        assert_ne!(first, second);

        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("the key on the recreated keyboard", |s| s.keys.len() == 4);
        assert_eq!(srv.keys[2].0, second);
        assert_eq!(srv.keys[3].0, second);
        assert!(
            srv.starts.contains(&second),
            "the recreated keyboard must ask to emulate again"
        );
        assert_eq!(srv.dropped, 0, "libeis drops a key outside EMULATING");
        assert_eq!(srv.stale, 0, "no key may go to a removed device");

        emu.stop().await.unwrap();
    });
}

/// Guarda de protocolo, não reprodução de bug: pause/resume já limpavam a flag.
/// Um resumed repetido sem pausa não pode causar outro start enquanto EMULATING.
#[test]
fn pausa_retoma_e_resumed_repetido_nao_duplicam_start() {
    let log = logs::capturar();
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::Both);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("primeiro par", |s| s.keys.len() == 2);
        let keyboard = srv.keyboard_id().unwrap();
        assert_eq!(srv.starts.iter().filter(|id| **id == keyboard).count(), 1);
        srv.pause_keyboard();
        srv.settle(SETTLE);
        assert!(
            emu.emit(&[key(true)]).await.is_ok(),
            "tecla pausada é descartada sem erro para o módulo"
        );
        srv.resume_keyboard();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("par após retomada", |s| s.keys.len() == 4);
        assert_eq!(srv.starts.iter().filter(|id| **id == keyboard).count(), 2);
        // Exercício defensivo do cliente: reenvio da notificação sem mudar o estado EIS.
        srv.resume_keyboard();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("par após resumed repetido", |s| s.keys.len() == 6);
        assert_eq!(srv.starts.iter().filter(|id| **id == keyboard).count(), 2);
        assert_eq!(srv.dropped, 0);
        // Novo episódio de indisponibilidade: um aviso por episódio, não por evento.
        srv.pause_keyboard();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.settle(30);
        assert_eq!(
            srv.keys.len(),
            6,
            "nenhuma tecla descartada é enviada ou reproduzida"
        );
        emu.stop().await.unwrap();
        assert_eq!(
            log.primeiro("eis.loop").unwrap().campo("descartados"),
            Some("3")
        );
        assert_eq!(
            log.todos("eis.device")
                .iter()
                .filter(|e| e.campo("acao") == Some("discarding"))
                .count(),
            2
        );
        assert!(
            log.todos("eis.device")
                .iter()
                .any(|e| e.campo("acao") == Some("removed")
                    && e.campo("capacidades") == Some("keyboard")
                    && e.campo("descartados") == Some("3"))
        );
    });
}

/// RED da regressão deixada por devices_ready: ponteiro não depende de teclado permanente.
/// Verifica também que uma tecla sem dispositivo não perde frames válidos do ponteiro.
#[test]
fn ponteiro_sem_teclado_inicia_sem_espera_e_preserva_o_lote_de_mouse() {
    let log = logs::capturar();
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::PointerOnly);
        let start = Instant::now();
        let mut emu = drive(
            &mut srv,
            tokio::time::timeout(
                Duration::from_millis(250),
                PortalEmulate::attach(TokenStore::new(dir.path()), client),
            ),
        )
        .await
        .expect("start deve retornar no tempo do ponteiro, sem esperar teclado")
        .unwrap();
        assert!(srv.has_pointer(), "o ponteiro obrigatório foi anunciado");
        assert!(
            start.elapsed() < Duration::from_millis(250),
            "não há espera pelo teclado no caminho de entrada"
        );
        assert!(
            log.todos("eis.device")
                .iter()
                .any(|e| e.campo("acao") == Some("missing")
                    && e.campo("capacidades") == Some("keyboard")),
            "ausência inicial registrada sem derrubar o mouse"
        );
        let result = emu
            .emit(&[
                InputEvent::Motion { dx: 1.0, dy: 0.0 },
                key(true),
                InputEvent::Motion { dx: 2.0, dy: 0.0 },
            ])
            .await;
        assert!(
            result.is_ok(),
            "teclado ausente não deve derrubar a sessão de mouse"
        );
        srv.pump_until("duas motions com frame mesmo sem teclado", |s| {
            s.motions == 2
        });
        srv.add_keyboard_now();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("teclado tardio funciona sem recriar o emulador", |s| {
            s.keys.len() == 2
        });
        emu.stop().await.unwrap();
    });
}

#[test]
fn teclado_sozinho_nao_substitui_o_ponteiro_obrigatorio() {
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::KeyboardOnly);
        let result = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await;
        assert!(
            matches!(result, Err(clipsync_input::InputError::Eis(_))),
            "sem ponteiro start precisa falhar"
        );
    });
}

/// A saída precisa distinguir o fim do laço e contar também o teclado já removido.
#[test]
fn log_conta_dispositivos_removidos_e_explica_o_fim_do_laco() {
    let log = logs::capturar();
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::Both);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("primeiro teclado", |s| s.keys.len() == 2);
        srv.remove_keyboard();
        srv.settle(SETTLE);
        srv.add_keyboard_now();
        srv.settle(SETTLE);
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("segundo teclado", |s| s.keys.len() == 4);
        emu.stop().await.unwrap();
    });
    let end = log
        .primeiro("eis.loop")
        .expect("fim do laço registrado na thread EIS");
    assert_eq!(end.campo("motivo"), Some("closed"));
    assert_eq!(
        end.campo("contagem"),
        Some("4"),
        "contagem não perde o teclado removido"
    );
    let removed = log
        .todos("eis.device")
        .into_iter()
        .filter(|e| e.campo("acao") == Some("removed"))
        .collect::<Vec<_>>();
    assert_eq!(
        removed.len(),
        3,
        "ponteiro e os dois teclados têm fechamento"
    );
    assert_eq!(
        removed
            .iter()
            .filter(|e| e.campo("capacidades") == Some("keyboard"))
            .count(),
        2
    );
    assert!(
        removed
            .iter()
            .filter(|e| e.campo("capacidades") == Some("keyboard"))
            .all(|e| e.campo("contagem") == Some("2"))
    );
    assert!(log.tem("eis.device"));
}

/// R1-E01: erro do conversor encerra o laço com categoria fixa, sem payload remoto.
#[test]
fn erro_do_conversor_tem_categoria_segura_no_log() {
    let log = logs::capturar();
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::Both);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        // done duplicado é um erro real detectado pelo conversor reis (setup após done).
        srv.devs[0].dev.device().done();
        srv.conn.as_ref().unwrap().flush().unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        while emu.alive() {
            assert!(
                Instant::now() < end,
                "laço deveria encerrar por erro de conversão"
            );
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        emu.stop().await.unwrap();
    });
    let end = log.primeiro("eis.loop").expect("motivo do fim do laço");
    assert_eq!(end.campo("motivo"), Some("converter"));
    assert_eq!(
        end.campo("erro"),
        Some("event"),
        "categoria fixa, sem texto do evento recebido"
    );
}

#[test]
fn fim_remoto_distingue_eof_de_desconexao_anunciada() {
    for announced in [false, true] {
        let log = logs::capturar();
        let (client, server) = UnixStream::pair().unwrap();
        runtime().block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let mut srv = FakeEis::new(server, Plan::Both);
            let mut emu = drive(
                &mut srv,
                PortalEmulate::attach(TokenStore::new(dir.path()), client),
            )
            .await
            .unwrap();
            srv.settle(30);
            if announced {
                srv.conn
                    .as_ref()
                    .unwrap()
                    .disconnected(reis::ei::connection::DisconnectReason::Disconnected, None);
            }
            drop(srv);
            let end = Instant::now() + Duration::from_secs(1);
            while emu.alive() {
                assert!(Instant::now() < end, "fim remoto precisa encerrar o laço");
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            emu.stop().await.unwrap();
        });
        let end = log.primeiro("eis.loop").expect("fim remoto registrado");
        assert_eq!(
            end.campo("motivo"),
            Some(if announced { "disconnected" } else { "eof" })
        );
        for event in log
            .todos("eis.device")
            .into_iter()
            .chain(log.todos("eis.loop"))
        {
            assert!(
                event.campos.keys().all(|k| matches!(
                    k.as_str(),
                    "message"
                        | "acao"
                        | "origem"
                        | "dispositivo"
                        | "capacidades"
                        | "contagem"
                        | "descartados"
                        | "motivo"
                        | "erro"
                        | "dispositivos"
                )),
                "somente campos de diagnóstico definidos, sem tecla ou keycode"
            );
        }
    }
}

const KEY_MODIFIER: u32 = 42;
fn modifier(pressed: bool) -> InputEvent {
    InputEvent::Key {
        code: KEY_MODIFIER,
        pressed,
    }
}

fn reconcile_transition(recreated: bool) {
    let log = logs::capturar();
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::Both);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        srv.settle(SETTLE);
        emu.emit(&[modifier(true)]).await.unwrap();
        srv.pump_until("modificador pressionado", |s| s.keys.len() == 1);
        let old = srv.keyboard_id().unwrap();
        assert!(srv.held.contains(&(old, KEY_MODIFIER)));
        srv.pause_keyboard();
        srv.settle(SETTLE);
        emu.emit(&[modifier(false)]).await.unwrap();
        srv.settle(30);
        assert_eq!(
            srv.keys.len(),
            1,
            "release indisponível ainda não foi enviado"
        );
        assert!(
            srv.held.contains(&(old, KEY_MODIFIER)),
            "servidor conservador ainda segura modificador"
        );
        if recreated {
            srv.remove_keyboard();
            srv.settle(SETTLE);
            srv.add_keyboard_now();
        } else {
            srv.resume_keyboard();
        }
        srv.settle(SETTLE);
        let current = srv.keyboard_id().unwrap();
        emu.emit(&[key(true), key(false)]).await.unwrap();
        srv.pump_until("novas teclas após retomada", |s| s.keys.len() >= 3);
        assert!(
            srv.transitions
                .get(1)
                .is_some_and(|(id, code, down)| *id == current && *code == KEY_MODIFIER && !*down),
            "release pendente precisa anteceder qualquer tecla nova"
        );
        assert!(
            !srv.held.contains(&(current, KEY_MODIFIER)),
            "modificador não pode continuar preso"
        );
        assert!(
            srv.starts.contains(&current),
            "start vem antes de release reconciliado"
        );
        assert_eq!(
            srv.transitions
                .iter()
                .filter(|(_, code, down)| *code == KEY_MODIFIER && *down)
                .count(),
            1,
            "nenhum press descartado é reproduzido"
        );
        emu.release_all().await.unwrap();
        srv.settle(30);
        assert_eq!(
            srv.keys.len(),
            4,
            "release_all não duplica uma soltura já reconciliada"
        );
        emu.stop().await.unwrap();
    });
    assert!(
        log.todos("eis.device")
            .iter()
            .any(|e| e.campo("acao") == Some("reconciled") && e.campo("contagem") == Some("1")),
        "só a quantidade de solturas reconciliadas entra no log"
    );
}

#[test]
fn release_de_modificador_descartado_e_reconciliado_antes_da_proxima_tecla() {
    reconcile_transition(false);
}

#[test]
fn release_pendente_tambem_e_reconciliado_no_teclado_recriado() {
    reconcile_transition(true);
}

#[test]
fn press_descartado_e_release_aplicado_nao_criam_tecla_presa() {
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::PointerOnly);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        emu.emit(&[modifier(true)]).await.unwrap();
        srv.settle(30);
        assert!(srv.keys.is_empty());
        srv.add_keyboard_now();
        srv.settle(SETTLE);
        emu.emit(&[modifier(false)]).await.unwrap();
        srv.pump_until("release sem press anterior", |s| s.keys.len() == 1);
        assert!(srv.held.is_empty());
        assert!(srv.transitions.iter().all(|(_, _, down)| !*down));
        emu.release_all().await.unwrap();
        srv.settle(30);
        assert_eq!(
            srv.keys.len(),
            1,
            "estado local não inventa soltura adicional"
        );
        emu.stop().await.unwrap();
    });
}

fn cleanup_while_keyboard_paused(release_was_dropped: bool) {
    let (client, server) = UnixStream::pair().unwrap();
    runtime().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut srv = FakeEis::new(server, Plan::Both);
        let mut emu = drive(
            &mut srv,
            PortalEmulate::attach(TokenStore::new(dir.path()), client),
        )
        .await
        .unwrap();
        srv.settle(SETTLE);
        emu.emit(&[modifier(true)]).await.unwrap();
        srv.pump_until("press anterior ao cleanup", |s| s.keys.len() == 1);
        let keyboard = srv.keyboard_id().unwrap();
        srv.pause_keyboard();
        srv.settle(SETTLE);
        if release_was_dropped {
            emu.emit(&[modifier(false)]).await.unwrap();
        }
        // Mesmo comando que o módulo chama no Leave/pausa. Repetir não aposenta a fila.
        emu.release_all().await.unwrap();
        emu.release_all().await.unwrap();
        srv.settle(30);
        assert_eq!(srv.keys.len(), 1, "nada é enviado para o teclado pausado");
        assert!(srv.held.contains(&(keyboard, KEY_MODIFIER)));
        srv.resume_keyboard();
        // Não envia tecla nova: a retomada sozinha deve neutralizar a sessão já encerrada.
        srv.pump_until("soltura adiada por release_all durante pausa", |s| {
            !s.held.contains(&(keyboard, KEY_MODIFIER))
        });
        assert_eq!(srv.keys.len(), 2, "uma única soltura após cleanup repetido");
        assert!(
            srv.transitions
                .get(1)
                .is_some_and(|(id, code, down)| *id == keyboard && *code == KEY_MODIFIER && !*down)
        );
        // Novo press do MESMO código não pode ser atingido por uma soltura antiga tardia.
        emu.emit(&[modifier(true)]).await.unwrap();
        srv.pump_until("novo press do mesmo modificador", |s| s.keys.len() == 3);
        assert!(srv.held.contains(&(keyboard, KEY_MODIFIER)));
        emu.release_all().await.unwrap();
        srv.pump_until("cleanup com teclado pronto", |s| s.keys.len() == 4);
        assert!(srv.held.is_empty());
        emu.stop().await.unwrap();
    });
}

#[test]
fn release_all_conserva_press_como_soltura_pendente_enquanto_teclado_pausado() {
    cleanup_while_keyboard_paused(false);
}

#[test]
fn release_all_conserva_soltura_ja_pendente_enquanto_teclado_pausado() {
    cleanup_while_keyboard_paused(true);
}
