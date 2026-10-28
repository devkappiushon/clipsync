//! 2 · Dispositivos (§12.5): `devices_list`, `device_set_perms`, `device_revoke` and
//! `device_pause`. Owned by Rivet.

use clipsync_proto::PeerPerms;
use serde::{Deserialize, Serialize};

use crate::status::Modules;

/// clipboard_in: the peer may SEND me clipboard · clipboard_out: I may send mine to it ·
/// input_in: the peer may CONTROL my keyboard and mouse (full trust).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Perms {
    pub clipboard_in: bool,
    pub clipboard_out: bool,
    pub input_in: bool,
}

impl From<PeerPerms> for Perms {
    fn from(p: PeerPerms) -> Self {
        Self {
            clipboard_in: p.clipboard_in,
            clipboard_out: p.clipboard_out,
            input_in: p.input_in,
        }
    }
}

impl From<Perms> for PeerPerms {
    fn from(p: Perms) -> Self {
        Self {
            clipboard_in: p.clipboard_in,
            clipboard_out: p.clipboard_out,
            input_in: p.input_in,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Connected,
    Connecting,
    Offline,
    Blocked,
}

/// A paired peer. Names and reasons are already `clean`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Device {
    /// 64 hex: what commands take.
    pub peer_id: String,
    /// Display only.
    pub short: String,
    pub name: String,
    pub state: DeviceState,
    pub blocked_reason: Option<String>,
    /// "IP:PORT".
    pub addr: Option<String>,
    pub perms: Perms,
    pub remote_modules: Modules,
    /// Protocol version of the peer's current connection.
    pub remote_version: Option<u16>,
    /// RFC 3339, UTC, from peers.toml.
    pub paired_at: Option<String>,
}
