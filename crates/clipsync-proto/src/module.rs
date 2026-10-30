//! Contract between core and modules, and the single source of authorization rules.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, watch};

use crate::clip::ClipMsg;
use crate::control::{ModuleCommand, ModuleReply};
use crate::ids::{EventId, ModuleKind, ModuleSet, PeerId};
use crate::input::{InputDatagram, InputMsg};
use crate::limits::MIN_PROTOCOL_VERSION;
use crate::wire::{ModuleMsg, RejectReason};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerPerms {
    pub clipboard_in: bool,
    pub clipboard_out: bool,
    pub input_in: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub name: String,
    pub perms: PeerPerms,
    pub remote_modules: ModuleSet,
    pub connected: bool,
    /// `Hello::version` of the peer's current connection (0 while not connected).
    pub remote_version: u16,
}

/// Short English messages. Portuguese text lives in the `msg.rs` of whoever displays it.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    #[error("peer not connected")]
    NotConnected,
    #[error("not authorized")]
    NotAuthorized,
    #[error("module disabled on peer")]
    RemoteModuleDisabled,
    #[error("module disabled locally")]
    LocalModuleDisabled,
    #[error("paused")]
    Paused,
    #[error("message too large")]
    TooLarge,
    #[error("transport: {0}")]
    Transport(String),
    /// The peer's protocol version predates the message (`min_version`): nothing was sent.
    #[error("peer too old")]
    RemoteTooOld,
}

/// Lowest protocol version that understands `msg`. A 0.1.0 peer closes the connection on a
/// module frame it cannot decode, so the core sends nothing newer than the peer's Hello.
pub fn min_version(msg: &ModuleMsg) -> u16 {
    match msg {
        ModuleMsg::Input(
            InputMsg::InventoryV4(_)
            | InputMsg::EnterRoutes(_)
            | InputMsg::ReturnedRoute(_)
            | InputMsg::LayoutOfferV4(_),
        ) => 4,
        ModuleMsg::Input(
            InputMsg::EnterSpan { .. } | InputMsg::Returned { .. } | InputMsg::Refused { .. },
        ) => 3,
        ModuleMsg::Input(
            InputMsg::Zones { .. } | InputMsg::LayoutOffer(_) | InputMsg::EnterAt { .. },
        ) => 2,
        ModuleMsg::Clip(_)
        | ModuleMsg::Input(
            InputMsg::Enter { .. }
            | InputMsg::Leave { .. }
            | InputMsg::ReleaseAll { .. }
            | InputMsg::StatsRequest { .. }
            | InputMsg::Stats(_),
        ) => MIN_PROTOCOL_VERSION,
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ModuleError {
    #[error("config: {0}")]
    Config(String),
    #[error("adapter: {0}")]
    Adapter(String),
    #[error("{0}")]
    Other(String),
}

/// Local admission metadata. Never serialized or supplied by the remote peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveContext {
    pub connection: u64,
    pub input_epoch: u64,
    pub stream: Option<u64>,
}

/// Authoritative snapshot from the host, independent of queued PeerUp/Paused notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerInputContext {
    pub connection: u64,
    pub input_epoch: u64,
    pub input_in: bool,
    pub paused: bool,
    pub connected: bool,
}

impl PeerInputContext {
    pub fn permits(self, received: ReceiveContext) -> bool {
        self.connected
            && self.input_in
            && !self.paused
            && self.connection == received.connection
            && self.input_epoch == received.input_epoch
    }
}

/// Local lifecycle authority, retained independently of module data queues. No wire fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLifecycle {
    pub paused: bool,
    /// Every pause invalidates old producers, even when Resume overwrites the latest state.
    pub pause_generation: u64,
    /// Terminal: a later Resume cannot reopen this host.
    pub stopped: bool,
}

impl Default for HostLifecycle {
    fn default() -> Self {
        Self {
            paused: true,
            pause_generation: 0,
            stopped: true,
        }
    }
}

