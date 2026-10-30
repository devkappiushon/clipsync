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

use crate::api::code;
use crate::status::AgentMode;
use crate::{Ctx, UiError, clean, lock, msg};

/// Bind, modules and the control socket; the input portal may take a while on first use.
const START_TIMEOUT: Duration = Duration::from_secs(15);

/// Modules get 3 s and peers a Bye inside `agent::run`; this bounds the whole stop.
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) enum Slot {
    Stopped,
    /// Last start failed or the agent died: the message goes to `app_state.agent_error`.
    Failed(String),
    InApp(Running),
}

pub(crate) struct Running {
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), CoreError>>,
    operation: Option<clipsync_core::ProfileOperationGuard>,
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

/// Validação pura do snapshot candidato: não constrói adapters nem abre portais.
pub(crate) fn validate_config(cfg: &Config) -> Result<(), UiError> {
    let plan = plan(cfg.modules, clipsync_input::SUPPORTED)?;
    if plan.clipboard {
        clipsync_clipboard::ClipboardConfig::from_table(cfg.clipboard.as_ref())
            .map_err(|_| UiError::new(code::MODULE_BUILD, "configuração de clipboard inválida"))?;
    }
    if plan.input {
        clipsync_input::InputConfig::from_table(cfg.input.as_ref())
            .map_err(|_| UiError::new(code::MODULE_BUILD, "configuração de input inválida"))?;
    }
    Ok(())
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

pub(crate) enum Presence {
    OwnRunning,
    ExternalResponding,
    ProvenAbsent,
    Unknown(UiError),
}

pub(crate) async fn presence(ctx: &Ctx) -> Presence {
    if matches!(&*ctx.agent.lock().await, Slot::InApp(r) if !r.task.is_finished()) {
        return Presence::OwnRunning;
    }
    match tokio::time::timeout(
        Duration::from_secs(3),
        clipsync_core::control::request(&ctx.profile, ControlRequest::Status),
    )
    .await
    {
        Ok(Ok(ControlResponse::Status(_))) => Presence::ExternalResponding,
        Ok(Err(CoreError::AgentNotRunning)) => Presence::ProvenAbsent,
        Ok(Err(e)) => Presence::Unknown(e.into()),
        _ => Presence::Unknown(UiError::new(
            "agent_unknown",
            "não foi possível confirmar o estado do agente",
        )),
    }
}

pub(crate) async fn start_reserved(
    ctx: &Ctx,
    startup: agent::Startup,
    initial_lock: Option<bool>,
) -> Result<AgentReply, UiError> {
    let modules = build(ctx, &ctx.profile, startup.config())?;
    start_modules(ctx, startup, modules, initial_lock).await
}

/// Posse e readiness da tarefa, compartilhadas por todos os módulos construídos.
async fn start_modules(
    ctx: &Ctx,
    startup: agent::Startup,
    modules: Vec<Box<dyn Module>>,
    initial_lock: Option<bool>,
) -> Result<AgentReply, UiError> {
    let (stop, stop_rx) = oneshot::channel();
    let (ready, ready_rx) = oneshot::channel();
    let mut slot = ctx.agent.lock().await;
    if matches!(&*slot, Slot::InApp(r) if !r.task.is_finished()) {
        return Err(UiError::new(
            "profile_busy",
            "o agente ainda está em execução",
        ));
    }
    let operation = startup.operation_guard();
    let task = tokio::spawn(agent::run_reserved(
        startup,
        modules,
        initial_lock,
        Some(ready),
        async move {
            let _ = stop_rx.await;
        },
    ));
    *slot = Slot::InApp(Running {
        stop: Some(stop),
        task,
        operation: Some(operation),
    });
    let result = tokio::time::timeout(START_TIMEOUT, ready_rx).await;
    if matches!(result, Ok(Ok(Ok(())))) {
        if let Slot::InApp(running) = &mut *slot {
            drop(running.operation.take());
        }
        return Ok(AgentReply {
            agent: AgentMode::InApp,
        });
    }
    let mut error = match result {
        Ok(Ok(Err(agent::StartupFailure::External))) => UiError::agent_external(),
        Ok(_) => UiError::new(
            "agent_start_failed",
            "a inicialização do agente não foi confirmada",
        ),
        Err(_) => UiError::new("agent_start_failed", msg::AGENT_START_TIMEOUT),
    };
    if let Slot::InApp(r) = &mut *slot
        && let Ok(ended) = stop_running(r).await
    {
        if let Some(cause) = ended {
            error = cause;
        }
        error.message = msg::agent_failed(&error.message);
        *slot = Slot::Failed(error.message.clone());
    }
    Err(error)
}

pub(crate) async fn start(ctx: &Ctx) -> Result<AgentReply, UiError> {
    ctx.require_profile()?;
    match presence(ctx).await {
        Presence::OwnRunning => {
            return Ok(AgentReply {
                agent: AgentMode::InApp,
            });
        }
        Presence::ExternalResponding => {
            return Ok(AgentReply {
                agent: AgentMode::External,
            });
        }
        Presence::Unknown(error) => return Err(error),
        Presence::ProvenAbsent => {}
    }
    let startup = match agent::Startup::reserve(&ctx.profile).await {
        Ok(startup) => startup,
        Err(CoreError::AgentAlreadyRunning) => {
            return Ok(AgentReply {
                agent: AgentMode::External,
            });
        }
        Err(e) => return Err(e.into()),
    };
    match start_reserved(ctx, startup, None).await {
        Ok(reply) => Ok(reply),
        Err(e) => {
            let mut slot = ctx.agent.lock().await;
            if !matches!(&*slot, Slot::InApp(_)) {
                *slot = Slot::Failed(e.message.clone());
            }
            Err(e)
        }
    }
}

pub(crate) async fn retain_operation(ctx: &Ctx, operation: clipsync_core::ProfileOperationGuard) {
    if let Slot::InApp(r) = &mut *ctx.agent.lock().await {
        r.operation = Some(operation);
    }
}

async fn stop_running(r: &mut Running) -> Result<Option<UiError>, UiError> {
    if let Some(stop) = r.stop.take() {
        let _ = stop.send(());
    }
    match tokio::time::timeout(STOP_TIMEOUT, &mut r.task).await {
        Ok(Ok(Ok(()))) => Ok(None),
        // O término foi observado; conservar a causa para o erro de inicialização.
        Ok(Ok(Err(error))) => Ok(Some(error.into())),
        Ok(Err(error)) => Ok(Some(UiError::internal(&error.to_string()))),
        Err(_) => Err(UiError::new(
            "agent_stop_pending",
            "a parada do agente ainda está em andamento",
        )),
    }
}

pub(crate) async fn stop(ctx: &Ctx) -> Result<AgentReply, UiError> {
    let mut slot = ctx.agent.lock().await;
    if let Slot::InApp(r) = &mut *slot {
        let _ = stop_running(r).await?;
        *slot = Slot::Stopped;
        return Ok(AgentReply {
            agent: AgentMode::Stopped,
        });
    }
    drop(slot);
    match presence(ctx).await {
        Presence::ExternalResponding => Err(UiError::agent_external()),
        Presence::Unknown(error) => Err(error),
        Presence::OwnRunning => Err(UiError::new("profile_busy", "o agente está ocupado")),
        Presence::ProvenAbsent => Ok(AgentReply {
            agent: AgentMode::Stopped,
        }),
    }
}

/// O supervisor mantém Ctx/Running e a reserva vivos mesmo depois de entregar um timeout.
/// Não segura o mutex durante a espera: consultas continuam vendo a tarefa possuída.
pub(crate) async fn finish_pending(ctx: &Ctx) {
    loop {
        let finished = {
            let slot = ctx.agent.lock().await;
            match &*slot {
                Slot::InApp(r) if r.stop.is_none() => r.task.is_finished(),
                _ => return,
            }
        };
        if finished {
            let _ = stop(ctx).await;
            ctx.emit(crate::UiEvent::Resync(crate::events::Resync {}));
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

pub(crate) async fn shutdown(ctx: &Ctx) {
    let _ = stop(ctx).await;
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod lifecycle_tests {
    use super::*;
    use crate::{Ui, UiOptions};
    use clipsync_core::InitOptions;
    use clipsync_proto::ModuleSet;
    use std::sync::Arc;

    async fn ui() -> (tempfile::TempDir, Ui) {
        let d = tempfile::Builder::new().prefix("csj-").tempdir().unwrap();
        let profile = Profile::resolve(Some(d.path().join("p"))).unwrap();
        profile
            .init(InitOptions {
                device_name: "job-fake".into(),
                modules: ModuleSet {
                    input: false,
                    clipboard: true,
                },
                listen: "127.0.0.1:0".parse().unwrap(),
                fake_adapters: true,
            })
            .unwrap();
        let ui = Ui::start(UiOptions {
            profile: Some(profile.dir),
            fake_adapters: true,
            ..UiOptions::default()
        })
        .await
        .unwrap();
        stop(&ui.ctx).await.unwrap();
        if let Some(poller) = crate::lock(&ui.ctx.poller).take() {
            poller.abort();
        }
        (d, ui)
    }

    #[tokio::test]
    async fn b09_reconsulta_sem_status_nao_confirma_pausa_persistida() {
        let (_d, ui) = ui().await;
        ui.ctx.profile.set_paused(true).unwrap();
        let (stop, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _ = rx.await;
            Ok(())
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let state = crate::status::app_state(&ui.ctx).await.unwrap();
        assert_eq!(state.agent, AgentMode::InApp);
        let unknown = state.agent_error.is_some();
        ui.shutdown().await;
        assert!(
            unknown,
            "estado persistido não confirma pausa atual sem Status"
        );
    }

    struct AdmissionBlocked {
        admit: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
        finish: Arc<tokio::sync::Notify>,
        stopping: oneshot::Sender<()>,
    }
    impl Module for AdmissionBlocked {
        fn kind(&self) -> clipsync_proto::ModuleKind {
            clipsync_proto::ModuleKind::Clipboard
        }
        fn adapter_name(&self) -> String {
            // Gate fake antes do bootstrap; apenas este worker fica retido, sem adapters/OS.
            let _ = self
                .admit
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(40));
            "fake-admission".into()
        }
        fn run<'async_trait>(
            self: Box<Self>,
            _host: Arc<dyn clipsync_proto::ModuleHost>,
            mut rx: tokio::sync::mpsc::Receiver<clipsync_proto::ModuleInput>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<(), clipsync_proto::ModuleError>>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            Self: 'async_trait,
        {
            Box::pin(async move {
                let mut stopping = Some(self.stopping);
                while let Some(input) = rx.recv().await {
                    match input {
                        clipsync_proto::ModuleInput::Command { reply, .. } => {
                            let _ = reply.send(clipsync_proto::ModuleReply::Status(
                                clipsync_proto::ModuleStatus {
                                    kind: clipsync_proto::ModuleKind::Clipboard,
                                    enabled: true,
                                    adapter: Some("fake".into()),
                                    authorized_peers: vec![],
                                    last_event: None,
                                    last_block: None,
                                },
                            ));
                        }
                        clipsync_proto::ModuleInput::Shutdown => {
                            if let Some(tx) = stopping.take() {
                                let _ = tx.send(());
                            }
                            self.finish.notified().await;
                            break;
                        }
                        _ => {}
                    }
                }
                Ok(())
            })
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn timeout_real_de_start_e_stop_retem_novo_running_ate_join() {
        let (_d, ui) = ui().await;
        let operation = ui.ctx.profile.reserve_operation(true).unwrap();
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let startup =
            agent::Startup::with_listener(&ui.ctx.profile, operation.clone(), listener).unwrap();
        let (admit, admission) = std::sync::mpsc::channel();
        let (stopping, stopped) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let result = start_modules(
            &ui.ctx,
            startup,
            vec![Box::new(AdmissionBlocked {
                admit: std::sync::Mutex::new(admission),
                finish: finish.clone(),
                stopping,
            })],
            None,
        )
        .await;
        assert!(result.is_err());
        drop(operation); // A chamada já devolveu erro; só a tarefa/supervisão pode reter o gate.
        assert!(ui.ctx.profile.reserve_operation(true).is_err());
        admit.send(()).unwrap();
        stopped.await.unwrap();
        let retained = ui.ctx.profile.reserve_operation(true).is_err();
        assert!(matches!(&*ui.ctx.agent.lock().await,Slot::InApp(r) if !r.task.is_finished()));
        finish.notify_one();
        finish_pending(&ui.ctx).await;
        ui.shutdown().await;
        assert!(
            retained,
            "bootstrap tardio não pode liberar o gate durante cleanup"
        );
        assert!(ui.ctx.profile.reserve_operation(true).is_ok());
    }

    #[tokio::test]
    async fn stop_lento_mantem_handle_e_reserva_ate_termino() {
        let (_d, ui) = ui().await;
        let operation = ui.ctx.profile.reserve_operation(true).unwrap();
        let (stop, rx) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let wait = finish.clone();
        let task = tokio::spawn(async move {
            let _ = rx.await;
            wait.notified().await;
            Ok(())
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: Some(operation),
        });
        assert!(super::stop(&ui.ctx).await.is_err());
        assert!(
            matches!(&*ui.ctx.agent.lock().await,Slot::InApp(r) if !r.task.is_finished() && r.stop.is_none())
        );
        assert!(ui.ctx.profile.reserve_operation(true).is_err());
        finish.notify_one();
        super::stop(&ui.ctx).await.unwrap();
        assert!(ui.ctx.profile.reserve_operation(true).is_ok());
        ui.shutdown().await;
    }

    #[tokio::test]
    async fn cancelar_caller_nao_cancela_job_de_parada() {
        let (_d, ui) = ui().await;
        let (stop, rx) = oneshot::channel();
        let (entered, seen) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let wait = finish.clone();
        let task = tokio::spawn(async move {
            let _ = rx.await;
            let _ = entered.send(());
            wait.notified().await;
            Ok(())
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let caller = ui.clone();
        let call =
            tokio::spawn(async move { caller.call(serde_json::json!({"cmd":"agent_stop"})).await });
        seen.await.unwrap();
        call.abort();
        let _ = call.await;
        assert!(ui.ctx.mutation.try_lock().is_err());
        finish.notify_one();
        let jobs = std::mem::take(&mut *crate::lock(&ui.ctx.jobs));
        for job in jobs {
            job.await.unwrap();
        }
        assert!(matches!(&*ui.ctx.agent.lock().await, Slot::Stopped));
        ui.shutdown().await;
    }
    struct SlowControl {
        profile: Profile,
    }
    impl clipsync_core::control::ControlHandler for SlowControl {
        fn handle<'a, 'b>(
            &'a self,
            request: ControlRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ControlResponse> + Send + 'b>>
        where
            'a: 'b,
            Self: 'b,
        {
            Box::pin(async move {
                if matches!(request, ControlRequest::Pause) {
                    return match self.profile.set_paused(true) {
                        Ok(()) => ControlResponse::Ok,
                        Err(e) => ControlResponse::Error {
                            message: e.to_string(),
                        },
                    };
                }
                ControlResponse::Error {
                    message: "sonda fake".into(),
                }
            })
        }
    }

    #[tokio::test]
    async fn cancelar_modules_set_durante_stop_preserva_job_e_reserva() {
        let (_d, ui) = ui().await;
        let text = std::fs::read_to_string(ui.ctx.profile.config_path()).unwrap();
        std::fs::write(
            ui.ctx.profile.config_path(),
            format!("{text}\n[input]\ncapture = \"fake\"\nemulate = \"fake\"\n"),
        )
        .unwrap();
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let (stop, rx) = oneshot::channel();
        let (entered, seen) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let wait = finish.clone();
        let handler = Arc::new(SlowControl {
            profile: ui.ctx.profile.clone(),
        });
        let task = tokio::spawn(async move {
            listener
                .serve(handler, async {
                    let _ = rx.await;
                    let _ = entered.send(());
                    wait.notified().await;
                })
                .await
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let caller = ui.clone();
        let call = tokio::spawn(async move {
            caller.call(serde_json::json!({"cmd":"modules_set","args":{"modules":{"input":true,"clipboard":true},"expected":{"input":false,"clipboard":true}}})).await
        });
        seen.await.unwrap();
        call.abort();
        let _ = call.await;
        assert!(ui.ctx.mutation.try_lock().is_err());
        assert!(ui.ctx.profile.reserve_operation(true).is_err());
        finish.notify_one();
        let jobs = std::mem::take(&mut *crate::lock(&ui.ctx.jobs));
        for job in jobs {
            job.await.unwrap();
        }
        assert!(ui.ctx.profile.load_config().unwrap().modules.input);
        assert!(ui.ctx.profile.load_state().unwrap().paused);
        assert!(matches!(&*ui.ctx.agent.lock().await,Slot::InApp(r) if !r.task.is_finished()));
        ui.shutdown().await;
    }

    #[tokio::test]
    async fn conflito_integral_durante_stop_preserva_editor_e_reporta_efeitos() {
        let (_d, ui) = ui().await;
        let text = std::fs::read_to_string(ui.ctx.profile.config_path()).unwrap();
        std::fs::write(
            ui.ctx.profile.config_path(),
            format!("{text}\n[input]\ncapture = \"fake\"\nemulate = \"fake\"\n"),
        )
        .unwrap();
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let (stop, rx) = oneshot::channel();
        let (entered, seen) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let wait = finish.clone();
        let handler = Arc::new(SlowControl {
            profile: ui.ctx.profile.clone(),
        });
        let task = tokio::spawn(async move {
            listener
                .serve(handler, async {
                    let _ = rx.await;
                    let _ = entered.send(());
                    wait.notified().await;
                })
                .await
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let caller = ui.clone();
        let call = tokio::spawn(async move {
            caller.call(serde_json::json!({"cmd":"modules_set","args":{"modules":{"input":true,"clipboard":true},"expected":{"input":false,"clipboard":true}}})).await
        });
        seen.await.unwrap();
        let changed = std::fs::read_to_string(ui.ctx.profile.config_path())
            .unwrap()
            .replace("job-fake", "editor-concorrente");
        std::fs::write(ui.ctx.profile.config_path(), &changed).unwrap();
        finish.notify_one();
        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.code, "stale_config");
        let details = error.details.unwrap();
        assert_eq!(details["effects"]["stop"], "confirmed");
        assert_eq!(details["effects"]["config_saved"], false);
        assert_eq!(details["observed"]["agent"], "stopped");
        assert_eq!(details["observed"]["paused"], true);
        assert_eq!(
            std::fs::read_to_string(ui.ctx.profile.config_path()).unwrap(),
            changed
        );
        ui.shutdown().await;
    }
    struct ExternalControl {
        profile: Profile,
        mutations: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl clipsync_core::control::ControlHandler for ExternalControl {
        fn handle<'a, 'b>(
            &'a self,
            request: ControlRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ControlResponse> + Send + 'b>>
        where
            'a: 'b,
            Self: 'b,
        {
            Box::pin(async move {
                if matches!(request, ControlRequest::Status) {
                    let cfg = self.profile.load_config().unwrap();
                    return ControlResponse::Status(clipsync_proto::AgentStatus {
                        device_name: cfg.device_name,
                        id: self.profile.load_identity().unwrap().id(),
                        listen: cfg.listen,
                        paused: true,
                        peers: vec![],
                        modules: vec![],
                        recent_blocks: vec![],
                        recent_removals: vec![],
                    });
                }
                self.mutations
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                ControlResponse::Error {
                    message: "não modificar externo".into(),
                }
            })
        }
    }

    #[tokio::test]
    async fn externo_apos_stop_impede_save_e_nao_recebe_mutacoes() {
        let (_d, ui) = ui().await;
        let config = std::fs::read(ui.ctx.profile.config_path()).unwrap();
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let (stop, rx) = oneshot::channel();
        let (external, receiver) = oneshot::channel();
        let handler = Arc::new(SlowControl {
            profile: ui.ctx.profile.clone(),
        });
        let socket = ui.ctx.profile.socket_path();
        let task = tokio::spawn(async move {
            listener
                .serve(handler, async {
                    let _ = rx.await;
                })
                .await?;
            let replacement = clipsync_core::control::bind(&socket).await?;
            let _ = external.send(replacement);
            Ok(())
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let caller = ui.clone();
        let call = tokio::spawn(async move {
            caller.call(serde_json::json!({"cmd":"modules_set","args":{"modules":{"input":true,"clipboard":true},"expected":{"input":false,"clipboard":true}}})).await
        });
        let replacement = receiver.await.unwrap();
        let (finish, finished) = oneshot::channel();
        let mutations = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let handler = Arc::new(ExternalControl {
            profile: ui.ctx.profile.clone(),
            mutations: mutations.clone(),
        });
        let external_task = tokio::spawn(replacement.serve(handler, async {
            let _ = finished.await;
        }));
        let error = call.await.unwrap().unwrap_err();
        assert_eq!(error.code, "agent_external");
        let details = error.details.unwrap();
        assert_eq!(details["effects"]["stop"], "confirmed");
        assert_eq!(details["effects"]["config_saved"], false);
        assert_eq!(details["observed"]["agent"], "external");
        assert_eq!(std::fs::read(ui.ctx.profile.config_path()).unwrap(), config);
        assert_eq!(mutations.load(std::sync::atomic::Ordering::Relaxed), 0);
        ui.shutdown().await;
        let _ = finish.send(());
        external_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn resposta_de_timeout_nao_abandona_reserva_quando_ui_e_descartada() {
        let (_d, ui) = ui().await;
        let profile = ui.ctx.profile.clone();
        let operation = profile.reserve_operation(true).unwrap();
        let (stop, rx) = oneshot::channel();
        let finish = Arc::new(tokio::sync::Notify::new());
        let wait = finish.clone();
        let task = tokio::spawn(async move {
            let _ = rx.await;
            wait.notified().await;
            Ok(())
        });
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: Some(operation),
        });
        let error = ui
            .call(serde_json::json!({"cmd":"agent_stop"}))
            .await
            .unwrap_err();
        assert_eq!(error.code, "agent_stop_pending");
        drop(ui);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(profile.reserve_operation(true).is_err());
        finish.notify_one();
        tokio::time::timeout(Duration::from_secs(2), async {
            while profile.reserve_operation(true).is_err() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    struct ObservacaoLenta {
        entered: std::sync::Mutex<Option<oneshot::Sender<()>>>,
        release: Arc<tokio::sync::Notify>,
    }
    impl clipsync_core::control::ControlHandler for ObservacaoLenta {
        fn handle<'a, 'b>(
            &'a self,
            _request: ControlRequest,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ControlResponse> + Send + 'b>>
        where
            'a: 'b,
            Self: 'b,
        {
            Box::pin(async move {
                if let Some(entered) = crate::lock(&self.entered).take() {
                    let _ = entered.send(());
                }
                self.release.notified().await;
                ControlResponse::Error {
                    message: "observação fake".into(),
                }
            })
        }
    }
    #[tokio::test]
    async fn reserva_permanece_ate_colher_observacao_final() {
        let (_d, ui) = ui().await;
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let (stop, rx) = oneshot::channel();
        let (entered, seen) = oneshot::channel();
        let release = Arc::new(tokio::sync::Notify::new());
        let handler = Arc::new(ObservacaoLenta {
            entered: std::sync::Mutex::new(Some(entered)),
            release: release.clone(),
        });
        let task = tokio::spawn(listener.serve(handler, async {
            let _ = rx.await;
        }));
        *ui.ctx.agent.lock().await = Slot::InApp(Running {
            stop: Some(stop),
            task,
            operation: None,
        });
        let caller = ui.clone();
        let call = tokio::spawn(async move {
            caller.call(serde_json::json!({"cmd":"modules_set","args":{"modules":{"input":false,"clipboard":true},"expected":{"input":false,"clipboard":true}}})).await
        });
        seen.await.unwrap();
        let blocked = ui.ctx.profile.reserve_operation(false).is_err();
        release.notify_waiters();
        call.await.unwrap().unwrap();
        ui.shutdown().await;
        assert!(
            blocked,
            "Resume não pode atravessar a coleta final de estado"
        );
    }
}
