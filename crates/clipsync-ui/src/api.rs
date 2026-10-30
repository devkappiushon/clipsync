//! The request surface (contract §12.3 to §12.5): `UiRequest`, `UiError` and the router.
//! Owned by Rivet. Vector's commands take their args types from `layout` and `input`.

use std::fmt;
use std::sync::Arc;

use clipsync_core::CoreError;
use clipsync_proto::{EventId, PeerId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::devices::Perms;
use crate::prefs::{Appearance, Notifications};
use crate::{
    Ctx, agent, clean, clipboard, devices, diag, input, layout, msg, onboarding, pairing, prefs,
    status,
};

/// Stable error codes: the front switches on these, never on the message.
pub mod code {
    pub const BAD_REQUEST: &str = "bad_request";
    pub const UNKNOWN_COMMAND: &str = "unknown_command";
    pub const NOT_INITIALIZED: &str = "not_initialized";
    pub const AGENT_NOT_RUNNING: &str = "agent_not_running";
    pub const AGENT_EXTERNAL: &str = "agent_external";
    pub const UNKNOWN_PEER: &str = "unknown_peer";
    pub const NEEDS_CONFIRMATION: &str = "needs_confirmation";
    pub const UNAVAILABLE: &str = "unavailable";
    pub const IO: &str = "io";
    pub const CONFIG: &str = "config";
    pub const INTERNAL: &str = "internal";
    // agent_start
    pub const NO_USABLE_MODULE: &str = "no_usable_module";
    pub const MODULE_BUILD: &str = "module_build";
    // onboarding
    pub const INVALID_NAME: &str = "invalid_name";
    pub const NO_MODULE: &str = "no_module";
    // devices
    pub const MODULE_OFF: &str = "module_off";
    pub const NAME_MISMATCH: &str = "name_mismatch";
    // pairing
    pub const PAIR_BUSY: &str = "pair_busy";
    pub const BIND: &str = "bind";
    pub const BAD_ADDR: &str = "bad_addr";
    pub const NO_ATTEMPT: &str = "no_attempt";
    pub const WRONG_STEP: &str = "wrong_step";
    // clipboard
    pub const NO_ITEM: &str = "no_item";
    pub const WRONG_STATE: &str = "wrong_state";
    pub const NO_TARGET: &str = "no_target";
    pub const EMPTY_CLIPBOARD: &str = "empty_clipboard";
}

/// Rejection of a command; serialized as `{code, message, field, details}`. `message` is
/// Portuguese, ready for the screen; `field` names the argument at fault, when there is one.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UiError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub details: Option<Value>,
}

impl UiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            field: None,
            details: None,
        }
    }

    pub fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_string());
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn bad_request(reason: &str) -> Self {
        Self::new(code::BAD_REQUEST, msg::BAD_REQUEST)
            .with_details(serde_json::json!({ "reason": reason }))
    }

    pub fn not_initialized() -> Self {
        Self::new(code::NOT_INITIALIZED, msg::NOT_INITIALIZED)
    }

    pub fn agent_not_running() -> Self {
        Self::new(code::AGENT_NOT_RUNNING, msg::AGENT_NOT_RUNNING)
    }

    pub fn agent_external() -> Self {
        Self::new(code::AGENT_EXTERNAL, msg::AGENT_EXTERNAL)
    }

    pub fn unavailable() -> Self {
        Self::new(code::UNAVAILABLE, msg::UNAVAILABLE)
    }

    pub fn internal(detail: &str) -> Self {
        Self::new(code::INTERNAL, msg::internal(detail))
    }
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for UiError {}

impl From<CoreError> for UiError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::AgentNotRunning => Self::agent_not_running(),
            CoreError::AgentAlreadyRunning => Self::agent_external(),
            CoreError::Config(d) if d.starts_with("perfil não inicializado") => {
                Self::not_initialized()
            }
            CoreError::Profile(d) if d == "profile_busy" => {
                Self::new("profile_busy", "reconfiguração do perfil em andamento")
            }
            CoreError::Profile(d) if d == "stale_config" => Self::new(
                "stale_config",
                "a configuração mudou; recarregue antes de aplicar",
            ),
            CoreError::Config(d) | CoreError::Identity(d) | CoreError::Profile(d) => {
                Self::new(code::CONFIG, msg::config(&clean(&d)))
            }
            // Transport is mostly a bind that failed (port taken).
            CoreError::Io(e) => Self::new(code::IO, msg::io(&clean(&e.to_string()))),
            CoreError::Transport(d) => Self::new(code::IO, msg::io(&clean(&d))),
            CoreError::Pairing(d) => Self::internal(&clean(&d)),
            CoreError::Closed(c) => Self::internal(&format!("{c:?}")),
        }
    }
}

