//! B07: ausência de snapshot mantém loopback fechado.
#![allow(clippy::unwrap_used, clippy::panic)]
use clipsync_input::{
    BarrierSpec, CaptureAdapter, CaptureEvent, FakeEmulate, InputConfig, InputError, Zone,
    build_with_adapters,
};
use clipsync_proto::*;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
struct MissingSnapshot;
fn zone() -> Zone {
    Zone {
        index: 0,
        name: Some("m".into()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}
#[async_trait::async_trait]
impl CaptureAdapter for MissingSnapshot {
    fn name(&self) -> &'static str {
        "missing-snapshot"
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        Ok(vec![zone()])
    }
    fn known_zones(&self) -> Vec<Zone> {
        vec![zone()]
    }
    async fn start(
        &mut self,
        _: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        let (tx, rx) = mpsc::channel(8);
        tx.try_send(CaptureEvent::Activated {
            barrier: 0,
            pos: 0.5,
            t_mono_us: 0,
        })
        .unwrap();
        tokio::spawn(async move {
            tx.closed().await;
        });
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        Ok(())
    }
}
async fn command(tx: &mpsc::Sender<ModuleInput>, cmd: ModuleCommand) -> ModuleReply {
    let (reply, rx) = oneshot::channel();
    tx.send(ModuleInput::Command { cmd, reply }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), rx)
        .await
        .unwrap()
        .unwrap()
}
#[tokio::test]
async fn loopback_none_without_initial_snapshot_never_sends_enter() {
    let hub = MemHub::new();
    let input = ModuleSet {
        clipboard: false,
        input: true,
    };
    let source = hub.add_node("source", input);
    let mut dest = hub.add_node("dest", input);
    hub.link(
        source.id,
        dest.id,
        PeerPerms::default(),
        PeerPerms {
            clipboard_in: false,
            clipboard_out: false,
            input_in: true,
        },
    );
    let (emu, _) = FakeEmulate::new();
    let module = build_with_adapters(
        InputConfig::default(),
        Box::new(MissingSnapshot),
        Box::new(emu),
    );
    let task = tokio::spawn(module.run(source.host, source.rx));
    let _ = command(&source.tx, ModuleCommand::Status).await;
    let pending = {
        let tx = source.tx.clone();
        let to = dest.id;
        tokio::spawn(
            async move { command(&tx, ModuleCommand::InputLoopback { to, seconds: 1 }).await },
        )
    };
    let mut entered = false;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
    while let Ok(Some(event)) = tokio::time::timeout_at(deadline, dest.rx.recv()).await {
        if matches!(
            event,
            ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::Enter { .. }),
                ..
            }
        ) {
            entered = true;
            break;
        }
    }
    pending.abort();
    task.abort();
    assert!(!entered, "None sem snapshot inicial abriu a sessão");
}

