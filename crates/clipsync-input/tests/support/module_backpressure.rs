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
        organization: Default::default(),
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

async fn s10_state() -> (
    State,
    MemHub,
    PeerId,
    ReceiveContext,
    clipsync_proto::EnterRoutes,
    crate::FakeCaptureHandle,
    crate::FakeEmulateHandle,
    tempfile::TempDir,
) {
    use clipsync_proto::*;
    let hub = MemHub::new();
    let a = hub.add_node(
        "s10-source",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let b = hub.add_node(
        "s10-receiver",
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
            clipboard_in: false,
            clipboard_out: false,
            input_in: true,
        },
    );
    let mut st = state(b.host);
    let dir = tempfile::tempdir().unwrap();
    st.layout_path = Some(dir.path().join("layout.toml"));
    let observation = crate::inventory::windows_observation(&[clipsync_win::Monitor {
        friendly: "m".into(),
        device: "m".into(),
        path: "s10-private".into(),
        x: 0,
        y: 0,
        width: 800,
        height: 600,
        dpi: 96,
        primary: true,
    }])
    .unwrap();
    let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
    ch.set_monitor_observation(Some(observation.clone()));
    st.cap = Some(Box::new(cap));
    st.observe_inventory(observation).await.unwrap();
    let (emu, eh) = FakeEmulate::new();
    st.emu = Box::new(emu);
    let c = st.host.peer_input_context(a.id).unwrap();
    let context = ReceiveContext {
        connection: c.connection,
        input_epoch: c.input_epoch,
        stream: Some(1),
    };
    let local = st.organization.catalog.get(st.host.local_id()).unwrap();
    let entry = EnterRoutes {
        session: 7,
        inventories: InventoryRevisions {
            source: 100,
            target: local.inventory.rev,
        },
        monitor: local.inventory.monitors[0].uid,
        edge: Edge::Left,
        pos: 0.5,
        routes: SessionRoutes {
            first: ExitRoute {
                id: RouteId(0),
                monitor: local.inventory.monitors[0].uid,
                edge: Edge::Left,
                from: 0.05,
                to: 0.95,
            },
            second: None,
        },
    };
    (st, hub, a.id, context, entry, ch, eh, dir)
}
struct S10Gate {
    stage: u8,
    arrived: Notify,
    proceed: Notify,
    fail: bool,
}
struct S10Emulate {
    inner: crate::FakeEmulate,
    gate: Arc<S10Gate>,
}
#[async_trait::async_trait]
impl EmulateAdapter for S10Emulate {
    fn name(&self) -> &'static str {
        "s10-test"
    }
    async fn start(&mut self) -> Result<(), InputError> {
        if self.gate.stage == 0 {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
        }
        self.inner.start().await
    }
    async fn enter(&mut self, z: &Zone, zones: &[Zone], e: Edge, p: f32) -> Result<(), InputError> {
        if self.gate.stage == 1 {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
            if self.gate.fail {
                return Err(InputError::Other("entry failure".into()));
            }
        }
        self.inner.enter(z, zones, e, p).await
    }
    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        if self.gate.stage == 2 {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
            if self.gate.fail {
                return Err(InputError::Other("emit failure".into()));
            }
        }
        self.inner.emit(events).await
    }
    async fn release_all(&mut self) -> Result<(), InputError> {
        if self.gate.stage == 3 {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
        }
        self.inner.release_all().await
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.inner.stop().await
    }
}
#[tokio::test]
async fn s10_v4_revalidates_after_start_enter_emit_and_never_retains_pressed_state() {
    for stage in 0..3 {
        for pause in [false, true] {
            let (mut st, hub, from, context, entry, _ch, _eh, _dir) = s10_state().await;
            let gate = Arc::new(S10Gate {
                stage,
                arrived: Notify::new(),
                proceed: Notify::new(),
                fail: false,
            });
            let (inner, eh) = FakeEmulate::new();
            st.emu = Box::new(S10Emulate {
                inner,
                gate: gate.clone(),
            });
            let local = st.host.local_id();
            if stage == 2 {
                st.enter_routes(from, entry.clone(), context).await;
            }
            let operation = async {
                if stage == 2 {
                    st.on_datagram(
                        from,
                        InputDatagram {
                            session: 7,
                            seq: 0,
                            t_mono_us: 0,
                            events: vec![InputEvent::Key {
                                code: 42,
                                pressed: true,
                            }],
                        },
                        context,
                    )
                    .await;
                } else {
                    st.enter_routes(from, entry, context).await;
                }
            };
            let invalidate = async {
                gate.arrived.notified().await;
                if pause {
                    hub.set_paused(local, true);
                    hub.set_paused(local, false);
                } else {
                    hub.link(from, local, PeerPerms::default(), PeerPerms::default());
                    hub.link(
                        from,
                        local,
                        PeerPerms::default(),
                        PeerPerms {
                            input_in: true,
                            ..PeerPerms::default()
                        },
                    );
                }
                gate.proceed.notify_one();
            };
            tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(operation, invalidate);
            })
            .await
            .unwrap();
            assert!(st.incoming.get(&from).is_none_or(|i| !i.active));
            assert_eq!(eh.pressed_now(), 0);
            assert!(eh.release_calls() > 0);
        }
    }
}
#[tokio::test]
async fn s10_v4_generic_entry_and_emit_errors_are_terminal() {
    for stage in [1, 2] {
        let (mut st, _hub, from, context, entry, _ch, _eh, _dir) = s10_state().await;
        let gate = Arc::new(S10Gate {
            stage,
            arrived: Notify::new(),
            proceed: Notify::new(),
            fail: true,
        });
        let (inner, eh) = FakeEmulate::new();
        st.emu = Box::new(S10Emulate {
            inner,
            gate: gate.clone(),
        });
        if stage == 2 {
            st.enter_routes(from, entry.clone(), context).await;
        }
        gate.proceed.notify_one();
        if stage == 1 {
            st.enter_routes(from, entry, context).await;
        } else {
            st.on_datagram(
                from,
                InputDatagram {
                    session: 7,
                    seq: 0,
                    t_mono_us: 0,
                    events: vec![InputEvent::Key {
                        code: 42,
                        pressed: true,
                    }],
                },
                context,
            )
            .await;
        }
        assert!(st.incoming.get(&from).is_none_or(|i| !i.active));
        assert_eq!(eh.pressed_now(), 0);
    }
}
#[tokio::test]
async fn s10_v4_leave_before_enter_newer_enter_and_inventory_change_obey_frontier() {
    let (mut st, _hub, from, context, entry, ch, eh, _dir) = s10_state().await;
    st.on_msg(
        from,
        InputMsg::Leave { session: 7 },
        ReceiveContext {
            stream: Some(2),
            ..context
        },
    )
    .await;
    st.enter_routes(from, entry.clone(), context).await;
    assert!(!eh.started());
    st.enter_routes(
        from,
        entry.clone(),
        ReceiveContext {
            stream: Some(4),
            ..context
        },
    )
    .await;
    assert!(st.incoming[&from].active);
    st.enter_routes(
        from,
        clipsync_proto::EnterRoutes {
            session: 8,
            ..entry
        },
        ReceiveContext {
            stream: Some(3),
            ..context
        },
    )
    .await;
    assert_eq!(st.incoming[&from].session, 7);
    ch.set_monitor_observation(None);
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 0,
            t_mono_us: 0,
            events: vec![InputEvent::Key {
                code: 42,
                pressed: true,
            }],
        },
        context,
    )
    .await;
    assert_eq!(eh.pressed_now(), 0);
    assert!(!st.incoming.contains_key(&from));
}
#[tokio::test]
async fn s10_v4_suspend_closes_incoming_admission_until_validated_reload() {
    let (mut st, _hub, from, context, entry, _ch, eh, _dir) = s10_state().await;
    st.quiesce_organization().await.unwrap();
    st.enter_routes(from, entry, context).await;
    assert!(!eh.started());
}

