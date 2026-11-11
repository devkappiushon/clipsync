//! A peer whose stored address stopped answering (DHCP gave it another one) is found again by
//! its device name. The pin still decides who the peer is: a resolved address only counts after
//! the handshake proved the peer's key.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_core::tls::{self, PinSet, PinVerifier, VerifyMode};
use clipsync_core::transport::{Mesh, MeshOptions, Resolver, system_resolver};
use clipsync_core::{Identity, PeerEntry, Peers};
use clipsync_proto::*;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const ALL: PeerPerms = PeerPerms {
    clipboard_in: true,
    clipboard_out: true,
    input_in: true,
};

/// Past the dial timeout of the stale address (10 s) plus the deferral of the larger id.
const WAIT: Duration = Duration::from_secs(45);

struct Node {
    mesh: Mesh,
    _input: mpsc::Receiver<ModuleInput>,
    _clip: mpsc::Receiver<ModuleInput>,
}

fn start(identity: Identity, name: &str, peer: PeerEntry) -> Node {
    let (itx, input) = mpsc::channel(64);
    let (ctx, clip) = mpsc::channel(64);
    let mut peers = Peers::default();
    peers.upsert(peer);
    let mesh = Mesh::start(MeshOptions {
        identity,
        device_name: name.into(),
        listen: "127.0.0.1:0".parse().unwrap(),
        modules: HashMap::from([(ModuleKind::Input, itx), (ModuleKind::Clipboard, ctx)]),
        peers,
        paused: false,
    })
    .unwrap();
    Node {
        mesh,
        _input: input,
        _clip: clip,
    }
}

fn entry(id: PeerId, name: &str, addr: Option<SocketAddr>) -> PeerEntry {
    PeerEntry {
        id,
        name: name.into(),
        addr,
        paired_at: "2026-11-11T12:00:00Z".into(),
        perms: ALL,
    }
}

async fn until(what: &str, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < WAIT {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for: {what}");
}

/// The address A keeps for B answers nothing. The resolver knows two addresses for B's name: one
/// where another device listens (its key is not B's) and B's real one. A connects to B, keeps the
/// new address and tells the agent; the stranger never gets B blocked.
#[tokio::test(flavor = "multi_thread")]
async fn a_peer_that_changed_address_is_found_by_its_name() {
    let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (a_id, b_id) = (ia.id(), ib.id());
    // B never dials: only A finding B's new address can connect them.
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();

    // Bound and silent: what the address B had yesterday looks like.
    let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
    let stale = silent.local_addr().unwrap();

    // Another device took an address the resolver also answers.
    let stranger = Identity::generate().unwrap();
    let trusts_nobody = PinVerifier::new(VerifyMode::Mesh(Arc::new(PinSet::new([]))));
    let ep = quinn::Endpoint::server(
        tls::server_config(&stranger, trusts_nobody, ALPN_MESH).unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    )
    .unwrap();
    let stranger_addr = ep.local_addr().unwrap();
    let answering = {
        let ep = ep.clone();
        tokio::spawn(async move {
            while let Some(incoming) = ep.accept().await {
                let _ = incoming.await;
            }
        })
    };

    let a = start(ia, "notebook", entry(b_id, "pc", Some(stale)));
    let asked = Arc::new(Mutex::new(Vec::new()));
    let resolver: Resolver = {
        let asked = asked.clone();
        Arc::new(move |name, port| {
            asked.lock().unwrap().push((name, port));
            Box::pin(async move { vec![stranger_addr, b_addr] })
        })
    };
    a.mesh.resolve_names_with(resolver);
    let mut updates = a.mesh.address_updates().expect("taken once");

    until("A connected to B", || {
        a.mesh
            .peer_statuses()
            .iter()
            .any(|s| s.id == b_id && s.state == PeerState::Connected)
    })
    .await;
    let status = a.mesh.peer_statuses().remove(0);
    assert_eq!(
        status.addr,
        Some(b_addr),
        "A keeps the address that answered"
    );
    assert_eq!(
        asked.lock().unwrap().first(),
        Some(&("pc".to_owned(), stale.port())),
        "the resolver gets the peer's name and the port A knew"
    );
    let told = tokio::time::timeout(Duration::from_secs(5), updates.recv())
        .await
        .expect("the agent hears about the new address");
    assert_eq!(told, Some((b_id, b_addr)));

    answering.abort();
    a.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// The system resolver keeps the port of the stored address and refuses names that are not host names.
#[tokio::test(flavor = "multi_thread")]
async fn the_system_resolver_answers_with_the_port_it_was_given() {
    let found = system_resolver()("localhost".to_owned(), 47800).await;
    assert!(
        found.contains(&"127.0.0.1:47800".parse().unwrap()),
        "{found:?}"
    );
    // A name that cannot be a host name is never sent to the system.
    assert!(
        system_resolver()("pc do joão".to_owned(), 47800)
            .await
            .is_empty()
    );
    assert!(system_resolver()("-pc".to_owned(), 47800).await.is_empty());
    assert!(system_resolver()(String::new(), 47800).await.is_empty());
}