fn unlocked(generation: u64) -> clipsync_input::adapter::CaptureLifecycle {
    clipsync_input::adapter::CaptureLifecycle {
        revision: generation + 1,
        invalidation_generation: generation,
        valid: true,
        session_locked: Some(false),
        desktop_available: Some(true),
        zones: vec![zone()],
        terminal_error: None,
    }
}
#[tokio::test]
async fn loopback_reads_valid_watch_immediately_before_first_activation() {
    let hub = MemHub::new();
    let input = ModuleSet {
        clipboard: false,
        input: true,
    };
    let source = hub.add_node("source", input);
    let mut dest = hub.add_node("dest", input);
    hub.link(
        source.id,
        dest.id,
        PeerPerms::default(),
        PeerPerms {
            clipboard_in: false,
            clipboard_out: false,
            input_in: true,
        },
    );
    let (cap, handle) = clipsync_input::FakeCapture::new(vec![zone()]);
    handle.enable_lifecycle(unlocked(5));
    let (emu, _) = FakeEmulate::new();
    let module = build_with_adapters(InputConfig::default(), Box::new(cap), Box::new(emu));
    let task = tokio::spawn(module.run(source.host, source.rx));
    let _ = command(&source.tx, ModuleCommand::Status).await;
    let pending = {
        let tx = source.tx.clone();
        let to = dest.id;
        tokio::spawn(
            async move { command(&tx, ModuleCommand::InputLoopback { to, seconds: 1 }).await },
        )
    };
    tokio::time::timeout(Duration::from_secs(1), async {
        while handle.starts() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    handle.activate(0, 0.5);
    let entered = tokio::time::timeout(Duration::from_millis(200), async {
        while let Some(event) = dest.rx.recv().await {
            if matches!(
                event,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::Enter { .. }),
                    ..
                }
            ) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    pending.abort();
    task.abort();
    assert!(entered, "snapshot watch válido não foi lido imediatamente");
}

use clipsync_input::adapter::CaptureLifecycle;
use clipsync_input::{Endpoint, Layout, LayoutDevice, LayoutOptions, Link, build_with_layout};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Notify, watch};

struct Probe {
    watch: watch::Sender<CaptureLifecycle>,
    lifecycle_enabled: bool,
    tx: Mutex<Option<mpsc::Sender<CaptureEvent>>>,
    starts: AtomicUsize,
    stops: AtomicUsize,
    prefill: bool,
    block_stop: AtomicBool,
    stopping: Notify,
    finish_stop: Notify,
    barriers: Mutex<Vec<Vec<BarrierSpec>>>,
}
impl Probe {
    fn new(prefill: bool, snapshot: Option<CaptureLifecycle>) -> Arc<Self> {
        Arc::new(Self {
            lifecycle_enabled: snapshot.is_some(),
            watch: watch::channel(snapshot.unwrap_or_default()).0,
            tx: Mutex::new(None),
            starts: AtomicUsize::new(0),
            stops: AtomicUsize::new(0),
            prefill,
            block_stop: AtomicBool::new(false),
            stopping: Notify::new(),
            finish_stop: Notify::new(),
            barriers: Mutex::new(vec![]),
        })
    }
    fn sender(&self) -> mpsc::Sender<CaptureEvent> {
        self.tx.lock().unwrap().as_ref().unwrap().clone()
    }
    fn invalidate(&self) {
        self.watch.send_modify(|s| {
            s.session_locked = Some(true);
            s.invalidation_generation += 1;
            s.revision += 1;
        });
        self.watch.send_modify(|s| {
            s.session_locked = Some(false);
            s.revision += 1;
        });
    }
}
struct ProbeCapture(Arc<Probe>);
#[async_trait::async_trait]
impl CaptureAdapter for ProbeCapture {
    fn name(&self) -> &'static str {
        "lifecycle-probe"
    }
    fn lifecycle_receiver(&self) -> Option<watch::Receiver<CaptureLifecycle>> {
        self.0.lifecycle_enabled.then(|| self.0.watch.subscribe())
    }
    fn known_zones(&self) -> Vec<Zone> {
        vec![zone()]
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        Ok(vec![zone()])
    }
    async fn start(
        &mut self,
        specs: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        let (tx, rx) = mpsc::channel(4096);
        let first = self.0.starts.load(Ordering::SeqCst) == 0;
        self.0.barriers.lock().unwrap().push(specs.to_vec());
        if first && self.0.prefill {
            tx.try_send(activation()).unwrap();
            for _ in 1..4096 {
                tx.try_send(input()).unwrap();
            }
            assert_eq!(tx.capacity(), 0, "pré-condição: saída4096 realmente cheia");
            self.0.invalidate(); // lock+unlock inteiros antes de devolver o rx
        }
        *self.0.tx.lock().unwrap() = Some(tx);
        self.0.starts.fetch_add(1, Ordering::SeqCst);
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        if self.0.block_stop.load(Ordering::SeqCst) {
            self.0.stopping.notify_one();
            self.0.finish_stop.notified().await;
        }
        self.0.stops.fetch_add(1, Ordering::SeqCst);
        *self.0.tx.lock().unwrap() = None;
        Ok(())
    }
    async fn set_barriers(&mut self, _: &[BarrierSpec]) -> Result<(), InputError> {
        Ok(())
    }
    fn holds_hooks(&self) -> bool {
        true
    }
}
fn activation() -> CaptureEvent {
    CaptureEvent::Activated {
        barrier: 0,
        pos: 0.5,
        t_mono_us: 0,
    }
}
fn input() -> CaptureEvent {
    CaptureEvent::Input {
        t_mono_us: 1,
        ev: InputEvent::Motion { dx: 9.0, dy: 0.0 },
    }
}
async fn wait_for(cond: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !cond() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
struct Rig {
    hub: MemHub,
    source: PeerId,
    dest: PeerId,
    tx: mpsc::Sender<ModuleInput>,
    wire: mpsc::Receiver<ModuleInput>,
    probe: Arc<Probe>,
    host: Arc<EnterGate>,
    task: tokio::task::JoinHandle<Result<(), ModuleError>>,
    _dir: tempfile::TempDir,
}
impl Rig {
    async fn new(continuous: bool, prefill: bool, paused: bool) -> Self {
        Self::with_snapshot(continuous, prefill, paused, Some(unlocked(0))).await
    }
    async fn with_snapshot(
        continuous: bool,
        prefill: bool,
        paused: bool,
        snapshot: Option<CaptureLifecycle>,
    ) -> Self {
        let hub = MemHub::new();
        let input = ModuleSet {
            clipboard: false,
            input: true,
        };
        let a = hub.add_node("watch-a", input);
        let b = hub.add_node("watch-b", input);
        hub.link(
            a.id,
            b.id,
            PeerPerms::default(),
            PeerPerms {
                input_in: true,
                ..Default::default()
            },
        );
        if paused {
            hub.set_paused(a.id, true);
        }
        let dir = tempfile::tempdir().unwrap();
        let probe = Probe::new(prefill, snapshot);
        let (emu, _) = FakeEmulate::new();
        let module = if continuous {
            let dev = |peer| LayoutDevice {
                peer,
                name: "m".into(),
                x: 0,
                y: 0,
                hotkey: None,
            };
            let ep = |peer, edge| Endpoint {
                peer,
                monitor: "m".into(),
                edge,
                from: 0.0,
                to: 1.0,
            };
            Layout {
                rev: 1,
                options: LayoutOptions { dead_corner_px: 0 },
                devices: vec![dev(a.id), dev(b.id)],
                links: vec![Link {
                    id: "l".into(),
                    a: ep(a.id, Edge::Right),
                    b: ep(b.id, Edge::Left),
                }],
            }
            .write(&dir.path().join("layout.toml"))
            .unwrap();
            build_with_layout(
                InputConfig::default(),
                Box::new(ProbeCapture(probe.clone())),
                Box::new(emu),
                dir.path(),
            )
            .unwrap()
        } else {
            build_with_adapters(
                InputConfig::default(),
                Box::new(ProbeCapture(probe.clone())),
                Box::new(emu),
            )
        };
        let host = Arc::new(EnterGate {
            inner: a.host,
            block: AtomicBool::new(false),
            inflight: AtomicUsize::new(0),
            complete: Notify::new(),
        });
        let task = tokio::spawn(module.run(host.clone(), a.rx));
        let r = Self {
            hub,
            source: a.id,
            dest: b.id,
            tx: a.tx,
            wire: b.rx,
            probe,
            host,
            task,
            _dir: dir,
        };
        let _ = command(&r.tx, ModuleCommand::Status).await;
        r
    }
    fn loopback(&self) -> tokio::task::JoinHandle<ModuleReply> {
        let tx = self.tx.clone();
        let to = self.dest;
        tokio::spawn(
            async move { command(&tx, ModuleCommand::InputLoopback { to, seconds: 2 }).await },
        )
    }
    async fn next_enter(&mut self) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(event) = self.wire.recv().await {
                if is_enter(&event) {
                    return;
                }
            }
            panic!("wire encerrado");
        })
        .await
        .unwrap();
    }
    async fn no_old_data(&mut self) {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(80);
        while let Ok(Some(event)) = tokio::time::timeout_at(deadline, self.wire.recv()).await {
            assert!(!is_enter(&event), "Activated velho atravessou a geração");
            assert!(
                !matches!(event, ModuleInput::Datagram { .. }),
                "Input velho atravessou a geração"
            );
        }
    }
    async fn finish(self) {
        self.probe.block_stop.store(false, Ordering::SeqCst);
        self.probe.finish_stop.notify_one();
        self.tx.send(ModuleInput::Shutdown).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
fn is_enter(event: &ModuleInput) -> bool {
    matches!(
        event,
        ModuleInput::Reliable {
            msg: ModuleMsg::Input(
                InputMsg::Enter { .. } | InputMsg::EnterAt { .. } | InputMsg::EnterSpan { .. }
            ),
            ..
        }
    )
}

#[tokio::test]
async fn continuous_discards_prefilled_4096_after_coalesced_lock_unlock_at_start() {
    let mut r = Rig::new(true, true, false).await;
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 2).await;
    assert!(r.probe.stops.load(Ordering::SeqCst) >= 1);
    r.no_old_data().await;
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await; // só uma ativação da geração nova pode abrir
    r.finish().await;
}

