//! 4 · Layout de telas (§12.5). Owned by VECTOR (§12.1). This file is Rivet's stub: it builds,
//! every command answers `unavailable`, and Vector replaces the bodies (and the types, freely).
//!
//! Interface with the rest of the crate, fixed by Rivet:
//! - Router: `api::route` calls, for each command,
//!   `pub(crate) async fn <cmd>(ctx: &Ctx, args: <Args>) -> Result<T, UiError>` with
//!   `T: serde::Serialize` (the router turns it into JSON; a typed `T` lets
//!   `tests/fixtures/<cmd>.json` deserialize into it). `get` takes no args. The names are the
//!   six functions below; the args types are the structs below, which `UiRequest` deserializes
//!   with `deny_unknown_fields` (rename or reshape them here, the router only names them).
//! - Profile: `ctx.profile` is the `clipsync_core::Profile` (`ctx.profile.dir` = DIR, so
//!   `layout.toml` is `ctx.profile.dir.join("layout.toml")`); `ctx.require_profile()?` gives
//!   `not_initialized`. Write through tmp + rename (§12.2 rule 5).
//! - Agent: `ctx.request(ControlRequest::..)` and
//!   `ctx.ask_module(ModuleKind::Input, ModuleCommand::InputReload)`; both go through
//!   `control::request`, so in_app and external agents behave the same. No agent answers =
//!   `Err(UiError)` with code `agent_not_running` (after a save, skip `InputReload` then).
//!   `ask_module` hands `ModuleReply::Error(text)` back as `Ok`, for you to map to your codes.
//! - Events: `ctx.emit(UiEvent::LayoutChanged(..))` with reason `saved` after a write. `zones`,
//!   `offer` and `screen_lock_changed` come from the poll in `events.rs` (Rivet) via `InputState`.
//! - Peer text (names, monitors) goes through `crate::clean` before reaching JSON.
//! - Errors: `UiError::new(code, message)` with the codes of §12.5 (`same_device`,
//!   `stale_rev`, ...); Portuguese text where you prefer (a `msg` section of yours is fine).
//! - Self device: `crate::status::DeviceSelf::new(id, name)`; features: `app_state.features`
//!   follows `InputState` answering (status.rs).
//! - Tests: `tests/layout.rs` (Vector), through `Ui::call` only; helpers to start agents in
//!   temp profiles are in `tests/common/mod.rs` (Rivet).

use serde::Deserialize;
use serde_json::Value;

use crate::{Ctx, UiError};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// `monitor` = the monitor ref ("DP-1" or "#0"); `from` and `to` are fractions 0..1.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct EndpointArgs {
    pub peer_id: String,
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetLinkArgs {
    /// null creates.
    pub id: Option<String>,
    pub a: EndpointArgs,
    pub b: EndpointArgs,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RemoveLinkArgs {
    pub id: String,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetDeviceArgs {
    pub peer_id: String,
    pub x: i32,
    pub y: i32,
    /// null or 1..9.
    pub hotkey: Option<u8>,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetOptionsArgs {
    /// 0..32.
    pub dead_corner_px: u32,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CopyToPeersArgs {
    /// null = every peer.
    pub peer_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OfferAnswerArgs {
    pub accept: bool,
}

pub(crate) async fn get(_ctx: &Ctx) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn set_link(_ctx: &Ctx, _args: SetLinkArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn remove_link(_ctx: &Ctx, _args: RemoveLinkArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn set_device(_ctx: &Ctx, _args: SetDeviceArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn set_options(_ctx: &Ctx, _args: SetOptionsArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn copy_to_peers(_ctx: &Ctx, _args: CopyToPeersArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn offer_answer(_ctx: &Ctx, _args: OfferAnswerArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}