async fn s10_transaction_state() -> (
    State,
    clipsync_proto::OrganizationRequest,
    crate::FakeCaptureHandle,
    tempfile::TempDir,
) {
    use clipsync_proto::*;
    let (mut st, _hub, remote, context, _entry, ch, _eh, dir) = s10_state().await;
    let me = st.host.local_id();
    let local = st.organization.catalog.get(me).unwrap();
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
                width: 800,
                height: 600,
            },
            scale: 1.0,
        }],
    };
    st.organization
        .catalog
        .insert(
            me,
            InventoryStamp {
                peer: remote,
                connection: context.connection,
                rev: 3,
            },
            remote_inventory,
        )
        .unwrap();
    st.organization
        .remote_epochs
        .insert(remote, context.input_epoch);
    let device = |peer| crate::LayoutDevice {
        peer,
        name: peer.short(),
        x: 0,
        y: 0,
        hotkey: None,
    };
    let layout = Layout {
        rev: 1,
        devices: vec![device(me), device(remote)],
        links: vec![],
        options: Default::default(),
    };
    layout.write(&dir.path().join("layout.toml")).unwrap();
    st.layout = Some(layout.clone());
    st.organization.document = Some(LayoutDocument {
        legacy: layout,
        organization: None,
    });
    let request = OrganizationRequest {
        base_rev: 1,
        inventories: st
            .organization
            .catalog
            .snapshot()
            .iter()
            .map(|v| v.stamp)
            .collect(),
        positions: vec![
            MonitorPosition {
                key: MonitorKey {
                    peer: me,
                    uid: local.inventory.monitors[0].uid,
                },
                x: 0.0,
                y: 0.0,
            },
            MonitorPosition {
                key: MonitorKey {
                    peer: remote,
                    uid: MonitorUid([3; 16]),
                },
                x: 800.0,
                y: 0.0,
            },
        ],
        replace_link_ids: vec![],
    };
    (st, request, ch, dir)
}
#[test]
fn s10_transaction_real_storage_faults_before_and_after_publication() {
    use crate::layout_storage::faults::{self, Stage};
    use clipsync_proto::{ActivationState, DurabilityState};
    for stage in [
        Stage::BeforeRename,
        Stage::ConfirmExclusive,
        Stage::ConfirmPublication,
    ] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (mut st, request, _ch, dir) = runtime.block_on(s10_transaction_state());
        let path = dir.path().join("layout.toml");
        let before = std::fs::read(&path).unwrap();
        let result = faults::during(stage, || runtime.block_on(st.apply_organization(request)));
        assert!(st.organization.suspended);
        assert!(st.cont.is_none());
        if stage == Stage::ConfirmPublication {
            let result = result.unwrap();
            assert_eq!(result.layout_rev, 2);
            assert_eq!(result.activation, ActivationState::Unconfirmed);
            assert_eq!(result.durability, DurabilityState::Unconfirmed);
            assert_eq!(LayoutDocument::read(&path).unwrap().unwrap().legacy.rev, 2);
        } else {
            assert!(result.is_err());
            assert_eq!(before, std::fs::read(path).unwrap());
        }
    }
}
struct S10FailCapture {
    inner: crate::FakeCapture,
    fail_stop: bool,
    pending_stop: bool,
    change_on_stop: Option<crate::FakeCaptureHandle>,
}
#[async_trait::async_trait]
impl CaptureAdapter for S10FailCapture {
    fn name(&self) -> &'static str {
        "s10-failure"
    }
    fn known_monitor_observation(&self) -> Result<crate::inventory::ObservedMonitors, InputError> {
        self.inner.known_monitor_observation()
    }
    fn known_zones(&self) -> Vec<Zone> {
        self.inner.known_zones()
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        self.inner.zones().await
    }
    async fn start(
        &mut self,
        _: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        Err(InputError::Other("start failure".into()))
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.inner.release().await
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        if self.pending_stop {
            std::future::pending::<()>().await;
        }
        tokio::task::yield_now().await;
        if let Some(ch) = &self.change_on_stop {
            ch.set_monitor_observation(None);
        }
        if self.fail_stop {
            Err(InputError::Other("stop failure".into()))
        } else {
            self.inner.stop().await
        }
    }
}
#[tokio::test]
async fn s10_quiesce_failure_or_inventory_change_prevents_write_rearm_failure_reports_saved() {
    use clipsync_proto::{ActivationState, DurabilityState};
    for mode in 0..3 {
        let (mut st, request, _old, dir) = s10_transaction_state().await;
        let path = dir.path().join("layout.toml");
        let before = std::fs::read(&path).unwrap();
        let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
        ch.set_monitor_observation(st.organization.observed.clone());
        st.cap = Some(Box::new(S10FailCapture {
            inner: cap,
            fail_stop: mode == 0,
            pending_stop: false,
            change_on_stop: (mode == 1).then_some(ch.clone()),
        }));
        let result = st.apply_organization(request).await;
        if mode < 2 {
            assert!(result.is_err());
            assert_eq!(before, std::fs::read(path).unwrap());
        } else {
            let applied = result.unwrap();
            assert_eq!(applied.layout_rev, 2);
            assert_eq!(applied.activation, ActivationState::Unconfirmed);
            assert_eq!(applied.durability, DurabilityState::Confirmed);
            assert_eq!(LayoutDocument::read(&path).unwrap().unwrap().legacy.rev, 2);
        }
        assert!(st.organization.suspended);
    }
}
#[tokio::test]
async fn s10_writer_lock_excludes_apply_and_validated_reload_recovers_after_failure() {
    let (mut st, request, _ch, dir) = s10_transaction_state().await;
    let storage = crate::layout_storage::LayoutStorage::new(dir.path());
    let guard = storage.try_lock().unwrap();
    assert_eq!(
        st.apply_organization(request.clone())
            .await
            .unwrap_err()
            .code,
        "busy"
    );
    drop(guard);
    std::fs::write(dir.path().join("layout.v1.backup.toml"), b"conflicting").unwrap();
    assert!(st.apply_organization(request.clone()).await.is_err());
    assert!(st.organization.suspended);
    std::fs::remove_file(dir.path().join("layout.v1.backup.toml")).unwrap();
    st.reload_layout().await.unwrap();
    assert!(!st.organization.suspended);
    let result = st.apply_organization(request).await.unwrap();
    assert_eq!(result.layout_rev, 2);
    st.stop_continuous().await;
}

