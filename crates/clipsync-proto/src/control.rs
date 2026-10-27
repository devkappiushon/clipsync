//! IPC CLI <-> agent. Unix socket DIR/agent.sock, frame = u32 BE + JSON (serde_json),
//! max MAX_FRAME_BYTES. One request, one response, close.

use std::fmt;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::clip::DeliveryState;
use crate::ids::{EventId, ModuleKind, ModuleSet, PeerId};
use crate::input::LoopbackStats;
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerStatus {
    pub name: String,
    pub id: PeerId,
    pub addr: Option<SocketAddr>,
    pub state: PeerState,
    pub perms: PeerPerms,
    pub remote_modules: ModuleSet,
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
