//! S9: revocation and reordered local delivery, without physical adapters.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::{
    EmulateAdapter, FakeCapture, FakeEmulate, FakeEmulateHandle, InputConfig, InputError, Zone,
    build_with_adapters,
};
use clipsync_proto::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
const INPUT: ModuleSet = ModuleSet {
    clipboard: false,
    input: true,
};
const GRANT: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: true,
};
struct Rig {
    hub: MemHub,
    a: PeerId,
    b: PeerId,
    host: Arc<dyn ModuleHost>,
    wire: mpsc::Receiver<ModuleInput>,
    replies: mpsc::Receiver<ModuleInput>,
    tx: mpsc::Sender<ModuleInput>,
    emu: FakeEmulateHandle,
    task: tokio::task::JoinHandle<Result<(), ModuleError>>,
}
impl Rig {
    async fn new(version: u16) -> Self {
        Self::gated(version, None).await
    }
    async fn gated(version: u16, gate: Option<Arc<Gate>>) -> Self {
        let hub = MemHub::new();
        let a = hub.add_node("a", INPUT);
        let b = hub.add_node("b", INPUT);
        hub.set_remote_version(a.id, version);
        let (cap, _) = FakeCapture::new(vec![Zone {
            index: 0,
            name: Some("m".into()),
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            scale: 1.0,
        }]);
        let (emu, handle) = FakeEmulate::new();
        let emu: Box<dyn EmulateAdapter> = if let Some(gate) = gate {
            Box::new(GatedEmulate { inner: emu, gate })
        } else {
            Box::new(emu)
        };
        let module = build_with_adapters(InputConfig::default(), Box::new(cap), emu);
        let (tx, rx) = mpsc::channel(64);
        let task = tokio::spawn(module.run(b.host, rx));
        let mut r = Self {
            hub,
            a: a.id,
            b: b.id,
            host: a.host,
            wire: b.rx,
            replies: a.rx,
            tx,
            emu: handle,
            task,
        };
        r.grant(true);
        r.forward().await;
        r.settle().await;
        r
    }
    fn grant(&self, allow: bool) {
        self.hub.link(
            self.a,
            self.b,
            PeerPerms::default(),
            if allow { GRANT } else { PeerPerms::default() },
        );
    }
    async fn next(&mut self) -> ModuleInput {
        tokio::time::timeout(Duration::from_secs(2), self.wire.recv())
            .await
            .unwrap()
            .unwrap()
    }
    async fn forward(&mut self) {
        let msg = self.next().await;
        self.tx.send(msg).await.unwrap();
    }
    async fn settle(&self) {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ModuleInput::Command {
                cmd: ModuleCommand::Status,
                reply: tx,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .unwrap()
            .unwrap();
    }
    async fn send(&self, m: InputMsg) {
        self.host
            .send_reliable(self.b, ModuleMsg::Input(m))
            .await
            .unwrap();
    }
    async fn enter(&mut self, session: u32) {
        self.send(InputMsg::Enter {
            session,
            edge: Edge::Left,
            pos: 0.2,
        })
        .await;
        self.forward().await;
        self.settle().await;
    }
    async fn press(&mut self, session: u32) {
        self.host
            .send_datagram(
                self.b,
                InputDatagram {
                    session,
                    seq: 0,
                    t_mono_us: mono_us(),
                    events: vec![
                        InputEvent::Key {
                            code: 42,
                            pressed: true,
                        },
                        InputEvent::Button {
                            code: 0x110,
                            pressed: true,
                        },
                    ],
                },
            )
            .unwrap();
        self.forward().await;
        self.settle().await;
    }
    async fn stop(self) {
        self.tx.send(ModuleInput::Shutdown).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
#[tokio::test]
async fn revoking_input_releases_key_and_button_for_every_protocol() {
    for version in [1, 2, 3] {
        let mut r = Rig::new(version).await;
        r.enter(7).await;
        r.press(7).await;
        assert_eq!(r.emu.pressed_now(), 2);
        r.grant(false);
        r.forward().await;
        r.settle().await;
        assert_eq!(
            r.emu.pressed_now(),
            0,
            "revocation must release locally for protocol {version}"
        );
        let response = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                match r.replies.recv().await.unwrap() {
                    ModuleInput::Reliable {
                        msg:
                            ModuleMsg::Input(InputMsg::Refused {
                                session: 7,
                                reason: RejectReason::NotAuthorized,
                            }),
                        ..
                    } => break 3,
                    ModuleInput::Rejected {
                        reason: RejectReason::NotAuthorized,
                        ..
                    } => break 1,
                    _ => {}
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(response, if version >= 3 { 3 } else { 1 });
        r.stop().await;
    }
}
#[tokio::test]
async fn queued_enter_cannot_survive_revocation_and_regrant() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Enter {
        session: 7,
        edge: Edge::Left,
        pos: 0.2,
    })
    .await;
    let old = r.next().await;
    r.grant(false);
    r.grant(true);
    r.forward().await;
    r.forward().await;
    r.settle().await;
    r.tx.send(old).await.unwrap();
    r.settle().await;
    assert!(
        !r.emu.started(),
        "old admitted Enter must not start emulation in a new authorization epoch"
    );
    r.enter(8).await;
    r.press(8).await;
    assert_eq!(r.emu.pressed_now(), 2);
    r.stop().await;
}
#[tokio::test]
async fn live_host_authority_wins_before_peer_up_is_delivered() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Enter {
        session: 7,
        edge: Edge::Left,
        pos: 0.2,
    })
    .await;
    let old = r.next().await;
    r.grant(false);
    r.tx.send(old).await.unwrap();
    r.settle().await;
    assert!(
        !r.emu.started(),
        "host already revoked before cached PeerUp arrived"
    );
    r.stop().await;
}
#[tokio::test]
async fn leave_received_before_its_enter_prevents_late_open() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Enter {
        session: 7,
        edge: Edge::Left,
        pos: 0.2,
    })
    .await;
    let enter = r.next().await;
    r.send(InputMsg::Leave { session: 7 }).await;
    r.forward().await;
    r.settle().await;
    r.tx.send(enter).await.unwrap();
    r.settle().await;
    assert!(!r.emu.started(), "late Enter cannot resurrect after Leave");
    r.stop().await;
}
#[tokio::test]
async fn earlier_enter_cannot_replace_newer_session() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Enter {
        session: 7,
        edge: Edge::Left,
        pos: 0.2,
    })
    .await;
    let old = r.next().await;
    r.enter(8).await;
    r.tx.send(old).await.unwrap();
    r.settle().await;
    r.press(8).await;
    assert_eq!(
        r.emu.pressed_now(),
        2,
        "newest session must still accept input"
    );
    r.stop().await;
}

