//! I1: Mesh e InputModule reais; somente adaptadores de sistema são falsos.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_core::{
    Identity, PeerEntry, Peers,
    transport::{Mesh, MeshOptions},
};
use clipsync_input::{
    EmulateAdapter, Endpoint, FakeCapture, FakeEmulate, InputConfig, InputError, Layout,
    LayoutDevice, LayoutOptions, Link, Zone, build_with_layout,
};
use clipsync_proto::*;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{Notify, mpsc};

#[derive(Default)]
struct Gate {
    entered: Notify,
    release: Notify,
}
struct GatedEmulate {
    inner: FakeEmulate,
    gate: Arc<Gate>,
}
type EmuFuture<'a> = Pin<Box<dyn Future<Output = Result<(), InputError>> + Send + 'a>>;
impl EmulateAdapter for GatedEmulate {
    fn name(&self) -> &'static str {
        "gated"
    }
    fn start<'a, 'f>(&'a mut self) -> EmuFuture<'f>
    where
        'a: 'f,
        Self: 'f,
    {
        Box::pin(async move {
            self.gate.entered.notify_one();
            self.gate.release.notified().await;
            self.inner.start().await
        })
    }
    fn emit<'a, 'b, 'f>(&'a mut self, events: &'b [InputEvent]) -> EmuFuture<'f>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        self.inner.emit(events)
    }
    fn release_all<'a, 'f>(&'a mut self) -> EmuFuture<'f>
    where
        'a: 'f,
        Self: 'f,
    {
        self.inner.release_all()
    }
    fn stop<'a, 'f>(&'a mut self) -> EmuFuture<'f>
    where
        'a: 'f,
        Self: 'f,
    {
        self.inner.stop()
    }
}
async fn until(why: &str, check: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect(why);
}
fn peers(id: PeerId, addr: Option<std::net::SocketAddr>) -> Peers {
    let mut p = Peers::default();
    p.upsert(PeerEntry {
        id,
        name: "peer".into(),
        addr,
        paired_at: "test".into(),
        perms: PeerPerms {
            input_in: true,
            ..Default::default()
        },
    });
    p
}
async fn next_enter(rx: &mut mpsc::Receiver<ModuleInput>) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let event = rx.recv().await.expect("peer queue closed before Enter");
            if matches!(
                event,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(
                        InputMsg::Enter { .. }
                            | InputMsg::EnterAt { .. }
                            | InputMsg::EnterSpan { .. }
                    ),
                    ..
                }
            ) {
                break;
            }
        }
    })
    .await
    .unwrap();
}
#[derive(Clone, Copy)]
enum Cancellation {
    Pause,
    Resume,
    Coalesced,
    PeerDown,
}
async fn pause_with_full_queue(mode: Cancellation, loopback: bool) {
    let coalesced = matches!(mode, Cancellation::Coalesced);
    let disconnect = matches!(mode, Cancellation::PeerDown);
    let resume_before_drain = !matches!(mode, Cancellation::Pause);
    let a = Identity::generate().unwrap();
    let aid = a.id();
    let b = Identity::generate().unwrap();
    let bid = b.id();
    let (tx, rx) = mpsc::channel(1024);
    let local = Mesh::start(MeshOptions {
        identity: a,
        device_name: "a".into(),
        listen: "127.0.0.1:0".parse().unwrap(),
        modules: HashMap::from([(ModuleKind::Input, tx.clone())]),
        peers: peers(bid, None),
        paused: false,
    })
    .unwrap();
    let (btx, mut brx) = mpsc::channel(1024);
    let remote = Mesh::start(MeshOptions {
        identity: b,
        device_name: "b".into(),
        listen: "127.0.0.1:0".parse().unwrap(),
        modules: HashMap::from([(ModuleKind::Input, btx)]),
        peers: peers(aid, Some(local.local_addr().unwrap())),
        paused: false,
    })
    .unwrap();
    until("QUIC connection", || {
        local
            .host()
            .peer_input_context(bid)
            .is_some_and(|c| c.connected)
    })
    .await;
    let dir = tempfile::tempdir().unwrap();
    Layout {
        rev: 1,
        options: LayoutOptions { dead_corner_px: 0 },
        devices: vec![
            LayoutDevice {
                peer: aid,
                name: "a".into(),
                x: 0,
                y: 0,
                hotkey: None,
            },
            LayoutDevice {
                peer: bid,
                name: "b".into(),
                x: 1,
                y: 0,
                hotkey: None,
            },
        ],
        links: vec![Link {
            id: "ab".into(),
            a: Endpoint {
                peer: aid,
                monitor: "m".into(),
                edge: Edge::Right,
                from: 0.0,
                to: 1.0,
            },
            b: Endpoint {
                peer: bid,
                monitor: "m".into(),
                edge: Edge::Left,
                from: 0.0,
                to: 1.0,
            },
        }],
    }
    .write(&dir.path().join("layout.toml"))
    .unwrap();
    let (cap, handle) = FakeCapture::new(vec![Zone {
        index: 0,
        name: Some("m".into()),
        x: 0,
        y: 0,
        width: 800,
        height: 600,
        scale: 1.0,
    }]);
    handle.set_hooks(true);
    let (emu, emu_handle) = FakeEmulate::new();
    let gate = Arc::new(Gate::default());
    let module = build_with_layout(
        InputConfig::default(),
        Box::new(cap),
        Box::new(GatedEmulate {
            inner: emu,
            gate: gate.clone(),
        }),
        dir.path(),
    )
    .unwrap();
    let task = tokio::spawn(module.run(local.host(), rx));
    until("capture start", || handle.started()).await;
    let loop_result = if loopback {
        let starts = handle.starts();
        let (reply, done) = tokio::sync::oneshot::channel();
        tx.send(ModuleInput::Command {
            cmd: ModuleCommand::InputLoopback {
                to: bid,
                seconds: 60,
            },
            reply,
        })
        .await
        .unwrap();
        until("loopback owns a new producer", || handle.starts() > starts).await;
        Some(done)
    } else {
        None
    };
    handle.activate(0, 0.5);
    next_enter(&mut brx).await;
    remote
        .host()
        .send_reliable(
            aid,
            ModuleMsg::Input(InputMsg::Enter {
                session: 7,
                edge: Edge::Left,
                pos: 0.5,
            }),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), gate.entered.notified())
        .await
        .unwrap();
    // Authenticated datagrams, not a second/helper queue, saturate the module's real ingress.
    let mut seq = 0;
    tokio::time::timeout(Duration::from_secs(3), async {
        while tx.capacity() != 0 {
            remote
                .host()
                .send_datagram(
                    aid,
                    InputDatagram {
                        session: 7,
                        seq,
                        t_mono_us: mono_us(),
                        events: vec![InputEvent::Key {
                            code: 30,
                            pressed: true,
                        }],
                    },
                )
                .unwrap();
            seq += 1;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(tx.capacity(), 0);
    let releases = handle.released();
    let starts = handle.starts();
    // No yield before changing authority: these belong to the old producer generation.
    if !disconnect {
        for _ in 0..256 {
            handle.push(InputEvent::Key {
                code: 99,
                pressed: true,
            });
        }
    }
    if disconnect {
        remote.shutdown().await;
    } else if coalesced {
        tokio::join!(local.set_paused(true), local.set_paused(false));
    } else {
        local.set_paused(true).await;
    }
    // Keep emu.start pending across the original broadcast's 2 second expiry.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let cancelled_while_pending = handle.released() > releases
        && if (coalesced || disconnect) && !loopback {
            handle.starts() > starts
        } else {
            !handle.started()
        };
    if resume_before_drain && !coalesced && !disconnect {
        local.set_paused(false).await;
    }
    gate.release.notify_one();
    let (reply, done) = tokio::sync::oneshot::channel();
    tx.send(ModuleInput::Command {
        cmd: ModuleCommand::Status,
        reply,
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), done)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        emu_handle.pressed_now(),
        0,
        "old input cannot enter after pause epoch"
    );
    // A delayed compatibility notification must not restore its obsolete bool.
    tx.send(ModuleInput::Paused(!local.paused())).await.unwrap();
    let (reply, done) = tokio::sync::oneshot::channel();
    tx.send(ModuleInput::Command {
        cmd: ModuleCommand::Status,
        reply,
    })
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(3), done)
        .await
        .unwrap()
        .unwrap();
    if local.paused() {
        assert!(!handle.started(), "stale Resume rearmed capture");
    }
    if let Some(done) = loop_result {
        let reply = tokio::time::timeout(Duration::from_secs(3), done)
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(reply, ModuleReply::Error(_)),
            "loopback cancellation cannot succeed"
        );
    }
    if !resume_before_drain {
        local.set_paused(false).await;
    }
    until("fresh capture after resume", || handle.started()).await;
    // Old producer/sessions must not produce an Enter just from Resume.
    while let Ok(msg) = brx.try_recv() {
        assert!(
            !matches!(
                msg,
                ModuleInput::Reliable {
                    msg: ModuleMsg::Input(
                        InputMsg::Enter { .. }
                            | InputMsg::EnterAt { .. }
                            | InputMsg::EnterSpan { .. }
                    ),
                    ..
                }
            ),
            "old session reopened"
        );
        assert!(
            !matches!(msg, ModuleInput::Datagram { .. }),
            "old producer data survived pause"
        );
    }
    if cancelled_while_pending && !disconnect {
        handle.activate(0, 0.5);
        next_enter(&mut brx).await;
    }
    tx.send(ModuleInput::Shutdown).await.unwrap();
    tokio::time::timeout(Duration::from_secs(4), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    remote.shutdown().await;
    local.shutdown().await;
    assert!(
        cancelled_while_pending,
        "pause must release and stop capture while emu.start and full Input queue remain pending"
    );
}
#[tokio::test]
async fn pause_cancels_capture_with_full_input_queue_and_pending_emulator() {
    pause_with_full_queue(Cancellation::Pause, false).await;
}
#[tokio::test]
async fn pause_resume_before_draining_cannot_revive_old_capture() {
    pause_with_full_queue(Cancellation::Resume, false).await;
}

#[tokio::test]
async fn coalesced_pause_resume_invalidates_capture_before_either_notification_is_consumed() {
    pause_with_full_queue(Cancellation::Coalesced, false).await;
}

#[tokio::test]
async fn loopback_pause_cancels_while_emulator_and_data_queue_are_pending() {
    pause_with_full_queue(Cancellation::Pause, true).await;
}
#[tokio::test]
async fn loopback_coalesced_pause_resume_cannot_continue_old_session() {
    pause_with_full_queue(Cancellation::Coalesced, true).await;
}
#[tokio::test]
async fn peer_down_cancels_capture_while_module_queue_is_full() {
    pause_with_full_queue(Cancellation::PeerDown, false).await;
}
