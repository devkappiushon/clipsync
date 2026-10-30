//! Reconfiguração local de módulos; nunca recria identidade nem retoma automaticamente.
use crate::status::Modules;
use crate::{Ctx, UiError, UiEvent, agent};
use clipsync_core::{agent::Startup, control};
use clipsync_proto::{
    ControlRequest, ControlResponse, ModuleCommand, ModuleKind, ModuleReply, ModuleSet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Args {
    pub modules: Modules,
    pub expected: Modules,
}
fn set(m: Modules) -> ModuleSet {
    ModuleSet {
        input: m.input,
        clipboard: m.clipboard,
    }
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    NotRequested,
    Requested,
    Confirmed,
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Start {
    NotRequested,
    Requested,
    OwnedReady,
    Failed,
    External,
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockRestore {
    NotNeeded,
    Pending,
    Confirmed,
    Failed,
}
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedAgent {
    InApp,
    External,
    Stopped,
    Failed,
    Unknown,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub pause: Action,
    pub pause_persisted: Option<bool>,
    pub stop: Action,
    pub config_saved: Option<bool>,
    pub start: Start,
    pub lock_restore: LockRestore,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            pause: Action::NotRequested,
            pause_persisted: None,
            stop: Action::NotRequested,
            config_saved: Some(false),
            start: Start::NotRequested,
            lock_restore: LockRestore::NotNeeded,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Observed {
    pub agent: ObservedAgent,
    pub paused: Option<bool>,
    pub screen_locked: Option<bool>,
    pub modules: Option<Modules>,
}
impl Default for Observed {
    fn default() -> Self {
        Self {
            agent: ObservedAgent::Unknown,
            paused: None,
            screen_locked: None,
            modules: None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub changed: bool,
    pub modules: Modules,
    pub effects: Effects,
    pub observed: Observed,
}

async fn observe(ctx: &Ctx) -> Observed {
    let mut out = Observed {
        modules: ctx.profile.load_config().ok().map(|c| c.modules.into()),
        ..Observed::default()
    };
    match agent::presence(ctx).await {
        agent::Presence::OwnRunning => out.agent = ObservedAgent::InApp,
        agent::Presence::ExternalResponding => out.agent = ObservedAgent::External,
        agent::Presence::ProvenAbsent => {
            out.agent = ObservedAgent::Stopped;
            out.paused = ctx.profile.load_state().ok().map(|s| s.paused);
            return out;
        }
        agent::Presence::Unknown(_) => return out,
    }
    if let Ok(Ok(ControlResponse::Status(s))) = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        ctx.request(ControlRequest::Status),
    )
    .await
    {
        out.paused = Some(s.paused);
    }
    if out.modules.is_some_and(|m| m.input)
        && let Ok(Ok(ModuleReply::InputState(s))) = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            ctx.ask_module(ModuleKind::Input, ModuleCommand::InputState),
        )
        .await
    {
        out.screen_locked = Some(s.screen_locked);
    }
    out
}

enum Edit {
    Modules(Args),
    ClipboardMode(crate::api::ClipMode),
}
struct Change {
    changed: bool,
    modules: ModuleSet,
    external: bool,
}
struct Transaction {
    change: Change,
    effects: Effects,
    observed: Observed,
}

pub(crate) async fn apply(ctx: &Ctx, args: Args) -> Result<Reply, UiError> {
    let result = transact(ctx, Edit::Modules(args)).await?;
    Ok(Reply {
        changed: result.change.changed,
        modules: result.change.modules.into(),
        effects: result.effects,
        observed: result.observed,
    })
}

pub(crate) async fn apply_mode(
    ctx: &Ctx,
    mode: crate::api::ClipMode,
) -> Result<crate::clipboard::ModeReply, UiError> {
    let result = transact(ctx, Edit::ClipboardMode(mode)).await?;
    Ok(crate::clipboard::ModeReply {
        mode,
        restart_needed: result.change.external,
        effects: Some(result.effects),
        observed: Some(result.observed),
    })
}

async fn transact(ctx: &Ctx, edit: Edit) -> Result<Transaction, UiError> {
    let mut effects = Effects::default();
    let operation = ctx
        .require_profile()
        .and_then(|()| ctx.profile.reserve_operation(true).map_err(UiError::from));
    let result = match &operation {
        Ok(operation) => apply_inner(ctx, edit, &mut effects, operation).await,
        Err(error) => Err(error.clone()),
    };
    let observed = observe(ctx).await;
    if !matches!(&result, Ok(change) if !change.changed) {
        ctx.emit(UiEvent::Resync(crate::events::Resync {}));
    }
    match result {
        Ok(change) => Ok(Transaction {
            change,
            effects,
            observed,
        }),
        Err(error) => {
            Err(error.with_details(serde_json::json!({"effects":effects,"observed":observed})))
        }
    }
}

async fn apply_inner(
    ctx: &Ctx,
    change: Edit,
    effects: &mut Effects,
    operation: &clipsync_core::ProfileOperationGuard,
) -> Result<Change, UiError> {
    let mut edit = match &change {
        Edit::Modules(args) => ctx
            .profile
            .begin_config_edit(operation, set(args.expected))?,
        Edit::ClipboardMode(_) => ctx.profile.begin_clipboard_edit(operation)?,
    };
    let next = match &change {
        Edit::Modules(args) => {
            let next = set(args.modules);
            agent::plan(next, clipsync_input::SUPPORTED)?;
            next
        }
        Edit::ClipboardMode(_) => {
            if !edit.config().modules.clipboard {
                return Err(UiError::new(
                    crate::api::code::MODULE_OFF,
                    crate::msg::CLIPBOARD_OFF,
                ));
            }
            edit.config().modules
        }
    };
    let mut candidate = edit.config().clone();
    candidate.modules = next;
    if let Edit::ClipboardMode(mode) = &change {
        candidate
            .clipboard
            .get_or_insert_with(toml::Table::new)
            .insert("mode".into(), toml::Value::String(mode_text(*mode).into()));
    }
    agent::validate_config(&candidate)?;
    let no_op = match &change {
        Edit::Modules(_) => edit.config().modules == next,
        Edit::ClipboardMode(mode) => edit
            .config()
            .clipboard
            .as_ref()
            .and_then(|table| table.get("mode"))
            .map_or(*mode == crate::api::ClipMode::Auto, |value| {
                value.as_str() == Some(mode_text(*mode))
            }),
    };
    if no_op {
        return Ok(Change {
            changed: false,
            modules: next,
            external: matches!(
                agent::presence(ctx).await,
                agent::Presence::ExternalResponding
            ),
        });
    }
    let own = match agent::presence(ctx).await {
        agent::Presence::OwnRunning => true,
        agent::Presence::ProvenAbsent => false,
        agent::Presence::ExternalResponding => {
            if matches!(&change, Edit::ClipboardMode(_)) {
                save(&mut edit, &change)?;
                effects.config_saved = Some(true);
                return Ok(Change {
                    changed: true,
                    modules: next,
                    external: true,
                });
            }
            return Err(UiError::agent_external());
        }
        agent::Presence::Unknown(error) => return Err(error),
    };
    let mut screen_lock = None;
    if own {
        effects.pause = Action::Requested;
        match crate::status::pause_set(ctx, true).await {
            Ok(_) => {
                effects.pause = Action::Confirmed;
                effects.pause_persisted = Some(true);
            }
            Err(error) => {
                if error.message == clipsync_core::msg::STATE_NOT_SAVED {
                    effects.pause_persisted = Some(false);
                }
                return Err(error);
            }
        }
        if edit.config().modules.input {
            match ctx
                .ask_module(ModuleKind::Input, ModuleCommand::InputState)
                .await?
            {
                ModuleReply::InputState(s) => screen_lock = Some(s.screen_locked),
                _ => {
                    return Err(UiError::new(
                        "input_state_unknown",
                        "agente pausado; não foi possível conferir a trava de tela",
                    ));
                }
            }
        }
        agent::retain_operation(ctx, operation.clone()).await;
        effects.stop = Action::Requested;
        agent::stop(ctx).await?;
        effects.stop = Action::Confirmed;
    }
    let listener = control::bind(&ctx.profile.socket_path())
        .await
        .map_err(UiError::from)?;
    save(&mut edit, &change)?;
    effects.config_saved = Some(true);
    drop(edit);
    if !own {
        drop(listener);
        return Ok(Change {
            changed: true,
            modules: next,
            external: false,
        });
    }
    effects.start = Start::Requested;
    let lock = if next.input { screen_lock } else { None };
    if lock.is_some() {
        effects.lock_restore = LockRestore::Pending;
    }
    let started = async {
        let startup = Startup::with_listener(&ctx.profile, operation.clone(), listener)?;
        agent::start_reserved(ctx, startup, lock).await
    }
    .await;
    match started {
        Ok(_) => {
            effects.start = Start::OwnedReady;
            if lock.is_some() {
                effects.lock_restore = LockRestore::Confirmed;
            }
            Ok(Change {
                changed: true,
                modules: next,
                external: false,
            })
        }
        Err(error) => {
            effects.start = if error.code == "agent_external" {
                Start::External
            } else {
                Start::Failed
            };
            if lock.is_some() {
                effects.lock_restore = LockRestore::Failed;
            }
            Err(error)
        }
    }
}

fn save(edit: &mut clipsync_core::profile::ConfigEdit<'_>, change: &Edit) -> Result<(), UiError> {
    match change {
        Edit::Modules(args) => {
            edit.set_modules(set(args.modules))?;
        }
        Edit::ClipboardMode(mode) => {
            edit.set_clipboard_mode(mode_text(*mode))?;
        }
    }
    Ok(())
}

fn mode_text(mode: crate::api::ClipMode) -> &'static str {
    match mode {
        crate::api::ClipMode::Auto => "auto",
        crate::api::ClipMode::Manual => "manual",
    }
}