struct Gate {
    at_start: bool,
    arrived: tokio::sync::Notify,
    proceed: tokio::sync::Notify,
}
struct GatedEmulate {
    inner: FakeEmulate,
    gate: Arc<Gate>,
}
#[async_trait::async_trait]
impl EmulateAdapter for GatedEmulate {
    fn name(&self) -> &'static str {
        "gated"
    }
    async fn start(&mut self) -> Result<(), InputError> {
        if self.gate.at_start {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
        }
        self.inner.start().await
    }
    async fn emit(&mut self, ev: &[InputEvent]) -> Result<(), InputError> {
        if !self.gate.at_start {
            self.gate.arrived.notify_one();
            self.gate.proceed.notified().await;
        }
        self.inner.emit(ev).await
    }
    async fn release_all(&mut self) -> Result<(), InputError> {
        self.inner.release_all().await
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.inner.stop().await
    }
}
#[tokio::test]
async fn authorization_is_rechecked_after_each_adapter_await() {
    for at_start in [true, false] {
        for pause in [true, false] {
            let gate = Arc::new(Gate {
                at_start,
                arrived: tokio::sync::Notify::new(),
                proceed: tokio::sync::Notify::new(),
            });
            let mut r = Rig::gated(3, Some(gate.clone())).await;
            r.send(InputMsg::Enter {
                session: 7,
                edge: Edge::Left,
                pos: 0.2,
            })
            .await;
            r.forward().await;
            tokio::time::timeout(Duration::from_secs(1), gate.arrived.notified())
                .await
                .unwrap();
            if pause {
                r.hub.set_paused(r.b, true);
                r.hub.set_paused(r.b, false);
            } else {
                r.grant(false);
                r.grant(true);
            }
            gate.proceed.notify_one();
            r.settle().await;
            assert!(
                r.emu.release_calls() > 0,
                "revoked attempt must release after await, start={at_start}"
            );
            // Probe with the fresh epoch: an obsolete Incoming must not authorize even fresh input.
            r.host
                .send_datagram(
                    r.b,
                    InputDatagram {
                        session: 7,
                        seq: 0,
                        t_mono_us: 0,
                        events: vec![InputEvent::Key {
                            code: 42,
                            pressed: true,
                        }],
                    },
                )
                .unwrap();
            while let Ok(m) = r.wire.try_recv() {
                r.tx.send(m).await.unwrap();
            }
            r.settle().await;
            assert_eq!(r.emu.pressed_now(), 0);
            r.stop().await;
        }
    }
}
#[tokio::test]
async fn paused_host_invalidates_queued_input_before_paused_notification() {
    let mut r = Rig::new(3).await;
    r.enter(7).await;
    r.press(7).await;
    r.host
        .send_datagram(
            r.b,
            InputDatagram {
                session: 7,
                seq: 1,
                t_mono_us: 0,
                events: vec![InputEvent::Key {
                    code: 43,
                    pressed: true,
                }],
            },
        )
        .unwrap();
    let old = r.next().await;
    r.hub.set_paused(r.b, true);
    r.tx.send(old).await.unwrap();
    r.settle().await;
    assert_eq!(r.emu.pressed_now(), 0);
    r.stop().await;
}
#[tokio::test]
async fn stale_peer_update_and_leave_do_not_destroy_new_epoch_session() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Leave { session: 7 }).await;
    let leave = r.next().await;
    r.grant(false);
    let update = r.next().await;
    r.grant(true);
    r.forward().await;
    r.settle().await;
    r.enter(8).await;
    r.press(8).await;
    r.tx.send(update).await.unwrap();
    r.tx.send(leave).await.unwrap();
    r.settle().await;
    assert_eq!(r.emu.pressed_now(), 2);
    r.stop().await;
}
#[tokio::test]
async fn old_connection_cannot_inject_into_reconnected_peer() {
    let mut r = Rig::new(3).await;
    r.send(InputMsg::Enter {
        session: 7,
        edge: Edge::Left,
        pos: 0.2,
    })
    .await;
    let old = r.next().await;
    r.hub.unlink(r.a, r.b);
    r.grant(true);
    r.forward().await;
    r.forward().await;
    r.settle().await;
    r.tx.send(old).await.unwrap();
    r.settle().await;
    assert!(!r.emu.started());
    r.enter(8).await;
    r.press(8).await;
    assert_eq!(r.emu.pressed_now(), 2);
    r.stop().await;
}