#[tokio::test]
async fn s10_v4_full_network_queue_still_releases_and_closes_before_return() {
    let (mut st, _hub, from, context, entry, _ch, eh, _dir) = s10_state().await;
    st.enter_routes(from, entry, context).await;
    for _ in 0..32 {
        assert!(st.network.spawn(std::future::pending::<()>()));
    }
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 0,
            t_mono_us: 0,
            events: vec![InputEvent::Key {
                code: 42,
                pressed: true,
            }],
        },
        context,
    )
    .await;
    assert_eq!(eh.pressed_now(), 1);
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 1,
            t_mono_us: 0,
            events: vec![
                InputEvent::Motion {
                    dx: -2000.0,
                    dy: 0.0,
                },
                InputEvent::Key {
                    code: 43,
                    pressed: true,
                },
            ],
        },
        context,
    )
    .await;
    assert!(!st.incoming[&from].active);
    assert_eq!(eh.pressed_now(), 0);
    assert!(!eh.events().contains(&InputEvent::Key {
        code: 43,
        pressed: true
    }));
    assert!(eh.release_calls() > 0);
    st.network.stop().await;
}
#[test]
fn s10_recovery_published_unconfirmed_reports_visible_restored_revision_without_retry() {
    use crate::layout_storage::faults::{self, Stage};
    use clipsync_proto::{ActivationState, DurabilityState};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (mut st, request, _ch, dir) = runtime.block_on(s10_transaction_state());
    runtime.block_on(st.apply_organization(request)).unwrap();
    runtime.block_on(st.stop_continuous());
    let result = faults::during(Stage::ConfirmPublication, || {
        runtime.block_on(st.restore_layout_v1(2))
    })
    .unwrap();
    assert_eq!(result.layout_rev, 3);
    assert_eq!(result.activation, ActivationState::Unconfirmed);
    assert_eq!(result.durability, DurabilityState::Unconfirmed);
    let restored = LayoutDocument::read(&dir.path().join("layout.toml"))
        .unwrap()
        .unwrap();
    assert_eq!(restored.legacy.rev, 3);
    assert_eq!(st.layout.as_ref().map(|l| l.rev), Some(3));
    assert!(
        st.organization
            .document
            .as_ref()
            .is_some_and(|d| d.organization.is_none())
    );
    assert!(restored.organization.is_none());
    assert!(st.organization.suspended);
    assert!(dir.path().join("layout.v2-before-recovery.2.toml").exists());
}

