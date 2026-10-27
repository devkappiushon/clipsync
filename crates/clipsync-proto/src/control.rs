//! IPC CLI <-> agent. Unix socket DIR/agent.sock, frame = u32 BE + JSON (serde_json),
//! max MAX_FRAME_BYTES. One request, one response, close.

use std::fmt;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::clip::DeliveryState;
use crate::ids::{EventId, ModuleKind, ModuleSet, PeerId};
use crate::input::{LayoutOffer, LoopbackStats, ZoneInfo};
use crate::module::PeerPerms;
use crate::redact::{ByteLen, Redacted};

/// `Debug` is manual: ClipSend text is redacted.
#[derive(Serialize, Deserialize)]
pub enum ControlRequest {
    Status,
    Pause,
    Resume,
    ReloadPeers,
    Module {
        kind: ModuleKind,
        cmd: ModuleCommand,
    },
}

impl fmt::Debug for ControlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlRequest::Status => f.write_str("Status"),
            ControlRequest::Pause => f.write_str("Pause"),
            ControlRequest::Resume => f.write_str("Resume"),
            ControlRequest::ReloadPeers => f.write_str("ReloadPeers"),
            ControlRequest::Module { kind, cmd } => f
                .debug_struct("Module")
                .field("kind", kind)
                .field("cmd", cmd)
                .finish(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ControlResponse {
    Status(AgentStatus),
    Ok,
    Module(ModuleReply),
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentStatus {
    pub device_name: String,
    pub id: PeerId,
    pub listen: SocketAddr,
    pub paused: bool,
    pub peers: Vec<PeerStatus>,
    pub modules: Vec<ModuleStatus>,
    /// Newest first, at most MAX_RECENT_BLOCKS. `default`: an older agent sends none.
    #[serde(default)]
    pub recent_blocks: Vec<BlockRecord>,
}

/// One refusal, newest first in `AgentStatus::recent_blocks`. Facts only, never content.
/// Sources: an inbound message refused by `authorize_inbound`, a peer turned
/// `PeerState::Blocked`, and a new `last_block` of a module (no peer then).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockRecord {
    /// Wall clock; display only.
    pub at_unix_ms: u64,
    pub peer: Option<PeerId>,
    pub peer_name: Option<String>,
    pub module: Option<ModuleKind>,
    /// Portuguese, ready to show.
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerStatus {
    pub name: String,
    pub id: PeerId,
    pub addr: Option<SocketAddr>,
    pub state: PeerState,
    pub perms: PeerPerms,
    pub remote_modules: ModuleSet,
    /// `Hello::version` of the current connection; 0 while not connected or from a 0.1.0 agent.
    #[serde(default)]
    pub remote_version: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    Connected,
    Connecting,
    Disconnected,
    Blocked { reason: String },
}

/// Texts already in Portuguese, never content.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModuleStatus {
    pub kind: ModuleKind,
    pub enabled: bool,
    pub adapter: Option<String>,
    pub authorized_peers: Vec<String>,
    pub last_event: Option<String>,
    pub last_block: Option<String>,
}

/// `Debug` is manual: ClipSend text is redacted.
#[derive(Serialize, Deserialize, Clone)]
pub enum ModuleCommand {
    Status,
    ClipSend { text: String, to: Option<PeerId> },
    ClipInbox,
    InputLoopback { to: PeerId, seconds: u32 },
    // ---- S6; new variants only at the end ----
    /// Writes an `in_inbox` or Stale item to the local clipboard. Works paused; no new offer.
    ClipApply { id: EventId },
    ClipDiscard { id: EventId },
    /// Sends what the local clipboard holds now, like `ClipSend`.
    ClipSendCurrent { to: Option<PeerId> },
    InputState,
    /// Reads `layout.toml` again and reinstalls the barriers.
    InputReload,
    InputLock { locked: bool },
    InputSwitch { to: PeerId },
    InputOffer { to: Vec<PeerId> },
    InputOfferClear,
}

impl fmt::Debug for ModuleCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleCommand::Status => f.write_str("Status"),
            ModuleCommand::ClipSend { text, to } => f
                .debug_struct("ClipSend")
                .field("text", &ByteLen(text.len()))
                .field("to", to)
                .finish(),
            ModuleCommand::ClipInbox => f.write_str("ClipInbox"),
            ModuleCommand::InputLoopback { to, seconds } => f
                .debug_struct("InputLoopback")
                .field("to", to)
                .field("seconds", seconds)
                .finish(),
            ModuleCommand::ClipApply { id } => f.debug_struct("ClipApply").field("id", id).finish(),
            ModuleCommand::ClipDiscard { id } => {
                f.debug_struct("ClipDiscard").field("id", id).finish()
            }
            ModuleCommand::ClipSendCurrent { to } => {
                f.debug_struct("ClipSendCurrent").field("to", to).finish()
            }
            ModuleCommand::InputState => f.write_str("InputState"),
            ModuleCommand::InputReload => f.write_str("InputReload"),
            ModuleCommand::InputLock { locked } => {
                f.debug_struct("InputLock").field("locked", locked).finish()
            }
            ModuleCommand::InputSwitch { to } => {
                f.debug_struct("InputSwitch").field("to", to).finish()
            }
            ModuleCommand::InputOffer { to } => {
                f.debug_struct("InputOffer").field("to", to).finish()
            }
            ModuleCommand::InputOfferClear => f.write_str("InputOfferClear"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ModuleReply {
    Status(ModuleStatus),
    ClipSent(Vec<SendOutcome>),
    ClipInbox(Vec<InboxItem>),
    InputLoopback(LoopbackStats),
    Error(String),
    // ---- S6; new variants only at the end ----
    Done,
    ClipApplied(DeliveryState),
    InputState(InputState),
    InputOffered(Vec<OfferOutcome>),
}

/// What the input module knows of the layout right now (answer to `InputState`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InputState {
    pub local: Vec<ZoneInfo>,
    /// Last `Zones` of each peer.
    pub peers: Vec<PeerZones>,
    pub screen_locked: bool,
    pub active_peer: Option<PeerId>,
    pub pending_offer: Option<PendingOffer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerZones {
    pub peer: PeerId,
    pub zones: Vec<ZoneInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PendingOffer {
    pub from: PeerId,
    pub offer: LayoutOffer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfferOutcome {
    pub peer: PeerId,
    pub state: OfferState,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferState {
    Offered,
    Offline,
    Denied,
    /// The peer speaks protocol 1 (`RemoteTooOld`).
    Unsupported,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendOutcome {
    pub peer: String,
    pub state: DeliveryState,
}

/// `Debug` is manual: no preview.
#[derive(Serialize, Deserialize, Clone)]
pub struct InboxItem {
    pub id: EventId,
    pub from: String,
    pub received_unix_ms: u64,
    pub mime: String,
    pub bytes: usize,
    /// Up to 200 chars; terminal only.
    pub preview: String,
    pub state: DeliveryState,
}

impl fmt::Debug for InboxItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InboxItem")
            .field("id", &self.id)
            .field("from", &self.from)
            .field("received_unix_ms", &self.received_unix_ms)
            .field("mime", &self.mime)
            .field("bytes", &self.bytes)
            .field("preview", &Redacted)
            .field("state", &self.state)
            .finish()
    }
}