#[tokio::test]
async fn loopback_discards_prefilled_4096_after_coalesced_lock_unlock_at_start() {
    let mut r = Rig::new(false, true, false).await;
    let job = r.loopback();
    assert!(matches!(job.await.unwrap(), ModuleReply::Error(_)));
    assert_eq!(r.probe.starts.load(Ordering::SeqCst), 1);
    assert!(r.probe.stops.load(Ordering::SeqCst) >= 1);
    r.no_old_data().await;
    r.finish().await;
}

#[tokio::test]
async fn paused_start_installs_no_capture_and_resume_reads_fresh_snapshot() {
    let mut r = Rig::new(true, false, true).await;
    assert_eq!(r.probe.starts.load(Ordering::SeqCst), 0);
    r.probe.invalidate();
    let _ = command(&r.tx, ModuleCommand::Status).await;
    assert_eq!(r.probe.starts.load(Ordering::SeqCst), 0);
    r.hub.set_paused(r.source, false);
    r.tx.send(ModuleInput::Paused(false)).await.unwrap();
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) > 0).await;
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    r.finish().await;
}

#[tokio::test]
async fn control_during_watch_cleanup_prevents_rearm_after_unlock() {
    for case in 0..4 {
        let r = Rig::new(true, false, false).await;
        wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
        r.probe.block_stop.store(true, Ordering::SeqCst);
        r.probe.invalidate();
        tokio::time::timeout(Duration::from_secs(2), r.probe.stopping.notified())
            .await
            .unwrap();
        let old = r.probe.sender();
        assert!(old.is_closed(), "fechar rx deve preceder await stop");
        match case {
            0 => r.tx.send(ModuleInput::Shutdown).await.unwrap(),
            1 => {
                r.tx.send(ModuleInput::Paused(true)).await.unwrap();
                // O módulo aguarda o join ao pausar hooks. Deixe-o encaminhar o
                // controle sem aguardar Status antes de liberar o gate do join.
                tokio::task::yield_now().await;
            }
            2 => {
                let _ = command(&r.tx, ModuleCommand::InputLock { locked: true }).await;
            }
            _ => r.hub.set_paused(r.source, true),
        }
        r.probe.block_stop.store(false, Ordering::SeqCst);
        r.probe.finish_stop.notify_one();
        wait_for(|| r.probe.stops.load(Ordering::SeqCst) > 0).await;
        if case == 0 {
            tokio::time::timeout(Duration::from_secs(3), r.task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(r.probe.starts.load(Ordering::SeqCst), 1);
        } else {
            let _ = command(&r.tx, ModuleCommand::Status).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
            assert_eq!(
                r.probe.starts.load(Ordering::SeqCst),
                1,
                "rearmou durante controle {case}"
            );
            r.finish().await;
        }
    }
}

// Sem o gate inicial do consumidor, Activated atravessa antes de qualquer snapshot.
#[tokio::test]
async fn continuous_none_requires_explicit_initial_unlock_snapshot() {
    let mut r = Rig::with_snapshot(true, false, false, None).await;
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
    r.probe.sender().send(activation()).await.unwrap();
    r.no_old_data().await;
    r.probe
        .sender()
        .send(CaptureEvent::SessionLocked(false))
        .await
        .unwrap();
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    r.finish().await;
}

#[tokio::test]
async fn invalid_watch_never_admits_activation_in_either_consumer() {
    for continuous in [false, true] {
        let mut r =
            Rig::with_snapshot(continuous, false, false, Some(CaptureLifecycle::default())).await;
        if continuous {
            wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
            r.probe.sender().send(activation()).await.unwrap();
        } else {
            assert!(matches!(r.loopback().await.unwrap(), ModuleReply::Error(_)));
        }
        r.no_old_data().await;
        r.finish().await;
    }
}

fn fill_old_generation(probe: &Probe) -> mpsc::Sender<CaptureEvent> {
    let tx = probe.sender();
    tx.try_send(activation()).unwrap();
    for _ in 1..4096 {
        tx.try_send(input()).unwrap();
    }
    assert_eq!(tx.capacity(), 0, "saída antiga deve estar realmente cheia");
    probe.invalidate();
    tx
}

// Remover a comparação de geração permitiria replay após lock+unlock coalescidos.
#[tokio::test]
async fn continuous_active_session_discards_full_old_generation_before_reopening() {
    let mut r = Rig::new(true, false, false).await;
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    let old = fill_old_generation(&r.probe);
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 2).await;
    assert!(old.is_closed());
    r.no_old_data().await;
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    r.finish().await;
}

