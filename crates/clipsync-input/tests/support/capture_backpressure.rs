use super::*;
use clipsync_proto::{MemHub, ModuleInput, ModuleSet, PeerInfo, PeerInputContext, PeerPerms};
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
    begins: AtomicUsize,
    block_stop: AtomicBool,
    stopping: Notify,
    stop_complete: Notify,
    block_restart: AtomicBool,
    restarting: Notify,
    restart_complete: Notify,
}
struct Capture(Arc<Cap>);
#[async_trait::async_trait]
impl CaptureAdapter for Capture {
    fn known_monitor_observation(&self) -> Result<crate::inventory::ObservedMonitors, InputError> {
        crate::inventory::windows_observation(&[clipsync_win::Monitor {
            friendly: "m".into(),
            device: "m".into(),
            path: "test-source".into(),
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            dpi: 96,
            primary: true,
        }])
    }

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
        if self.0.starts.load(Ordering::SeqCst) > 0
            && self.0.block_restart.swap(false, Ordering::SeqCst)
        {
            self.0.restarting.notify_one();
            self.0.restart_complete.notified().await;
        }
        let (tx, rx) = mpsc::channel(1024);
        tx.try_send(CaptureEvent::SessionLocked(false)).unwrap();
        *self.0.sender.lock().unwrap() = Some(tx);
        self.0.starts.fetch_add(1, Ordering::SeqCst);
        Ok(rx)
    }
    async fn begin(&mut self, _: usize, _: f32) -> Result<(), InputError> {
        self.0.begins.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.0.released.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        *self.0.sender.lock().unwrap() = None;
        if self.0.block_stop.swap(false, Ordering::SeqCst) {
            self.0.stopping.notify_one();
            self.0.stop_complete.notified().await;
        }
        Ok(())
    }
}
struct Host {
    cleanup: DeferredCleanup,
    observe_lifecycle: bool,
    hub: MemHub,
    source: PeerId,
    destination: PeerId,
    inner: Arc<dyn ModuleHost>,
    sent: Notify,
    complete: Notify,
    forwarded: Mutex<Vec<InputEvent>>,
    block_enter: AtomicBool,
    block_leave: AtomicBool,
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
    fn lifecycle_receiver(
        &self,
    ) -> Option<tokio::sync::watch::Receiver<clipsync_proto::HostLifecycle>> {
        if self.observe_lifecycle {
            self.inner.lifecycle_receiver()
        } else {
            None
        }
    }

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
    fn peer_input_context(&self, p: PeerId) -> Option<PeerInputContext> {
        self.inner.peer_input_context(p)
    }
    fn request_pause(&self, p: bool) {
        self.inner.request_pause(p)
    }
    fn send_datagram(&self, to: PeerId, d: InputDatagram) -> Result<(), SendError> {
        self.datagrams.fetch_add(1, Ordering::SeqCst);
        self.forwarded.lock().unwrap().extend(d.events.clone());
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
                InputMsg::EnterSpan { .. }
                    | InputMsg::Enter { .. }
                    | InputMsg::EnterAt { .. }
                    | InputMsg::EnterRoutes(_)
            )
        );
        let leave = matches!(&m, ModuleMsg::Input(InputMsg::Leave { .. }));
        self.inner.send_reliable(to, m).await?;
        if (enter && self.block_enter.load(Ordering::SeqCst))
            || (leave && self.block_leave.load(Ordering::SeqCst))
        {
            self.inflight.fetch_add(1, Ordering::SeqCst);
            let _count = Count(&self.inflight);
            self.sent.notify_one();
            self.complete.notified().await;
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
async fn setup() -> (
    Arc<Cap>,
    Arc<Host>,
    mpsc::UnboundedSender<Ctl>,
    tokio::task::JoinHandle<Box<dyn CaptureAdapter>>,
    mpsc::Receiver<ModuleInput>,
    PeerId,
) {
    setup_with(None).await
}
async fn setup_with(
    adapter: Option<Box<dyn CaptureAdapter>>,
) -> (
    Arc<Cap>,
    Arc<Host>,
    mpsc::UnboundedSender<Ctl>,
    tokio::task::JoinHandle<Box<dyn CaptureAdapter>>,
    mpsc::Receiver<ModuleInput>,
    PeerId,
) {
    setup_mode(adapter, false).await
}
async fn setup_mode(
    adapter: Option<Box<dyn CaptureAdapter>>,
    v4: bool,
) -> (
    Arc<Cap>,
    Arc<Host>,
    mpsc::UnboundedSender<Ctl>,
    tokio::task::JoinHandle<Box<dyn CaptureAdapter>>,
    mpsc::Receiver<ModuleInput>,
    PeerId,
) {
    let hub = MemHub::new();
    let a = hub.add_node(
        "a",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let b = hub.add_node(
        "b",
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
    let host = Arc::new(Host {
        cleanup: Default::default(),
        observe_lifecycle: v4,
        hub,
        source: a.id,
        destination: b.id,
        inner: a.host,
        sent: Notify::new(),
        complete: Notify::new(),
        forwarded: Mutex::new(vec![]),
        block_enter: AtomicBool::new(true),
        block_leave: AtomicBool::new(false),
        inflight: AtomicUsize::new(0),
        datagrams: AtomicUsize::new(0),
    });
    let cap = Arc::new(Cap::default());
    let (ctl, rx) = mpsc::unbounded_channel();
    let (notes, _) = mpsc::unbounded_channel();
    let target = Target {
        spec: BarrierSpec {
            zone: crate::adapter::ZoneRef::Index(0),
            edge: Edge::Right,
            from: 0.0,
            to: 1.0,
        },
        peer: b.id,
        peer_monitor: Some("m".into()),
        peer_edge: Edge::Left,
        local: (0.0, 1.0),
        remote: (0.0, 1.0),
    };
    let standard = adapter.is_none();
    let catalog = InventoryCatalog::default();
    let mut route_plans = HashMap::new();
    if v4 {
        use clipsync_proto::*;
        let inventory = |uid, rev, space| MonitorInventory {
            rev,
            space,
            source: GeometrySource::Observed,
            capabilities: MonitorCapabilities {
                internal_edges: true,
                positioned_return: true,
                route_receiver: true,
            },
            monitors: vec![MonitorInfo {
                uid: MonitorUid([uid; 16]),
                label: "m".into(),
                rect: NativeRect {
                    x: 0,
                    y: 0,
                    width: 800,
                    height: 600,
                },
                scale: 1.0,
            }],
        };
        let connection = host.peer_input_context(b.id).unwrap().connection;
        catalog
            .insert(
                a.id,
                InventoryStamp {
                    peer: a.id,
                    connection: 0,
                    rev: 1,
                },
                inventory(1, 1, CoordinateSpace::Physical),
            )
            .unwrap();
        catalog
            .insert(
                a.id,
                InventoryStamp {
                    peer: b.id,
                    connection,
                    rev: 2,
                },
                inventory(2, 2, CoordinateSpace::Logical),
            )
            .unwrap();
        let inventories = InventoryRevisions {
            source: 1,
            target: 2,
        };
        route_plans.insert(
            0,
            crate::route_session::RoutePlan {
                entry: EnterRoutes {
                    session: 1,
                    inventories,
                    monitor: MonitorUid([2; 16]),
                    edge: Edge::Left,
                    pos: 0.5,
                    routes: SessionRoutes {
                        first: ExitRoute {
                            id: RouteId(0),
                            monitor: MonitorUid([2; 16]),
                            edge: Edge::Left,
                            from: 0.0,
                            to: 1.0,
                        },
                        second: None,
                    },
                },
                session: crate::route_session::RouteSession {
                    peer: b.id,
                    connection,
                    session: 1,
                    inventories,
                    returns: vec![crate::route_session::LocalReturn {
                        id: RouteId(0),
                        key: MonitorKey {
                            peer: a.id,
                            uid: MonitorUid([1; 16]),
                        },
                        barrier: 0,
                        local: (0.0, 1.0),
                        remote: (0.0, 1.0),
                    }],
                },
            },
        );
    }
    let engine = Engine {
        cleanup: host.cleanup.clone(),
        expected_observation: None,
        catalog,
        route_plans,
        host: host.clone(),
        cap: adapter.unwrap_or_else(|| Box::new(Capture(cap.clone()))),
        targets: vec![target],
        ctl: rx,
        notes,
        locked: false,
        paused: false,
        dead_corner_px: 0,
        peer_zones: HashMap::new(),
    };
    let task = tokio::spawn(run(engine));
    if standard {
        wait(|| cap.starts.load(Ordering::SeqCst) > 0).await;
    }
    (cap, host, ctl, task, b.rx, b.id)
}
fn sender(cap: &Cap) -> mpsc::Sender<CaptureEvent> {
    cap.sender.lock().unwrap().as_ref().unwrap().clone()
}
async fn activate(cap: &Cap) {
    sender(cap)
        .send(CaptureEvent::Activated {
            barrier: 0,
            pos: 0.3,
            t_mono_us: 0,
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn lifecycle_cancels_enter_that_was_delivered_but_never_completed() {
    for case in 0..12 {
        let (cap, host, ctl, task, mut wire, peer) = setup().await;
        activate(&cap).await;
        host.sent.notified().await;
        let old = sender(&cap);
        match case {
            0 => ctl.send(Ctl::Paused(true)).unwrap(),
            1 => ctl.send(Ctl::Lock(true)).unwrap(),
            2 => ctl.send(Ctl::Home).unwrap(),
            3 => ctl.send(Ctl::PeerDown(peer)).unwrap(),
            4 => old.send(CaptureEvent::SessionLocked(true)).await.unwrap(),
            5 => old.send(CaptureEvent::Deactivated).await.unwrap(),
            6 => old
                .send(CaptureEvent::Error("synthetic".into()))
                .await
                .unwrap(),
            7 => ctl.send(Ctl::Retarget(vec![])).unwrap(),
            8 => ctl
                .send(Ctl::Rejected {
                    from: peer,
                    reason: RejectReason::NotAuthorized,
                })
                .unwrap(),
            9 => {
                *cap.sender.lock().unwrap() = None;
                drop(old.clone());
            }
            10 => ctl.send(Ctl::Stop).unwrap(),
            _ => {
                let session = loop {
                    if let Some(ModuleInput::Reliable {
                        msg: ModuleMsg::Input(InputMsg::EnterSpan { session, .. }),
                        ..
                    }) = wire.recv().await
                    {
                        break session;
                    }
                };
                ctl.send(Ctl::Refused {
                    from: peer,
                    session,
                    reason: RejectReason::NotAuthorized,
                })
                .unwrap();
            }
        }
        if case == 9 {
            drop(old);
        }
        wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
        assert_eq!(
            host.inflight.load(Ordering::SeqCst),
            0,
            "cancelled open future still alive, case {case}"
        );
        let _ = ctl.send(Ctl::Stop);
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        let mut leave = false;
        while let Ok(m) = wire.try_recv() {
            if matches!(
                m,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::Leave { .. }),
                    ..
                }
            ) {
                leave = true;
            }
        }
        assert!(leave, "delivered Enter requires Leave, case {case}");
    }
}
#[tokio::test]
async fn closing_network_never_precedes_local_release() {
    let (cap, host, ctl, task, _wire, _) = setup().await;
    host.block_enter.store(false, Ordering::SeqCst);
    host.block_leave.store(true, Ordering::SeqCst);
    activate(&cap).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    ctl.send(Ctl::Home).unwrap();
    host.sent.notified().await;
    assert!(
        cap.released.load(Ordering::SeqCst) > 0,
        "Leave was awaited before releasing local capture"
    );
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn overflow_discards_old_receiver_and_never_replays_into_next_attempt() {
    let (cap, host, ctl, task, _wire, _) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    let old = sender(&cap);
    for _ in 0..257 {
        old.send(CaptureEvent::Input {
            t_mono_us: 0,
            ev: InputEvent::Motion { dx: 1.0, dy: 0.0 },
        })
        .await
        .unwrap();
    }
    wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
    wait(|| cap.starts.load(Ordering::SeqCst) > 1).await;
    assert!(
        old.send(CaptureEvent::Activated {
            barrier: 0,
            pos: 0.3,
            t_mono_us: 0
        })
        .await
        .is_err()
    );
    host.block_enter.store(false, Ordering::SeqCst);
    activate(&cap).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn cancelled_switch_never_becomes_success_after_grace() {
    let (cap, host, ctl, task, _wire, peer) = setup().await;
    let (tx, rx) = oneshot::channel();
    ctl.send(Ctl::Switch {
        to: peer,
        reply: tx,
    })
    .unwrap();
    host.sent.notified().await;
    ctl.send(Ctl::Home).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), rx)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
    tokio::time::sleep(SWITCH_GRACE + Duration::from_millis(20)).await;
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
    assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn input_buffer_preserves_press_release_order_when_opening_completes() {
    let (cap, host, ctl, task, _wire, _) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    let tx = sender(&cap);
    let events = vec![
        InputEvent::Key {
            code: 42,
            pressed: true,
        },
        InputEvent::Key {
            code: 42,
            pressed: false,
        },
        InputEvent::Key {
            code: 43,
            pressed: true,
        },
    ];
    for ev in &events[..2] {
        tx.send(CaptureEvent::Input {
            t_mono_us: 0,
            ev: *ev,
        })
        .await
        .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(20)).await;
    host.complete.notify_one();
    tx.send(CaptureEvent::Input {
        t_mono_us: 0,
        ev: events[2],
    })
    .await
    .unwrap();
    wait(|| host.forwarded.lock().unwrap().len() == 3).await;
    assert_eq!(*host.forwarded.lock().unwrap(), events);
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn cancelled_cleanup_never_migrates_to_a_replacement_connection() {
    let (cap, host, ctl, task, mut wire, _) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    host.hub.unlink(host.source, host.destination);
    host.hub.link(
        host.source,
        host.destination,
        PeerPerms::default(),
        PeerPerms {
            input_in: true,
            ..Default::default()
        },
    );
    ctl.send(Ctl::Home).unwrap();
    wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
    assert!(
        !std::iter::from_fn(|| wire.try_recv().ok()).any(|m| matches!(
            m,
            ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::Leave { .. }),
                ..
            }
        )),
        "cleanup leaked into replacement connection"
    );
}

#[tokio::test]
async fn ready_session_lock_wins_over_simultaneously_completed_switch() {
    let (cap, host, ctl, task, _wire, peer) = setup().await;
    let (tx, rx) = oneshot::channel();
    ctl.send(Ctl::Switch {
        to: peer,
        reply: tx,
    })
    .unwrap();
    host.sent.notified().await;
    sender(&cap)
        .send(CaptureEvent::SessionLocked(true))
        .await
        .unwrap();
    host.complete.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_millis(500), rx)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        cap.begins.load(Ordering::SeqCst),
        0,
        "ready lifecycle signal must precede open completion"
    );
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn expiring_or_dropping_switch_bridge_cancels_the_attempt() {
    for expire in [false, true] {
        let (cap, host, ctl, task, _wire, peer) = setup().await;
        let (reply, rx) = oneshot::channel();
        ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
        host.sent.notified().await;
        let mut owner = crate::network::Network::default();
        if expire {
            assert!(owner.spawn_for(Duration::from_millis(20), async {
                let _ = rx.await;
            }));
        } else {
            drop(rx);
        }
        wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
        assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
        owner.stop().await;
        ctl.send(Ctl::Stop).unwrap();
        task.await.unwrap();
    }
}

#[tokio::test]
async fn fix1_full_producer_send_is_joined_on_reset_and_shutdown() {
    for shutdown in [false, true] {
        let h = Arc::new(crate::stop_capture::Handle::default());
        let (_, host, ctl, mut task, mut wire, peer) =
            setup_with(Some(Box::new(crate::stop_capture::Capture::new(h.clone())))).await;
        host.block_leave.store(shutdown, Ordering::SeqCst);
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
        ctl.send(if shutdown { Ctl::Stop } else { Ctl::Home })
            .unwrap();
        h.releasing.notified().await; // controller has stopped consuming rx
        if !shutdown {
            old.send(CaptureEvent::SessionLocked(true)).await.unwrap();
            old.send(CaptureEvent::Activated {
                barrier: 0,
                pos: 0.8,
                t_mono_us: 0,
            })
            .await
            .unwrap();
        }
        h.fill_and_block().await; // actual thread has polled send and got Pending
        h.release.notify_one();
        let joined = tokio::time::timeout(Duration::from_millis(500), async {
            while h.joined.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_ok();
        if !joined {
            h.rescue(); // failure cleanup only, never the passing path
            let _ = ctl.send(Ctl::Stop);
            h.release.notify_one();
            let _ = tokio::time::timeout(Duration::from_secs(3), &mut task).await;
            panic!("full producer send prevented join, shutdown={shutdown}");
        }
        assert!(old.is_closed(), "old input receiver survived join");
        let leave = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if matches!(
                    wire.recv().await,
                    Some(ModuleInput::Reliable {
                        msg: ModuleMsg::Input(InputMsg::Leave { .. }),
                        ..
                    })
                ) {
                    break;
                }
            }
        })
        .await;
        assert!(leave.is_ok(), "Leave was trapped behind producer stop");
        if !shutdown {
            wait(|| h.starts.load(Ordering::SeqCst) == 2).await;
            let (reply, result) = oneshot::channel();
            ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
            assert_eq!(
                tokio::time::timeout(Duration::from_millis(500), result)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(msg::SOURCE_SCREEN_LOCKED.to_owned()),
                "queued lifecycle was lost at the receiver boundary"
            );
            h.sender()
                .send(CaptureEvent::SessionLocked(false))
                .await
                .unwrap();
            wait(|| h.starts.load(Ordering::SeqCst) == 3).await;
            h.sender()
                .send(CaptureEvent::SessionLocked(false))
                .await
                .unwrap();
            host.block_enter.store(false, Ordering::SeqCst);
            h.sender()
                .send(CaptureEvent::Activated {
                    barrier: 0,
                    pos: 0.4,
                    t_mono_us: 0,
                })
                .await
                .unwrap();
            h.sender()
                .send(CaptureEvent::Input {
                    t_mono_us: 1,
                    ev: InputEvent::Motion { dx: 1.0, dy: 0.0 },
                })
                .await
                .unwrap();
            wait(|| host.datagrams.load(Ordering::SeqCst) == 1).await;
            assert_eq!(
                *host.forwarded.lock().unwrap(),
                vec![InputEvent::Motion { dx: 1.0, dy: 0.0 }]
            );
            ctl.send(Ctl::Stop).unwrap();
            h.release.notify_one();
        }
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            h.joined.load(Ordering::SeqCst),
            if shutdown { 1 } else { 3 }
        );
        assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
    }
}

async fn control_during_stop(case: usize) {
    let (cap, host, ctl, mut task, _wire, peer) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    cap.block_stop.store(true, Ordering::SeqCst);
    ctl.send(Ctl::Home).unwrap();
    cap.stopping.notified().await;
    match case {
        0 => ctl.send(Ctl::Stop).unwrap(),
        1 => ctl.send(Ctl::Paused(true)).unwrap(),
        2 => ctl.send(Ctl::Lock(true)).unwrap(),
        _ => host.hub.set_paused(host.source, true),
    }
    let (reply, result) = oneshot::channel();
    ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
    cap.stop_complete.notify_one();
    let response = tokio::time::timeout(Duration::from_millis(500), result)
        .await
        .unwrap();
    assert!(
        response.is_err() || response.unwrap().is_err(),
        "cancelled Switch succeeded"
    );
    let starts = cap.starts.load(Ordering::SeqCst);
    let _ = ctl.send(Ctl::Stop);
    tokio::time::timeout(Duration::from_secs(3), &mut task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(starts, 1, "rearmed before queued cancellation, case {case}");
}
#[tokio::test]
async fn fix1_stop_during_join_prevents_rearm() {
    control_during_stop(0).await;
}
#[tokio::test]
async fn fix1_pause_during_join_prevents_rearm() {
    control_during_stop(1).await;
}
#[tokio::test]
async fn fix1_lock_during_join_prevents_rearm() {
    control_during_stop(2).await;
}
#[tokio::test]
async fn fix1_host_pause_during_join_prevents_rearm() {
    control_during_stop(3).await;
}

#[tokio::test]
async fn fix1_bridge_cancelled_during_stop_cannot_open_replacement_session() {
    let (cap, host, ctl, task, mut wire, peer) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    cap.block_stop.store(true, Ordering::SeqCst);
    let (reply, result) = oneshot::channel();
    ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
    cap.stopping.notified().await;
    drop(result);
    cap.stop_complete.notify_one();
    // Synchronize with consumption of a subsequent control rather than sleeping.
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        cap.starts.load(Ordering::SeqCst),
        1,
        "rearmed before bridge cancellation and Stop"
    );
    assert_eq!(cap.begins.load(Ordering::SeqCst), 0);
    let enters = std::iter::from_fn(|| wire.try_recv().ok())
        .filter(|m| {
            matches!(
                m,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::EnterSpan { .. }),
                    ..
                }
            )
        })
        .count();
    assert_eq!(enters, 1, "cancelled replacement Enter escaped");
}

