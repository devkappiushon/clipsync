//! In-memory network with the SAME `authorize_*` rules and codec checks as core.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::mpsc;

use crate::clip::ClipMsg;
use crate::ids::{EventId, ModuleSet, PeerId};
use crate::input::InputDatagram;
use crate::limits::{MAX_FRAME_BYTES, PROTOCOL_VERSION};
use crate::module::{
    ModuleHost, ModuleInput, PeerInfo, PeerPerms, SendError, authorize_inbound,
    authorize_inbound_datagram, authorize_outbound, authorize_outbound_datagram, min_version,
};
use crate::wire::{Datagram, Frame, ModuleMsg, WireError, encode_datagram, encode_frame};

const CHANNEL_CAP: usize = 1024;

pub struct MemHub {
    state: Arc<Mutex<HubState>>,
}

pub struct MemNode {
    pub id: PeerId,
    pub host: Arc<dyn ModuleHost>,
    pub tx: mpsc::Sender<ModuleInput>,
    pub rx: mpsc::Receiver<ModuleInput>,
}

#[derive(Default)]
struct HubState {
    nodes: HashMap<PeerId, NodeState>,
    /// (granter, grantee) -> permissions the granter gave the grantee (stored on the granter).
    grants: HashMap<(PeerId, PeerId), PeerPerms>,
    delivered: HashMap<(PeerId, PeerId), usize>,
    /// `ModuleHost::request_pause` calls per node.
    pause_requests: HashMap<PeerId, usize>,
}

struct NodeState {
    name: String,
    modules: ModuleSet,
    paused: bool,
    /// Protocol version the node announces; PROTOCOL_VERSION unless a test sets another.
    version: u16,
    tx: mpsc::Sender<ModuleInput>,
}

impl HubState {
    fn peer_info(&self, me: PeerId, other: PeerId) -> Option<PeerInfo> {
        let perms = *self.grants.get(&(me, other))?;
        let node = self.nodes.get(&other)?;
        Some(PeerInfo {
            id: other,
            name: node.name.clone(),
            perms,
            remote_modules: node.modules,
            connected: true,
            remote_version: node.version,
        })
    }
}

fn lock(state: &Mutex<HubState>) -> MutexGuard<'_, HubState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Default for MemHub {
    fn default() -> Self {
        Self::new()
    }
}

impl MemHub {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HubState::default())),
        }
    }

    /// The node id is derived from `name`, so names must be unique per hub.
    pub fn add_node(&self, name: &str, modules: ModuleSet) -> MemNode {
        let id = PeerId::from_spki_der(format!("memhub:{name}").as_bytes());
        let (tx, rx) = mpsc::channel(CHANNEL_CAP);
        lock(&self.state).nodes.insert(
            id,
            NodeState {
                name: name.to_string(),
                modules,
                paused: false,
                version: PROTOCOL_VERSION,
                tx: tx.clone(),
            },
        );
        let host = Arc::new(MemHost {
            state: self.state.clone(),
            id,
            name: name.to_string(),
        });
        MemNode { id, host, tx, rx }
    }

    /// a_grants = permissions A gives B (stored on A); same for b_grants.
    /// Delivers ModuleInput::PeerUp to BOTH sides.
    pub fn link(&self, a: PeerId, b: PeerId, a_grants: PeerPerms, b_grants: PeerPerms) {
        let mut s = lock(&self.state);
        s.grants.insert((a, b), a_grants);
        s.grants.insert((b, a), b_grants);
        for (me, other) in [(a, b), (b, a)] {
            let info = s.peer_info(me, other);
            if let (Some(info), Some(node)) = (info, s.nodes.get(&me)) {
                let _ = node.tx.try_send(ModuleInput::PeerUp(info));
            }
        }
    }

    /// Undoes `link`, like a peer revoked on both sides (§13.4 item 6): the grants go, `peers()`
    /// stops listing the other, sends answer NotConnected, and BOTH sides get
    /// ModuleInput::PeerDown. Nodes that were not linked get nothing.
    pub fn unlink(&self, a: PeerId, b: PeerId) {
        let mut s = lock(&self.state);
        let ab = s.grants.remove(&(a, b)).is_some();
        let ba = s.grants.remove(&(b, a)).is_some();
        if !(ab || ba) {
            return;
        }
        for (me, other) in [(a, b), (b, a)] {
            if let Some(node) = s.nodes.get(&me) {
                let _ = node.tx.try_send(ModuleInput::PeerDown(other));
            }
        }
    }

    /// Also delivers ModuleInput::Paused.
    pub fn set_paused(&self, node: PeerId, paused: bool) {
        let mut s = lock(&self.state);
        if let Some(n) = s.nodes.get_mut(&node) {
            n.paused = paused;
            let _ = n.tx.try_send(ModuleInput::Paused(paused));
        }
    }

    /// The protocol version `node` announces (a 0.1.0 peer is 1). Call before `link`: the
    /// PeerUp of `link` carries it, and `send_reliable` refuses newer messages with RemoteTooOld.
    pub fn set_remote_version(&self, node: PeerId, version: u16) {
        if let Some(n) = lock(&self.state).nodes.get_mut(&node) {
            n.version = version;
        }
    }

    /// How many times `node` called `ModuleHost::request_pause`.
    pub fn pause_requests(&self, node: PeerId) -> usize {
        lock(&self.state)
            .pause_requests
            .get(&node)
            .copied()
            .unwrap_or(0)
    }

    /// Counts only reliable frames DELIVERED to the destination. A rejected inbound frame does
    /// not count and becomes ModuleInput::Rejected{from: destination, ref_id, reason} at the sender.
    pub fn sent_count(&self, from: PeerId, to: PeerId) -> usize {
        lock(&self.state)
            .delivered
            .get(&(from, to))
            .copied()
            .unwrap_or(0)
    }
}

