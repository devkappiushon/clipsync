//! 4 · Comandos de input da tela de layout (§12.5): `lock_to_screen`, `switch_to_device` and
//! `input_test`. Owned by VECTOR (§12.1).
//!
//! All three go to the agent as module commands (`InputLock`, `InputSwitch`, `InputLoopback`),
//! so an in_app and an external agent behave the same. The module answers refusals as text;
//! they are mapped to the codes of §12.5 by the consts of `clipsync_input::msg`. The events
//! `screen_lock_changed` come from the poll in `events.rs`, nothing is emitted here.

use clipsync_input::msg as m;
use clipsync_proto::{LoopbackStats, ModuleCommand, ModuleKind, ModuleReply, PeerId, PeerState};
use serde::{Deserialize, Serialize};

use crate::api::{self, code as base};
use crate::{Ctx, UiError, clean, layout, msg, status};

/// Own codes of these commands (§12.5).
pub mod code {
    pub const OFFLINE: &str = "offline";
    pub const NOT_LINKED: &str = "not_linked";
    pub const DENIED: &str = "denied";
}

const TEST_SECONDS: std::ops::RangeInclusive<u32> = 5..=60;

mod txt {
    pub const BAD_SECONDS: &str = "a duração do teste vai de 5 a 60 segundos";
}

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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockReply {
    pub screen_locked: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SwitchReply {
    pub active_peer_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub received: u64,
    pub lost: u64,
    pub out_of_order: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub max_us: u64,
}

impl From<LoopbackStats> for Stats {
    fn from(s: LoopbackStats) -> Self {
        Self {
            received: s.received,
            lost: s.lost,
            out_of_order: s.out_of_order,
            p50_us: s.p50_us,
            p95_us: s.p95_us,
            max_us: s.max_us,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TestReply {
    pub stats: Stats,
}

/// A device the profile paired with (`peers.toml`); commands take ids, never names.
fn paired(ctx: &Ctx, s: &str) -> Result<PeerId, UiError> {
    let id = api::peer_id(s, "peer_id")?;
    if ctx.profile.load_peers()?.get(&id).is_some() {
        Ok(id)
    } else {
        Err(UiError::new(base::UNKNOWN_PEER, msg::UNKNOWN_PEER).with_field("peer_id"))
    }
}

/// The text the module refused with, as a code. What nobody expects stays `internal`, with the
/// module's own Portuguese.
fn refusal(text: &str) -> UiError {
    let to = |c: &str| UiError::new(c, clean(text));
    match text {
        t if t == m::NOT_LINKED => to(code::NOT_LINKED),
        t if t == m::PEER_OFFLINE => to(code::OFFLINE),
        t if t == m::LOCKED
            || t == m::NOT_AUTHORIZED
            || t == m::REMOTE_INPUT_OFF
            || t == m::PEER_TOO_OLD
            || t == m::PEER_UNKNOWN
            || t.starts_with(m::PORTAL_DENIED) =>
        {
            to(code::DENIED)
        }
        t if t == clipsync_proto::msg::MODULE_DISABLED => {
            UiError::new(layout::code::INPUT_OFF, layout::txt::INPUT_OFF)
        }
        _ => to(base::INTERNAL),
    }
}

/// A paired device the agent does not hold a connection with is `offline`. The module itself
/// cannot tell that apart from a stranger (it only knows peers that connected once).
async fn require_online(ctx: &Ctx, to: PeerId) -> Result<(), UiError> {
    let Some(status) = status::agent_status(ctx).await else {
        // No agent: the command itself answers `agent_not_running`.
        return Ok(());
    };
    let online = status
        .peers
        .iter()
        .any(|p| p.id == to && p.state == PeerState::Connected);
    if online {
        Ok(())
    } else {
        Err(UiError::new(code::OFFLINE, m::PEER_OFFLINE))
    }
}

async fn ask(ctx: &Ctx, cmd: ModuleCommand) -> Result<ModuleReply, UiError> {
    match ctx.ask_module(ModuleKind::Input, cmd).await? {
        ModuleReply::Error(t) => Err(refusal(&t)),
        r => Ok(r),
    }
}

/// The lock lives in the agent (it does not persist): while on, the edges stop switching.
pub(crate) async fn lock_to_screen(ctx: &Ctx, args: LockArgs) -> Result<LockReply, UiError> {
    layout::require_input(ctx)?;
    match ask(
        ctx,
        ModuleCommand::InputLock {
            locked: args.locked,
        },
    )
    .await?
    {
        ModuleReply::Done => Ok(LockReply {
            screen_locked: args.locked,
        }),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

/// Hands the keyboard and mouse to a linked, connected device that allows it. The answer comes
/// after the peer had its chance to refuse.
pub(crate) async fn switch_to_device(ctx: &Ctx, args: SwitchArgs) -> Result<SwitchReply, UiError> {
    layout::require_input(ctx)?;
    let to = paired(ctx, &args.peer_id)?;
    require_online(ctx, to).await?;
    match ask(ctx, ModuleCommand::InputSwitch { to }).await? {
        ModuleReply::Done => Ok(SwitchReply {
            active_peer_id: to.to_hex(),
        }),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

/// The loopback of contract §8: the peer echoes a burst of events back and the agent measures
/// it. It may open the portal dialog on the first use.
pub(crate) async fn input_test(ctx: &Ctx, args: TestArgs) -> Result<TestReply, UiError> {
    layout::require_input(ctx)?;
    if !TEST_SECONDS.contains(&args.seconds) {
        return Err(UiError::new(base::BAD_REQUEST, txt::BAD_SECONDS).with_field("seconds"));
    }
    let to = paired(ctx, &args.peer_id)?;
    require_online(ctx, to).await?;
    match ask(
        ctx,
        ModuleCommand::InputLoopback {
            to,
            seconds: args.seconds,
        },
    )
    .await?
    {
        ModuleReply::InputLoopback(s) => Ok(TestReply { stats: s.into() }),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modules_refusals_become_the_codes_of_the_contract() {
        for (text, want) in [
            (m::NOT_LINKED, code::NOT_LINKED),
            (m::PEER_OFFLINE, code::OFFLINE),
            (m::LOCKED, code::DENIED),
            (m::NOT_AUTHORIZED, code::DENIED),
            (m::REMOTE_INPUT_OFF, code::DENIED),
            (m::PEER_TOO_OLD, code::DENIED),
            (m::PEER_UNKNOWN, code::DENIED),
            (m::LOOPBACK_BUSY, base::INTERNAL),
            (m::NO_ACTIVATION, base::INTERNAL),
        ] {
            assert_eq!(refusal(text).code, want, "{text}");
        }
        assert_eq!(refusal(&m::denied("fechou")).code, code::DENIED);
    }

    #[test]
    fn a_refusal_keeps_the_modules_portuguese_and_cleans_it() {
        let e = refusal(m::NOT_LINKED);
        assert_eq!(e.message, m::NOT_LINKED);
        assert_eq!(refusal("x\u{1b}[2Jy").message, "x?[2Jy");
    }

    #[test]
    fn stats_leave_without_the_session_number() {
        let s = Stats::from(LoopbackStats {
            session: 9,
            received: 600,
            lost: 1,
            out_of_order: 2,
            p50_us: 410,
            p95_us: 1180,
            max_us: 3920,
        });
        let v = serde_json::to_value(TestReply { stats: s }).unwrap();
        assert_eq!(v["stats"]["received"], 600);
        assert!(v["stats"].get("session").is_none());
    }
}