#[tokio::test]
async fn fix1_cancelled_bridge_releases_before_slow_rearm() {
    let (cap, host, ctl, task, mut wire, peer) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    cap.block_stop.store(true, Ordering::SeqCst);
    cap.block_restart.store(true, Ordering::SeqCst);
    let (reply, result) = oneshot::channel();
    ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
    cap.stopping.notified().await;
    drop(result); // bridge disappears while join is suspended; no Stop or pause here
    cap.stop_complete.notify_one();
    cap.restarting.notified().await;
    let releases_before_start = cap.released.load(Ordering::SeqCst);
    cap.restart_complete.notify_one();
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        releases_before_start, 2,
        "cancelled replacement retained capture until start completed"
    );
    assert_eq!(cap.begins.load(Ordering::SeqCst), 0);
    assert_eq!(
        std::iter::from_fn(|| wire.try_recv().ok())
            .filter(|m| matches!(
                m,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::EnterSpan { .. }),
                    ..
                }
            ))
            .count(),
        1
    );
}

#[tokio::test]
async fn fix1_retarget_during_join_cancels_the_waiting_switch() {
    let (cap, host, ctl, task, mut wire, peer) = setup().await;
    activate(&cap).await;
    host.sent.notified().await;
    cap.block_stop.store(true, Ordering::SeqCst);
    let (reply, mut result) = oneshot::channel();
    ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
    cap.stopping.notified().await;
    ctl.send(Ctl::Retarget(vec![])).unwrap();
    cap.stop_complete.notify_one();
    let response = tokio::time::timeout(Duration::from_millis(500), &mut result).await;
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(response, Ok(Ok(Err(_)))),
        "retarget left old Switch pending"
    );
    assert_eq!(cap.starts.load(Ordering::SeqCst), 1);
    assert_eq!(
        std::iter::from_fn(|| wire.try_recv().ok())
            .filter(|m| matches!(
                m,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(InputMsg::EnterSpan { .. }),
                    ..
                }
            ))
            .count(),
        1
    );
}