#[async_trait::async_trait]
pub trait ModuleHost: Send + Sync + 'static {
    fn local_id(&self) -> PeerId;
    fn local_name(&self) -> String;
    /// All trusted peers, connected or not.
    fn peers(&self) -> Vec<PeerInfo>;
    fn paused(&self) -> bool;
    /// Core and MemHub always provide a snapshot. None is the legacy notification-only path.
    /// Peer/connection changes also wake this receiver; inspect peer_input_context for authority.
    fn lifecycle_receiver(&self) -> Option<watch::Receiver<HostLifecycle>> {
        None
    }

    fn peer_input_context(&self, peer: PeerId) -> Option<PeerInputContext>;
    /// Input jobs cannot migrate to a replacement connection.
    async fn send_reliable_on(
        &self,
        to: PeerId,
        connection: u64,
        msg: ModuleMsg,
    ) -> Result<(), SendError>;
    /// The existing v1 rejection frame, including for peers without v3 Refused.
    async fn reject_input_on(
        &self,
        to: PeerId,
        connection: u64,
        reason: RejectReason,
    ) -> Result<(), SendError>;
    /// One module frame. Core bounds opening/writing/finish together to 2 seconds and resets
    /// unfinished streams on cancellation. Success is not acknowledgement of remote application.
    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError>;
    /// QUIC datagram. Checks authorize_outbound_datagram and size.
    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError>;
    /// Pauses (true) or resumes this agent the way the control's `pause` and `resume` do:
    /// state.toml, `ModuleInput::Paused` to every module, `pause_changed` in the app. Returns at
    /// once; the change arrives as `ModuleInput::Paused`. Used by the Win+Alt+P shortcut.
    fn request_pause(&self, paused: bool);
}

#[derive(Debug)]
pub enum ModuleInput {
    /// Already authorized by core.
    Reliable {
        from: PeerId,
        msg: ModuleMsg,
        context: ReceiveContext,
    },
    /// Already authorized by core.
    Datagram {
        from: PeerId,
        dg: InputDatagram,
        context: ReceiveContext,
    },
    Rejected {
        from: PeerId,
        ref_id: Option<EventId>,
        reason: RejectReason,
    },
    PeerUp(PeerInfo),
    PeerDown(PeerId),
    Paused(bool),
    Command {
        cmd: ModuleCommand,
        reply: oneshot::Sender<ModuleReply>,
    },
    Shutdown,
}

#[async_trait::async_trait]
pub trait Module: Send + 'static {
    fn kind(&self) -> ModuleKind;
    fn adapter_name(&self) -> String;
    /// Runs until Shutdown or a fatal error. Must not block the runtime
    /// (a synchronous adapter goes to spawn_blocking).
    async fn run(
        self: Box<Self>,
        host: Arc<dyn ModuleHost>,
        rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError>;
}

/// Messages gated by permission and/or pause. Everything else (Ack, Stats, StatsRequest,
/// Leave, ReleaseAll) only needs the local module enabled.
enum Gate {
    /// Out: clipboard_out. In: clipboard_in. Pause blocks.
    ClipOffer,
    /// `Enter`, `EnterAt`, `EnterSpan` and `EnterRoutes`. Out: no local permission. In: input_in. Pause blocks.
    InputEnter,
    /// `Zones`/`InventoryV4`. Out: input_in (I only reveal monitors to whoever may control me). In: open.
    /// Pause ignored.
    Zones,
    /// `LayoutOffer`/`LayoutOfferV4`. Out: no local permission. In: input_in. Pause ignored.
    LayoutOffer,
    /// Includes returned routes: the runtime checks the opening session and connection.
    Open,
}