async fn s10_copy_with_options(sender_dead: u32, local_dead: Option<u32>, bad: bool) {
    use clipsync_proto::*;
    let (mut st, request, ch, dir) = s10_transaction_state().await;
    let local = st.host.local_id();
    let mut observation = st.organization.observed.clone().unwrap();
    observation.space = CoordinateSpace::Logical;
    observation.capabilities = MonitorCapabilities {
        internal_edges: false,
        positioned_return: false,
        route_receiver: true,
    };
    ch.set_monitor_observation(Some(observation.clone()));
    st.observe_inventory(observation).await.unwrap();
    let remote = request
        .inventories
        .iter()
        .find(|s| s.peer != local)
        .unwrap();
    let mut remote_inventory = st.organization.catalog.get(remote.peer).unwrap().inventory;
    remote_inventory.space = CoordinateSpace::Physical;
    remote_inventory.capabilities = MonitorCapabilities {
        internal_edges: true,
        positioned_return: true,
        route_receiver: true,
    };
    st.organization
        .catalog
        .insert(local, *remote, remote_inventory)
        .unwrap();
    let mut current = st.organization.document.clone().unwrap();
    current.legacy.options.dead_corner_px =
        local_dead.unwrap_or(crate::layout::DEFAULT_DEAD_CORNER_PX);
    if local_dead.is_some() {
        current
            .legacy
            .write(&dir.path().join("layout.toml"))
            .unwrap();
        st.layout = Some(current.legacy.clone());
        st.organization.document = Some(current.clone());
    } else {
        std::fs::remove_file(dir.path().join("layout.toml")).unwrap();
        st.layout = None;
        st.organization.document = None;
    }
    let before = std::fs::read(dir.path().join("layout.toml")).ok();
    let mut offered_base = current.clone();
    offered_base.legacy.options.dead_corner_px = sender_dead;

    let views: Vec<_> = st
        .organization
        .catalog
        .snapshot()
        .iter()
        .map(|v| crate::layout_geometry::InventoryView {
            peer: v.stamp.peer,
            inventory: v.inventory.clone(),
            aliases: vec![],
        })
        .collect();
    let compiled = crate::layout_geometry::compile_organization(
        &offered_base,
        &crate::layout_geometry::OrganizationDraft {
            positions: request.positions,
            replace_link_ids: vec![],
        },
        &views,
    )
    .unwrap();
    let org = compiled.document.organization.unwrap();
    let mut offer = LayoutOfferV4 {
        rev: 55,
        devices: layout_offer(&current.legacy).devices,
        positions: org.positions.clone(),
        links: org.links.clone(),
        legacy_links: vec![],
    };
    if bad {
        offer.links[0].a.from += 0.03125;
    }
    st.organization.pending = Some(PendingMonitorOffer {
        from: remote.peer,
        connection: remote.connection,
        offer,
    });
    let result = st
        .answer_offer_v4(
            remote.peer,
            remote.connection,
            55,
            if local_dead.is_some() { 1 } else { 0 },
            true,
        )
        .await;
    if bad {
        assert!(result.is_err());
        assert_eq!(std::fs::read(dir.path().join("layout.toml")).ok(), before);
        return;
    }
    let applied = result.unwrap().unwrap();
    assert_eq!(applied.layout_rev, if local_dead.is_some() { 2 } else { 1 });
    assert_eq!(applied.durability, DurabilityState::Confirmed);
    assert_eq!(applied.activation, ActivationState::Unconfirmed);
    assert!(!ch.started());
    assert!(!st.organization.ingress_suspended);
    let saved = LayoutDocument::read(&dir.path().join("layout.toml"))
        .unwrap()
        .unwrap();
    let saved_org = saved.organization.unwrap();
    assert_eq!(saved_org.positions, org.positions);
    assert_eq!(saved_org.links, org.links);
    assert_eq!(saved.legacy.options, current.legacy.options);
    let local_inv = st.organization.catalog.get(local).unwrap();
    let context = st.host.peer_input_context(remote.peer).unwrap();
    let entry = EnterRoutes {
        session: 11,
        inventories: InventoryRevisions {
            source: 999,
            target: local_inv.stamp.rev,
        },
        monitor: local_inv.inventory.monitors[0].uid,
        edge: Edge::Left,
        pos: 0.5,
        routes: SessionRoutes {
            first: ExitRoute {
                id: RouteId(0),
                monitor: local_inv.inventory.monitors[0].uid,
                edge: Edge::Left,
                from: 0.1,
                to: 0.9,
            },
            second: None,
        },
    };
    st.enter_routes(
        remote.peer,
        entry,
        ReceiveContext {
            connection: context.connection,
            input_epoch: context.input_epoch,
            stream: Some(7),
        },
    )
    .await;
    assert!(st.incoming[&remote.peer].active);
}

