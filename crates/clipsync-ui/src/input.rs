//! 4 · Comandos de input da tela de layout (§12.5): `lock_to_screen`, `switch_to_device` and
//! `input_test`. Owned by VECTOR (§12.1). This file is Rivet's stub: it builds, every command
//! answers `unavailable`, and Vector replaces the bodies (and the types, freely).
//!
//! Interface with the rest of the crate, fixed by Rivet (same as `layout.rs`):
//! - Router: `api::route` calls `pub(crate) async fn <cmd>(ctx: &Ctx, args: <Args>) ->
//!   Result<T, UiError>` with `T: serde::Serialize`; the args types are the structs below,
//!   deserialized with `deny_unknown_fields`.
//! - Profile: `ctx.profile` (`clipsync_core::Profile`, `ctx.profile.dir` = DIR);
//!   `ctx.require_profile()?` gives `not_initialized`.
//! - Agent: `ctx.ask_module(ModuleKind::Input, ModuleCommand::InputLock { locked })`,
//!   `InputSwitch { to }`, `InputLoopback { to, seconds }`; through `control::request`, in_app or
//!   external alike. No agent = code `agent_not_running`. `ModuleReply::Error(text)` comes back
//!   as `Ok`, for you to map to `offline`, `not_linked`, `denied`.
//! - `peer_id` arrives as 64 hex: `crate::api::peer_id(&args.peer_id, "peer_id")?` parses it
//!   (`bad_request` with `field`).
//! - Events: `screen_lock_changed` comes from the poll in `events.rs` (Rivet) via `InputState`;
//!   nothing to emit here.
//! - Tests: `tests/layout.rs` (Vector), through `Ui::call` only.

use serde::Deserialize;
use serde_json::Value;

use crate::{Ctx, UiError};

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct LockArgs {
    pub locked: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SwitchArgs {
    pub peer_id: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TestArgs {
    pub peer_id: String,
    /// 5..60.
    pub seconds: u32,
}

pub(crate) async fn lock_to_screen(_ctx: &Ctx, _args: LockArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn switch_to_device(_ctx: &Ctx, _args: SwitchArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}

pub(crate) async fn input_test(_ctx: &Ctx, _args: TestArgs) -> Result<Value, UiError> {
    Err(UiError::unavailable())
}
