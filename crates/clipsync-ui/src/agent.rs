//! The agent inside the app: `agent_start`, `agent_stop` and the mode the screens show.
//! Owned by Rivet. An agent already answering on the profile's socket (a `clipsync run` in a
//! terminal) makes the app a client: mode `external`, nothing started here (§12.2 rule 1).
//!
//! `plan` + `build` repeat the CLI's `clipsync run` with its 3 tests. Debt (§12.2 rule 9,
//! decision 12): unify later.

use std::time::Duration;

use clipsync_core::{Config, CoreError, Profile, agent};
use clipsync_proto::{ControlRequest, ControlResponse, Module, ModuleSet};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::api::code;
use crate::status::AgentMode;
use crate::{Ctx, UiError, clean, lock, msg};

/// Only `layout.rs` (Vector) still reads this: its Windows screen 4 comes with §13.9. Everything
/// else asks `clipsync_input::SUPPORTED`; this goes away with its last use (§13.11 item 3).
pub(crate) const INPUT_SUPPORTED: bool = cfg!(target_os = "linux");

/// Bind, modules and the control socket; the input portal may take a while on first use.
const START_TIMEOUT: Duration = Duration::from_secs(15);
const START_POLL: Duration = Duration::from_millis(25);
/// Modules get 3 s and peers a Bye inside `agent::run`; this bounds the whole stop.
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) enum Slot {
    Stopped,
    /// Last start failed or the agent died: the message goes to `app_state.agent_error`.
    Failed(String),
    InApp(Running),
}

pub(crate) struct Running {
    stop: oneshot::Sender<()>,
    task: JoinHandle<Result<(), CoreError>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentReply {
    pub agent: AgentMode,
}

/// What `run` builds: the configured modules minus what the platform cannot provide.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub clipboard: bool,
    pub input: bool,
    /// Configured but unavailable here: announced, not built.
    pub input_skipped: bool,
}

/// An agent without any module would serve nothing (contract §11 rule 8): that is an error.
pub(crate) fn plan(set: ModuleSet, input_supported: bool) -> Result<Plan, UiError> {
    let plan = Plan {
        clipboard: set.clipboard,
        input: set.input && input_supported,
        input_skipped: set.input && !input_supported,
    };
    if plan.clipboard || plan.input {
        Ok(plan)
    } else {
        Err(UiError::new(code::NO_USABLE_MODULE, msg::NO_USABLE_MODULE))
    }
}

fn module_err(e: clipsync_proto::ModuleError) -> UiError {
    UiError::new(
        code::MODULE_BUILD,
        msg::module_build(&clean(&e.to_string())),
    )
}

/// A disabled module is not built: no adapter loaded, no permission asked, not announced. A
/// fake clipboard keeps its handle in `ctx` for the tests.
fn build(ctx: &Ctx, profile: &Profile, cfg: &Config) -> Result<Vec<Box<dyn Module>>, UiError> {
    let plan = plan(cfg.modules, clipsync_input::SUPPORTED)?;
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    if plan.clipboard {
        let c = clipsync_clipboard::ClipboardConfig::from_table(cfg.clipboard.as_ref())
            .map_err(module_err)?;
        if c.adapter == clipsync_clipboard::AdapterChoice::Fake {
            let (adapter, handle) = clipsync_clipboard::FakeClipboard::new();
            *lock(&ctx.fake_clipboard) = Some(handle);
            modules.push(clipsync_clipboard::build_with_adapter(c, Box::new(adapter)));
        } else {
            modules.push(clipsync_clipboard::build(c, &profile.dir).map_err(module_err)?);
        }
    }
    if plan.input {
        let c = clipsync_input::InputConfig::from_table(cfg.input.as_ref()).map_err(module_err)?;
        modules.push(clipsync_input::build(c, &profile.dir).map_err(module_err)?);
    }
    Ok(modules)
}

async fn alive(ctx: &Ctx) -> bool {
    matches!(
        ctx.request(ControlRequest::Status).await,
        Ok(ControlResponse::Status(_))
    )
}

enum Launched {
    InApp(Running),
    /// Another agent took the profile between our check and the bind.
    External,
}

async fn launch(ctx: &Ctx) -> Result<Launched, UiError> {
    let cfg = ctx.profile.load_config()?;
    let modules = build(ctx, &ctx.profile, &cfg)?;
    let (stop, stop_rx) = oneshot::channel::<()>();
    let profile = ctx.profile.clone();
    let mut task = tokio::spawn(async move {
        agent::run(&profile, modules, async move {
            let _ = stop_rx.await;
        })
        .await
    });
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if task.is_finished() {
            return match (&mut task).await {
                Ok(Err(CoreError::AgentAlreadyRunning)) => Ok(Launched::External),
                Ok(Err(e)) => {
                    let e = UiError::from(e);
                    Err(UiError::new(&e.code, msg::agent_failed(&e.message)))
                }
                Ok(Ok(())) => Err(UiError::internal(msg::AGENT_STOPPED_ITSELF)),
                Err(e) => Err(UiError::internal(&e.to_string())),
            };
        }
        if alive(ctx).await {
            return Ok(Launched::InApp(Running { stop, task }));
        }
        if Instant::now() >= deadline {
            let _ = stop.send(());
            return Err(UiError::internal(msg::AGENT_START_TIMEOUT));
        }
        tokio::time::sleep(START_POLL).await;
    }
}

