//! 2 · Dispositivos (§12.5): `devices_list`, `device_set_perms`, `device_revoke` and
//! `device_pause`. Owned by Rivet.
//!
//! `peers.toml` is the source of the list and of the permissions; the agent, when one answers,
//! adds the live part (state, address, the peer's modules and version). Every write goes through
//! the core (tmp + rename) and is followed by `ReloadPeers`, so the running agent applies it at
//! once.

use clipsync_core::{PeerEntry, Peers};
use clipsync_proto::{
    ControlRequest, ControlResponse, ModuleSet, PeerId, PeerPerms, PeerState, PeerStatus,
};
use serde::{Deserialize, Serialize};

use crate::api::{self, DeviceRevokeArgs, DeviceSetPermsArgs, code};
use crate::status::{self, Modules};
use crate::{Ctx, UiError, clean, msg};

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

impl Device {
    /// A `peers.toml` entry, with the agent's view of it when there is one.
    pub(crate) fn new(entry: &PeerEntry, live: Option<&PeerStatus>) -> Self {
        let (state, blocked_reason) = match live.map(|l| &l.state) {
            Some(PeerState::Connected) => (DeviceState::Connected, None),
            Some(PeerState::Connecting) => (DeviceState::Connecting, None),
            Some(PeerState::Blocked { reason }) => (DeviceState::Blocked, Some(clean(reason))),
            Some(PeerState::Disconnected) | None => (DeviceState::Offline, None),
        };
        let addr = live.and_then(|l| l.addr).or(entry.addr);
        Self {
            peer_id: entry.id.to_hex(),
            short: entry.id.short(),
            name: clean(&entry.name),
            state,
            blocked_reason,
            addr: addr.map(|a| a.to_string()),
            perms: entry.perms.into(),
            remote_modules: live.map_or(Modules::default(), |l| l.remote_modules.into()),
            remote_version: live.map(|l| l.remote_version).filter(|v| *v > 0),
            paired_at: Some(clean(&entry.paired_at)).filter(|p| !p.is_empty()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DevicesList {
    pub agent_running: bool,
    pub devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceReply {
    pub device: Device,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Removed {
    pub removed: bool,
}

fn unknown_peer() -> UiError {
    UiError::new(code::UNKNOWN_PEER, msg::UNKNOWN_PEER).with_field("peer_id")
}

fn find<'a>(peers: &'a Peers, id: &PeerId) -> Result<&'a PeerEntry, UiError> {
    peers.get(id).ok_or_else(unknown_peer)
}

/// The agent applies a new `peers.toml` at once; no agent is fine (it reads the file when it
/// starts).
pub(crate) async fn reload_peers(ctx: &Ctx) -> Result<(), UiError> {
    match ctx.request(ControlRequest::ReloadPeers).await {
        Ok(ControlResponse::Ok) => Ok(()),
        Err(e) if e.code == code::AGENT_NOT_RUNNING => Ok(()),
        Err(e) => Err(e),
        Ok(ControlResponse::Error { message }) => {
            Err(UiError::new(code::CONFIG, msg::config(&clean(&message))))
        }
        Ok(_) => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

pub(crate) async fn list(ctx: &Ctx) -> Result<DevicesList, UiError> {
    ctx.require_profile()?;
    let peers = ctx.profile.load_peers()?;
    let live = status::agent_status(ctx).await;
    let devices = peers
        .peer
        .iter()
        .map(|e| {
            let l = live
                .as_ref()
                .and_then(|s| s.peers.iter().find(|p| p.id == e.id));
            Device::new(e, l)
        })
        .collect();
    Ok(DevicesList {
        agent_running: live.is_some(),
        devices,
    })
}

/// A permission for a module that is off here would never apply: `module_off`, nothing saved.
fn check_modules(perms: Perms, local: ModuleSet) -> Result<(), UiError> {
    let clipboard = perms.clipboard_in || perms.clipboard_out;
    if (clipboard && !local.clipboard) || (perms.input_in && !local.input) {
        Err(UiError::new(code::MODULE_OFF, msg::MODULE_OFF).with_field("perms"))
    } else {
        Ok(())
    }
}

/// Turning `input_in` on asks for `acknowledge_full_trust` (§12.2 rule 8).
pub(crate) fn check_full_trust(was: bool, now: bool, acknowledged: bool) -> Result<(), UiError> {
    if now && !was && !acknowledged {
        Err(UiError::new(code::NEEDS_CONFIRMATION, msg::FULL_TRUST)
            .with_field("acknowledge_full_trust"))
    } else {
        Ok(())
    }
}

pub(crate) async fn set_perms(ctx: &Ctx, args: DeviceSetPermsArgs) -> Result<DeviceReply, UiError> {
    let id = api::peer_id(&args.peer_id, "peer_id")?;
    ctx.require_profile()?;
    let local = ctx.profile.load_config()?.modules;
    let entry = {
        // S6-01: one writer at a time here; the file lock in `update_peers` covers the agent
        // and other processes. The checks read the entry as it is inside the lock.
        let _one = ctx.peers_lock.lock().await;
        ctx.profile.update_peers(|peers| {
            let e = peers
                .peer
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or_else(unknown_peer)?;
            check_full_trust(
                e.perms.input_in,
                args.perms.input_in,
                args.acknowledge_full_trust,
            )?;
            check_modules(args.perms, local)?;
            e.perms = args.perms.into();
            Ok::<_, UiError>(e.clone())
        })?
    };
    reload_peers(ctx).await?;
    let live = status::agent_status(ctx).await;
    let l = live
        .as_ref()
        .and_then(|s| s.peers.iter().find(|p| p.id == id));
    Ok(DeviceReply {
        device: Device::new(&entry, l),
    })
}

/// The name typed on screen must be the peer's name as the list shows it.
pub(crate) async fn revoke(ctx: &Ctx, args: DeviceRevokeArgs) -> Result<Removed, UiError> {
    let id = api::peer_id(&args.peer_id, "peer_id")?;
    ctx.require_profile()?;
    {
        let _one = ctx.peers_lock.lock().await;
        ctx.profile.update_peers(|peers| {
            let name = clean(&find(peers, &id)?.name);
            if args.typed_name.trim() != name {
                return Err(
                    UiError::new(code::NAME_MISMATCH, msg::NAME_MISMATCH).with_field("typed_name")
                );
            }
            peers.peer.retain(|e| e.id != id);
            Ok(())
        })?;
    }
    reload_peers(ctx).await?;
    Ok(Removed { removed: true })
}

/// Feature `peer_pause` is off in this version (§12.2 rule 6).
pub(crate) fn pause(args: &api::DevicePauseArgs) -> Result<(), UiError> {
    api::peer_id(&args.peer_id, "peer_id")?;
    Err(UiError::unavailable())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_trust_is_asked_only_when_input_in_turns_on() {
        assert!(check_full_trust(false, true, false).is_err());
        assert!(check_full_trust(false, true, true).is_ok());
        assert!(check_full_trust(true, true, false).is_ok());
        assert!(check_full_trust(true, false, false).is_ok());
    }

    #[test]
    fn a_permission_for_a_module_off_here_is_refused() {
        let clip_only = ModuleSet {
            clipboard: true,
            input: false,
        };
        let input = Perms {
            input_in: true,
            ..Perms::default()
        };
        assert_eq!(
            check_modules(input, clip_only).unwrap_err().code,
            code::MODULE_OFF
        );
        let clip = Perms {
            clipboard_in: true,
            clipboard_out: true,
            input_in: false,
        };
        assert!(check_modules(clip, clip_only).is_ok());
    }
}