#[tokio::test]
async fn old_epoch_datagram_does_not_affect_a_new_session() {
    let mut r = Rig::new(3).await;
    r.enter(7).await;
    r.host
        .send_datagram(
            r.b,
            InputDatagram {
                session: 7,
                seq: 0,
                t_mono_us: 0,
                events: vec![InputEvent::Key {
                    code: 42,
                    pressed: true,
                }],
            },
        )
        .unwrap();
    let old = r.next().await;
    r.grant(false);
    r.grant(true);
    r.forward().await;
    r.forward().await;
    r.settle().await;
    r.enter(8).await;
    r.tx.send(old).await.unwrap();
    r.settle().await;
    assert_eq!(r.emu.pressed_now(), 0);
    r.press(8).await;
    assert_eq!(r.emu.pressed_now(), 2);
    r.stop().await;
}

#[tokio::test]
async fn stream_frontier_and_revocation_are_isolated_from_a_third_peer() {
    let mut r = Rig::new(3).await;
    r.enter(7).await;
    r.send(InputMsg::Leave { session: 7 }).await;
    r.forward().await;
    r.settle().await;
    let c = r.hub.add_node("third", INPUT);
    r.hub.link(c.id, r.b, PeerPerms::default(), GRANT);
    r.forward().await;
    c.host
        .send_reliable(
            r.b,
            ModuleMsg::Input(InputMsg::Enter {
                session: 9,
                edge: Edge::Left,
                pos: 0.2,
            }),
        )
        .await
        .unwrap();
    r.forward().await;
    r.settle().await;
    c.host
        .send_datagram(
            r.b,
            InputDatagram {
                session: 9,
                seq: 0,
                t_mono_us: 0,
                events: vec![InputEvent::Key {
                    code: 42,
                    pressed: true,
                }],
            },
        )
        .unwrap();
    r.forward().await;
    r.settle().await;
    assert_eq!(r.emu.pressed_now(), 1);
    r.grant(false);
    r.forward().await;
    r.settle().await;
    assert_eq!(
        r.emu.pressed_now(),
        1,
        "revoking an inactive peer must not release another peer"
    );
    r.stop().await;
}

#[tokio::test]
async fn revocation_still_releases_when_peer_update_cannot_be_delivered() {
    let mut r = Rig::new(3).await;
    r.enter(7).await;
    r.press(7).await;
    r.grant(false); // deliberately retain PeerUp in wire, as with a congested notification queue
    tokio::time::timeout(Duration::from_secs(3), async {
        while r.emu.pressed_now() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("live revocation must be reconciled within 2 s poll + 1 s tolerance");
    r.stop().await;
}
