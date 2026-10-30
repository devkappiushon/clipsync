//! Mesh on real QUIC in 127.0.0.1, two (or three) `agent::run` in this process, profiles in a
//! tempdir, no MemHub (contract §9 S1 items 4 and 5). Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::future::Future;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clipsync_core::tls::{self, PinSet, PinVerifier, VerifyMode};
use clipsync_core::transport::read_control;
use clipsync_core::{CoreError, InitOptions, PeerEntry, Peers, Profile, agent, control};
use clipsync_proto::*;
use quinn::VarInt;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const WAIT: Duration = Duration::from_secs(20);

fn free_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Node {
    profile: Profile,
    addr: SocketAddr,
    id: PeerId,
}

fn node(dir: &tempfile::TempDir, name: &str) -> Node {
    let profile = Profile {
        dir: dir.path().join(name),
    };
    let addr: SocketAddr = format!("127.0.0.1:{}", free_port()).parse().unwrap();
    let identity = profile
        .init(InitOptions {
            device_name: name.into(),
            modules: ModuleSet {
                clipboard: true,
                input: true,
            },
            listen: addr,
            fake_adapters: true,
        })
        .unwrap();
    Node {
        profile,
        addr,
        id: identity.id(),
    }
}

/// `who` trusts `other` with these grants.
fn trust(who: &Node, other: &Node, name: &str, perms: PeerPerms) {
    let mut peers = who
        .profile
        .load_peers()
        .unwrap_or_else(|_| Peers::default());
    peers.upsert(PeerEntry {
        id: other.id,
        name: name.into(),
        addr: Some(other.addr),
        paired_at: "2026-10-27T12:00:00Z".into(),
        perms,
    });
    who.profile.save_peers(&peers).unwrap();
}

const ALL: PeerPerms = PeerPerms {
    clipboard_in: true,
    clipboard_out: true,
    input_in: true,
};

/// A module that reports what reaches it and hands its host to the test.
struct Probe {
    kind: ModuleKind,
    events: mpsc::UnboundedSender<ModuleInput>,
    host: Option<oneshot::Sender<Arc<dyn ModuleHost>>>,
    /// Input probe echoes StatsRequest as Stats and datagrams back (RTT measurement).
    echo: bool,
}

#[async_trait::async_trait]
impl Module for Probe {
    fn kind(&self) -> ModuleKind {
        self.kind
    }

    fn adapter_name(&self) -> String {
        "probe".into()
    }

    async fn run(
        mut self: Box<Self>,
        host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        if let Some(tx) = self.host.take() {
            let _ = tx.send(host.clone());
        }
        while let Some(input) = rx.recv().await {
            match input {
                ModuleInput::Shutdown => break,
                ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(ModuleReply::Status(ModuleStatus {
                        kind: self.kind,
                        enabled: true,
                        adapter: Some("probe".into()),
                        authorized_peers: vec![],
                        last_event: None,
                        last_block: None,
                    }));
                }
                ModuleInput::Reliable {
                    from,
                    msg: ModuleMsg::Input(InputMsg::StatsRequest { session }),
                    ..
                } if self.echo => {
                    let stats = LoopbackStats {
                        session,
                        ..Default::default()
                    };
                    let _ = host
                        .send_reliable(from, ModuleMsg::Input(InputMsg::Stats(stats)))
                        .await;
                }
                ModuleInput::Datagram { from, dg, .. } if self.echo => {
                    let _ = host.send_datagram(from, dg);
                }
                other => {
                    let _ = self.events.send(other);
                }
            }
        }
        Ok(())
    }
}

/// Asks the agent for its status the moment it starts, like a module reacting to its first
/// PeerUp: the control channel must already answer by then.
struct AskOnStart {
    profile: Profile,
    answered: Option<oneshot::Sender<bool>>,
}

