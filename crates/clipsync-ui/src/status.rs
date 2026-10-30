//! `app_state` and `pause_set` (§12.5, common commands), plus `observe`: one look at the agent
//! that both `app_state` and the event poll use. Owned by Rivet.

use clipsync_core::msg as core_msg;
use clipsync_proto::{
    AgentStatus, ControlRequest, ControlResponse, InputState, ModuleCommand, ModuleKind,
    ModuleReply, ModuleSet, PeerId, PeerState,
};
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::api::code;
use crate::{Ctx, UiError, clean, msg};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    InApp,
    External,
    Stopped,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleName {
    Clipboard,
    Input,
}

impl From<ModuleKind> for ModuleName {
    fn from(k: ModuleKind) -> Self {
        match k {
            ModuleKind::Clipboard => ModuleName::Clipboard,
            ModuleKind::Input => ModuleName::Input,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Modules {
    pub clipboard: bool,
    pub input: bool,
}

impl From<ModuleSet> for Modules {
    fn from(m: ModuleSet) -> Self {
        Self {
            clipboard: m.clipboard,
            input: m.input,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceSelf {
    /// 64 hex.
    pub peer_id: String,
    pub short: String,
    pub name: String,
}

impl DeviceSelf {
    pub(crate) fn new(id: PeerId, name: &str) -> Self {
        Self {
            peer_id: id.to_hex(),
            short: id.short(),
            name: clean(name),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeshCount {
    pub connected: usize,
    pub total: usize,
}

/// What the screens may offer. The first seven follow the agent's input (Linux, module on,
/// `InputState` answering); `os_notifications` is on (the casca shows them, filtered by the
/// preferences: decision 10, S6-13); the rest has no backend in this version (§12.2 rule 6).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Features {
    pub input: bool,
    pub peer_monitors: bool,
    pub layout_links: bool,
    pub dead_corners: bool,
    pub screen_lock: bool,
    pub switch_device: bool,
    pub layout_copy: bool,
    pub os_notifications: bool,
    pub global_shortcuts: bool,
    pub peer_pause: bool,
    pub peer_controls_column: bool,
    pub peer_rtt: bool,
    pub peer_last_activity: bool,
    pub peer_addr_history: bool,
    pub clipboard_confirm_mode: bool,
    pub clipboard_pause_capture: bool,
    pub clipboard_private_mode: bool,
    pub pair_peer_declined_notice: bool,
    /// I-S7-2: the system refused Win+Alt+P to the input module (from `InputState`; false
    /// without an agent). Preferences read this, not the text of the block.
    pub resume_hotkey_refused: bool,
}

impl Features {
    pub(crate) fn new(input_live: bool, input: Option<&InputState>) -> Self {
        Self {
            input: input_live,
            peer_monitors: input_live,
            layout_links: input_live,
            dead_corners: input_live,
            screen_lock: input_live,
            switch_device: input_live && input.is_some_and(|i| i.switch_supported),
            layout_copy: input_live,
            os_notifications: true,
            // Win+Alt+1 to 9, L and P (§13.6): the Windows capture adapter, none on Linux yet.
            global_shortcuts: cfg!(windows) && input_live,
            resume_hotkey_refused: input.is_some_and(|i| i.resume_hotkey_refused),
            ..Self::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppState {
    pub initialized: bool,
    pub profile_dir: String,
    pub device: Option<DeviceSelf>,
    pub agent: AgentMode,
    pub agent_error: Option<String>,
    pub paused: bool,
    #[serde(default)]
    pub pause_warning: Option<String>,
    pub screen_locked: bool,
    pub mesh: MeshCount,
    pub modules: Modules,
    pub input_supported: bool,
    pub version: String,
    /// "linux" | "windows".
    pub platform: String,
    pub features: Features,
    #[serde(default)]
    pub traversal: crate::traversal::Traversal,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PausedReply {
    pub paused: bool,
}

/// One look at the agent of the profile.
pub(crate) struct Observed {
    pub(crate) agent: AgentMode,
    pub(crate) agent_error: Option<String>,
    /// `Some` when an agent (in_app or external) answered `Status`.
    pub(crate) status: Option<AgentStatus>,
    /// `Some` when the input module answered `InputState`.
    pub(crate) input: Option<InputState>,
}

/// Só NotRunning comprova ausência; erro/timeout/resposta inesperada conservam Unknown.
async fn read_status(ctx: &Ctx) -> Result<Option<AgentStatus>, UiError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(3),
        ctx.request(ControlRequest::Status),
    )
    .await
    {
        Ok(Ok(ControlResponse::Status(status))) => Ok(Some(status)),
        Ok(Err(error)) if error.code == code::AGENT_NOT_RUNNING => Ok(None),
        _ => Err(UiError::new(
            "agent_unknown",
            "não foi possível confirmar o estado atual do agente",
        )),
    }
}

pub(crate) async fn agent_status(ctx: &Ctx) -> Option<AgentStatus> {
    read_status(ctx).await.ok().flatten()
}

pub(crate) async fn observe(ctx: &Ctx) -> Observed {
    let reading = read_status(ctx).await;
    let unknown = reading.as_ref().err().map(|error| error.message.clone());
    let status = reading.ok().flatten();
    let (mut agent, mut agent_error) = agent::mode(ctx, status.is_some()).await;
    if let Some(error) = unknown {
        if agent != AgentMode::InApp {
            agent = AgentMode::Failed;
        }
        agent_error = Some(error);
    } else if agent == AgentMode::InApp && status.is_none() {
        agent_error =
            Some("o agente próprio ainda existe, mas seu estado não foi confirmado".into());
    }
    let input_on = status.as_ref().is_some_and(|s| {
        s.modules
            .iter()
            .any(|m| m.kind == ModuleKind::Input && m.enabled)
    });
    let input = if input_on {
        match ctx
            .ask_module(ModuleKind::Input, ModuleCommand::InputState)
            .await
        {
            Ok(ModuleReply::InputState(i)) => Some(i),
            _ => None,
        }
    } else {
        None
    };
    Observed {
        agent,
        agent_error,
        status,
        input,
    }
}

fn platform() -> &'static str {
    if cfg!(windows) { "windows" } else { "linux" }
}

pub(crate) async fn app_state(ctx: &Ctx) -> Result<AppState, UiError> {
    let o = observe(ctx).await;
    let p = &ctx.profile;
    let initialized = p.is_initialized();
    let cfg = if initialized {
        p.load_config().ok()
    } else {
        None
    };
    let device = match (&o.status, &cfg) {
        (Some(s), _) => Some(DeviceSelf::new(s.id, &s.device_name)),
        (None, Some(c)) => p
            .load_identity()
            .ok()
            .map(|i| DeviceSelf::new(i.id(), &c.device_name)),
        (None, None) => None,
    };
    let (paused, mesh) = match &o.status {
        Some(s) => (
            s.paused,
            MeshCount {
                connected: s
                    .peers
                    .iter()
                    .filter(|p| p.state == PeerState::Connected)
                    .count(),
                total: s.peers.len(),
            },
        ),
        None => (
            initialized && p.load_state().is_ok_and(|s| s.paused),
            MeshCount {
                connected: 0,
                total: if initialized {
                    p.load_peers().map_or(0, |peers| peers.peer.len())
                } else {
                    0
                },
            },
        ),
    };
    let modules: Modules = cfg.as_ref().map(|c| c.modules.into()).unwrap_or_default();
    let input_live = clipsync_input::SUPPORTED && modules.input && o.input.is_some();
    let traversal = crate::traversal::observe(ctx, &o);
    Ok(AppState {
        initialized,
        profile_dir: p.dir.display().to_string(),
        device,
        agent: o.agent,
        agent_error: o.agent_error,
        paused,
        pause_warning: crate::lock(&ctx.pause_warning).message.clone(),
        screen_locked: o.input.as_ref().is_some_and(|i| i.screen_locked),
        mesh,
        modules,
        input_supported: clipsync_input::SUPPORTED,
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: platform().to_string(),
        features: Features::new(input_live, o.input.as_ref()),
        traversal,
    })
}

/// Through the agent, like `clipsync pause` (§12.2 rule 1). The event `pause_changed` comes
/// from the poll, so a pause from the terminal looks the same.
#[derive(Default)]
pub(crate) struct PauseWarning {
    latest: Option<std::sync::Arc<()>>,
    message: Option<String>,
}
impl PauseWarning {
    fn begin(&mut self) -> std::sync::Arc<()> {
        let ticket = std::sync::Arc::new(());
        self.latest = Some(ticket.clone());
        ticket
    }
    fn finish(&mut self, ticket: &std::sync::Arc<()>, message: Option<String>) {
        if self
            .latest
            .as_ref()
            .is_some_and(|current| std::sync::Arc::ptr_eq(current, ticket))
        {
            self.message = message;
        }
    }
}

pub(crate) async fn pause_set(ctx: &Ctx, paused: bool) -> Result<PausedReply, UiError> {
    ctx.require_profile()?;
    let ticket = crate::lock(&ctx.pause_warning).begin();
    let req = if paused {
        ControlRequest::Pause
    } else {
        ControlRequest::Resume
    };
    let result = match ctx.request(req).await {
        Ok(ControlResponse::Ok) => Ok(PausedReply { paused }),
        Ok(ControlResponse::Error { message }) if message == core_msg::STATE_NOT_SAVED => {
            Err(UiError::new(code::IO, message))
        }
        Ok(ControlResponse::Error { message }) => Err(UiError::internal(&clean(&message))),
        Ok(_) => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
        Err(error) => Err(error),
    };
    let warning = result.as_ref().err().map(|error| {
        let action = if paused { "pausa" } else { "retomada" };
        if error.message == core_msg::STATE_NOT_SAVED {
            format!("A {action} foi aplicada em memória, mas não foi salva. Esse estado não sobreviverá ao encerramento.")
        } else {
            format!("A {action} solicitada não foi confirmada. Confira o estado antes de tentar novamente.")
        }
    });
    crate::lock(&ctx.pause_warning).finish(&ticket, warning);
    ctx.emit(crate::UiEvent::Resync(crate::events::Resync {}));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resposta_antiga_nao_apaga_falha_mais_recente() {
        let mut warning = PauseWarning::default();
        let old = warning.begin();
        let new = warning.begin();
        warning.finish(&new, Some("retomada não persistida".into()));
        warning.finish(&old, None);
        assert_eq!(warning.message.as_deref(), Some("retomada não persistida"));
        let confirmed = warning.begin();
        warning.finish(&confirmed, None);
        warning.finish(&new, Some("falha atrasada".into()));
        assert!(warning.message.is_none());
    }

    #[test]
    fn acao_nova_pendente_preserva_aviso_ate_sua_confirmacao() {
        let mut warning = PauseWarning::default();
        let failed = warning.begin();
        warning.finish(&failed, Some("pausa não persistida".into()));
        let next = warning.begin();
        warning.finish(&failed, None);
        assert!(warning.message.is_some());
        warning.finish(&next, None);
        assert!(warning.message.is_none());
    }

    fn input(refused: bool, switch: bool) -> InputState {
        InputState {
            local: vec![],
            peers: vec![],
            screen_locked: false,
            active_peer: None,
            pending_offer: None,
            keyboard_layout: None,
            resume_hotkey_refused: refused,
            switch_supported: switch,
        }
    }

    /// I-S7-2: the feature only says what the input module reported; no agent, no refusal.
    #[test]
    fn resume_hotkey_refused_comes_from_the_input_state() {
        assert!(Features::new(true, Some(&input(true, true))).resume_hotkey_refused);
        assert!(!Features::new(true, Some(&input(false, true))).resume_hotkey_refused);
        assert!(!Features::new(false, None).resume_hotkey_refused);
    }

    /// §13.9: "go to device" only where the capture adapter can start (the module says so in
    /// `InputState`); no agent, no switch.
    #[test]
    fn switch_device_comes_from_the_input_state() {
        assert!(Features::new(true, Some(&input(false, true))).switch_device);
        assert!(!Features::new(true, Some(&input(false, false))).switch_device);
        assert!(!Features::new(false, None).switch_device);
    }
}
