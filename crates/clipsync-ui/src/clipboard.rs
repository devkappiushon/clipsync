//! 5 · Clipboard (§12.5): the box of received items, preview, mode and sending. Owned by Rivet.
//!
//! Everything goes through the clipboard module of the agent (`ClipInbox`, `ClipApply`,
//! `ClipDiscard`, `ClipSendCurrent`); its error texts are matched by the public consts of
//! `clipsync_clipboard::msg`. The list never carries a preview: only `clipboard_preview` does,
//! cleaned and cut to 200 characters.

use clipsync_clipboard::{AdapterChoice, ClipboardConfig};
use clipsync_proto::{
    CLIP_TTL_MS, DeliveryState, InboxItem as ProtoItem, MAX_INBOX_ITEMS, ModuleCommand, ModuleKind,
    ModuleReply,
};
use serde::{Deserialize, Serialize};

use crate::api::{self, ClipMode, ClipboardModeArgs, ItemArgs, SendNowArgs, code};
use crate::status;
use crate::{Ctx, UiError, clean, msg};

/// `clipboard_preview` shows at most this many characters.
const PREVIEW_CHARS: usize = 200;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Received,
    Applied,
    InInbox,
    Rejected,
    Expired,
    Unreachable,
}

impl From<DeliveryState> for ItemState {
    fn from(s: DeliveryState) -> Self {
        match s {
            DeliveryState::Received => ItemState::Received,
            DeliveryState::Applied => ItemState::Applied,
            DeliveryState::InInbox => ItemState::InInbox,
            DeliveryState::Rejected(_) => ItemState::Rejected,
            DeliveryState::Expired => ItemState::Expired,
            DeliveryState::Unreachable => ItemState::Unreachable,
        }
    }
}

/// Why a state is what it is, in Portuguese; only refusals have one.
fn reason(s: DeliveryState) -> Option<String> {
    match s {
        DeliveryState::Rejected(r) => Some(r.pt().to_string()),
        _ => None,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Validated,
    Experimental,
    Unavailable,
    Test,
}

/// One item of the box, without its text.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InboxItem {
    /// 32 hex: what commands take.
    pub id: String,
    pub short: String,
    pub from: String,
    pub received_unix_ms: u64,
    pub mime: String,
    pub bytes: usize,
    pub state: ItemState,
    pub reason: Option<String>,
}

impl InboxItem {
    pub(crate) fn new(i: &ProtoItem) -> Self {
        Self {
            id: hex_of(&i.id.0),
            short: i.id.short(),
            from: clean(&i.from),
            received_unix_ms: i.received_unix_ms,
            mime: clean(&i.mime),
            bytes: i.bytes,
            state: i.state.into(),
            reason: reason(i.state),
        }
    }
}

