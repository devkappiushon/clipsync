use super::*;
use crate::adapter::FakeEmulate;
use clipsync_proto::{
    InputEvent, MemHub, ModuleInput, ModuleSet, PeerInfo, PeerInputContext, PeerPerms,
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::Notify;
#[derive(Default)]
struct Cap {
    sender: Mutex<Option<mpsc::Sender<CaptureEvent>>>,
    released: AtomicUsize,
    starts: AtomicUsize,
}
struct Capture(Arc<Cap>);
#[async_trait::async_trait]
impl CaptureAdapter for Capture {
    fn name(&self) -> &'static str {
        "test-pressure"
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
        let (tx, rx) = mpsc::channel(1024);
        tx.try_send(CaptureEvent::SessionLocked(false)).unwrap();
        *self.0.sender.lock().unwrap() = Some(tx);
        self.0.starts.fetch_add(1, Ordering::SeqCst);
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.0.released.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        *self.0.sender.lock().unwrap() = None;
        Ok(())
    }
}
struct Host {
    inner: Arc<dyn ModuleHost>,
    sent: Notify,
    block_enter: AtomicBool,
    block_leave: AtomicBool,
    block_aux: AtomicBool,
    inflight: AtomicUsize,
    datagrams: AtomicUsize,
}
struct Count<'a>(&'a AtomicUsize);
impl Drop for Count<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
#[async_trait::async_trait]
impl ModuleHost for Host {
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
    fn lifecycle_receiver(
        &self,
    ) -> Option<tokio::sync::watch::Receiver<clipsync_proto::HostLifecycle>> {
        self.inner.lifecycle_receiver()
    }
    fn peer_input_context(&self, p: PeerId) -> Option<PeerInputContext> {
        self.inner.peer_input_context(p)
    }
    fn request_pause(&self, p: bool) {
        self.inner.request_pause(p)
    }
    fn send_datagram(&self, to: PeerId, d: InputDatagram) -> Result<(), SendError> {
        self.datagrams.fetch_add(1, Ordering::SeqCst);
        self.inner.send_datagram(to, d)
    }
    async fn reject_input_on(&self, to: PeerId, c: u64, r: RejectReason) -> Result<(), SendError> {
        self.inner.reject_input_on(to, c, r).await
    }
    async fn send_reliable_on(&self, to: PeerId, c: u64, m: ModuleMsg) -> Result<(), SendError> {
        if self
            .inner
            .peer_input_context(to)
            .is_none_or(|x| x.connection != c)
        {
            return Err(SendError::NotConnected);
        }
        self.send_reliable(to, m).await
    }
    async fn send_reliable(&self, to: PeerId, m: ModuleMsg) -> Result<(), SendError> {
        let enter = matches!(
            &m,
            ModuleMsg::Input(
                InputMsg::EnterSpan { .. } | InputMsg::Enter { .. } | InputMsg::EnterAt { .. }
            )
        );
        let leave = matches!(&m, ModuleMsg::Input(InputMsg::Leave { .. }));
        let auxiliary = matches!(
            &m,
            ModuleMsg::Input(
                InputMsg::Refused { .. }
                    | InputMsg::Returned { .. }
                    | InputMsg::Stats(_)
                    | InputMsg::Zones { .. }
            )
        );
        self.inner.send_reliable(to, m).await?;
        if (enter && self.block_enter.load(Ordering::SeqCst))
            || (leave && self.block_leave.load(Ordering::SeqCst))
            || (auxiliary && self.block_aux.load(Ordering::SeqCst))
        {
            self.inflight.fetch_add(1, Ordering::SeqCst);
            let _count = Count(&self.inflight);
            self.sent.notify_one();
            std::future::pending::<()>().await;
        }
        Ok(())
    }
}
fn zone() -> Zone {
    Zone {
        index: 0,
        name: Some("m".into()),
        x: 0,
        y: 0,
        width: 800,
        height: 600,
        scale: 1.0,
    }
}
async fn wait(check: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_millis(500), async {
        while !check() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("local action waited for blocked network");
}

fn pair() -> (Arc<Cap>, Arc<Host>, PeerId, mpsc::Receiver<ModuleInput>) {
    let hub = MemHub::new();
    let a = hub.add_node(
        "source",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let b = hub.add_node(
        "dest",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    hub.link(
        a.id,
        b.id,
        PeerPerms::default(),
        PeerPerms {
            input_in: true,
            ..Default::default()
        },
    );
    (
        Arc::new(Cap::default()),
        Arc::new(Host {
            inner: a.host,
            sent: Notify::new(),
            block_enter: AtomicBool::new(true),
            block_leave: AtomicBool::new(false),
            block_aux: AtomicBool::new(false),
            inflight: AtomicUsize::new(0),
            datagrams: AtomicUsize::new(0),
        }),
        b.id,
        b.rx,
    )
}
#[tokio::test]
async fn loopback_cancels_delivered_open_and_stops_adapter_before_network_cleanup() {
    for case in 0..6 {
        let (cap, host, peer, mut wire) = pair();
        let (sig, rx) = mpsc::unbounded_channel();
        let (reply, result) = oneshot::channel();
        let task = tokio::spawn(run_loopback(LoopJob {
            host: host.clone(),
            cap: Box::new(Capture(cap.clone())),
            barriers: vec![],
            to: peer,
            seconds: 10,
            session: Arc::new(AtomicU32::new(0)),
            signals: rx,
            reply,
        }));
        wait(|| cap.starts.load(Ordering::SeqCst) > 0).await;
        let sender = cap.sender.lock().unwrap().as_ref().unwrap().clone();
        sender
            .send(CaptureEvent::Activated {
                barrier: 0,
                pos: 0.2,
                t_mono_us: 0,
            })
            .await
            .unwrap();
        host.sent.notified().await;
        match case {
            0 => sig.send(LoopSignal::Stop).unwrap(),
            1 => sig.send(LoopSignal::Paused).unwrap(),
            2 => sig.send(LoopSignal::PeerDown).unwrap(),
            3 => sig
                .send(LoopSignal::Rejected(RejectReason::NotAuthorized))
                .unwrap(),
            4 => {
                let tx = cap.sender.lock().unwrap().as_ref().unwrap().clone();
                tx.send(CaptureEvent::SessionLocked(true)).await.unwrap();
            }
            _ => {
                let tx = cap.sender.lock().unwrap().as_ref().unwrap().clone();
                for _ in 0..257 {
                    tx.send(CaptureEvent::Input {
                        t_mono_us: 0,
                        ev: InputEvent::Motion { dx: 1.0, dy: 0.0 },
                    })
                    .await
                    .unwrap();
                }
            }
        }
        wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
        let done = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        assert!(done.outcome.is_err());
        assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
        assert!(cap.sender.lock().unwrap().is_none());
        assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
        assert!(
            std::iter::from_fn(|| wire.try_recv().ok()).any(|m| matches!(
                m,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::Leave { .. }),
                    ..
                }
            ))
        );
        drop(done);
        assert!(result.await.is_err());
    }
}
fn state(host: Arc<dyn ModuleHost>) -> State {
    let (notes, _) = mpsc::unbounded_channel();
    let (emu, _) = FakeEmulate::new();
    State {
        network: Default::default(),
        frontiers: HashMap::new(),
        cfg: InputConfig::default(),
        authority: capture::HostCursor::new(host.as_ref()),
        host,
        emu: Box::new(emu),
        emu_started: false,
        cap: None,
        cap_name: "fake",
        emu_name: "fake",
        peers: HashMap::new(),
        incoming: HashMap::new(),
        buckets: HashMap::new(),
        ctl: None,
        ignored: 0,
        rate_dropped: 0,
        paused: false,
        last_event: None,
        last_block: None,
        layout: None,
        layout_path: None,
        locked: false,
        cont: None,
        notes,
        local_zones: vec![],
        peer_zones: HashMap::new(),
        pending_offer: None,
        active_peer: None,
        offer_job: None,
        cap_hooks: false,
        resume_rx: None,
        suspended: false,
        retry_on_zones: false,
        targets_key: String::new(),
        cap_can_begin: true,
        resume_refused: false,
    }
}
#[tokio::test]
async fn stale_hotkeys_obey_live_pause_without_losing_resume_request() {
    let hub = MemHub::new();
    let node = hub.add_node(
        "local",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let id = node.id;
    let mut state = state(node.host);
    hub.set_paused(id, true); // State has deliberately not consumed Paused yet.
    state.on_hotkey(Hotkey::ToggleLock).await;
    assert!(!state.locked);
    state.on_hotkey(Hotkey::GoTo(1)).await;
    assert!(state.last_block.is_none());
    state.on_hotkey(Hotkey::TogglePause).await;
    assert_eq!(hub.pause_requests(id), 1);
}
#[tokio::test]
async fn network_owner_bounds_and_joins_all_blocked_jobs() {
    let mut owner = crate::network::Network::default();
    let active = Arc::new(AtomicUsize::new(0));
    for _ in 0..32 {
        let active = active.clone();
        assert!(owner.spawn(async move {
            active.fetch_add(1, Ordering::SeqCst);
            let _count = Count(&active);
            std::future::pending::<()>().await;
        }));
    }
    wait(|| active.load(Ordering::SeqCst) == 32).await;
    assert!(!owner.spawn(async {}));
    owner.stop().await;
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert!(owner.has_capacity());
}

#[tokio::test]
async fn refused_returned_and_stats_are_cancelled_and_joined_on_shutdown() {
    let (_, host, peer, _wire) = pair();
    host.block_aux.store(true, Ordering::SeqCst);
    let mut state = state(host.clone());
    let c = host.peer_input_context(peer).unwrap();
    let context = clipsync_proto::ReceiveContext {
        connection: c.connection,
        input_epoch: c.input_epoch,
        stream: Some(0),
    };
    state.incoming.insert(peer, Incoming::new(9, context));
    state.refuse_on(peer, 9, c.connection, RejectReason::NotAuthorized);
    state.send_v3(
        peer,
        c.connection,
        InputMsg::Returned {
            session: 9,
            pos: 0.3,
        },
    );
    state
        .on_msg(peer, InputMsg::StatsRequest { session: 9 }, context)
        .await;
    wait(|| host.inflight.load(Ordering::SeqCst) == 3).await;
    state.network.stop().await;
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn fix1_loopback_joins_producer_blocked_in_full_send_before_leave() {
    let (_, host, peer, mut wire) = pair();
    host.block_leave.store(true, Ordering::SeqCst);
    let h = Arc::new(crate::stop_capture::Handle::default());
    let (sig, signals) = mpsc::unbounded_channel();
    let (reply, _result) = oneshot::channel();
    let mut task = tokio::spawn(run_loopback(LoopJob {
        host: host.clone(),
        cap: Box::new(crate::stop_capture::Capture::new(h.clone())),
        barriers: vec![],
        to: peer,
        seconds: 10,
        session: Arc::new(AtomicU32::new(0)),
        signals,
        reply,
    }));
    wait(|| h.starts.load(Ordering::SeqCst) == 1).await;
    let old = h.sender();
    old.send(CaptureEvent::SessionLocked(false)).await.unwrap();
    old.send(CaptureEvent::Activated {
        barrier: 0,
        pos: 0.3,
        t_mono_us: 0,
    })
    .await
    .unwrap();
    host.sent.notified().await;
    sig.send(LoopSignal::Stop).unwrap();
    h.releasing.notified().await;
    h.fill_and_block().await;
    h.release.notify_one();
    let result = tokio::time::timeout(Duration::from_secs(3), &mut task).await;
    if result.is_err() {
        h.rescue();
        let _ = tokio::time::timeout(Duration::from_secs(3), &mut task).await;
        panic!("loopback stop could not join producer blocked in full send");
    }
    assert!(result.unwrap().unwrap().outcome.is_err());
    assert_eq!(h.joined.load(Ordering::SeqCst), 1);
    assert!(old.is_closed());
    assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
    assert!(
        std::iter::from_fn(|| wire.try_recv().ok()).any(|m| matches!(
            m,
            ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::Leave { .. }),
                ..
            }
        ))
    );
}