#[tokio::test]
async fn loopback_active_session_discards_full_old_generation_before_returning() {
    let mut r = Rig::new(false, false, false).await;
    let job = r.loopback();
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    let old = fill_old_generation(&r.probe);
    assert!(matches!(job.await.unwrap(), ModuleReply::Error(_)));
    assert!(old.is_closed());
    assert_eq!(r.probe.starts.load(Ordering::SeqCst), 1);
    r.no_old_data().await;
    r.finish().await;
}

#[tokio::test]
async fn system_lock_observer_has_no_barriers_and_manual_lock_blocks_unlock_rearm() {
    let mut r = Rig::new(true, false, false).await;
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
    r.probe.watch.send_modify(|s| {
        s.session_locked = Some(true);
        s.invalidation_generation += 1;
        s.revision += 1;
    });
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 2).await;
    assert!(r.probe.barriers.lock().unwrap()[1].is_empty());
    r.probe.sender().send(activation()).await.unwrap();
    r.no_old_data().await;
    let _ = command(&r.tx, ModuleCommand::InputLock { locked: true }).await;
    wait_for(|| r.probe.stops.load(Ordering::SeqCst) >= 2).await;
    r.probe.watch.send_modify(|s| {
        s.session_locked = Some(false);
        s.revision += 1;
    });
    r.no_old_data().await;
    assert_eq!(r.probe.starts.load(Ordering::SeqCst), 2);
    let _ = command(&r.tx, ModuleCommand::InputLock { locked: false }).await;
    wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 3).await;
    assert!(!r.probe.barriers.lock().unwrap()[2].is_empty());
    r.probe.sender().send(activation()).await.unwrap();
    r.next_enter().await;
    r.finish().await;
}

