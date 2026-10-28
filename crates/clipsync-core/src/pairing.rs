//! Pairing by numeric comparison with commitment (contract §5 rule 3). Owned by Rivet.
//!
//! On the control stream opened by the joiner (B): Hello B->A, Hello A->B, then
//! B->A Commit{H(nB)}, A->B Nonce{nA}, B->A Reveal{nB}, A checks H(nB). Only then both sides
//! derive the code from the TLS exporter with context nB||nA, so a man in the middle cannot
//! grind his TLS sessions until the two codes match. One attempt per window: any failure closes
//! with CloseCode::PairFailed and ends the host. The core prints nothing.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use clipsync_proto::{
    ALPN_PAIR, CloseCode, Frame, Hello, MAX_CONTROL_FRAME_BYTES, MIN_PROTOCOL_VERSION, ModuleSet,
    PAIR_COMMIT_DOMAIN, PAIR_EXPORTER_LABEL, PAIR_WINDOW_SECS, PROTOCOL_VERSION, PairMsg, PeerId,
    PeerPerms, WireError, read_frame, write_frame,
};
use quinn::{Connection, ConnectionError, Endpoint, RecvStream, SendStream};
use sha2::{Digest, Sha256};
use tokio::time::{Instant, timeout, timeout_at};

use crate::tls::{self, PinVerifier, VerifyMode};
use crate::transport::{close, hello_compatible, read_control};
use crate::{Config, CoreError, Identity, PeerEntry, Profile, control};

/// Bound on one attempt once a joiner arrived (both users still have to confirm).
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(PAIR_WINDOW_SECS);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a side waits for the peer to acknowledge its Confirm before closing.
const CONFIRM_FLUSH: Duration = Duration::from_secs(2);

pub struct PairPrompt {
    /// "123 456"
    pub code: String,
    pub peer_name: String,
    pub peer_id: PeerId,
    pub peer_modules: ModuleSet,
}

pub struct PairDecision {
    pub accept: bool,
    /// What I grant the peer. Masked to my enabled modules before saving.
    pub grants: PeerPerms,
}

#[derive(Debug)]
pub enum PairOutcome {
    Paired(PeerEntry),
    Declined { by_peer: bool },
}

/// H(nB) = SHA-256(PAIR_COMMIT_DOMAIN || nB).
pub fn commitment(nb: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(PAIR_COMMIT_DOMAIN);
    h.update(nb);
    h.finalize().into()
}

/// u64 BE of export_keying_material(8 bytes, PAIR_EXPORTER_LABEL, nB||nA) mod 1_000_000,
/// formatted "NNN NNN".
pub fn pair_code(conn: &Connection, nb: &[u8; 32], na: &[u8; 32]) -> Result<String, CoreError> {
    let mut out = [0u8; 8];
    conn.export_keying_material(&mut out, PAIR_EXPORTER_LABEL, &[&nb[..], &na[..]].concat())
        .map_err(|_| CoreError::Pairing("exporter unavailable".into()))?;
    let n = u64::from_be_bytes(out) % 1_000_000;
    Ok(format!("{:03} {:03}", n / 1000, n % 1000))
}