pub(crate) fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub max_items: usize,
    pub ttl_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClipboardState {
    pub enabled: bool,
    pub adapter: Option<String>,
    pub classification: Classification,
    pub mode: ClipMode,
    pub last_event: Option<String>,
    pub last_block: Option<String>,
    pub inbox: Vec<InboxItem>,
    pub retention: Retention,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub preview: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModeReply {
    pub mode: ClipMode,
    /// The agent runs in a terminal (external): it reads the mode when restarted there.
    pub restart_needed: bool,
    #[serde(default)]
    pub effects: Option<crate::modules::Effects>,
    #[serde(default)]
    pub observed: Option<crate::modules::Observed>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Applied {
    pub state: ItemState,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Removed {
    pub removed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub peer_id: Option<String>,
    pub name: String,
    pub state: ItemState,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Sent {
    pub outcomes: Vec<Outcome>,
}

/// The adapter the agent reports; with no agent, what this platform would use.
fn classify(enabled: bool, adapter: Option<&str>, choice: Option<AdapterChoice>) -> Classification {
    if !enabled {
        return Classification::Unavailable;
    }
    match (adapter, choice) {
        (Some("fake"), _) | (None, Some(AdapterChoice::Fake)) => Classification::Test,
        (Some("x11-xfixes"), _) => Classification::Validated,
        (Some(_), _) => Classification::Experimental,
        (None, _) if cfg!(target_os = "linux") => Classification::Validated,
        (None, _) if cfg!(windows) => Classification::Experimental,
        (None, _) => Classification::Unavailable,
    }
}

fn module_off() -> UiError {
    UiError::new(code::MODULE_OFF, msg::CLIPBOARD_OFF)
}

/// The module's own refusals, by the consts of `clipsync_clipboard::msg`.
fn module_error(text: &str) -> UiError {
    use clipsync_clipboard::msg as m;
    match text {
        t if t == m::NO_ITEM => UiError::new(code::NO_ITEM, m::NO_ITEM).with_field("id"),
        t if t == m::WRONG_STATE => UiError::new(code::WRONG_STATE, m::WRONG_STATE),
        t if t == m::EMPTY_CLIPBOARD || t == m::EMPTY_TEXT => {
            UiError::new(code::EMPTY_CLIPBOARD, m::EMPTY_CLIPBOARD)
        }
        t if t == m::NO_TARGET => UiError::new(code::NO_TARGET, m::NO_TARGET).with_field("peer_id"),
        t if t == m::TOO_LARGE => UiError::new(code::IO, m::TOO_LARGE),
        t if t == clipsync_proto::msg::MODULE_DISABLED => module_off(),
        t => UiError::internal(&clean(t)),
    }
}

async fn ask(ctx: &Ctx, cmd: ModuleCommand) -> Result<ModuleReply, UiError> {
    match ctx.ask_module(ModuleKind::Clipboard, cmd).await? {
        ModuleReply::Error(t) => Err(module_error(&t)),
        r => Ok(r),
    }
}

/// The box as the module holds it, previews included (never leave this crate whole).
pub(crate) async fn inbox(ctx: &Ctx) -> Result<Vec<ProtoItem>, UiError> {
    match ask(ctx, ModuleCommand::ClipInbox).await? {
        ModuleReply::ClipInbox(items) => Ok(items),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

fn require_clipboard(ctx: &Ctx) -> Result<(), UiError> {
    ctx.require_profile()?;
    if ctx.profile.load_config()?.modules.clipboard {
        Ok(())
    } else {
        Err(module_off())
    }
}

/// Works with the agent stopped (the box is then empty) and with the module off.
pub(crate) async fn get(ctx: &Ctx) -> Result<ClipboardState, UiError> {
    ctx.require_profile()?;
    let cfg = ctx.profile.load_config()?;
    let enabled = cfg.modules.clipboard;
    let parsed = ClipboardConfig::from_table(cfg.clipboard.as_ref()).ok();
    let live = match status::agent_status(ctx).await {
        Some(s) => s
            .modules
            .into_iter()
            .find(|m| m.kind == ModuleKind::Clipboard && m.enabled),
        None => None,
    };
    let items = match &live {
        Some(_) => inbox(ctx).await?,
        None => Vec::new(),
    };
    let adapter = live.as_ref().and_then(|m| m.adapter.as_deref()).map(clean);
    Ok(ClipboardState {
        enabled,
        classification: classify(
            enabled,
            adapter.as_deref(),
            parsed.as_ref().map(|c| c.adapter),
        ),
        adapter,
        mode: match parsed.map(|c| c.mode) {
            Some(clipsync_clipboard::ClipMode::Manual) => ClipMode::Manual,
            _ => ClipMode::Auto,
        },
        last_event: live
            .as_ref()
            .and_then(|m| m.last_event.as_deref())
            .map(clean),
        last_block: live
            .as_ref()
            .and_then(|m| m.last_block.as_deref())
            .map(clean),
        inbox: items.iter().map(InboxItem::new).collect(),
        retention: Retention {
            max_items: MAX_INBOX_ITEMS,
            ttl_ms: u64::from(CLIP_TTL_MS),
        },
    })
}

pub(crate) async fn preview(ctx: &Ctx, args: ItemArgs) -> Result<Preview, UiError> {
    let id = api::event_id(&args.id, "id")?;
    require_clipboard(ctx)?;
    let items = inbox(ctx).await?;
    let item = items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| module_error(clipsync_clipboard::msg::NO_ITEM))?;
    Ok(Preview {
        preview: clean(&item.preview).chars().take(PREVIEW_CHARS).collect(),
    })
}

/// Writes `[clipboard] mode`. The in_app agent restarts to read it; an external one is told
/// to restart (`restart_needed`); a stopped one reads it when it starts.
pub(crate) async fn set_mode(ctx: &Ctx, args: ClipboardModeArgs) -> Result<ModeReply, UiError> {
    crate::modules::apply_mode(ctx, args.mode).await
}

/// Only an item `in_inbox` or refused as old (Stale) that still holds its text. Works paused
/// (explicit action) and makes no offer.
pub(crate) async fn copy_item(ctx: &Ctx, args: ItemArgs) -> Result<Applied, UiError> {
    let id = api::event_id(&args.id, "id")?;
    require_clipboard(ctx)?;
    match ask(ctx, ModuleCommand::ClipApply { id }).await? {
        ModuleReply::ClipApplied(DeliveryState::Applied) => Ok(Applied {
            state: ItemState::Applied,
        }),
        ModuleReply::ClipApplied(DeliveryState::Rejected(r)) => {
            Err(UiError::new(code::IO, msg::apply_failed(r.pt())))
        }
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

pub(crate) async fn discard_item(ctx: &Ctx, args: ItemArgs) -> Result<Removed, UiError> {
    let id = api::event_id(&args.id, "id")?;
    require_clipboard(ctx)?;
    match ask(ctx, ModuleCommand::ClipDiscard { id }).await? {
        ModuleReply::Done => Ok(Removed { removed: true }),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

/// What the clipboard holds now, to one peer or to every peer with `clipboard_out`.
pub(crate) async fn send_now(ctx: &Ctx, args: SendNowArgs) -> Result<Sent, UiError> {
    let to = args
        .peer_id
        .as_deref()
        .map(|p| api::peer_id(p, "peer_id"))
        .transpose()?;
    require_clipboard(ctx)?;
    let ModuleReply::ClipSent(outcomes) = ask(ctx, ModuleCommand::ClipSendCurrent { to }).await?
    else {
        return Err(UiError::internal(msg::UNEXPECTED_REPLY));
    };
    // The module names peers; names are unique in peers.toml, which gives the id back.
    let peers = ctx.profile.load_peers().unwrap_or_default();
    Ok(Sent {
        outcomes: outcomes
            .iter()
            .map(|o| Outcome {
                peer_id: peers
                    .peer
                    .iter()
                    .find(|p| p.name == o.peer)
                    .map(|p| p.id.to_hex()),
                name: clean(&o.peer),
                state: o.state.into(),
                reason: reason(o.state),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_follows_the_adapter() {
        assert_eq!(
            classify(false, Some("x11-xfixes"), None),
            Classification::Unavailable
        );
        assert_eq!(classify(true, Some("fake"), None), Classification::Test);
        assert_eq!(
            classify(true, None, Some(AdapterChoice::Fake)),
            Classification::Test
        );
        assert_eq!(
            classify(true, Some("x11-xfixes"), None),
            Classification::Validated
        );
        assert_eq!(
            classify(true, Some("win-poll"), None),
            Classification::Experimental
        );
    }

    #[test]
    fn module_texts_map_to_their_codes() {
        use clipsync_clipboard::msg as m;
        assert_eq!(module_error(m::NO_ITEM).code, code::NO_ITEM);
        assert_eq!(module_error(m::WRONG_STATE).code, code::WRONG_STATE);
        assert_eq!(module_error(m::EMPTY_CLIPBOARD).code, code::EMPTY_CLIPBOARD);
        assert_eq!(module_error(m::NO_TARGET).code, code::NO_TARGET);
        assert_eq!(
            module_error(clipsync_proto::msg::MODULE_DISABLED).code,
            code::MODULE_OFF
        );
        assert_eq!(module_error("outra coisa").code, code::INTERNAL);
    }

    #[test]
    fn ids_are_lowercase_hex() {
        assert_eq!(hex_of(&[0x0f, 0xa0]), "0fa0");
    }
}