// A espera fica na fronteira de envio; cancelar a abertura deve dropar o future
// sem transmitir Enter nem reaproveitar seu lote após a mudança de geração.
struct EnterGate {
    inner: Arc<dyn ModuleHost>,
    block: AtomicBool,
    inflight: AtomicUsize,
    complete: Notify,
}
struct EnterInFlight<'a>(&'a AtomicUsize);
impl Drop for EnterInFlight<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl EnterGate {
    async fn before(&self, msg: &ModuleMsg) -> Option<EnterInFlight<'_>> {
        if self.block.load(Ordering::SeqCst)
            && matches!(
                msg,
                ModuleMsg::Input(
                    InputMsg::Enter { .. } | InputMsg::EnterAt { .. } | InputMsg::EnterSpan { .. }
                )
            )
        {
            self.inflight.fetch_add(1, Ordering::SeqCst);
            let guard = EnterInFlight(&self.inflight);
            self.complete.notified().await;
            Some(guard)
        } else {
            None
        }
    }
}
#[async_trait::async_trait]
impl ModuleHost for EnterGate {
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
    fn peer_input_context(&self, peer: PeerId) -> Option<PeerInputContext> {
        self.inner.peer_input_context(peer)
    }
    fn request_pause(&self, p: bool) {
        self.inner.request_pause(p);
    }
    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError> {
        self.inner.send_datagram(to, dg)
    }
    async fn reject_input_on(&self, to: PeerId, c: u64, r: RejectReason) -> Result<(), SendError> {
        self.inner.reject_input_on(to, c, r).await
    }
    async fn send_reliable_on(&self, to: PeerId, c: u64, msg: ModuleMsg) -> Result<(), SendError> {
        let _guard = self.before(&msg).await;
        self.inner.send_reliable_on(to, c, msg).await
    }
    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError> {
        let _guard = self.before(&msg).await;
        self.inner.send_reliable(to, msg).await
    }
}

#[tokio::test]
async fn coalesced_watch_cancels_pending_enter_and_batch_in_both_consumers() {
    for continuous in [false, true] {
        let mut r = Rig::new(continuous, false, false).await;
        r.host.block.store(true, Ordering::SeqCst);
        let job = if continuous { None } else { Some(r.loopback()) };
        wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 1).await;
        r.probe.sender().send(activation()).await.unwrap();
        wait_for(|| r.host.inflight.load(Ordering::SeqCst) == 1).await;
        // Deixe dados entrarem no lote de abertura antes de invalidá-la.
        r.probe.sender().send(input()).await.unwrap();
        wait_for(|| r.probe.sender().capacity() == 4096).await;
        let old = fill_old_generation(&r.probe);
        // A conclusão de Enter fica pronta na mesma poll que o watch.
        r.host.complete.notify_one();
        if let Some(job) = job {
            assert!(matches!(job.await.unwrap(), ModuleReply::Error(_)));
        } else {
            wait_for(|| r.probe.starts.load(Ordering::SeqCst) == 2).await;
        }
        assert!(old.is_closed());
        assert_eq!(
            r.host.inflight.load(Ordering::SeqCst),
            0,
            "abertura velha não foi cancelada"
        );
        r.no_old_data().await;
        if continuous {
            r.host.block.store(false, Ordering::SeqCst);
            r.probe.sender().send(activation()).await.unwrap();
            r.next_enter().await;
        }
        r.finish().await;
    }
}