#[async_trait::async_trait]
impl Module for AskOnStart {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Clipboard
    }

    fn adapter_name(&self) -> String {
        "probe".into()
    }

    async fn run(
        mut self: Box<Self>,
        _host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        // First thing, no await: the control channel is already published.
        let published = self.profile.socket_path().exists();
        // Asked from a task: the agent's Status asks this module for its own status in turn.
        let (profile, answered) = (self.profile.clone(), self.answered.take());
        tokio::spawn(async move {
            let ok = published
                && matches!(
                    control::request(&profile, ControlRequest::Status).await,
                    Ok(ControlResponse::Status(_))
                );
            if let Some(tx) = answered {
                let _ = tx.send(ok);
            }
        });
        while let Some(input) = rx.recv().await {
            match input {
                ModuleInput::Shutdown => break,
                ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(ModuleReply::Status(ModuleStatus {
                        kind: ModuleKind::Clipboard,
                        enabled: true,
                        adapter: Some("probe".into()),
                        authorized_peers: vec![],
                        last_event: None,
                        last_block: None,
                    }));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

struct Running {
    task: JoinHandle<Result<(), CoreError>>,
    stop: oneshot::Sender<()>,
    clip: mpsc::UnboundedReceiver<ModuleInput>,
    input: mpsc::UnboundedReceiver<ModuleInput>,
    _clip_host: oneshot::Receiver<Arc<dyn ModuleHost>>,
    input_host: oneshot::Receiver<Arc<dyn ModuleHost>>,
}

fn start(n: &Node, echo: bool) -> Running {
    let (clip_tx, clip) = mpsc::unbounded_channel();
    let (input_tx, input) = mpsc::unbounded_channel();
    let (ch_tx, _clip_host) = oneshot::channel();
    let (ih_tx, input_host) = oneshot::channel();
    let modules: Vec<Box<dyn Module>> = vec![
        Box::new(Probe {
            kind: ModuleKind::Clipboard,
            events: clip_tx,
            host: Some(ch_tx),
            echo,
        }),
        Box::new(Probe {
            kind: ModuleKind::Input,
            events: input_tx,
            host: Some(ih_tx),
            echo,
        }),
    ];
    let (stop, stop_rx) = oneshot::channel::<()>();
    let profile = n.profile.clone();
    let task = tokio::spawn(async move {
        agent::run(&profile, modules, async move {
            let _ = stop_rx.await;
        })
        .await
    });
    Running {
        task,
        stop,
        clip,
        input,
        _clip_host,
        input_host,
    }
}

async fn stop(r: Running) {
    // A host handed to the test holds the mesh, and with it the UDP port: let go first, or
    // `agent::run` waits for the port until its deadline.
    let Running {
        task,
        stop,
        _clip_host,
        input_host,
        ..
    } = r;
    drop((_clip_host, input_host));
    let _ = stop.send(());
    task.await.unwrap().unwrap();
}

async fn status(p: &Profile) -> Option<AgentStatus> {
    match control::request(p, ControlRequest::Status).await {
        Ok(ControlResponse::Status(s)) => Some(s),
        _ => None,
    }
}

async fn wait_until<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = Instant::now();
    while start.elapsed() < WAIT {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for: {what}");
}

async fn peer_state(p: &Profile, peer: PeerId) -> Option<PeerState> {
    status(p)
        .await?
        .peers
        .into_iter()
        .find(|s| s.id == peer)
        .map(|s| s.state)
}

async fn next_peer_up(rx: &mut mpsc::UnboundedReceiver<ModuleInput>) -> PeerInfo {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Some(ModuleInput::PeerUp(info))) => return info,
            Ok(Some(_)) => continue,
            other => panic!("no PeerUp: {other:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mesh_connects_after_hello_both_ways() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    trust(
        &b,
        &a,
        "a",
        PeerPerms {
            clipboard_in: true,
            ..PeerPerms::default()
        },
    );
    let (mut ra, mut rb) = (start(&a, false), start(&b, false));
    // (a) PeerUp only comes from install, which only runs after Hello went both ways.
    let up_a = next_peer_up(&mut ra.clip).await;
    let up_b = next_peer_up(&mut rb.clip).await;
    assert_eq!((up_a.id, up_a.name.as_str(), up_a.perms), (b.id, "b", ALL));
    assert_eq!(up_b.id, a.id);
    // The version of the peer's Hello reaches the modules and the status.
    assert_eq!(
        (up_a.remote_version, up_b.remote_version),
        (PROTOCOL_VERSION, PROTOCOL_VERSION)
    );
    assert_eq!(
        up_a.remote_modules,
        ModuleSet {
            clipboard: true,
            input: true
        }
    );
    let sa = status(&a.profile).await.unwrap();
    let pb = sa.peers.iter().find(|p| p.id == b.id).unwrap();
    assert_eq!(pb.state, PeerState::Connected);
    assert_eq!(pb.perms, ALL);
    assert_eq!(pb.remote_version, PROTOCOL_VERSION);
    assert_eq!(sa.listen, a.addr);
    assert!(
        sa.modules
            .iter()
            .all(|m| m.enabled && m.adapter.as_deref() == Some("probe"))
    );
    stop(ra).await;
    stop(rb).await;
}

/// The control channel is bound before the mesh and the modules start (CI Windows, 4edfe23: a
/// `Status` right after PeerUp found no agent, because the pipe was published only after the
/// mesh had connected). On Linux the old order passed too (binding a Unix socket is immediate);
/// on Windows publishing the pipe name takes a file write with fsync, and the mesh won the race.
#[tokio::test(flavor = "multi_thread")]
async fn the_control_channel_answers_before_the_modules_start() {
    let dir = tempfile::tempdir().unwrap();
    let a = node(&dir, "a");
    let (tx, answered) = oneshot::channel();
    let modules: Vec<Box<dyn Module>> = vec![Box::new(AskOnStart {
        profile: a.profile.clone(),
        answered: Some(tx),
    })];
    let (stop, stop_rx) = oneshot::channel::<()>();
    let p = a.profile.clone();
    let task = tokio::spawn(async move {
        agent::run(&p, modules, async move {
            let _ = stop_rx.await;
        })
        .await
    });
    let ok = tokio::time::timeout(WAIT, answered).await.unwrap().unwrap();
    assert!(
        ok,
        "a module that just started must find the agent answering status"
    );
    let _ = stop.send(());
    task.await.unwrap().unwrap();
    assert!(!a.profile.socket_path().exists(), "unpublished on exit");
}

#[tokio::test(flavor = "multi_thread")]
async fn mesh_unknown_identity_never_connects() {
    let dir = tempfile::tempdir().unwrap();
    let (a, c) = (node(&dir, "a"), node(&dir, "intruder"));
    // C trusts A, A has never heard of C.
    trust(&c, &a, "a", ALL);
    let (mut ra, rc) = (start(&a, false), start(&c, false));
    // C keeps dialing (1 s, 2 s, ...): give it a few attempts.
    tokio::time::sleep(Duration::from_secs(4)).await;
    let sa = status(&a.profile).await.unwrap();
    assert!(sa.peers.is_empty(), "A lists no peer: {:?}", sa.peers);
    assert_ne!(
        peer_state(&c.profile, a.id).await,
        Some(PeerState::Connected)
    );
    while let Ok(ev) = ra.clip.try_recv() {
        assert!(
            !matches!(ev, ModuleInput::PeerUp(_)),
            "A must never see PeerUp: {ev:?}"
        );
    }
    // The refused side records it once, however many times it dialed; A records nothing (an
    // unknown key could be anyone on the network).
    let sc = status(&c.profile).await.unwrap();
    assert_eq!(sc.recent_blocks.len(), 1, "{:?}", sc.recent_blocks);
    let b = &sc.recent_blocks[0];
    assert_eq!(
        (b.peer, b.peer_name.as_deref(), b.module, b.reason.as_str()),
        (
            Some(a.id),
            Some("a"),
            None,
            clipsync_core::msg::PEER_REFUSED_US
        )
    );
    assert!(sa.recent_blocks.is_empty(), "{:?}", sa.recent_blocks);
    stop(ra).await;
    stop(rc).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn mesh_restart_reconnects() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    trust(&b, &a, "a", ALL);
    let (mut ra, rb) = (start(&a, false), start(&b, false));
    next_peer_up(&mut ra.clip).await;
    stop(rb).await;
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        match tokio::time::timeout_at(deadline, ra.clip.recv()).await {
            Ok(Some(ModuleInput::PeerDown(id))) if id == b.id => break,
            Ok(Some(_)) => {}
            other => panic!("no PeerDown: {other:?}"),
        }
    }
    // (c) B comes back on the same address: a new handshake, so a new verifier run (resumption
    // is off; tls_ proves the counter), and a new PeerUp.
    let rb = start(&b, false);
    assert_eq!(next_peer_up(&mut ra.clip).await.id, b.id);
    wait_until("A sees B connected again", || async {
        peer_state(&a.profile, b.id).await == Some(PeerState::Connected)
    })
    .await;
    stop(ra).await;
    stop(rb).await;
}

/// `agent::run` returns only with its UDP port free (CI Windows, 601cd7f: the test below bound
/// B's port right after stopping B and found it taken). Here the old code left it taken in 3
/// stops out of 30.
#[tokio::test(flavor = "multi_thread")]
async fn a_stopped_agent_has_released_its_port() {
    for _ in 0..10 {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (node(&dir, "a"), node(&dir, "b"));
        trust(&a, &b, "b", ALL);
        trust(&b, &a, "a", ALL);
        let (mut ra, rb) = (start(&a, false), start(&b, false));
        next_peer_up(&mut ra.clip).await;
        stop(rb).await;
        let rebound = UdpSocket::bind(b.addr);
        assert!(rebound.is_ok(), "b's port still taken: {rebound:?}");
        drop(rebound);
        stop(ra).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mesh_changed_key_is_blocked_with_constant_reason() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    trust(&b, &a, "a", ALL);
    let (mut ra, rb) = (start(&a, false), start(&b, false));
    next_peer_up(&mut ra.clip).await;
    stop(rb).await;
    // (d) B re-initialized: new identity, same address. It even trusts A.
    let b2 = clipsync_core::Identity::generate().unwrap();
    let pins = Arc::new(PinSet::new([a.id]));
    let server =
        tls::server_config(&b2, PinVerifier::new(VerifyMode::Mesh(pins)), ALPN_MESH).unwrap();
    let ep = quinn::Endpoint::server(server, b.addr).unwrap();
    let incoming = tokio::time::timeout(WAIT, ep.accept())
        .await
        .unwrap()
        .unwrap();
    let err = incoming.await.expect_err("A must refuse B's new key");
    let reason = match &err {
        quinn::ConnectionError::ConnectionClosed(c) => {
            String::from_utf8_lossy(&c.reason).into_owned()
        }
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(reason, tls::HANDSHAKE_REJECT_REASONS[0]);
    assert!(!reason.contains(&b2.id().to_hex()[..8]) && !reason.contains(&b.id.to_hex()[..8]));
    wait_until("A blocks B", || async {
        peer_state(&a.profile, b.id).await
            == Some(PeerState::Blocked {
                reason: "chave do par mudou; repareie".into(),
            })
    })
    .await;
    let blocks = status(&a.profile).await.unwrap().recent_blocks;
    assert!(
        matches!(&blocks[..], [r] if r.peer == Some(b.id) && r.peer_name.as_deref() == Some("b") && r.module.is_none() && r.reason == "chave do par mudou; repareie" && r.at_unix_ms > 0),
        "{blocks:?}"
    );
    // Blocked: A does not dial again.
    let again = tokio::time::timeout(Duration::from_secs(3), ep.accept()).await;
    assert!(again.is_err(), "A must not redial a blocked peer");
    ep.close(0u32.into(), b"");
    stop(ra).await;
}

/// B01: repeated refusals on real QUIC form one group, without content.
/// Distinct-key cap/LRU: transport::block_groups_tests::arrival_lru_preserves_other_causes_and_ignores_names_and_wall_clock.
#[tokio::test(flavor = "multi_thread")]
async fn mesh_inbound_refusals_are_grouped_and_redacted() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    // B lets A do nothing: every offer from A is refused at B's door.
    trust(&b, &a, "a", PeerPerms::default());
    let (mut ra, rb) = (start(&a, false), start(&b, false));
    next_peer_up(&mut ra.clip).await;
    let host = (&mut ra._clip_host).await.unwrap();
    const SECRET: &str = "segredo-que-nao-pode-vazar";
    for _ in 0..MAX_RECENT_BLOCKS + 3 {
        let offer = ClipOffer {
            id: EventId::random(),
            lamport: 1,
            mime: MIME_TEXT_UTF8.into(),
            text: SECRET.into(),
            created_unix_ms: 1,
            ttl_ms: CLIP_TTL_MS,
        };
        host.send_reliable(b.id, ModuleMsg::Clip(ClipMsg::Offer(offer)))
            .await
            .unwrap();
    }
    wait_until("B records the refusals", || async {
        status(&b.profile)
            .await
            .is_some_and(|s| s.recent_blocks.len() == 1 && s.recent_blocks[0].occurrences == 23)
    })
    .await;
    let s = status(&b.profile).await.unwrap();
    assert_eq!(s.recent_blocks.len(), 1);
    assert_eq!(s.recent_blocks[0].occurrences, 23);
    for r in &s.recent_blocks {
        assert_eq!((r.peer, r.peer_name.as_deref()), (Some(a.id), Some("a")));
        assert_eq!(
            (r.module, r.reason.as_str()),
            (
                Some(ModuleKind::Clipboard),
                RejectReason::NotAuthorized.pt()
            )
        );
    }
    assert!(s.recent_blocks[0].at_unix_ms > 0);
    assert!(s.recent_blocks[0].last_unix_ms >= s.recent_blocks[0].at_unix_ms);
    assert!(!format!("{s:?}").contains(SECRET));
    assert!(!serde_json::to_string(&s).unwrap().contains(SECRET));
    // A was not refused by anyone.
    assert!(status(&a.profile).await.unwrap().recent_blocks.is_empty());
    drop(host);
    stop(ra).await;
    stop(rb).await;
}

fn pct(mut v: Vec<Duration>, q: f64) -> Duration {
    v.sort();
    v[((v.len() - 1) as f64 * q).round() as usize]
}

/// Functional on every run (CI included): every stream frame arrives, in order; every datagram
/// arrives intact (a short retry covers loss on a busy runner). Latency is always printed and only
/// asserted (p95 < 1 ms) with CLIPSYNC_PERF=1, on a quiet machine.
#[tokio::test(flavor = "multi_thread")]
async fn stream_and_datagram() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    trust(&b, &a, "a", ALL);
    let (mut ra, rb) = (start(&a, false), start(&b, true));
    next_peer_up(&mut ra.input).await;
    let host = (&mut ra.input_host).await.unwrap();

    // Same connection: one frame per uni stream (StatsRequest -> Stats) and datagrams (echoed).
    const N: usize = 300;
    let mut stream_rtt = Vec::with_capacity(N);
    for session in 0..N as u32 {
        let t = Instant::now();
        host.send_reliable(b.id, ModuleMsg::Input(InputMsg::StatsRequest { session }))
            .await
            .unwrap();
        loop {
            let input = tokio::time::timeout(WAIT, ra.input.recv())
                .await
                .unwrap()
                .unwrap();
            if let ModuleInput::Reliable {
                from,
                msg: ModuleMsg::Input(InputMsg::Stats(s)),
                ..
            } = input
            {
                assert_eq!(from, b.id);
                assert_eq!(s.session, session, "stream replies out of order");
                break;
            }
        }
        stream_rtt.push(t.elapsed());
    }

    const ATTEMPTS: usize = 3;
    const DATAGRAM_WAIT: Duration = Duration::from_millis(250);
    let (mut dgram_rtt, mut retries) = (Vec::with_capacity(N), 0usize);
    for seq in 0..N as u32 {
        let mut delivered = false;
        for attempt in 0..ATTEMPTS {
            let dg = InputDatagram {
                session: 1,
                seq,
                t_mono_us: mono_us(),
                events: vec![InputEvent::Motion { dx: 1.0, dy: 0.0 }; 4],
            };
            let t = Instant::now();
            host.send_datagram(b.id, dg).unwrap();
            let deadline = tokio::time::Instant::now() + DATAGRAM_WAIT;
            while let Ok(Some(input)) = tokio::time::timeout_at(deadline, ra.input.recv()).await {
                // Late echoes of an earlier attempt or seq are ignored.
                if let ModuleInput::Datagram { from, dg, .. } = input
                    && dg.seq == seq
                {
                    assert_eq!(from, b.id);
                    assert_eq!(dg.events, vec![InputEvent::Motion { dx: 1.0, dy: 0.0 }; 4]);
                    if attempt == 0 {
                        dgram_rtt.push(t.elapsed());
                    }
                    delivered = true;
                    break;
                }
            }
            if delivered {
                break;
            }
            retries += 1;
        }
        assert!(delivered, "datagram {seq} lost {ATTEMPTS} times in a row");
    }

    let (s95, d95) = (pct(stream_rtt.clone(), 0.95), pct(dgram_rtt.clone(), 0.95));
    eprintln!(
        "RTT fluxo 1 frame: p50 {:?} p95 {:?} · datagrama 4 eventos: p50 {:?} p95 {:?} \
         (1ª tentativa {} de {N}, reenvios {retries})",
        pct(stream_rtt, 0.5),
        s95,
        pct(dgram_rtt.clone(), 0.5),
        d95,
        dgram_rtt.len()
    );
    if std::env::var_os("CLIPSYNC_PERF").is_some_and(|v| v == "1") {
        assert!(s95 < Duration::from_millis(1), "stream p95 {s95:?} >= 1 ms");
        assert!(
            d95 < Duration::from_millis(1),
            "datagram p95 {d95:?} >= 1 ms"
        );
    }
    drop(host);
    stop(ra).await;
    stop(rb).await;
}

/// Rule 10 on a real QUIC stream: a 64 KiB + 1 control frame is refused before allocating and
/// the stream is stopped and reset with CloseCode::Protocol.
#[tokio::test(flavor = "multi_thread")]
async fn stream_and_datagram_control_frame_over_limit_is_reset() {
    let (a, b) = (
        clipsync_core::Identity::generate().unwrap(),
        clipsync_core::Identity::generate().unwrap(),
    );
    let server = tls::server_config(&a, PinVerifier::new(VerifyMode::Pairing), ALPN_MESH).unwrap();
    let srv = quinn::Endpoint::server(server, "127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = srv.local_addr().unwrap();
    let cli = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    let cfg = tls::client_config(&b, PinVerifier::new(VerifyMode::Pairing), ALPN_MESH).unwrap();
    let (c, s) = tokio::join!(
        async {
            cli.connect_with(cfg, addr, clipsync_core::SERVER_NAME)
                .unwrap()
                .await
                .unwrap()
        },
        async { srv.accept().await.unwrap().await.unwrap() }
    );
    let (mut send, mut recv) = c.open_bi().await.unwrap();
    send.write_all(&((MAX_CONTROL_FRAME_BYTES + 1) as u32).to_be_bytes())
        .await
        .unwrap();
    let (mut ssend, mut srecv) = s.accept_bi().await.unwrap();
    let res = read_control(&mut ssend, &mut srecv).await;
    assert!(
        matches!(
            res,
            Err(WireError::TooLarge {
                len: 65_537,
                max: 65_536
            })
        ),
        "{res:?}"
    );
    let code = VarInt::from_u32(CloseCode::Protocol.code());
    assert_eq!(send.stopped().await.unwrap(), Some(code));
    let mut buf = [0u8; 1];
    assert!(matches!(recv.read(&mut buf).await, Err(quinn::ReadError::Reset(c)) if c == code));
}

/// S1-08: pause fails closed. With state.toml impossible to write, Pause still pauses (the
/// response is an error that says so) and the agent stays paused.
#[cfg(unix)] // a read-only profile dir needs mode bits
#[tokio::test(flavor = "multi_thread")]
async fn pause_fails_closed_when_state_cannot_be_saved() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let a = node(&dir, "a");
    let ra = start(&a, false);
    wait_until("agent up", || async { status(&a.profile).await.is_some() }).await;
    // Read-only profile dir: the atomic write of state.toml (tmp + rename) fails.
    let ro = std::fs::Permissions::from_mode(0o500);
    std::fs::set_permissions(&a.profile.dir, ro).unwrap();
    let resp = control::request(&a.profile, ControlRequest::Pause).await;
    std::fs::set_permissions(&a.profile.dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(resp, Ok(ControlResponse::Error { .. })),
        "{resp:?}"
    );
    assert!(
        status(&a.profile).await.unwrap().paused,
        "must stay paused in memory"
    );
    assert!(
        !a.profile.load_state().unwrap().paused,
        "nothing was persisted"
    );
    stop(ra).await;
}

/// The next `Paused` that reaches a probe; other inputs are skipped.
async fn next_paused(rx: &mut mpsc::UnboundedReceiver<ModuleInput>) -> bool {
    tokio::time::timeout(WAIT, async {
        loop {
            match rx.recv().await {
                Some(ModuleInput::Paused(p)) => return p,
                Some(_) => {}
                None => panic!("module channel closed"),
            }
        }
    })
    .await
    .expect("no Paused")
}

/// §13.11 item 1 (f): a module's `request_pause` takes the control's path. Both modules get
/// `Paused`, the status says it (what the app compares to emit `pause_changed`) and state.toml
/// keeps it; `request_pause(false)` undoes all three.
#[tokio::test(flavor = "multi_thread")]
async fn request_pause_pauses_both_modules_like_the_control() {
    let dir = tempfile::tempdir().unwrap();
    let a = node(&dir, "a");
    let mut ra = start(&a, false);
    wait_until("agent up", || async { status(&a.profile).await.is_some() }).await;
    let host = (&mut ra.input_host).await.unwrap();

    host.request_pause(true);
    assert!(next_paused(&mut ra.clip).await);
    assert!(next_paused(&mut ra.input).await);
    assert!(host.paused());
    assert!(status(&a.profile).await.unwrap().paused);
    assert!(a.profile.load_state().unwrap().paused);

    host.request_pause(false);
    assert!(!next_paused(&mut ra.clip).await);
    assert!(!next_paused(&mut ra.input).await);
    assert!(!host.paused());
    assert!(!status(&a.profile).await.unwrap().paused);
    assert!(!a.profile.load_state().unwrap().paused);

    drop(host);
    stop(ra).await;
}

/// §12.9 item 1 (e) and §13.11 item 1 (e): a 0.1.0 agent accepts a Hello when
/// `version >= 1 && 1 >= min_version` (its MIN and PROTOCOL were both 1), a 0.2.x agent when
/// `version >= 1 && 2 >= min_version`. Ours (3, minimum 1) passes both, and we still accept a
/// 0.1.0 and a 0.2.x Hello.
#[test]
fn hello_v3_passes_the_0_1_0_and_0_2_x_compatibility_rules() {
    let v010 = |h: &Hello| h.version >= 1 && 1 >= h.min_version;
    let v02x = |h: &Hello| h.version >= 1 && 2 >= h.min_version;
    let ours = Hello {
        version: PROTOCOL_VERSION,
        min_version: MIN_PROTOCOL_VERSION,
        device_name: "a".into(),
        modules: ModuleSet::default(),
        listen_port: 47800,
    };
    assert_eq!((ours.version, ours.min_version), (3, 1));
    assert!(v010(&ours) && v02x(&ours));
    assert!(clipsync_core::transport::hello_compatible(&ours));
    for version in [1, 2] {
        let old = Hello {
            version,
            min_version: 1,
            ..ours.clone()
        };
        assert!(clipsync_core::transport::hello_compatible(&old));
    }
    let future = Hello {
        version: 4,
        min_version: 4,
        ..ours
    };
    assert!(!clipsync_core::transport::hello_compatible(&future));
}

/// §12.9 item 1 (d), on real QUIC: B is a 0.1.0 agent written by hand (Hello version 1). It
/// connects, but Zones, LayoutOffer and EnterAt never leave for it: `RemoteTooOld`, no stream.
#[tokio::test(flavor = "multi_thread")]
async fn mesh_never_sends_a_protocol_2_message_to_a_version_1_peer() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    let b_identity = b.profile.load_identity().unwrap();
    let pins = Arc::new(PinSet::new([a.id]));
    let server = tls::server_config(
        &b_identity,
        PinVerifier::new(VerifyMode::Mesh(pins)),
        ALPN_MESH,
    )
    .unwrap();
    let ep = quinn::Endpoint::server(server, b.addr).unwrap();
    let mut ra = start(&a, false);
    let conn = tokio::time::timeout(WAIT, async { ep.accept().await.unwrap().await.unwrap() })
        .await
        .unwrap();
    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    let Frame::Hello(h) = read_control(&mut send, &mut recv).await.unwrap() else {
        panic!("expected Hello");
    };
    assert_eq!(
        (h.version, h.min_version),
        (PROTOCOL_VERSION, MIN_PROTOCOL_VERSION)
    );
    let old = Hello {
        version: 1,
        min_version: 1,
        device_name: "b".into(),
        modules: ModuleSet {
            clipboard: true,
            input: true,
        },
        listen_port: b.addr.port(),
    };
    write_frame(&mut send, &Frame::Hello(old), MAX_CONTROL_FRAME_BYTES)
        .await
        .unwrap();
    let up = next_peer_up(&mut ra.input).await;
    assert_eq!((up.id, up.remote_version), (b.id, 1));
    let pb = status(&a.profile).await.unwrap().peers;
    assert_eq!(pb.iter().find(|p| p.id == b.id).unwrap().remote_version, 1);

    let host = (&mut ra.input_host).await.unwrap();
    let endpoint = |peer: PeerId| OfferEndpoint {
        peer,
        monitor: "HDMI-1".into(),
        edge: Edge::Right,
        from: 0.0,
        to: 1.0,
    };
    let newer = [
        InputMsg::Zones {
            zones: vec![ZoneInfo {
                index: 0,
                name: Some("eDP-1".into()),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale: 1.0,
            }],
        },
        InputMsg::LayoutOffer(LayoutOffer {
            rev: 1,
            devices: vec![],
            links: vec![OfferLink {
                a: endpoint(a.id),
                b: endpoint(b.id),
            }],
        }),
        InputMsg::EnterAt {
            session: 1,
            edge: Edge::Left,
            monitor: None,
            pos: 0.5,
        },
    ];
    for m in newer {
        assert_eq!(
            host.send_reliable(b.id, ModuleMsg::Input(m)).await,
            Err(SendError::RemoteTooOld)
        );
    }
    let nothing = tokio::time::timeout(Duration::from_secs(1), conn.accept_uni()).await;
    assert!(nothing.is_err(), "no stream may open for a refused message");
    // A protocol-1 message still reaches the old peer.
    let enter = InputMsg::Enter {
        session: 1,
        edge: Edge::Left,
        pos: 0.5,
    };
    host.send_reliable(b.id, ModuleMsg::Input(enter))
        .await
        .unwrap();
    let mut uni = tokio::time::timeout(WAIT, conn.accept_uni())
        .await
        .unwrap()
        .unwrap();
    let got = read_frame(&mut uni, MAX_FRAME_BYTES).await.unwrap();
    assert!(
        matches!(got, Frame::Module(ModuleMsg::Input(InputMsg::Enter { .. }))),
        "{got:?}"
    );
    drop(host);
    stop(ra).await;
    ep.close(0u32.into(), b"");
}