fn random32() -> [u8; 32] {
    rand::random()
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn hello(cfg: &Config) -> Hello {
    Hello {
        version: PROTOCOL_VERSION,
        min_version: MIN_PROTOCOL_VERSION,
        device_name: cfg.device_name.clone(),
        modules: cfg.modules,
        listen_port: cfg.listen.port(),
    }
}

/// A failed attempt: we close with PairFailed (or Version) and report it.
#[derive(Debug)]
enum Fail {
    /// Detected here; we close the connection with this code.
    Local(CloseCode, &'static str),
    /// The peer closed (or the connection died).
    Peer(ConnectionError),
}

impl Fail {
    fn into_error(self, conn: &Connection) -> CoreError {
        match self {
            Fail::Local(code, why) => {
                close(conn, code);
                CoreError::Pairing(why.into())
            }
            Fail::Peer(ConnectionError::ApplicationClosed(c)) => {
                let code = CloseCode::from_code(c.error_code.into_inner());
                CoreError::Pairing(format!("closed by peer: {code:?}"))
            }
            Fail::Peer(e) => CoreError::Pairing(e.to_string()),
        }
    }
}

fn wire_fail(e: WireError) -> Fail {
    match e {
        WireError::Io(_) => Fail::Local(CloseCode::PairFailed, "control stream lost"),
        _ => Fail::Local(CloseCode::PairFailed, "bad frame"),
    }
}

async fn read_pair(send: &mut SendStream, recv: &mut RecvStream) -> Result<PairMsg, Fail> {
    match read_control(send, recv).await.map_err(wire_fail)? {
        Frame::Pair(m) => Ok(m),
        _ => Err(Fail::Local(CloseCode::PairFailed, "frame out of order")),
    }
}

async fn write_pair(send: &mut SendStream, m: PairMsg) -> Result<(), Fail> {
    write_frame(send, &Frame::Pair(m), MAX_CONTROL_FRAME_BYTES)
        .await
        .map_err(wire_fail)
}

async fn read_hello(send: &mut SendStream, recv: &mut RecvStream) -> Result<Hello, Fail> {
    match read_control(send, recv).await.map_err(wire_fail)? {
        Frame::Hello(h) if hello_compatible(&h) => Ok(h),
        Frame::Hello(_) => Err(Fail::Local(CloseCode::Version, "protocol version mismatch")),
        _ => Err(Fail::Local(CloseCode::PairFailed, "frame out of order")),
    }
}

/// Both nonces exchanged, code derived: ask the user while reading the peer's Confirm.
struct Agreed {
    conn: Connection,
    send: SendStream,
    recv: RecvStream,
    prompt: PairPrompt,
    peer_hello: Hello,
}

/// Runs `confirm` off the runtime (it may read stdin), sends our Confirm and reads the peer's
/// concurrently, then saves peers.toml only if both accepted.
async fn finish<F>(
    p: &Profile,
    cfg: &Config,
    agreed: Agreed,
    peer_addr: SocketAddr,
    confirm: F,
) -> Result<PairOutcome, CoreError>
where
    F: FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
{
    let Agreed {
        conn,
        mut send,
        mut recv,
        prompt,
        peer_hello,
    } = agreed;
    let peer_id = prompt.peer_id;
    let local = async {
        let decision = tokio::task::spawn_blocking(move || confirm(&prompt))
            .await
            .map_err(|_| Fail::Local(CloseCode::PairFailed, "confirmation aborted"))?;
        write_pair(
            &mut send,
            PairMsg::Confirm {
                accept: decision.accept,
            },
        )
        .await?;
        let _ = send.finish();
        Ok::<_, Fail>((decision, send))
    };
    let remote = async {
        match read_frame(&mut recv, MAX_CONTROL_FRAME_BYTES).await {
            Ok(Frame::Pair(PairMsg::Confirm { accept })) => Ok(accept),
            // A declining peer always sends Confirm{false} before closing, so a close without
            // Confirm is a failed attempt (bad reveal, timeout), not a decline.
            Err(WireError::Io(_)) => match conn.close_reason() {
                Some(e) => Err(Fail::Peer(e)),
                None => Err(Fail::Local(CloseCode::PairFailed, "control stream lost")),
            },
            _ => Err(Fail::Local(CloseCode::PairFailed, "frame out of order")),
        }
    };
    // try_join: if the peer fails first, do not wait for the user (the blocking prompt is left
    // behind; the CLI exits on the error).
    let ((decision, send), remote_accept) =
        match timeout(ATTEMPT_TIMEOUT, async { tokio::try_join!(local, remote) }).await {
            Ok(Ok(r)) => r,
            Ok(Err(f)) => return Err(f.into_error(&conn)),
            Err(_) => return Err(Fail::Local(CloseCode::PairFailed, "timeout").into_error(&conn)),
        };
    // Let the peer receive our Confirm before the connection goes away.
    let _ = timeout(CONFIRM_FLUSH, send.stopped()).await;

    if !(decision.accept && remote_accept) {
        close(&conn, CloseCode::PairFailed);
        return Ok(PairOutcome::Declined {
            by_peer: decision.accept,
        });
    }
    let local_modules = cfg.modules;
    let entry = PeerEntry {
        id: peer_id,
        name: peer_hello.device_name,
        addr: Some(SocketAddr::new(peer_addr.ip(), peer_hello.listen_port)),
        paired_at: now_rfc3339(),
        perms: PeerPerms {
            clipboard_in: decision.grants.clipboard_in && local_modules.clipboard,
            clipboard_out: decision.grants.clipboard_out && local_modules.clipboard,
            input_in: decision.grants.input_in && local_modules.input,
        },
    };
    let saved = save(p, entry);
    close(
        &conn,
        if saved.is_ok() {
            CloseCode::Normal
        } else {
            CloseCode::PairFailed
        },
    );
    let entry = saved?;
    // A running agent picks the new pin up at once; no agent is fine.
    match control::request(p, clipsync_proto::ControlRequest::ReloadPeers).await {
        Ok(_) | Err(CoreError::AgentNotRunning) => {}
        Err(e) => tracing::warn!(error = %e, "agent did not reload peers"),
    }
    Ok(PairOutcome::Paired(entry))
}

/// Same id replaces; an entry at the same address with another key is the same device re-keyed
/// (re-pairing after "chave do par mudou"), so it goes too.
/// The name is checked again here (S1-02): Hello validation already refuses a bad name, but a
/// name that `load_peers` would refuse must never reach peers.toml.
fn save(p: &Profile, entry: PeerEntry) -> Result<PeerEntry, CoreError> {
    clipsync_proto::validate_device_name(&entry.name)
        .map_err(|_| CoreError::Pairing("invalid peer name".into()))?;
    p.update_peers(|peers| {
        peers
            .peer
            .retain(|e| e.id == entry.id || entry.addr.is_none() || e.addr != entry.addr);
        peers.upsert(entry.clone());
        Ok::<_, CoreError>(())
    })?;
    Ok(entry)
}

/// Host side (A). `on_listen` gets the bound address (ephemeral port resolved) before waiting.
pub async fn host(
    p: &Profile,
    port: u16,
    window: Duration,
    on_listen: impl FnOnce(SocketAddr) + Send,
    confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
) -> Result<PairOutcome, CoreError> {
    host_with(p, port, window, on_listen, confirm, HostHooks::default()).await
}

#[derive(Default)]
struct HostHooks {
    /// Tests: reports the nonce sent to the joiner.
    #[cfg(test)]
    sent_na: Option<tokio::sync::oneshot::Sender<[u8; 32]>>,
}

async fn host_with(
    p: &Profile,
    port: u16,
    window: Duration,
    on_listen: impl FnOnce(SocketAddr) + Send,
    confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
    #[allow(unused_mut, unused_variables)] mut hooks: HostHooks,
) -> Result<PairOutcome, CoreError> {
    let cfg = p.load_config()?;
    let identity = p.load_identity()?;
    let server = tls::server_config(&identity, PinVerifier::new(VerifyMode::Pairing), ALPN_PAIR)?;
    let bind = SocketAddr::new(cfg.listen.ip(), port);
    let endpoint = Endpoint::server(server, bind)
        .map_err(|e| CoreError::Transport(format!("bind {bind}: {e}")))?;
    on_listen(endpoint.local_addr()?);

    let deadline = Instant::now() + window;
    let incoming = match timeout_at(deadline, endpoint.accept()).await {
        Ok(Some(i)) => i,
        Ok(None) => return Err(CoreError::Transport("endpoint closed".into())),
        Err(_) => {
            endpoint.close(
                crate::transport::varint(CloseCode::PairFailed),
                CloseCode::PairFailed.reason(),
            );
            return Err(CoreError::Pairing("window expired".into()));
        }
    };
    // One attempt per window: every other connection is refused while this one runs.
    let refuser = {
        let endpoint = endpoint.clone();
        tokio::spawn(async move {
            while let Some(extra) = endpoint.accept().await {
                extra.refuse();
            }
        })
    };
    let peer_addr = incoming.remote_address();
    let result = async {
        let conn = timeout(CONNECT_TIMEOUT, incoming)
            .await
            .map_err(|_| CoreError::Pairing("handshake timeout".into()))?
            .map_err(|e| CoreError::Pairing(e.to_string()))?;
        let peer_id = tls::peer_id_of(&conn)
            .ok_or_else(|| CoreError::Pairing("no peer certificate".into()))?;
        let steps = async {
            let (mut send, mut recv) = conn.accept_bi().await.map_err(Fail::Peer)?;
            let peer_hello = read_hello(&mut send, &mut recv).await?;
            write_frame(
                &mut send,
                &Frame::Hello(hello(&cfg)),
                MAX_CONTROL_FRAME_BYTES,
            )
            .await
            .map_err(wire_fail)?;
            let PairMsg::Commit { h_nb } = read_pair(&mut send, &mut recv).await? else {
                return Err(Fail::Local(CloseCode::PairFailed, "expected Commit"));
            };
            let na = random32();
            write_pair(&mut send, PairMsg::Nonce { na }).await?;
            #[cfg(test)]
            if let Some(tx) = hooks.sent_na.take() {
                let _ = tx.send(na);
            }
            let PairMsg::Reveal { nb } = read_pair(&mut send, &mut recv).await? else {
                return Err(Fail::Local(CloseCode::PairFailed, "expected Reveal"));
            };
            if commitment(&nb) != h_nb {
                return Err(Fail::Local(
                    CloseCode::PairFailed,
                    "reveal does not match commitment",
                ));
            }
            let code = pair_code(&conn, &nb, &na)
                .map_err(|_| Fail::Local(CloseCode::PairFailed, "exporter"))?;
            Ok((send, recv, peer_hello, code))
        };
        let (send, recv, peer_hello, code) = match timeout(ATTEMPT_TIMEOUT, steps).await {
            Ok(Ok(v)) => v,
            Ok(Err(f)) => return Err(f.into_error(&conn)),
            Err(_) => return Err(Fail::Local(CloseCode::PairFailed, "timeout").into_error(&conn)),
        };
        let prompt = PairPrompt {
            code,
            peer_name: peer_hello.device_name.clone(),
            peer_id,
            peer_modules: peer_hello.modules,
        };
        let agreed = Agreed {
            conn,
            send,
            recv,
            prompt,
            peer_hello,
        };
        finish(p, &cfg, agreed, peer_addr, confirm).await
    }
    .await;
    refuser.abort();
    endpoint.close(
        crate::transport::varint(CloseCode::Normal),
        CloseCode::Normal.reason(),
    );
    let _ = timeout(Duration::from_secs(2), endpoint.wait_idle()).await;
    result
}

/// Joiner side (B).
pub async fn join(
    p: &Profile,
    addr: SocketAddr,
    confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
) -> Result<PairOutcome, CoreError> {
    join_with(p, addr, confirm, JoinHooks::default()).await
}

/// Tests: (nB, nA, connection) of an attempt.
#[cfg(test)]
type SessionNonces = ([u8; 32], [u8; 32], Connection);

#[derive(Default)]
struct JoinHooks {
    /// Tests: reveal a nonce that does not match the commitment.
    #[cfg(test)]
    bad_reveal: bool,
    /// Tests: nonces used, to recompute the code.
    #[cfg(test)]
    nonces: Option<tokio::sync::oneshot::Sender<SessionNonces>>,
    /// Tests: send a Hello with this name, bypassing validate() like an attacker would.
    #[cfg(test)]
    raw_hello_name: Option<String>,
}

async fn join_with(
    p: &Profile,
    addr: SocketAddr,
    confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
    #[allow(unused_mut, unused_variables)] mut hooks: JoinHooks,
) -> Result<PairOutcome, CoreError> {
    let cfg = p.load_config()?;
    let identity: Identity = p.load_identity()?;
    let any: IpAddr = if addr.is_ipv4() {
        Ipv4Addr::UNSPECIFIED.into()
    } else {
        Ipv6Addr::UNSPECIFIED.into()
    };
    let endpoint = Endpoint::client(SocketAddr::new(any, 0))
        .map_err(|e| CoreError::Transport(format!("bind: {e}")))?;
    let client = tls::client_config(&identity, PinVerifier::new(VerifyMode::Pairing), ALPN_PAIR)?;
    let connecting = endpoint
        .connect_with(client, addr, crate::SERVER_NAME)
        .map_err(|e| CoreError::Transport(e.to_string()))?;
    let conn = timeout(CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| CoreError::Transport(format!("no answer from {addr}")))?
        .map_err(|e| match e {
            ConnectionError::ConnectionClosed(_) | ConnectionError::ApplicationClosed(_) => {
                CoreError::Pairing(e.to_string())
            }
            other => CoreError::Transport(other.to_string()),
        })?;
    let peer_id =
        tls::peer_id_of(&conn).ok_or_else(|| CoreError::Pairing("no peer certificate".into()))?;
    let steps = async {
        let (mut send, mut recv) = conn.open_bi().await.map_err(Fail::Peer)?;
        #[cfg(test)]
        if let Some(name) = hooks.raw_hello_name.take() {
            // An attacker skips validate(): put the frame on the wire by hand.
            let mut h = hello(&cfg);
            h.device_name = name;
            let body = postcard::to_allocvec(&Frame::Hello(h))
                .map_err(|_| Fail::Local(CloseCode::PairFailed, "encode"))?;
            let mut raw = u32::try_from(body.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes()
                .to_vec();
            raw.extend_from_slice(&body);
            send.write_all(&raw)
                .await
                .map_err(|_| Fail::Local(CloseCode::PairFailed, "write"))?;
        } else {
            write_frame(
                &mut send,
                &Frame::Hello(hello(&cfg)),
                MAX_CONTROL_FRAME_BYTES,
            )
            .await
            .map_err(wire_fail)?;
        }
        #[cfg(not(test))]
        write_frame(
            &mut send,
            &Frame::Hello(hello(&cfg)),
            MAX_CONTROL_FRAME_BYTES,
        )
        .await
        .map_err(wire_fail)?;
        let peer_hello = match read_hello(&mut send, &mut recv).await {
            Ok(h) => h,
            Err(Fail::Local(_, _)) if conn.close_reason().is_some() => {
                return Err(Fail::Peer(
                    conn.close_reason().unwrap_or(ConnectionError::TimedOut),
                ));
            }
            Err(f) => return Err(f),
        };
        let nb = random32();
        write_pair(
            &mut send,
            PairMsg::Commit {
                h_nb: commitment(&nb),
            },
        )
        .await?;
        let PairMsg::Nonce { na } = read_pair(&mut send, &mut recv).await? else {
            return Err(Fail::Local(CloseCode::PairFailed, "expected Nonce"));
        };
        #[cfg(test)]
        let reveal = if hooks.bad_reveal { random32() } else { nb };
        #[cfg(not(test))]
        let reveal = nb;
        write_pair(&mut send, PairMsg::Reveal { nb: reveal }).await?;
        #[cfg(test)]
        if let Some(tx) = hooks.nonces.take() {
            let _ = tx.send((nb, na, conn.clone()));
        }
        let code = pair_code(&conn, &nb, &na)
            .map_err(|_| Fail::Local(CloseCode::PairFailed, "exporter"))?;
        Ok((send, recv, peer_hello, code))
    };
    let (send, recv, peer_hello, code) = match timeout(ATTEMPT_TIMEOUT, steps).await {
        Ok(Ok(v)) => v,
        Ok(Err(f)) => return Err(f.into_error(&conn)),
        Err(_) => return Err(Fail::Local(CloseCode::PairFailed, "timeout").into_error(&conn)),
    };
    let prompt = PairPrompt {
        code,
        peer_name: peer_hello.device_name.clone(),
        peer_id,
        peer_modules: peer_hello.modules,
    };
    let agreed = Agreed {
        conn,
        send,
        recv,
        prompt,
        peer_hello,
    };
    let result = finish(p, &cfg, agreed, addr, confirm).await;
    endpoint.close(
        crate::transport::varint(CloseCode::Normal),
        CloseCode::Normal.reason(),
    );
    let _ = timeout(Duration::from_secs(2), endpoint.wait_idle()).await;
    result
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use clipsync_proto::ModuleSet;
    use tokio::sync::oneshot;

    use super::*;
    use crate::InitOptions;

    fn profile(dir: &tempfile::TempDir, name: &str) -> Profile {
        let p = Profile {
            dir: dir.path().join(name),
        };
        p.init(InitOptions {
            device_name: name.into(),
            modules: ModuleSet {
                clipboard: true,
                input: true,
            },
            listen: "127.0.0.1:0".parse().unwrap(),
            fake_adapters: true,
        })
        .unwrap();
        p
    }

    const ALL: PeerPerms = PeerPerms {
        clipboard_in: true,
        clipboard_out: true,
        input_in: true,
    };

    fn accept(
        grants: PeerPerms,
        seen: Arc<Mutex<Option<String>>>,
    ) -> impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static {
        move |prompt: &PairPrompt| {
            *seen.lock().unwrap() = Some(prompt.code.clone());
            PairDecision {
                accept: true,
                grants,
            }
        }
    }

    async fn spawn_host(
        p: Profile,
        window: Duration,
        confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static,
        hooks: HostHooks,
    ) -> (
        SocketAddr,
        tokio::task::JoinHandle<Result<PairOutcome, CoreError>>,
    ) {
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            host_with(
                &p,
                0,
                window,
                move |a| {
                    let _ = tx.send(a);
                },
                confirm,
                hooks,
            )
            .await
        });
        (rx.await.unwrap(), task)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_happy_path_same_code_and_both_saved() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (profile(&dir, "a"), profile(&dir, "b"));
        let (code_a, code_b) = (Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None)));
        let grants_a = PeerPerms {
            clipboard_in: true,
            ..PeerPerms::default()
        };
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(10),
            accept(grants_a, code_a.clone()),
            HostHooks::default(),
        )
        .await;
        let joined = join(&b, addr, accept(ALL, code_b.clone())).await.unwrap();
        let hosted = host.await.unwrap().unwrap();
        let (ca, cb) = (
            code_a.lock().unwrap().clone().unwrap(),
            code_b.lock().unwrap().clone().unwrap(),
        );
        assert_eq!(ca, cb);
        assert_eq!(ca.len(), 7);
        let (PairOutcome::Paired(ea), PairOutcome::Paired(eb)) = (hosted, joined) else {
            panic!("not paired")
        };
        assert_eq!(ea.id, b.load_identity().unwrap().id());
        assert_eq!(eb.id, a.load_identity().unwrap().id());
        assert_eq!(ea.perms, grants_a);
        assert_eq!(eb.perms, ALL);
        assert_eq!(
            a.load_peers().unwrap().get(&ea.id).map(|e| e.name.clone()),
            Some("b".into())
        );
        assert_eq!(
            b.load_peers().unwrap().get(&eb.id).map(|e| e.name.clone()),
            Some("a".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_declined_on_one_side_saves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (profile(&dir, "a"), profile(&dir, "b"));
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(10),
            accept(ALL, Arc::default()),
            HostHooks::default(),
        )
        .await;
        let joined = join(&b, addr, |_: &PairPrompt| PairDecision {
            accept: false,
            grants: ALL,
        })
        .await
        .unwrap();
        let hosted = host.await.unwrap().unwrap();
        assert!(
            matches!(joined, PairOutcome::Declined { by_peer: false }),
            "{joined:?}"
        );
        assert!(
            matches!(hosted, PairOutcome::Declined { by_peer: true }),
            "{hosted:?}"
        );
        assert!(a.load_peers().unwrap().peer.is_empty());
        assert!(b.load_peers().unwrap().peer.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_bad_reveal_fails_saves_nothing_and_host_ends() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (profile(&dir, "a"), profile(&dir, "b"));
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(10),
            accept(ALL, Arc::default()),
            HostHooks::default(),
        )
        .await;
        let hooks = JoinHooks {
            bad_reveal: true,
            ..JoinHooks::default()
        };
        let joined = join_with(&b, addr, accept(ALL, Arc::default()), hooks).await;
        let hosted = timeout(Duration::from_secs(5), host)
            .await
            .expect("host must end")
            .unwrap();
        assert!(matches!(hosted, Err(CoreError::Pairing(_))), "{hosted:?}");
        assert!(matches!(joined, Err(CoreError::Pairing(_))), "{joined:?}");
        assert!(a.load_peers().unwrap().peer.is_empty());
        assert!(b.load_peers().unwrap().peer.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_window_takes_one_attempt_only() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, c) = (profile(&dir, "a"), profile(&dir, "b"), profile(&dir, "c"));
        // The host's user takes a while: the attempt is in progress when C knocks.
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let slow = move |_: &PairPrompt| {
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
            PairDecision {
                accept: true,
                grants: ALL,
            }
        };
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(20),
            slow,
            HostHooks::default(),
        )
        .await;
        let b_task = tokio::spawn(async move { join(&b, addr, accept(ALL, Arc::default())).await });
        tokio::time::sleep(Duration::from_millis(300)).await;
        // (b) a second connection during the attempt is refused.
        let second = join(&c, addr, accept(ALL, Arc::default())).await;
        assert!(
            second.is_err(),
            "second connection must be refused: {second:?}"
        );
        release_tx.send(()).unwrap();
        assert!(matches!(host.await.unwrap(), Ok(PairOutcome::Paired(_))));
        assert!(matches!(b_task.await.unwrap(), Ok(PairOutcome::Paired(_))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_after_a_failure_the_window_is_closed() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, c) = (profile(&dir, "a"), profile(&dir, "b"), profile(&dir, "c"));
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(30),
            accept(ALL, Arc::default()),
            HostHooks::default(),
        )
        .await;
        let hooks = JoinHooks {
            bad_reveal: true,
            ..JoinHooks::default()
        };
        let _ = join_with(&b, addr, accept(ALL, Arc::default()), hooks).await;
        // (c) the host ended long before its 30 s window...
        assert!(
            timeout(Duration::from_secs(5), host)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        // ...and nobody answers on that port any more.
        let retry = timeout(
            Duration::from_secs(4),
            join(&c, addr, accept(ALL, Arc::default())),
        )
        .await;
        assert!(
            !matches!(retry, Ok(Ok(_))),
            "a new attempt must not be accepted"
        );
        assert!(a.load_peers().unwrap().peer.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_code_depends_on_both_nonces() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (profile(&dir, "a"), profile(&dir, "b"));
        let (na_tx, na_rx) = oneshot::channel();
        let (n_tx, n_rx) = oneshot::channel();
        let seen = Arc::new(Mutex::new(None));
        let (addr, host) = spawn_host(
            a,
            Duration::from_secs(10),
            accept(ALL, seen.clone()),
            HostHooks {
                sent_na: Some(na_tx),
            },
        )
        .await;
        let hooks = JoinHooks {
            nonces: Some(n_tx),
            ..JoinHooks::default()
        };
        let b_task =
            tokio::spawn(
                async move { join_with(&b, addr, accept(ALL, Arc::default()), hooks).await },
            );
        let (nb, na, conn) = n_rx.await.unwrap();
        assert_eq!(na_rx.await.unwrap(), na);
        // (d) same TLS session, different nonces: different code.
        let code = pair_code(&conn, &nb, &na).unwrap();
        assert_ne!(pair_code(&conn, &random32(), &na).unwrap(), code);
        assert_ne!(pair_code(&conn, &nb, &random32()).unwrap(), code);
        assert_eq!(
            pair_code(&conn, &nb, &na).unwrap(),
            code,
            "deterministic for the same inputs"
        );
        assert!(matches!(host.await.unwrap(), Ok(PairOutcome::Paired(_))));
        assert!(b_task.await.unwrap().is_ok());
        assert_eq!(seen.lock().unwrap().clone(), Some(code));
    }

    /// S1-01/S1-02: a joiner whose Hello carries escape sequences (the forged "Código:" line)
    /// is refused before any code is shown, and nothing reaches peers.toml on either side.
    #[tokio::test(flavor = "multi_thread")]
    async fn pairing_malicious_peer_name_fails_and_saves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (profile(&dir, "a"), profile(&dir, "b"));
        let shown = Arc::new(Mutex::new(None));
        let (addr, host) = spawn_host(
            a.clone(),
            Duration::from_secs(10),
            accept(ALL, shown.clone()),
            HostHooks::default(),
        )
        .await;
        let evil = "x\u{1b}[1A\r\u{1b}[2KCódigo: 123 456\n".to_string();
        let hooks = JoinHooks {
            raw_hello_name: Some(evil),
            ..JoinHooks::default()
        };
        let joined = join_with(&b, addr, accept(ALL, Arc::default()), hooks).await;
        let hosted = timeout(Duration::from_secs(5), host)
            .await
            .expect("host must end")
            .unwrap();
        assert!(matches!(hosted, Err(CoreError::Pairing(_))), "{hosted:?}");
        assert!(joined.is_err(), "{joined:?}");
        assert_eq!(*shown.lock().unwrap(), None, "the host never prompted");
        assert!(a.load_peers().unwrap().peer.is_empty());
        assert!(b.load_peers().unwrap().peer.is_empty());
    }

    #[test]
    fn pairing_save_refuses_a_bad_name() {
        let dir = tempfile::tempdir().unwrap();
        let a = profile(&dir, "a");
        let entry = PeerEntry {
            id: PeerId([9; 32]),
            name: "b\u{202e}".into(),
            addr: None,
            paired_at: now_rfc3339(),
            perms: ALL,
        };
        assert!(matches!(save(&a, entry), Err(CoreError::Pairing(_))));
        assert!(a.load_peers().unwrap().peer.is_empty());
    }

    #[test]
    fn pairing_commitment_is_domain_separated() {
        let nb = [7u8; 32];
        assert_ne!(commitment(&nb), <[u8; 32]>::from(Sha256::digest(nb)));
        assert_eq!(commitment(&nb), commitment(&nb));
    }
}