/// `AgentAlreadyRunning` is not an error: it becomes `external`. A failure is kept in the
/// slot, so `app_state` shows `failed` with the message.
pub(crate) async fn start(ctx: &Ctx) -> Result<AgentReply, UiError> {
    ctx.require_profile()?;
    let mut slot = ctx.agent.lock().await;
    if let Slot::InApp(r) = &*slot
        && !r.task.is_finished()
    {
        return Ok(AgentReply {
            agent: AgentMode::InApp,
        });
    }
    if alive(ctx).await {
        *slot = Slot::Stopped;
        return Ok(AgentReply {
            agent: AgentMode::External,
        });
    }
    match launch(ctx).await {
        Ok(Launched::InApp(r)) => {
            *slot = Slot::InApp(r);
            Ok(AgentReply {
                agent: AgentMode::InApp,
            })
        }
        Ok(Launched::External) => {
            *slot = Slot::Stopped;
            Ok(AgentReply {
                agent: AgentMode::External,
            })
        }
        Err(e) => {
            *slot = Slot::Failed(e.message.clone());
            Err(e)
        }
    }
}

async fn stop_running(r: Running) {
    let _ = r.stop.send(());
    match tokio::time::timeout(STOP_TIMEOUT, r.task).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(e))) => tracing::warn!(error = %e, "agent stopped with an error"),
        Ok(Err(e)) => tracing::warn!(error = %e, "agent task failed"),
        Err(_) => tracing::warn!("agent did not stop in time"),
    }
}

/// Stops the in_app agent (peers get Bye). An external agent is not ours: `agent_external`.
pub(crate) async fn stop(ctx: &Ctx) -> Result<AgentReply, UiError> {
    let mut slot = ctx.agent.lock().await;
    let stopped = AgentReply {
        agent: AgentMode::Stopped,
    };
    match std::mem::replace(&mut *slot, Slot::Stopped) {
        Slot::InApp(r) => {
            stop_running(r).await;
            Ok(stopped)
        }
        Slot::Stopped | Slot::Failed(_) if alive(ctx).await => Err(UiError::agent_external()),
        Slot::Stopped | Slot::Failed(_) => Ok(stopped),
    }
}

/// Restarts the in_app agent so it reads `config.toml` again. `false` when there is no in_app
/// agent (an external one is not ours; a stopped one reads the file when it starts).
pub(crate) async fn restart(ctx: &Ctx) -> Result<bool, UiError> {
    let running = matches!(&*ctx.agent.lock().await, Slot::InApp(r) if !r.task.is_finished());
    if !running {
        return Ok(false);
    }
    stop(ctx).await?;
    start(ctx).await?;
    Ok(true)
}

pub(crate) async fn shutdown(ctx: &Ctx) {
    let mut slot = ctx.agent.lock().await;
    if let Slot::InApp(r) = std::mem::replace(&mut *slot, Slot::Stopped) {
        stop_running(r).await;
    }
}

/// The mode for `app_state` and the poll. `alive`: an agent answered `Status` just now.
pub(crate) async fn mode(ctx: &Ctx, alive: bool) -> (AgentMode, Option<String>) {
    let mut slot = ctx.agent.lock().await;
    if matches!(&*slot, Slot::InApp(r) if r.task.is_finished())
        && let Slot::InApp(r) = std::mem::replace(&mut *slot, Slot::Stopped)
    {
        let why = match r.task.await {
            Ok(Ok(())) => msg::AGENT_STOPPED_ITSELF.to_string(),
            Ok(Err(e)) => msg::agent_died(&UiError::from(e).message),
            Err(e) => msg::agent_died(&e.to_string()),
        };
        *slot = Slot::Failed(why);
    }
    match &*slot {
        Slot::InApp(_) => (AgentMode::InApp, None),
        _ if alive => (AgentMode::External, None),
        Slot::Failed(m) => (AgentMode::Failed, Some(m.clone())),
        Slot::Stopped => (AgentMode::Stopped, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(clipboard: bool, input: bool) -> ModuleSet {
        ModuleSet { clipboard, input }
    }

    #[test]
    fn plan_without_input_support_and_clipboard_off_is_an_error() {
        let e = plan(set(false, true), false).unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.message.as_str()),
            (code::NO_USABLE_MODULE, msg::NO_USABLE_MODULE)
        );
    }

    #[test]
    fn plan_without_input_support_skips_input_and_keeps_clipboard() {
        let p = plan(set(true, true), false).unwrap();
        assert_eq!(
            p,
            Plan {
                clipboard: true,
                input: false,
                input_skipped: true
            }
        );
    }

    #[test]
    fn plan_with_input_support_builds_what_is_enabled() {
        let p = plan(set(false, true), true).unwrap();
        assert_eq!(
            p,
            Plan {
                clipboard: false,
                input: true,
                input_skipped: false
            }
        );
        assert!(plan(set(false, false), true).is_err());
    }

    #[test]
    fn plan_for_this_platform_matches_its_support() {
        assert_eq!(
            plan(set(false, true), clipsync_input::SUPPORTED).is_ok(),
            clipsync_input::SUPPORTED
        );
    }
}