/// v1.16 on the wire: B written by hand with Hello `version`; A then drops B from peers.toml and
/// reloads. Returns the first control frame B got (None when the stream ended without one) and
/// the code A closed with.
async fn revoke_hand_written_peer(version: u16) -> (Option<Frame>, Option<u64>) {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    let b_identity = b.profile.load_identity().unwrap();
    let pins = Arc::new(PinSet::new([a.id]));
    let server = tls::server_config(
        &b_identity,
        PinVerifier::new(VerifyMode::Mesh(pins)),
        ALPN_MESH,
    )
    .unwrap();
    let ep = quinn::Endpoint::server(server, b.addr).unwrap();
    let mut ra = start(&a, false);
    let conn = tokio::time::timeout(WAIT, async { ep.accept().await.unwrap().await.unwrap() })
        .await
        .unwrap();
    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    read_control(&mut send, &mut recv).await.unwrap();
    let hello = Hello {
        version,
        min_version: 1,
        device_name: "b".into(),
        modules: ModuleSet {
            clipboard: true,
            input: true,
        },
        listen_port: b.addr.port(),
    };
    write_frame(&mut send, &Frame::Hello(hello), MAX_CONTROL_FRAME_BYTES)
        .await
        .unwrap();
    next_peer_up(&mut ra.input).await;

    a.profile.save_peers(&Peers::default()).unwrap();
    assert!(matches!(
        control::request(&a.profile, ControlRequest::ReloadPeers).await,
        Ok(ControlResponse::Ok)
    ));
    let frame = tokio::time::timeout(WAIT, read_control(&mut send, &mut recv))
        .await
        .unwrap()
        .ok();
    let code = match tokio::time::timeout(WAIT, conn.closed()).await.unwrap() {
        quinn::ConnectionError::ApplicationClosed(c) => Some(c.error_code.into_inner()),
        _ => None,
    };
    stop(ra).await;
    ep.close(0u32.into(), b"");
    (frame, code)
}