#[tokio::test]
async fn s10_linux_receiver_accepts_complete_copy_without_enabling_local_capture() {
    s10_copy_with_options(8, Some(8), false).await;
}
#[tokio::test]
async fn s10_fix1_offer_preserves_effective_endpoints_with_different_local_options() {
    for sender in [0, 8, 32] {
        for local in [None, Some(0), Some(8), Some(32)] {
            s10_copy_with_options(sender, local, false).await;
        }
    }
}
#[tokio::test]
async fn s10_fix1_offer_still_rejects_arbitrary_subset_of_valid_contact() {
    for sender in [0, 8, 32] {
        s10_copy_with_options(sender, Some(16), true).await;
    }
}

#[tokio::test]
async fn s10_fix1_quiesce_releases_incoming_before_capture_stop_can_fail() {
    let (mut st, _hub, from, context, entry, _ch, eh, _dir) = s10_state().await;
    st.enter_routes(from, entry, context).await;
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 0,
            t_mono_us: 0,
            events: vec![InputEvent::Key {
                code: 42,
                pressed: true,
            }],
        },
        context,
    )
    .await;
    assert_eq!(eh.pressed_now(), 1);
    let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
    ch.set_monitor_observation(st.organization.observed.clone());
    st.cap = Some(Box::new(S10FailCapture {
        inner: cap,
        fail_stop: true,
        pending_stop: false,
        change_on_stop: None,
    }));
    assert!(st.quiesce_organization().await.is_err());
    assert_eq!(eh.pressed_now(), 0);
    assert!(!st.incoming[&from].active);
}