#[tokio::test]
async fn manual_lock_and_pause_keep_their_refusal_after_stopping_the_producer() {
    for paused in [false, true] {
        let (cap, host, ctl, task, _wire, peer) = setup().await;
        activate(&cap).await;
        host.sent.notified().await;
        ctl.send(if paused {
            Ctl::Paused(true)
        } else {
            Ctl::Lock(true)
        })
        .unwrap();
        wait(|| cap.sender.lock().unwrap().is_none()).await;
        let (reply, result) = oneshot::channel();
        ctl.send(Ctl::Switch { to: peer, reply }).unwrap();
        let refusal = tokio::time::timeout(Duration::from_secs(1), result)
            .await
            .unwrap()
            .unwrap();
        ctl.send(Ctl::Stop).unwrap();
        let _cap = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
        assert_eq!(
            refusal,
            Err(if paused {
                RejectReason::Paused.pt().to_owned()
            } else {
                msg::LOCKED.to_owned()
            })
        );
    }
}

#[tokio::test]
async fn unavailable_without_lock_or_pause_keeps_the_adapter_failure() {
    let node = MemHub::new().add_node(
        "unavailable",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let cap = Arc::new(Cap::default());
    let (notes, _) = mpsc::unbounded_channel();
    let mut state = Loop {
        cleanup: Default::default(),
        quiesce_reply: None,
        prepare_reply: None,
        quiesced: false,
        catalog: Default::default(),
        route_plans: Default::default(),
        observed: None,
        host: node.host,
        cap: Box::new(Capture(cap.clone())),
        targets: vec![],
        notes,
        locked: false,
        paused: false,
        session_locked: false,
        snapshot_ready: true,
        dead_corner_px: 0,
        peer_zones: HashMap::new(),
        last_zones: vec![],
        session: None,
        opening: None,
        closing: None,
        reset_rx: false,
        available: false,
        pending: None,
        queue: VecDeque::new(),
        attempt: 0,
        close_reason: None,
        close_detail: None,
    };
    assert_eq!(
        state.switch(PeerId([1; 32])).await,
        Err(msg::adapter_failed("captura indisponível"))
    );
    assert_eq!(cap.starts.load(Ordering::SeqCst), 0);
    assert_eq!(cap.released.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn s10_v4_open_buffer_boundaries_and_timeout_never_replay_stale_input() {
    for count in [255, 256, 257] {
        let (cap, host, ctl, task, _wire, _peer) = setup_mode(None, true).await;
        activate(&cap).await;
        host.sent.notified().await;
        let tx = sender(&cap);
        for _ in 0..count {
            tx.send(CaptureEvent::Input {
                t_mono_us: 0,
                ev: InputEvent::Motion { dx: 1.0, dy: 0.0 },
            })
            .await
            .unwrap();
        }
        if count <= 256 {
            host.complete.notify_one();
            wait(|| host.forwarded.lock().unwrap().len() == count).await;
        } else {
            wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
            assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
            wait(|| tx.is_closed()).await;
        }
        ctl.send(Ctl::Stop).unwrap();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }
    let (cap, host, ctl, task, _wire, _peer) = setup_mode(None, true).await;
    activate(&cap).await;
    host.sent.notified().await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while cap.released.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(host.datagrams.load(Ordering::SeqCst), 0);
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}
#[tokio::test]
async fn s10_v4_stop_cancels_pending_return_network_after_local_release() {
    use clipsync_proto::*;
    let (cap, host, ctl, task, mut wire, peer) = setup_mode(None, true).await;
    host.block_enter.store(false, Ordering::SeqCst);
    host.block_leave.store(true, Ordering::SeqCst);
    activate(&cap).await;
    let entry = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Some(ModuleInput::Reliable {
                msg: ModuleMsg::Input(InputMsg::EnterRoutes(e)),
                ..
            }) = wire.recv().await
            {
                break e;
            }
        }
    })
    .await
    .unwrap();
    let context = host.peer_input_context(peer).unwrap();
    ctl.send(Ctl::ReturnedRoute {
        from: peer,
        context: ReceiveContext {
            connection: context.connection,
            input_epoch: context.input_epoch,
            stream: Some(3),
        },
        returned: ReturnedRoute {
            session: entry.session,
            inventories: entry.inventories,
            route: RouteId(0),
            pos: 0.4,
        },
    })
    .unwrap();
    wait(|| cap.released.load(Ordering::SeqCst) > 0).await;
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn s10_v4_quiesce_ack_keeps_admission_closed_until_owner_stops_engine() {
    let (cap, _host, ctl, task, _wire, _peer) = setup_mode(None, true).await;
    let starts = cap.starts.load(Ordering::SeqCst);
    let (reply, ack) = oneshot::channel();
    ctl.send(Ctl::Quiesce { reply }).unwrap();
    ack.await.unwrap().unwrap();
    ctl.send(Ctl::Paused(false)).unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(
        cap.starts.load(Ordering::SeqCst),
        starts,
        "host's unpaused state must not undo transactional quiesce"
    );
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn s10_fix1_quiesce_closes_full_receiver_before_joining_blocked_producer() {
    let h = Arc::new(crate::stop_capture::Handle::default());
    let (_cap, host, ctl, task, _wire, _peer) =
        setup_with(Some(Box::new(crate::stop_capture::Capture::new(h.clone())))).await;
    wait(|| h.starts.load(Ordering::SeqCst) == 1).await;
    let tx = h.sender();
    tx.send(CaptureEvent::SessionLocked(false)).await.unwrap();
    tx.send(CaptureEvent::Activated {
        barrier: 0,
        pos: 0.4,
        t_mono_us: 0,
    })
    .await
    .unwrap();
    host.sent.notified().await;
    let (reply, mut ack) = oneshot::channel();
    ctl.send(Ctl::Quiesce { reply }).unwrap();
    h.releasing.notified().await;
    h.fill_and_block().await;
    h.release_open.store(true, Ordering::SeqCst);
    h.release.notify_one();
    let result = tokio::time::timeout(Duration::from_millis(500), &mut ack).await;
    let completed = result.is_ok();
    if !completed {
        h.rescue();
        let _ = tokio::time::timeout(Duration::from_secs(2), ack).await;
    }
    ctl.send(Ctl::Stop).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        completed,
        "quiesce waited on a producer whose receiver was still open"
    );
    assert!(tx.is_closed());
    assert_eq!(h.joined.load(Ordering::SeqCst), 1);
}

include!("prepare_capture.rs");
#[tokio::test]
async fn s10_fix1_engine_stop_cancels_pending_prepare_and_returns_adapter() {
    let (cap, _ch) = crate::FakeCapture::new(vec![zone()]);
    let gate = Arc::new(PrepareGate::default());
    let (_cap, _host, ctl, mut task, _raw, _peer) = setup_with(Some(Box::new(PreparingCapture {
        inner: cap,
        gate: gate.clone(),
    })))
    .await;
    let (reply, _rx) = oneshot::channel();
    ctl.send(Ctl::PrepareInventory { reply }).unwrap();
    tokio::time::timeout(Duration::from_secs(1), gate.entered.notified())
        .await
        .unwrap();
    ctl.send(Ctl::Stop).unwrap();
    let stopped = tokio::time::timeout(Duration::from_millis(100), &mut task)
        .await
        .is_ok();
    if !stopped {
        gate.complete.notify_one();
        let _ = task.await;
    }
    assert!(stopped, "Engine blocked Stop during prepare");
    assert!(!gate.pending.load(Ordering::SeqCst));
}

#[tokio::test]
async fn s10_fix1_engine_pending_prepare_cancels_on_pause_or_revoke_without_late_inventory() {
    for pause in [false, true] {
        let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
        let gate = Arc::new(PrepareGate::default());
        let (_cap, host, ctl, task, _raw, peer) = setup_mode(
            Some(Box::new(PreparingCapture {
                inner: cap,
                gate: gate.clone(),
            })),
            true,
        )
        .await;
        let (reply, rx) = oneshot::channel();
        ctl.send(Ctl::PrepareInventory { reply }).unwrap();
        tokio::time::timeout(Duration::from_secs(1), gate.entered.notified())
            .await
            .unwrap();
        if pause {
            host.hub.set_paused(host.source, true);
        } else {
            host.hub.unlink(host.source, peer);
            ctl.send(Ctl::PeerDown(peer)).unwrap();
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(500), rx)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert!(!gate.pending.load(Ordering::SeqCst));
        gate.complete.notify_one();
        let (reply, rx) = oneshot::channel();
        ctl.send(Ctl::Ready { reply }).unwrap();
        assert!(!rx.await.unwrap());
        ctl.send(Ctl::Stop).unwrap();
        task.await.unwrap();
        assert_eq!(ch.starts(), 1);
    }
}
#[tokio::test]
async fn s10_fix1_quiesce_preserves_started_open_cleanup_and_bounds_network() {
    let (cap, host, ctl, task, mut wire, _peer) = setup().await;
    host.block_leave.store(true, Ordering::SeqCst);
    activate(&cap).await;
    host.sent.notified().await;
    let entry = loop {
        if let Some(ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::EnterSpan { session, .. }),
            ..
        }) = wire.recv().await
        {
            break session;
        }
    };
    let (reply, rx) = oneshot::channel();
    ctl.send(Ctl::Quiesce { reply }).unwrap();
    tokio::time::timeout(Duration::from_millis(500), rx)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(cap.released.load(Ordering::SeqCst) > 0);
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
    assert!(host.cleanup.busy());
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
    let mut cleanup = host.cleanup.take();
    assert_eq!(cleanup.len(), 1);
    // Simulate State after releasing the layout lock; no network had been polled earlier.
    let job = tokio::spawn(cleanup.pop().unwrap());
    host.sent.notified().await;
    let leave = loop {
        if let Some(ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::Leave { session }),
            ..
        }) = wire.recv().await
        {
            break session;
        }
    };
    assert_eq!(leave, entry);
    assert!(host.cleanup.busy());
    assert!(
        tokio::time::timeout(Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(!host.cleanup.busy());
    assert_eq!(host.inflight.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn s10_fix1_engine_does_not_prepare_after_failed_stop() {
    let (cap, _) = crate::FakeCapture::new(vec![zone()]);
    let gate = Arc::new(PrepareGate::default());
    gate.stop_fails.store(true, Ordering::SeqCst);
    let (_cap, _host, ctl, task, _raw, _peer) = setup_with(Some(Box::new(PreparingCapture {
        inner: cap,
        gate: gate.clone(),
    })))
    .await;
    let (reply, rx) = oneshot::channel();
    ctl.send(Ctl::PrepareInventory { reply }).unwrap();
    let result = tokio::time::timeout(Duration::from_millis(100), rx).await;
    let started = gate.pending.load(Ordering::SeqCst);
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
    assert!(!started, "prepare must not proceed after stop failed");
    assert!(result.unwrap().unwrap().is_err());
}

#[tokio::test]
async fn s10_fix2_new_engine_releases_refused_activation_before_external_cleanup() {
    let (cap, host, ctl, task, mut wire, peer) = setup().await;
    host.block_leave.store(true, Ordering::SeqCst);
    activate(&cap).await;
    host.sent.notified().await;
    let (reply, response) = oneshot::channel();
    ctl.send(Ctl::Quiesce { reply }).unwrap();
    response.await.unwrap().unwrap();
    ctl.send(Ctl::Stop).unwrap();
    let adapter = task.await.unwrap();
    let cleanup = tokio::spawn(host.cleanup.take().pop().unwrap());
    host.sent.notified().await;
    while wire.try_recv().is_ok() {}
    assert!(host.cleanup.busy());
    let releases = cap.released.load(Ordering::SeqCst);
    let starts = cap.starts.load(Ordering::SeqCst);
    let (ctl, rx) = mpsc::unbounded_channel();
    let (notes, _) = mpsc::unbounded_channel();
    let task = tokio::spawn(run(Engine {
        cleanup: host.cleanup.clone(),
        expected_observation: None,
        catalog: Default::default(),
        route_plans: Default::default(),
        host: host.clone(),
        cap: adapter,
        targets: vec![Target {
            spec: BarrierSpec {
                zone: crate::adapter::ZoneRef::Index(0),
                edge: Edge::Right,
                from: 0.0,
                to: 1.0,
            },
            peer,
            peer_monitor: Some("m".into()),
            peer_edge: Edge::Left,
            local: (0.0, 1.0),
            remote: (0.0, 1.0),
        }],
        ctl: rx,
        notes,
        locked: false,
        paused: false,
        dead_corner_px: 0,
        peer_zones: Default::default(),
    }));
    wait(|| cap.starts.load(Ordering::SeqCst) > starts).await;
    let old = sender(&cap);
    // Queue a second activation in the same generation before the Engine is polled.
    for _ in 0..2 {
        old.try_send(CaptureEvent::Activated {
            barrier: 0,
            pos: 0.3,
            t_mono_us: 0,
        })
        .unwrap();
    }
    let released = tokio::time::timeout(Duration::from_millis(100), async {
        while cap.released.load(Ordering::SeqCst) == releases {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    let busy_at_release = host.cleanup.busy();
    let discarded = old.is_closed();
    let no_enter = wire.try_recv().is_err();
    // Complete the prior network cleanup only after checking immediate local release.
    host.complete.notify_one();
    cleanup.await.unwrap().unwrap();
    let (reply, confirmed) = oneshot::channel();
    ctl.send(Ctl::KnownInventory { reply }).unwrap();
    confirmed.await.unwrap().unwrap();
    let still_no_enter = wire.try_recv().is_err();
    if !released || !discarded {
        ctl.send(Ctl::Stop).unwrap();
        task.await.unwrap();
        assert!(released, "cleanup refusal retained physical capture");
        assert!(discarded, "refused generation remained admitted");
        return;
    }
    assert!(busy_at_release && no_enter && still_no_enter);
    assert_eq!(cap.released.load(Ordering::SeqCst), releases + 1);
    wait(|| cap.starts.load(Ordering::SeqCst) > starts + 1).await;
    activate(&cap).await;
    host.sent.notified().await;
    assert!(matches!(
        wire.recv().await,
        Some(ModuleInput::Reliable {
            msg: ModuleMsg::Input(InputMsg::EnterSpan { .. }),
            ..
        })
    ));
    ctl.send(Ctl::Stop).unwrap();
    task.await.unwrap();
}