/// v1.16: a version 2 peer that leaves peers.toml is told `Bye{Revoked}` before the close; a
/// version 1 peer would not decode it, so it only sees the close.
#[tokio::test(flavor = "multi_thread")]
async fn revoked_bye_goes_only_to_a_version_2_peer() {
    let unknown = Some(u64::from(CloseCode::UnknownPeer.code()));
    let (frame, code) = revoke_hand_written_peer(2).await;
    assert!(
        matches!(
            frame,
            Some(Frame::Bye {
                code: CloseCode::Revoked
            })
        ),
        "{frame:?}"
    );
    assert_eq!(code, unknown);
    let (frame, code) = revoke_hand_written_peer(1).await;
    assert!(frame.is_none(), "a version 1 peer gets no frame: {frame:?}");
    assert_eq!(code, unknown);
}

/// v1.16 between two real agents: A revokes B; in up to 3 s B has taken A out of its own
/// peers.toml and status, and says who removed it. B offline at that moment keeps A.
#[tokio::test(flavor = "multi_thread")]
async fn a_revoked_peer_removes_the_revoker_itself() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (node(&dir, "a"), node(&dir, "b"));
    trust(&a, &b, "b", ALL);
    trust(&b, &a, "a", ALL);
    let (mut ra, mut rb) = (start(&a, false), start(&b, false));
    next_peer_up(&mut ra.clip).await;
    next_peer_up(&mut rb.clip).await;

    a.profile.save_peers(&Peers::default()).unwrap();
    control::request(&a.profile, ControlRequest::ReloadPeers)
        .await
        .unwrap();
    let began = Instant::now();
    wait_until("b drops a from peers.toml", || async {
        b.profile.load_peers().unwrap().get(&a.id).is_none()
    })
    .await;
    assert!(
        began.elapsed() <= Duration::from_secs(3),
        "{:?}",
        began.elapsed()
    );
    let sb = status(&b.profile).await.unwrap();
    assert!(sb.peers.iter().all(|p| p.id != a.id), "{:?}", sb.peers);
    assert_eq!(sb.recent_removals.len(), 1);
    assert_eq!(
        (
            sb.recent_removals[0].peer,
            sb.recent_removals[0].peer_name.as_str()
        ),
        (a.id, "a")
    );
    let sa = status(&a.profile).await.unwrap();
    assert!(sa.peers.is_empty() && sa.recent_removals.is_empty());
    stop(ra).await;
    stop(rb).await;

    // B offline when A revokes: B keeps A after it comes back (A just refuses its key).
    let (c, d) = (node(&dir, "c"), node(&dir, "d"));
    trust(&c, &d, "d", ALL);
    trust(&d, &c, "c", ALL);
    let rc = start(&c, false);
    wait_until("c up", || async { status(&c.profile).await.is_some() }).await;
    c.profile.save_peers(&Peers::default()).unwrap();
    control::request(&c.profile, ControlRequest::ReloadPeers)
        .await
        .unwrap();
    let rd = start(&d, false);
    wait_until("d up", || async { status(&d.profile).await.is_some() }).await;
    // d dials c (after DEFER when its id is the larger one) and is refused.
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert!(
        d.profile.load_peers().unwrap().get(&c.id).is_some(),
        "d keeps c"
    );
    assert_ne!(
        peer_state(&d.profile, c.id).await,
        Some(PeerState::Connected)
    );
    // d learns why: one entry, c refused its certificate; c records nothing.
    let sd = status(&d.profile).await.unwrap();
    assert!(sd.recent_removals.is_empty());
    let refusals: Vec<_> = sd
        .recent_blocks
        .iter()
        .filter(|b| b.peer == Some(c.id))
        .collect();
    assert_eq!(refusals.len(), 1, "{:?}", sd.recent_blocks);
    assert_eq!(refusals[0].reason, clipsync_core::msg::PEER_REFUSED_US);
    assert!(status(&c.profile).await.unwrap().recent_blocks.is_empty());
    stop(rc).await;
    stop(rd).await;
}