struct MemHost {
    state: Arc<Mutex<HubState>>,
    id: PeerId,
    name: String,
}

enum Route {
    Deliver(mpsc::Sender<ModuleInput>),
    Reject(mpsc::Sender<ModuleInput>, crate::wire::RejectReason),
}

fn send_error(e: WireError) -> SendError {
    match e {
        WireError::TooLarge { .. } => SendError::TooLarge,
        other => SendError::Transport(other.to_string()),
    }
}

fn ref_id(msg: &ModuleMsg) -> Option<EventId> {
    match msg {
        ModuleMsg::Clip(ClipMsg::Offer(o)) => Some(o.id),
        _ => None,
    }
}

#[async_trait::async_trait]
impl ModuleHost for MemHost {
    fn local_id(&self) -> PeerId {
        self.id
    }

    fn local_name(&self) -> String {
        self.name.clone()
    }

    fn peers(&self) -> Vec<PeerInfo> {
        let s = lock(&self.state);
        let mut peers: Vec<PeerInfo> = s
            .grants
            .keys()
            .filter(|(granter, _)| *granter == self.id)
            .filter_map(|(_, other)| s.peer_info(self.id, *other))
            .collect();
        peers.sort_by_key(|p| p.id);
        peers
    }

    fn paused(&self) -> bool {
        lock(&self.state)
            .nodes
            .get(&self.id)
            .is_some_and(|n| n.paused)
    }

    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError> {
        let route = {
            let s = lock(&self.state);
            let (Some(me), Some(peer), Some(mine), Some(theirs)) = (
                s.nodes.get(&self.id),
                s.nodes.get(&to),
                s.grants.get(&(self.id, to)),
                s.grants.get(&(to, self.id)),
            ) else {
                return Err(SendError::NotConnected);
            };
            authorize_outbound(me.modules, mine, peer.modules, me.paused, &msg)?;
            if peer.version < min_version(&msg) {
                return Err(SendError::RemoteTooOld);
            }
            match authorize_inbound(peer.modules, theirs, peer.paused, &msg) {
                Ok(()) => Route::Deliver(peer.tx.clone()),
                Err(reason) => Route::Reject(me.tx.clone(), reason),
            }
        };
        let frame = Frame::Module(msg);
        encode_frame(&frame, MAX_FRAME_BYTES).map_err(send_error)?;
        let Frame::Module(msg) = frame else {
            return Err(SendError::Transport("internal".into()));
        };
        match route {
            Route::Deliver(tx) => {
                tx.send(ModuleInput::Reliable { from: self.id, msg })
                    .await
                    .map_err(|_| SendError::NotConnected)?;
                *lock(&self.state)
                    .delivered
                    .entry((self.id, to))
                    .or_default() += 1;
            }
            Route::Reject(tx, reason) => {
                let rejected = ModuleInput::Rejected {
                    from: to,
                    ref_id: ref_id(&msg),
                    reason,
                };
                let _ = tx.send(rejected).await;
            }
        }
        Ok(())
    }

    /// Counted (`MemHub::pause_requests`) and applied like `MemHub::set_paused`.
    fn request_pause(&self, paused: bool) {
        let mut s = lock(&self.state);
        *s.pause_requests.entry(self.id).or_default() += 1;
        if let Some(n) = s.nodes.get_mut(&self.id) {
            n.paused = paused;
            let _ = n.tx.try_send(ModuleInput::Paused(paused));
        }
    }

    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError> {
        let s = lock(&self.state);
        let (Some(me), Some(peer), Some(theirs)) = (
            s.nodes.get(&self.id),
            s.nodes.get(&to),
            s.grants.get(&(to, self.id)),
        ) else {
            return Err(SendError::NotConnected);
        };
        if !s.grants.contains_key(&(self.id, to)) {
            return Err(SendError::NotConnected);
        }
        authorize_outbound_datagram(me.modules, peer.modules, me.paused)?;
        let d = Datagram::Input(dg);
        encode_datagram(&d).map_err(send_error)?;
        // Rejected inbound datagrams are dropped without a reply, like core.
        if authorize_inbound_datagram(peer.modules, theirs, peer.paused).is_ok() {
            let Datagram::Input(dg) = d;
            // A full queue drops the datagram, like a lossy link.
            let _ = peer
                .tx
                .try_send(ModuleInput::Datagram { from: self.id, dg });
        }
        Ok(())
    }
}
