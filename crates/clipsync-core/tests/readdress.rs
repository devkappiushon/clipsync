//! A peer whose pairing address stopped answering (DHCP gave it another one) is found again by
//! its device name. The pin still decides who the peer is: a resolved address only counts after
//! the handshake proved the peer's key, and it is kept as a cache beside the pairing address,
//! never in its place: what a name answers proves nothing about who owns an address.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_core::tls::{self, PinSet, PinVerifier, VerifyMode};
use clipsync_core::transport::{AddressUpdate, Mesh, MeshOptions, Resolver, system_resolver};
use clipsync_core::{Identity, LearnedAddr, PeerEntry, Peers};
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

/// Past two rounds of the dial timeout of the stale address (10 s each), the candidates of the
/// first round and the deferral of the larger id.
const WAIT: Duration = Duration::from_secs(75);

struct Node {
    mesh: Mesh,
    _input: mpsc::Receiver<ModuleInput>,
    _clip: mpsc::Receiver<ModuleInput>,
}

fn start(identity: Identity, name: &str, peer: PeerEntry) -> Node {
    start_cached(identity, name, peer, Vec::new())
}

/// `start`, with the address cache a previous run left.
fn start_cached(identity: Identity, name: &str, peer: PeerEntry, cached: Vec<LearnedAddr>) -> Node {
    let (itx, input) = mpsc::channel(64);
    let (ctx, clip) = mpsc::channel(64);
    let mut peers = Peers::default();
    peers.upsert(peer);
    let mesh = Mesh::start_with_cache(
        MeshOptions {
            identity,
            device_name: name.into(),
            listen: "127.0.0.1:0".parse().unwrap(),
            modules: HashMap::from([(ModuleKind::Input, itx), (ModuleKind::Clipboard, ctx)]),
            peers,
            paused: false,
        },
        cached,
    )
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
        "A shows the address that answered"
    );
    assert_eq!(
        asked.lock().unwrap().first(),
        Some(&("pc".to_owned(), stale.port())),
        "the resolver gets the peer's name and the port A knew"
    );
    let told = tokio::time::timeout(Duration::from_secs(5), updates.recv())
        .await
        .expect("the agent hears about the new address");
    assert_eq!(
        told,
        Some(AddressUpdate::Learned(LearnedAddr {
            peer: b_id,
            paired: stale,
            learned: b_addr,
        })),
        "the cache entry names the pairing address it stands in for"
    );

    answering.abort();
    a.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// A silent UDP socket: an address where nothing answers.
fn silent() -> (UdpSocket, SocketAddr) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr = socket.local_addr().unwrap();
    (socket, addr)
}

/// Another device answering with its own key, and the task that keeps it answering.
fn stranger() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let identity = Identity::generate().unwrap();
    let trusts_nobody = PinVerifier::new(VerifyMode::Mesh(Arc::new(PinSet::new([]))));
    let ep = quinn::Endpoint::server(
        tls::server_config(&identity, trusts_nobody, ALPN_MESH).unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    )
    .unwrap();
    let addr = ep.local_addr().unwrap();
    let answering = tokio::spawn(async move {
        while let Some(incoming) = ep.accept().await {
            let _ = incoming.await;
        }
    });
    (addr, answering)
}

fn resolves_to(found: Vec<SocketAddr>) -> Resolver {
    Arc::new(move |_, _| {
        let found = found.clone();
        Box::pin(async move { found })
    })
}

fn connected(mesh: &Mesh, peer: PeerId) -> bool {
    mesh.peer_statuses()
        .iter()
        .any(|s| s.id == peer && s.state == PeerState::Connected)
}