#[derive(Debug)]
struct Shows(Arc<rustls::sign::CertifiedKey>);

impl rustls::server::ResolvesServerCert for Shows {
    fn resolve(
        &self,
        _: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        Some(self.0.clone())
    }
}

/// S6-21: whoever answers at the peer's address without the peer's key (another key, or the
/// peer's public certificate signing with another key) never makes A record "recusou a
/// conexão". Each server trusts nobody, so it would refuse A's certificate if it ever saw it.
#[tokio::test(flavor = "multi_thread")]
async fn a_server_without_the_peers_key_never_records_a_refusal() {
    for shows_the_peers_certificate in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (node(&dir, "a"), node(&dir, "b"));
        trust(&a, &b, "b", ALL);
        let other = clipsync_core::Identity::generate().unwrap();
        let trusts_nobody = PinVerifier::new(VerifyMode::Mesh(Arc::new(PinSet::new([]))));
        let cfg = if shows_the_peers_certificate {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let key = provider
                .key_provider
                .load_private_key(other.key_der())
                .unwrap();
            let shown = rustls::sign::CertifiedKey::new(
                vec![b.profile.load_identity().unwrap().cert_der()],
                key,
            );
            let mut tls = rustls::ServerConfig::builder_with_provider(provider)
                .with_protocol_versions(&[&rustls::version::TLS13])
                .unwrap()
                .with_client_cert_verifier(trusts_nobody)
                .with_cert_resolver(Arc::new(Shows(Arc::new(shown))));
            tls.alpn_protocols = vec![ALPN_MESH.to_vec()];
            let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(tls).unwrap();
            let mut cfg = quinn::ServerConfig::with_crypto(Arc::new(crypto));
            cfg.transport_config(tls::transport_config());
            cfg
        } else {
            tls::server_config(&other, trusts_nobody, ALPN_MESH).unwrap()
        };
        let ep = quinn::Endpoint::server(cfg, b.addr).unwrap();
        let dialed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let answering = {
            let (ep, dialed) = (ep.clone(), dialed.clone());
            tokio::spawn(async move {
                while let Some(incoming) = ep.accept().await {
                    dialed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let _ = incoming.await;
                }
            })
        };
        let ra = start(&a, false);
        // A dials b (after DEFER when its id is the larger one, then with backoff).
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(
            dialed.load(std::sync::atomic::Ordering::SeqCst) > 0,
            "A reached the server at b's address"
        );
        let sa = status(&a.profile).await.unwrap();
        assert!(
            sa.recent_blocks
                .iter()
                .all(|r| r.reason != clipsync_core::msg::PEER_REFUSED_US),
            "shows the peer's certificate: {shows_the_peers_certificate}; {:?}",
            sa.recent_blocks
        );
        assert_ne!(
            peer_state(&a.profile, b.id).await,
            Some(PeerState::Connected)
        );
        stop(ra).await;
        answering.abort();
        ep.close(0u32.into(), b"");
    }
}

