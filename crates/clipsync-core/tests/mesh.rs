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
                } if self.echo => {
                    let stats = LoopbackStats {
                        session,
                        ..Default::default()
                    };
                    let _ = host
                        .send_reliable(from, ModuleMsg::Input(InputMsg::Stats(stats)))
                        .await;
                }
                ModuleInput::Datagram { from, dg } if self.echo => {
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
    let _ = r.stop.send(());
    r.task.await.unwrap().unwrap();
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
    assert_eq!(sa.listen, a.addr);
    assert!(
        sa.modules
            .iter()
            .all(|m| m.enabled && m.adapter.as_deref() == Some("probe"))
    );
    stop(ra).await;
    stop(rb).await;
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
        matches!(&blocks[..], [r] if r.peer == b.id && r.peer_name == "b" && r.cause == BlockCause::KeyChanged && r.at_unix_ms > 0),
        "{blocks:?}"
    );
    // Blocked: A does not dial again.
    let again = tokio::time::timeout(Duration::from_secs(3), ep.accept()).await;
    assert!(again.is_err(), "A must not redial a blocked peer");
    ep.close(0u32.into(), b"");
    stop(ra).await;
}

/// L6: every refusal of `authorize_inbound` lands in `recent_blocks`, newest first, capped, without content.
#[tokio::test(flavor = "multi_thread")]
async fn mesh_inbound_refusals_are_recorded_newest_first_and_capped() {
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
            .is_some_and(|s| s.recent_blocks.len() == MAX_RECENT_BLOCKS)
    })
    .await;
    let s = status(&b.profile).await.unwrap();
    for r in &s.recent_blocks {
        assert_eq!((r.peer, r.peer_name.as_str()), (a.id, "a"));
        assert_eq!(
            r.cause,
            BlockCause::Inbound {
                kind: ModuleKind::Clipboard,
                reason: RejectReason::NotAuthorized
            }
        );
    }
    assert!(
        s.recent_blocks
            .windows(2)
            .all(|w| w[0].at_unix_ms >= w[1].at_unix_ms),
        "newest first"
    );
    assert!(!format!("{s:?}").contains(SECRET));
    assert!(!serde_json::to_string(&s).unwrap().contains(SECRET));
    // A was not refused by anyone.
    assert!(status(&a.profile).await.unwrap().recent_blocks.is_empty());
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
                if let ModuleInput::Datagram { from, dg } = input
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