/// Every `cmd` the router knows, in the order of §12.5. A name outside this list is
/// `unknown_command`; a known name with bad args is `bad_request`.
pub const COMMANDS: [&str; 40] = [
    "modules_set",
    "app_state",
    "agent_start",
    "agent_stop",
    "pause_set",
    "onboarding_start",
    "onboarding_check_name",
    "onboarding_create",
    "onboarding_self_test",
    "devices_list",
    "device_set_perms",
    "device_revoke",
    "device_pause",
    "pair_host_start",
    "pair_join_start",
    "pair_code_matches",
    "pair_finish",
    "pair_cancel",
    "pair_state",
    "layout_get",
    "layout_set_link",
    "layout_remove_link",
    "layout_set_device",
    "layout_set_options",
    "layout_copy_to_peers",
    "layout_offer_answer",
    "lock_to_screen",
    "switch_to_device",
    "input_test",
    "clipboard_get",
    "clipboard_preview",
    "clipboard_set_mode",
    "clipboard_copy_item",
    "clipboard_discard_item",
    "clipboard_send_now",
    "diagnostics_get",
    "support_bundle_preview",
    "support_bundle_export",
    "prefs_get",
    "prefs_set",
];

/// `{cmd, args}`; a command without arguments is `{cmd}` alone.
#[derive(Deserialize, Debug)]
#[serde(
    tag = "cmd",
    content = "args",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UiRequest {
    // common
    ModulesSet(crate::modules::Args),
    AppState,
    AgentStart,
    AgentStop,
    PauseSet(PauseSetArgs),
    // 1 · onboarding
    OnboardingStart,
    OnboardingCheckName(NameArgs),
    OnboardingCreate(OnboardingCreateArgs),
    OnboardingSelfTest,
    // 2 · devices
    DevicesList,
    DeviceSetPerms(DeviceSetPermsArgs),
    DeviceRevoke(DeviceRevokeArgs),
    DevicePause(DevicePauseArgs),
    // 3 · pairing
    PairHostStart(PairHostStartArgs),
    PairJoinStart(PairJoinStartArgs),
    PairCodeMatches(AttemptArgs),
    PairFinish(PairFinishArgs),
    PairCancel(AttemptArgs),
    PairState,
    // 4 · layout (Vector)
    LayoutGet,
    LayoutSetLink(layout::SetLinkArgs),
    LayoutRemoveLink(layout::RemoveLinkArgs),
    LayoutSetDevice(layout::SetDeviceArgs),
    LayoutSetOptions(layout::SetOptionsArgs),
    LayoutCopyToPeers(layout::CopyToPeersArgs),
    LayoutOfferAnswer(layout::OfferAnswerArgs),
    LockToScreen(input::LockArgs),
    SwitchToDevice(input::SwitchArgs),
    InputTest(input::TestArgs),
    // 5 · clipboard
    ClipboardGet,
    ClipboardPreview(ItemArgs),
    ClipboardSetMode(ClipboardModeArgs),
    ClipboardCopyItem(ItemArgs),
    ClipboardDiscardItem(ItemArgs),
    ClipboardSendNow(SendNowArgs),
    // 6 · diagnostics
    DiagnosticsGet,
    SupportBundlePreview,
    SupportBundleExport,
    // 7 · preferences
    PrefsGet,
    PrefsSet(PrefsSetArgs),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PauseSetArgs {
    pub paused: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameArgs {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct ModulesArg {
    pub input: bool,
    pub clipboard: bool,
}

fn default_port() -> u16 {
    47800
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OnboardingCreateArgs {
    pub name: String,
    pub modules: ModulesArg,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub recreate: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeviceSetPermsArgs {
    pub peer_id: String,
    pub perms: Perms,
    #[serde(default)]
    pub acknowledge_full_trust: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeviceRevokeArgs {
    pub peer_id: String,
    pub typed_name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DevicePauseArgs {
    pub peer_id: String,
    pub paused: bool,
}

fn default_window() -> u64 {
    clipsync_proto::PAIR_WINDOW_SECS
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PairHostStartArgs {
    #[serde(default)]
    pub port: u16,
    #[serde(default = "default_window")]
    pub window_secs: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PairJoinStartArgs {
    pub addr: String,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct AttemptArgs {
    pub attempt_id: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PairFinishArgs {
    pub attempt_id: u64,
    pub grants: Perms,
    #[serde(default)]
    pub acknowledge_full_trust: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ItemArgs {
    /// 32 hex.
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipMode {
    Auto,
    Manual,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ClipboardModeArgs {
    pub mode: ClipMode,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct SendNowArgs {
    #[serde(default)]
    pub peer_id: Option<String>,
}

/// The three are optional; any other field is `bad_request`.
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct PrefsSetArgs {
    #[serde(default)]
    pub autostart: Option<bool>,
    #[serde(default)]
    pub notifications: Option<Notifications>,
    #[serde(default)]
    pub appearance: Option<Appearance>,
}

/// `peer_id` must be 64 hex (§12.2 rule 4: commands take ids, never names).
pub fn peer_id(s: &str, field: &str) -> Result<PeerId, UiError> {
    PeerId::from_hex(s).map_err(|_| UiError::bad_request("peer_id: 64 hex").with_field(field))
}

/// Item id of the clipboard box: 32 hex.
pub fn event_id(s: &str, field: &str) -> Result<EventId, UiError> {
    // Hex digits only: `from_str_radix` alone would also take a sign ("+f" read as "0f", S6-12).
    let digit = |b: u8| (b as char).to_digit(16);
    let mut out = [0u8; 16];
    let ok = s.len() == 32
        && s.as_bytes().chunks(2).zip(out.iter_mut()).all(|(pair, o)| {
            match (digit(pair[0]), digit(pair[1])) {
                (Some(hi), Some(lo)) => {
                    *o = u8::try_from(hi * 16 + lo).unwrap_or_default();
                    true
                }
                _ => false,
            }
        });
    if ok {
        Ok(EventId(out))
    } else {
        Err(UiError::bad_request("id: 32 hex").with_field(field))
    }
}

/// The argument serde complained about, when its message names one.
fn field_of(reason: &str) -> Option<String> {
    let start = reason.find("field `")? + "field `".len();
    let len = reason[start..].find('`')?;
    Some(reason[start..start + len].to_string())
}

pub fn parse(req: Value) -> Result<UiRequest, UiError> {
    let Some(cmd) = req.get("cmd").and_then(Value::as_str) else {
        return Err(UiError::bad_request("cmd ausente ou não é texto").with_field("cmd"));
    };
    if !COMMANDS.contains(&cmd) {
        return Err(UiError::new(
            code::UNKNOWN_COMMAND,
            msg::unknown_command(&clean(cmd)),
        ));
    }
    serde_json::from_value(req).map_err(|e| {
        let reason = e.to_string();
        let err = UiError::bad_request(&reason);
        match field_of(&reason) {
            Some(f) => err.with_field(&f),
            None => err,
        }
    })
}

fn json<T: Serialize>(v: T) -> Result<Value, UiError> {
    serde_json::to_value(v).map_err(|e| UiError::internal(&e.to_string()))
}

pub(crate) async fn route(ctx: &Arc<Ctx>, req: UiRequest) -> Result<Value, UiError> {
    use UiRequest as R;
    let mutation = matches!(
        &req,
        R::ModulesSet(_)
            | R::AgentStart
            | R::AgentStop
            | R::OnboardingCreate(_)
            | R::ClipboardSetMode(_)
            | R::PairHostStart(_)
            | R::PairJoinStart(_)
            | R::PairCodeMatches(_)
            | R::PairFinish(_)
            | R::PairCancel(_)
            | R::LockToScreen(_)
    );
    if !mutation {
        return route_inner(ctx, req).await;
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let owned = ctx.clone();
    let task = tokio::spawn(async move {
        let result = match owned.mutation.try_lock() {
            Ok(_guard) => route_inner(&owned, req).await,
            Err(_) => {
                let error = UiError::new(
                    "profile_busy",
                    "outra alteração do perfil está em andamento",
                );
                Err(error.with_details(serde_json::json!({"effects":crate::modules::Effects::default(),"observed":crate::modules::Observed::default()})))
            }
        };
        let _ = sender.send(result);
        agent::finish_pending(&owned).await;
    });
    {
        let mut jobs = crate::lock(&ctx.jobs);
        jobs.retain(|job| !job.is_finished());
        jobs.push(task);
    }
    receiver
        .await
        .map_err(|_| UiError::internal("operação interrompida"))?
}

async fn route_inner(ctx: &Arc<Ctx>, req: UiRequest) -> Result<Value, UiError> {
    use UiRequest as R;
    match req {
        R::ModulesSet(args) => json(crate::modules::apply(ctx, args).await?),
        R::AppState => json(status::app_state(ctx).await?),
        R::AgentStart => json(agent::start(ctx).await?),
        R::AgentStop => json(agent::stop(ctx).await?),
        R::PauseSet(a) => json(status::pause_set(ctx, a.paused).await?),
        R::OnboardingStart => json(onboarding::start(ctx)?),
        R::OnboardingCheckName(a) => json(onboarding::check_name(&a.name)?),
        R::OnboardingCreate(a) => json(onboarding::create(ctx, a).await?),
        R::OnboardingSelfTest => json(onboarding::self_test(ctx).await?),
        R::DevicesList => json(devices::list(ctx).await?),
        R::DeviceSetPerms(a) => json(devices::set_perms(ctx, a).await?),
        R::DeviceRevoke(a) => json(devices::revoke(ctx, a).await?),
        R::DevicePause(a) => json(devices::pause(&a)?),
        R::PairHostStart(a) => json(pairing::host_start(ctx, a).await?),
        R::PairJoinStart(a) => json(pairing::join_start(ctx, a).await?),
        R::PairCodeMatches(a) => json(pairing::code_matches(ctx, a).await?),
        R::PairFinish(a) => json(pairing::finish(ctx, a)?),
        R::PairCancel(a) => json(pairing::cancel(ctx, a)?),
        R::PairState => json(pairing::state(ctx)),
        R::LayoutGet => json(layout::get(ctx).await?),
        R::LayoutSetLink(a) => json(layout::set_link(ctx, a).await?),
        R::LayoutRemoveLink(a) => json(layout::remove_link(ctx, a).await?),
        R::LayoutSetDevice(a) => json(layout::set_device(ctx, a).await?),
        R::LayoutSetOptions(a) => json(layout::set_options(ctx, a).await?),
        R::LayoutCopyToPeers(a) => json(layout::copy_to_peers(ctx, a).await?),
        R::LayoutOfferAnswer(a) => json(layout::offer_answer(ctx, a).await?),
        R::LockToScreen(a) => json(input::lock_to_screen(ctx, a).await?),
        R::SwitchToDevice(a) => json(input::switch_to_device(ctx, a).await?),
        R::InputTest(a) => json(input::input_test(ctx, a).await?),
        R::ClipboardGet => json(clipboard::get(ctx).await?),
        R::ClipboardPreview(a) => json(clipboard::preview(ctx, a).await?),
        R::ClipboardSetMode(a) => json(clipboard::set_mode(ctx, a).await?),
        R::ClipboardCopyItem(a) => json(clipboard::copy_item(ctx, a).await?),
        R::ClipboardDiscardItem(a) => json(clipboard::discard_item(ctx, a).await?),
        R::ClipboardSendNow(a) => json(clipboard::send_now(ctx, a).await?),
        R::DiagnosticsGet => json(diag::get(ctx).await?),
        R::SupportBundlePreview => json(diag::bundle_preview(ctx).await?),
        R::SupportBundleExport => json(diag::bundle_export(ctx).await?),
        R::PrefsGet => json(prefs::get(ctx)),
        R::PrefsSet(a) => json(prefs::set(ctx, a)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_listed_command_is_a_variant_and_no_variant_is_missing() {
        for cmd in COMMANDS {
            let err = parse(json!({ "cmd": cmd })).err();
            // Variants with args complain about `args`, never about an unknown variant.
            if let Some(e) = err {
                let reason = e.details.unwrap()["reason"].to_string();
                assert!(!reason.contains("unknown variant"), "{cmd}: {reason}");
            }
        }
        let unknown = parse(json!({ "cmd": "rm_rf" })).unwrap_err();
        assert_eq!(unknown.code, code::UNKNOWN_COMMAND);
    }

    #[test]
    fn bad_shapes_are_bad_request_with_the_field() {
        let e = parse(json!({ "cmd": "pause_set", "args": { "paused": "sim" } })).unwrap_err();
        assert_eq!(e.code, code::BAD_REQUEST);
        let e =
            parse(json!({ "cmd": "pause_set", "args": { "paused": true, "x": 1 } })).unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            (code::BAD_REQUEST, Some("x"))
        );
        let e = parse(json!({ "cmd": "pause_set" })).unwrap_err();
        assert_eq!(e.code, code::BAD_REQUEST);
        let e = parse(json!({ "cmd": "app_state", "extra": 1 })).unwrap_err();
        assert_eq!(e.code, code::BAD_REQUEST);
        let e = parse(json!({ "args": {} })).unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.field.as_deref()),
            (code::BAD_REQUEST, Some("cmd"))
        );
        assert!(parse(json!({ "cmd": "app_state" })).is_ok());
        assert!(parse(json!({ "cmd": "pair_host_start", "args": {} })).is_ok());
    }

    #[test]
    fn ids_are_parsed_strictly() {
        let hex64 = "ab".repeat(32);
        assert!(peer_id(&hex64, "peer_id").is_ok());
        assert_eq!(
            peer_id("ab12-cd34", "peer_id")
                .unwrap_err()
                .field
                .as_deref(),
            Some("peer_id")
        );
        let id = event_id(&"0f".repeat(16), "id").unwrap();
        assert_eq!(id.0, [0x0f; 16]);
        assert!(event_id(&"zz".repeat(16), "id").is_err());
        assert!(event_id("0f0f", "id").is_err());
        // S6-12: a sign is not a hex digit.
        assert!(event_id(&"+f".repeat(16), "id").is_err());
        assert!(event_id(&"-f".repeat(16), "id").is_err());
        assert_eq!(event_id(&"AB".repeat(16), "id").unwrap().0, [0xab; 16]);
    }
}