/// A module whose `Status` reports a settable `last_block`.
struct Blocking {
    last_block: Arc<std::sync::Mutex<Option<String>>>,
}

#[async_trait::async_trait]
impl Module for Blocking {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Clipboard
    }

    fn adapter_name(&self) -> String {
        "blocking".into()
    }

    async fn run(
        self: Box<Self>,
        _host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        while let Some(input) = rx.recv().await {
            match input {
                ModuleInput::Shutdown => break,
                ModuleInput::Command { reply, .. } => {
                    let last_block = self.last_block.lock().unwrap().clone();
                    let _ = reply.send(ModuleReply::Status(ModuleStatus {
                        kind: ModuleKind::Clipboard,
                        enabled: true,
                        adapter: Some("blocking".into()),
                        authorized_peers: vec![],
                        last_event: None,
                        last_block,
                    }));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// §12.6 core item 3: a NEW `last_block` of a module joins `recent_blocks`, without a peer.
#[tokio::test(flavor = "multi_thread")]
async fn module_last_block_joins_recent_blocks_once() {
    let dir = tempfile::tempdir().unwrap();
    let a = node(&dir, "a");
    let last_block = Arc::new(std::sync::Mutex::new(None));
    let modules: Vec<Box<dyn Module>> = vec![Box::new(Blocking {
        last_block: last_block.clone(),
    })];
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let profile = a.profile.clone();
    let task = tokio::spawn(async move {
        agent::run(&profile, modules, async move {
            let _ = stop_rx.await;
        })
        .await
    });
    wait_until("agent up", || async { status(&a.profile).await.is_some() }).await;
    assert!(status(&a.profile).await.unwrap().recent_blocks.is_empty());
    *last_block.lock().unwrap() = Some("texto grande demais".into());
    status(&a.profile).await.unwrap();
    let blocks = status(&a.profile).await.unwrap().recent_blocks;
    assert_eq!(
        blocks.len(),
        1,
        "the same last_block counts once: {blocks:?}"
    );
    assert_eq!(
        (
            blocks[0].peer,
            blocks[0].peer_name.as_deref(),
            blocks[0].module,
            blocks[0].reason.as_str()
        ),
        (
            None,
            None,
            Some(ModuleKind::Clipboard),
            "texto grande demais"
        )
    );
    *last_block.lock().unwrap() = Some("pausado".into());
    let blocks = status(&a.profile).await.unwrap().recent_blocks;
    assert_eq!(
        blocks.iter().map(|b| b.reason.as_str()).collect::<Vec<_>>(),
        ["pausado", "texto grande demais"]
    );
    let _ = stop_tx.send(());
    task.await.unwrap().unwrap();
}