/// The address learned on an earlier run now belongs to another device (S1-SEG-01: a third party
/// can get its own address learned by relaying the handshake, then answer there with its own
/// key). A learned address is only a cache: the stranger gets it dropped and never gets B
/// blocked, and A finds B by name again.
#[tokio::test(flavor = "multi_thread")]
async fn a_stranger_at_a_learned_address_never_blocks_the_peer() {
    let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (a_id, b_id) = (ia.id(), ib.id());
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();
    let (_silent, stale) = silent();
    let (stranger_addr, answering) = stranger();

    let a = start_cached(
        ia,
        "notebook",
        entry(b_id, "pc", Some(stale)),
        vec![LearnedAddr {
            peer: b_id,
            paired: stale,
            learned: stranger_addr,
        }],
    );
    let mut updates = a.mesh.address_updates().expect("taken once");
    a.mesh.resolve_names_with(resolves_to(vec![b_addr]));

    until("A connected to B", || connected(&a.mesh, b_id)).await;
    assert_eq!(a.mesh.peer_statuses().remove(0).addr, Some(b_addr));
    let mut told = Vec::new();
    while let Ok(update) = updates.try_recv() {
        told.push(update);
    }
    assert_eq!(
        told,
        vec![
            AddressUpdate::Forgotten(b_id),
            AddressUpdate::Learned(LearnedAddr {
                peer: b_id,
                paired: stale,
                learned: b_addr,
            }),
        ],
        "the stranger's address leaves the cache before B's enters it"
    );

    answering.abort();
    a.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// The resolver keeps answering three dead addresses before the live one (R1-D03). Each round
/// tries a few candidates and the next round goes on from where the last one stopped, so the
/// fourth is reached.
#[tokio::test(flavor = "multi_thread")]
async fn a_candidate_past_the_first_three_is_reached_on_a_later_round() {
    let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (a_id, b_id) = (ia.id(), ib.id());
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();
    let (_silent, stale) = silent();
    let dead: Vec<(UdpSocket, SocketAddr)> = (0..3).map(|_| silent()).collect();
    let mut found: Vec<SocketAddr> = dead.iter().map(|(_, addr)| *addr).collect();
    found.push(b_addr);

    let a = start(ia, "notebook", entry(b_id, "pc", Some(stale)));
    a.mesh.resolve_names_with(resolves_to(found));

    until("A connected to B", || connected(&a.mesh, b_id)).await;
    assert_eq!(a.mesh.peer_statuses().remove(0).addr, Some(b_addr));

    a.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// A learned address only stands in for the pairing address it was found for: a reload that
/// changes the pairing address drops what was learned for the old one, and a cache entry
/// written for another pairing address is not taken at the start (R1-D01 and R1-D02).
#[tokio::test(flavor = "multi_thread")]
async fn a_reload_that_changes_the_pairing_address_drops_what_was_learned_for_the_old_one() {
    let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (a_id, b_id) = (ia.id(), ib.id());
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();
    let (_silent, stale) = silent();
    let (_silent_2, edited) = silent();
    let cached = vec![LearnedAddr {
        peer: b_id,
        paired: stale,
        learned: b_addr,
    }];

    let a = start_cached(
        ia,
        "notebook",
        entry(b_id, "pc", Some(stale)),
        cached.clone(),
    );
    let mut updates = a.mesh.address_updates().expect("taken once");
    until("A connected to B at the cached address", || {
        connected(&a.mesh, b_id)
    })
    .await;
    assert_eq!(a.mesh.peer_statuses().remove(0).addr, Some(b_addr));

    // The user edits the pairing address (CLI, UI or a new pairing).
    let mut peers = Peers::default();
    peers.upsert(entry(b_id, "pc", Some(edited)));
    a.mesh.reload(peers).await;
    assert_eq!(
        a.mesh.peer_statuses().remove(0).addr,
        Some(edited),
        "the edit wins over what was learned for the old address"
    );
    assert_eq!(
        updates.try_recv().ok(),
        Some(AddressUpdate::Forgotten(b_id))
    );

    // The peer leaves peers.toml: nothing about it stays cached (S1b-PRIV-01).
    a.mesh.reload(Peers::default()).await;
    assert_eq!(
        updates.try_recv().ok(),
        Some(AddressUpdate::Forgotten(b_id))
    );
    a.mesh.shutdown().await;

    // A mesh started with the edited pairing address does not take the old cache entry.
    let c = start_cached(
        Identity::generate().unwrap(),
        "notebook",
        entry(b_id, "pc", Some(edited)),
        cached,
    );
    assert_eq!(c.mesh.peer_statuses().remove(0).addr, Some(edited));
    c.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// While the dial to the cached address is still waiting, the user corrects the pairing address
/// (R3-N01). The round must not go on to the old pairing address, where another device now
/// answers with its own key, and block the corrected entry on that answer.
#[tokio::test(flavor = "multi_thread")]
async fn a_pairing_address_corrected_during_a_round_is_not_blocked_by_the_old_one() {
    // A dials first only when its id is the smaller one: no deferral before the round.
    let (ia, ib) = loop {
        let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
        if ia.id() < ib.id() {
            break (ia, ib);
        }
    };
    let (a_id, b_id) = (ia.id(), ib.id());
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();
    let (old_pairing, answering) = stranger();
    let (_silent, cached) = silent();

    let a = start_cached(
        ia,
        "notebook",
        entry(b_id, "pc", Some(old_pairing)),
        vec![LearnedAddr {
            peer: b_id,
            paired: old_pairing,
            learned: cached,
        }],
    );
    // The cached address answers nothing for 3 s: the correction lands inside that wait.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let mut peers = Peers::default();
    peers.upsert(entry(b_id, "pc", Some(b_addr)));
    a.mesh.reload(peers).await;

    until("A connected to B at the corrected address", || {
        let status = a.mesh.peer_statuses().remove(0);
        assert!(
            !matches!(status.state, PeerState::Blocked { .. }),
            "the answer of the old address blocked the corrected entry"
        );
        status.state == PeerState::Connected
    })
    .await;
    assert_eq!(a.mesh.peer_statuses().remove(0).addr, Some(b_addr));

    answering.abort();
    a.mesh.shutdown().await;
    b.mesh.shutdown().await;
}

/// The cached address answers nothing and the pairing address works again (R3-N03): the cache
/// entry goes, so the status shows where the peer really is and the next round does not wait
/// on the dead address.
#[tokio::test(flavor = "multi_thread")]
async fn a_pairing_address_that_works_again_drops_the_cache() {
    let (ia, ib) = (Identity::generate().unwrap(), Identity::generate().unwrap());
    let (a_id, b_id) = (ia.id(), ib.id());
    let b = start(ib, "pc", entry(a_id, "notebook", None));
    let b_addr = b.mesh.local_addr().unwrap();
    let (_silent, cached) = silent();

    let a = start_cached(
        ia,
        "notebook",
        entry(b_id, "pc", Some(b_addr)),
        vec![LearnedAddr {
            peer: b_id,
            paired: b_addr,
            learned: cached,
        }],
    );
    let mut updates = a.mesh.address_updates().expect("taken once");
    until("A connected to B", || connected(&a.mesh, b_id)).await;
    assert_eq!(
        a.mesh.peer_statuses().remove(0).addr,
        Some(b_addr),
        "the status shows the address that answered, not the dead cache"
    );
    assert_eq!(
        updates.try_recv().ok(),
        Some(AddressUpdate::Forgotten(b_id))
    );

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
    // Only a single label: a dotted name would be a query for somebody else's domain.
    assert!(
        system_resolver()("pc.example.com".to_owned(), 47800)
            .await
            .is_empty()
    );
}
