//! Contract between core and modules, and the single source of authorization rules.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

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

#[async_trait::async_trait]
pub trait ModuleHost: Send + Sync + 'static {
    fn local_id(&self) -> PeerId;
    fn local_name(&self) -> String;
    /// All trusted peers, connected or not.
    fn peers(&self) -> Vec<PeerInfo>;
    fn paused(&self) -> bool;
    /// One `Frame::Module` on a new QUIC uni stream. Checks authorize_outbound first.
    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError>;
    /// QUIC datagram. Checks authorize_outbound_datagram and size.
    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError>;
}

#[derive(Debug)]
pub enum ModuleInput {
    /// Already authorized by core.
    Reliable {
        from: PeerId,
        msg: ModuleMsg,
    },
    /// Already authorized by core.
    Datagram {
        from: PeerId,
        dg: InputDatagram,
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
    /// `Enter` and `EnterAt`. Out: no local permission. In: input_in. Pause blocks.
    InputEnter,
    /// Out: input_in (I only reveal my monitors to whoever may control me). In: open.
    /// Pause ignored.
    Zones,
    /// Out: no local permission. In: input_in. Pause ignored.
    LayoutOffer,
    Open,
}

fn gate(msg: &ModuleMsg) -> Gate {
    match msg {
        ModuleMsg::Clip(ClipMsg::Offer(_)) => Gate::ClipOffer,
        ModuleMsg::Input(InputMsg::Enter { .. } | InputMsg::EnterAt { .. }) => Gate::InputEnter,
        ModuleMsg::Input(InputMsg::Zones { .. }) => Gate::Zones,
        ModuleMsg::Input(InputMsg::LayoutOffer(_)) => Gate::LayoutOffer,
        ModuleMsg::Clip(ClipMsg::Ack(_))
        | ModuleMsg::Input(
            InputMsg::Leave { .. }
            | InputMsg::ReleaseAll { .. }
            | InputMsg::StatsRequest { .. }
            | InputMsg::Stats(_),
        ) => Gate::Open,
    }
}

/// Outbound, checked in THIS ORDER, first failure wins:
/// LocalModuleDisabled -> RemoteModuleDisabled -> NotAuthorized -> Paused.
/// NotAuthorized on the way out exists for Clip::Offer (perms.clipboard_out) and Input::Zones
/// (perms.input_in); Enter, EnterAt and LayoutOffer have no local outbound permission (control
/// permission is checked at the DESTINATION). Zones and LayoutOffer ignore pause.
/// Ack, Stats, StatsRequest, Leave and ReleaseAll ignore pause and permission.
/// The protocol version (`min_version`, `RemoteTooOld`) is checked by the host, after this.
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
/// Clip::Offer needs perms.clipboard_in; Enter, EnterAt and LayoutOffer need perms.input_in.
/// Pause blocks Offer, Enter and EnterAt; LayoutOffer ignores it. Zones, Ack, Stats,
/// StatsRequest, Leave and ReleaseAll ignore pause and permission, so the MODULE only acts on
/// them if the session/id belongs to the sender.
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
