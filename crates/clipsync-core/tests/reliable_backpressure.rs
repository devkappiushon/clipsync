//! S9 ingress progress on authenticated loopback QUIC. No agent profile or system adapter.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_core::tls::{self, PinVerifier, VerifyMode};
use clipsync_core::transport::{Mesh, MeshOptions};
use clipsync_core::{Identity, PeerEntry, Peers, SERVER_NAME};
use clipsync_proto::*;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
struct Rig {
    mesh: Mesh,
    client: quinn::Endpoint,
    conn: quinn::Connection,
    input: mpsc::Receiver<ModuleInput>,
    _blocked: mpsc::Receiver<ModuleInput>,
    _control: (quinn::SendStream, quinn::RecvStream),
    peer: PeerId,
}
impl Rig {
    async fn new(credits: u32) -> Self {
        let local = Identity::generate().unwrap();
        let remote = Identity::generate().unwrap();
        let local_id = local.id();
        let peer = remote.id();
        let (itx, input) = mpsc::channel(64);
        let (ctx, blocked) = mpsc::channel(1);
        let mut peers = Peers::default();
        peers.upsert(PeerEntry {
            id: peer,
            name: "peer".into(),
            addr: None,
            paired_at: "test".into(),
            perms: PeerPerms {
                input_in: true,
                clipboard_in: true,
                clipboard_out: true,
            },
        });
        let mesh = Mesh::start(MeshOptions {
            identity: local,
            device_name: "local".into(),
            listen: "127.0.0.1:0".parse().unwrap(),
            modules: HashMap::from([(ModuleKind::Input, itx), (ModuleKind::Clipboard, ctx)]),
            peers,
            paused: false,
        })
        .unwrap();
        let client = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        let mut config = tls::client_config(
            &remote,
            PinVerifier::new(VerifyMode::Expect(local_id)),
            ALPN_MESH,
        )
        .unwrap();
        let mut transport = quinn::TransportConfig::default();
        transport.max_concurrent_uni_streams(credits.into());
        config.transport_config(Arc::new(transport));
        let conn = client
            .connect_with(config, mesh.local_addr().unwrap(), SERVER_NAME)
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        write_frame(
            &mut send,
            &Frame::Hello(Hello {
                version: 3,
                min_version: 1,
                device_name: "peer".into(),
                modules: ModuleSet {
                    clipboard: true,
                    input: true,
                },
                listen_port: client.local_addr().unwrap().port(),
            }),
            MAX_CONTROL_FRAME_BYTES,
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame(&mut recv, MAX_CONTROL_FRAME_BYTES)
                .await
                .unwrap(),
            Frame::Hello(_)
        ));
        let mut r = Self {
            mesh,
            client,
            conn,
            input,
            _blocked: blocked,
            _control: (send, recv),
            peer,
        };
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(3), r.input.recv())
                .await
                .unwrap()
                .unwrap(),
            ModuleInput::PeerUp(_)
        ));
        r
    }
    async fn closed(&mut self, start: Instant) {
        tokio::time::timeout(Duration::from_secs(3), self.conn.closed())
            .await
            .expect("ingress must close within 2 s + 1 s scheduling tolerance");
        assert!(start.elapsed() < Duration::from_secs(3));
        assert!(
            self.mesh
                .host()
                .peer_input_context(self.peer)
                .is_none_or(|c| !c.connected)
        );
        assert!(
            matches!(tokio::time::timeout(Duration::from_secs(1),self.input.recv()).await.unwrap().unwrap(),ModuleInput::PeerDown(p) if p==self.peer)
        );
    }
    async fn stop(self) {
        self.client.close(0u32.into(), b"test");
        self.mesh.shutdown().await;
    }
}
#[tokio::test]
async fn thirty_two_incomplete_readers_close_instead_of_starving_leave() {
    let mut r = Rig::new(100).await;
    let now = Instant::now();
    let mut held = Vec::new();
    for _ in 0..32 {
        let mut s = r.conn.open_uni().await.unwrap();
        s.write_all(&100u32.to_be_bytes()).await.unwrap();
        held.push(s);
    }
    r.closed(now).await;
    drop(held);
    r.stop().await;
}
#[tokio::test]
async fn thirty_two_blocked_deliveries_invalidate_and_notify_other_module() {
    let mut r = Rig::new(100).await;
    let now = Instant::now();
    for _ in 0..32 {
        let mut s = r.conn.open_uni().await.unwrap();
        write_frame(
            &mut s,
            &Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(ClipOffer {
                id: EventId::random(),
                lamport: 1,
                mime: MIME_TEXT_UTF8.into(),
                text: "synthetic".into(),
                created_unix_ms: 1,
                ttl_ms: 1000,
            }))),
            MAX_FRAME_BYTES,
        )
        .await
        .unwrap();
        s.finish().unwrap();
    }
    r.closed(now).await;
    r.stop().await;
}
#[tokio::test]
async fn send_with_no_stream_credit_has_a_total_deadline() {
    let r = Rig::new(0).await;
    let now = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        r.mesh.host().send_reliable(
            r.peer,
            ModuleMsg::Input(InputMsg::StatsRequest { session: 1 }),
        ),
    )
    .await
    .expect("send remained pending beyond 2 s + 1 s tolerance");
    assert!(matches!(result, Err(SendError::Transport(_))));
    assert!(now.elapsed() < Duration::from_secs(3));
    r.stop().await;
}

#[tokio::test]
async fn retained_pause_epoch_and_terminal_stop_survive_full_module_queue() {
    let (blocked, mut backlog) = mpsc::channel(1);
    blocked.try_send(ModuleInput::Paused(false)).unwrap();
    let (free, mut progressing) = mpsc::channel(8);
    let mesh = Mesh::start(MeshOptions {
        identity: Identity::generate().unwrap(),
        device_name: "lifecycle".into(),
        listen: "127.0.0.1:0".parse().unwrap(),
        modules: HashMap::from([(ModuleKind::Input, blocked), (ModuleKind::Clipboard, free)]),
        peers: Peers::default(),
        paused: false,
    })
    .unwrap();
    let mut lifecycle = mesh.host().lifecycle_receiver().unwrap();
    assert!(!lifecycle.borrow().paused);
    tokio::join!(mesh.set_paused(true), async {
        tokio::time::timeout(Duration::from_millis(500), lifecycle.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(lifecycle.borrow_and_update().paused);
        assert!(matches!(
            tokio::time::timeout(Duration::from_millis(500), progressing.recv())
                .await
                .unwrap(),
            Some(ModuleInput::Paused(true))
        ));
    });
    mesh.set_paused(false).await;
    let state = *lifecycle.borrow_and_update();
    assert!(!state.paused);
    assert_eq!(
        state.pause_generation, 1,
        "Resume must retain the intermediate invalidation"
    );
    // Only the old filler exists: both compatibility notifications really expired.
    assert!(matches!(backlog.try_recv(), Ok(ModuleInput::Paused(false))));
    assert!(backlog.try_recv().is_err());
    mesh.shutdown().await;
    mesh.set_paused(false).await;
    assert!(
        lifecycle.borrow().stopped,
        "Resume cannot undo terminal Stop"
    );
}