fn gate(msg: &ModuleMsg) -> Gate {
    match msg {
        ModuleMsg::Clip(ClipMsg::Offer(_)) => Gate::ClipOffer,
        ModuleMsg::Input(
            InputMsg::Enter { .. }
            | InputMsg::EnterAt { .. }
            | InputMsg::EnterSpan { .. }
            | InputMsg::EnterRoutes(_),
        ) => Gate::InputEnter,
        ModuleMsg::Input(InputMsg::Zones { .. } | InputMsg::InventoryV4(_)) => Gate::Zones,
        ModuleMsg::Input(InputMsg::LayoutOffer(_) | InputMsg::LayoutOfferV4(_)) => {
            Gate::LayoutOffer
        }
        ModuleMsg::Clip(ClipMsg::Ack(_))
        | ModuleMsg::Input(
            InputMsg::Leave { .. }
            | InputMsg::ReleaseAll { .. }
            | InputMsg::StatsRequest { .. }
            | InputMsg::Stats(_)
            | InputMsg::Returned { .. }
            | InputMsg::Refused { .. }
            | InputMsg::ReturnedRoute(_),
        ) => Gate::Open,
    }
}

/// Outbound, checked in THIS ORDER, first failure wins:
/// LocalModuleDisabled -> RemoteModuleDisabled -> NotAuthorized -> Paused.
/// Clipboard offers require clipboard_out; Zones and InventoryV4 require input_in.
/// Entry and layout offers have no outbound grant: control permission is checked at the
/// destination. Pause blocks clipboard offers and entries. Open messages (including both
/// returned variants) only require the local module; their session authority is runtime-owned.
/// The protocol version (`min_version`, `RemoteTooOld`) is checked by the host after this.
pub fn authorize_outbound(
    local: ModuleSet,
    perms: &PeerPerms,
    remote: ModuleSet,
    paused: bool,
    msg: &ModuleMsg,
) -> Result<(), SendError> {
    let kind = msg.kind();
    if !local.contains(kind) {
        return Err(SendError::LocalModuleDisabled);
    }
    let (needs_perm, pausable) = match gate(msg) {
        Gate::Open => return Ok(()),
        Gate::ClipOffer => (!perms.clipboard_out, true),
        Gate::InputEnter => (false, true),
        Gate::Zones => (!perms.input_in, false),
        Gate::LayoutOffer => (false, false),
    };
    if !remote.contains(kind) {
        return Err(SendError::RemoteModuleDisabled);
    }
    if needs_perm {
        return Err(SendError::NotAuthorized);
    }
    if paused && pausable {
        return Err(SendError::Paused);
    }
    Ok(())
}

pub fn authorize_outbound_datagram(
    local: ModuleSet,
    remote: ModuleSet,
    paused: bool,
) -> Result<(), SendError> {
    if !local.input {
        return Err(SendError::LocalModuleDisabled);
    }
    if !remote.input {
        return Err(SendError::RemoteModuleDisabled);
    }
    if paused {
        return Err(SendError::Paused);
    }
    Ok(())
}

/// Inbound, checked in THIS ORDER: ModuleDisabled -> NotAuthorized -> Paused.
/// Clipboard offers require clipboard_in; all entry and layout-offer variants require
/// input_in. Pause blocks clipboard offers and entries; layout offers ignore it.
/// Inventories and open messages ignore permission and pause. The runtime must bind returned
/// routes to the peer, connection and session this node opened; they grant no reverse control.
pub fn authorize_inbound(
    local: ModuleSet,
    perms: &PeerPerms,
    paused: bool,
    msg: &ModuleMsg,
) -> Result<(), RejectReason> {
    if !local.contains(msg.kind()) {
        return Err(RejectReason::ModuleDisabled);
    }
    let (allowed, pausable) = match gate(msg) {
        Gate::Open | Gate::Zones => return Ok(()),
        Gate::ClipOffer => (perms.clipboard_in, true),
        Gate::InputEnter => (perms.input_in, true),
        Gate::LayoutOffer => (perms.input_in, false),
    };
    if !allowed {
        return Err(RejectReason::NotAuthorized);
    }
    if paused && pausable {
        return Err(RejectReason::Paused);
    }
    Ok(())
}

pub fn authorize_inbound_datagram(
    local: ModuleSet,
    perms: &PeerPerms,
    paused: bool,
) -> Result<(), RejectReason> {
    if !local.input {
        return Err(RejectReason::ModuleDisabled);
    }
    if !perms.input_in {
        return Err(RejectReason::NotAuthorized);
    }
    if paused {
        return Err(RejectReason::Paused);
    }
    Ok(())
}