include!("prepare_capture.rs");
#[tokio::test]
async fn s10_fix1_prepare_releases_incoming_and_returns_control_before_dialog() {
    let (mut st, _hub, from, context, entry, _ch, eh, _dir) = s10_state().await;
    st.enter_routes(from, entry, context).await;
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 0,
            t_mono_us: 0,
            events: vec![InputEvent::Key {
                code: 42,
                pressed: true,
            }],
        },
        context,
    )
    .await;
    let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
    ch.set_monitor_observation(st.organization.observed.clone());
    let gate = Arc::new(PrepareGate::default());
    st.cap = Some(Box::new(PreparingCapture {
        inner: cap,
        gate: gate.clone(),
    }));
    let (reply, _rx) = oneshot::channel();
    let mut task = None;
    let ready = tokio::time::timeout(
        Duration::from_millis(100),
        st.command(ModuleCommand::InputPrepareInventory, reply, &mut task),
    )
    .await
    .is_ok();
    assert!(ready, "State remained blocked on prepare");
    assert_eq!(eh.pressed_now(), 0);
    assert!(st.organization.catalog.get(st.host.local_id()).is_none());
}

#[tokio::test]
async fn s10_fix1_state_pending_prepare_cancels_on_pause_revoke_or_quiesce_without_late_publish() {
    for action in 0..3 {
        let (mut st, hub, from, _context, _entry, _old, _eh, _dir) = s10_state().await;
        let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
        ch.set_monitor_observation(st.organization.observed.clone());
        let gate = Arc::new(PrepareGate::default());
        st.cap = Some(Box::new(PreparingCapture {
            inner: cap,
            gate: gate.clone(),
        }));
        let (reply, rx) = oneshot::channel();
        st.begin_preparation(Some(reply)).await;
        assert!(
            tokio::time::timeout(
                Duration::from_millis(10),
                &mut st.organization.preparation.as_mut().unwrap().future
            )
            .await
            .is_err()
        );
        assert!(gate.pending.load(Ordering::SeqCst));
        match action {
            0 => st.apply_pause(true).await,
            1 => {
                hub.unlink(st.host.local_id(), from);
                st.handle(ModuleInput::PeerDown(from)).await;
            }
            _ => {
                st.quiesce_organization().await.unwrap();
            }
        }
        assert!(matches!(rx.await.unwrap(), ModuleReply::Error(_)));
        assert!(st.cap.is_some());
        assert!(st.organization.preparation.is_none());
        assert!(!gate.pending.load(Ordering::SeqCst));
        assert!(gate.cancelled.load(Ordering::SeqCst));
        gate.complete.notify_one(); // A late portal answer cannot complete the dropped preparation.
        st.refresh_known_inventory().await;
        assert!(st.organization.catalog.get(st.host.local_id()).is_none());
        assert!(st.cont.is_none());
        assert_eq!(ch.starts(), 0);
    }
}
#[tokio::test]
async fn s10_fix1_state_checks_authority_after_ready_prepare_before_publication() {
    let (mut st, hub, from, _context, _entry, _old, _eh, _dir) = s10_state().await;
    let (cap, ch) = crate::FakeCapture::new(vec![zone()]);
    ch.set_monitor_observation(st.organization.observed.clone());
    let gate = Arc::new(PrepareGate::default());
    st.cap = Some(Box::new(PreparingCapture {
        inner: cap,
        gate: gate.clone(),
    }));
    let (reply, rx) = oneshot::channel();
    st.begin_preparation(Some(reply)).await;
    gate.complete.notify_one();
    let done = st
        .organization
        .preparation
        .as_mut()
        .unwrap()
        .future
        .as_mut()
        .await;
    hub.unlink(st.host.local_id(), from);
    st.finish_preparation(done, false).await;
    assert!(matches!(rx.await.unwrap(), ModuleReply::Error(_)));
    assert!(st.cap.is_some());
    assert!(st.organization.catalog.get(st.host.local_id()).is_none());
}
#[tokio::test]
async fn s10_fix1_module_shutdown_joins_pending_prepare_without_dialog_completion() {
    let hub = MemHub::new();
    let node = hub.add_node(
        "source",
        ModuleSet {
            clipboard: false,
            input: true,
        },
    );
    let (cap, _ch) = crate::FakeCapture::new(vec![zone()]);
    let (emu, _) = crate::FakeEmulate::new();
    let gate = Arc::new(PrepareGate::default());
    let module = build_with_adapters(
        InputConfig::default(),
        Box::new(PreparingCapture {
            inner: cap,
            gate: gate.clone(),
        }),
        Box::new(emu),
    );
    let (tx, rx) = mpsc::channel(8);
    let task = tokio::spawn(module.run(node.host, rx));
    let (reply, result) = oneshot::channel();
    tx.send(ModuleInput::Command {
        cmd: ModuleCommand::InputPrepareInventory,
        reply,
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(1), gate.entered.notified())
        .await
        .unwrap();
    tx.send(ModuleInput::Shutdown).await.unwrap();
    tokio::time::timeout(Duration::from_millis(500), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(result.await.unwrap(), ModuleReply::Error(_)));
    assert!(!gate.pending.load(Ordering::SeqCst));
}

#[tokio::test]
async fn s10_fix1_timed_out_join_keeps_the_capture_task_owned() {
    let (mut st, _hub, _from, _context, _entry, _old, _eh, _dir) = s10_state().await;
    let cap = st.cap.take().unwrap();
    let gate = Arc::new(Notify::new());
    let complete = gate.clone();
    let (ctl, _rx) = mpsc::unbounded_channel();
    st.cont = Some(Cont {
        ctl,
        task: tokio::spawn(async move {
            complete.notified().await;
            cap
        }),
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(10), st.stop_continuous())
            .await
            .is_err()
    );
    assert!(!st.cont.as_ref().unwrap().task.is_finished());
    assert!(st.cap.is_none());
    gate.notify_one();
    st.stop_continuous().await;
    assert!(st.cont.is_none());
    assert!(st.cap.is_some());
}

#[tokio::test]
async fn s10_fix1_quiesce_timeout_still_releases_incoming_and_keeps_adapter_owned() {
    let (mut st, _hub, from, context, entry, _ch, eh, _dir) = s10_state().await;
    st.enter_routes(from, entry, context).await;
    st.on_datagram(
        from,
        InputDatagram {
            session: 7,
            seq: 0,
            t_mono_us: 0,
            events: vec![InputEvent::Button {
                code: 272,
                pressed: true,
            }],
        },
        context,
    )
    .await;
    assert_eq!(eh.pressed_now(), 1);
    let (cap, _) = crate::FakeCapture::new(vec![zone()]);
    st.cap = Some(Box::new(S10FailCapture {
        inner: cap,
        fail_stop: false,
        pending_stop: true,
        change_on_stop: None,
    }));
    assert_eq!(
        st.quiesce_organization().await.unwrap_err().code,
        "quiesce_timeout"
    );
    assert_eq!(eh.pressed_now(), 0);
    assert!(!st.incoming[&from].active);
    assert!(st.cap.is_some());
}

// The proxy holds only KnownInventory; preparation and confirmation use the real Engine.
#[tokio::test]
async fn s10_fix2_preparation_cancelled_during_engine_confirmation() {
    for pause in [false, true] {
        let (mut st, hub, from, _, _, _, _, _dir) = s10_state().await;
        let host = st.host.clone();
        let (engine_ctl, engine_rx) = mpsc::unbounded_channel();
        let (proxy_ctl, mut proxy_rx) = mpsc::unbounded_channel();
        let (notes, _notes_rx) = mpsc::unbounded_channel();
        let gate = Arc::new(S10Gate {
            stage: 0,
            arrived: Notify::new(),
            proceed: Notify::new(),
            fail: false,
        });
        let proxy_gate = gate.clone();
        let proxy = tokio::spawn(async move {
            while let Some(command) = proxy_rx.recv().await {
                if matches!(&command, Ctl::KnownInventory { .. }) {
                    proxy_gate.arrived.notify_one();
                    proxy_gate.proceed.notified().await;
                }
                let stop = matches!(&command, Ctl::Stop);
                engine_ctl.send(command).unwrap();
                if stop {
                    break;
                }
            }
        });
        let cap = st.cap.take().unwrap();
        st.cont = Some(Cont {
            ctl: proxy_ctl,
            task: tokio::spawn(capture::run(capture::Engine {
                cleanup: Default::default(),
                expected_observation: None,
                catalog: Default::default(),
                route_plans: Default::default(),
                host: host.clone(),
                cap,
                targets: vec![],
                ctl: engine_rx,
                notes,
                locked: false,
                paused: false,
                dead_corner_px: 0,
                peer_zones: Default::default(),
            })),
        });
        let (reply, response) = oneshot::channel();
        st.begin_preparation(Some(reply)).await;
        let done = st
            .organization
            .preparation
            .as_mut()
            .unwrap()
            .future
            .as_mut()
            .await;
        tokio::join!(st.finish_preparation(done, false), async {
            gate.arrived.notified().await;
            if pause {
                host.request_pause(true);
            } else {
                hub.unlink(host.local_id(), from);
            }
            gate.proceed.notify_one();
        });
        let response = response.await.unwrap();
        st.stop_continuous().await;
        proxy.await.unwrap();
        assert!(
            matches!(response, ModuleReply::Error(_)),
            "late confirmation published: {response:?}"
        );
        assert!(st.organization.inventory_blocked);
        assert!(st.organization.preparation.is_none());
        assert!(st.organization.catalog.get(host.local_id()).is_none());
        st.refresh_known_inventory().await;
        assert!(st.organization.catalog.get(host.local_id()).is_none());
    }
}

#[tokio::test]
async fn s10_fix2_preparation_cancelled_during_emulation_release() {
    for pause in [false, true] {
        let (mut st, hub, from, _, _, _, _, _dir) = s10_state().await;
        let host = st.host.clone();
        let (reply, response) = oneshot::channel();
        st.begin_preparation(Some(reply)).await;
        let done = st
            .organization
            .preparation
            .as_mut()
            .unwrap()
            .future
            .as_mut()
            .await;
        let gate = Arc::new(S10Gate {
            stage: 3,
            arrived: Notify::new(),
            proceed: Notify::new(),
            fail: false,
        });
        let (emu, _) = FakeEmulate::new();
        st.emu = Box::new(S10Emulate {
            inner: emu,
            gate: gate.clone(),
        });
        st.emu_started = true;
        tokio::join!(st.finish_preparation(done, false), async {
            gate.arrived.notified().await;
            if pause {
                host.request_pause(true);
            } else {
                hub.unlink(host.local_id(), from);
            }
            gate.proceed.notify_one();
        });
        let response = response.await.unwrap();
        assert!(
            matches!(response, ModuleReply::Error(_)),
            "late release published: {response:?}"
        );
        assert!(st.organization.inventory_blocked);
        assert!(st.organization.preparation.is_none());
        assert!(st.organization.catalog.get(host.local_id()).is_none());
        st.refresh_known_inventory().await;
        assert!(st.organization.catalog.get(host.local_id()).is_none());
    }
}
