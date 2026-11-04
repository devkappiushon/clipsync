//! The agent (`clipsync run`): mesh + modules + control socket. Owned by Rivet.
//!
//! A module the config turns off is never started, never announced and refused at the door.
//! Pause is persisted in state.toml and applied by the mesh through `authorize_*`.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use clipsync_proto::{
    AgentStatus, ControlRequest, ControlResponse, Module, ModuleCommand, ModuleInput, ModuleKind,
    ModuleReply, ModuleStatus, PeerId,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::control::{self, ControlHandler};
use crate::transport::{Mesh, MeshOptions};
use crate::{Config, CoreError, Profile, ProfileOperationGuard, msg};

const MODULE_QUEUE: usize = 1024;
const MODULE_STOP_GRACE: Duration = Duration::from_secs(3);
/// Bound on waiting for the UDP port to be free after the mesh shut down.
const PORT_RELEASE_WAIT: Duration = Duration::from_secs(3);
const PORT_RELEASE_POLL: Duration = Duration::from_millis(5);
/// Preparação interativa aguarda consentimento, abaixo dos 120s do controle externo.
const INPUT_PREPARE_TIMEOUT: Duration = Duration::from_secs(110);
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const CLIP_SEND_TIMEOUT: Duration = Duration::from_secs(15);
const INPUT_OFFER_TIMEOUT: Duration = Duration::from_secs(5);
const LOOPBACK_SLACK: Duration = Duration::from_secs(30);

/// Runs until `shutdown` completes: then modules get `Shutdown`, peers get Bye and the
/// connections close with CloseCode::Shutdown.
pub async fn run(
    p: &Profile,
    modules: Vec<Box<dyn Module>>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    let startup = Startup::reserve(p).await?;
    run_reserved(startup, modules, None, None, shutdown).await
}

/// Reserva o perfil antes de ler a configuração usada para construir adapters.
pub struct Startup {
    profile: Profile,
    config: Config,
    operation: ProfileOperationGuard,
    control: control::ControlListener,
}

impl Startup {
    pub async fn reserve(profile: &Profile) -> Result<Self, CoreError> {
        let operation = profile.reserve_operation(false)?;
        let control = control::bind(&profile.socket_path()).await?;
        Self::with_listener(profile, operation, control)
    }

    pub fn with_listener(
        profile: &Profile,
        operation: ProfileOperationGuard,
        control: control::ControlListener,
    ) -> Result<Self, CoreError> {
        profile.check_operation(&operation, false)?;
        if !control.belongs_to(&profile.socket_path()) {
            return Err(CoreError::AgentAlreadyRunning);
        }
        let config = profile.load_config()?;
        Ok(Self {
            profile: profile.clone(),
            config,
            operation,
            control,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// A supervisão também retém a operação enquanto a readiness estiver indeterminada.
    pub fn operation_guard(&self) -> ProfileOperationGuard {
        self.operation.clone()
    }
}

/// Motivo interno da recusa de readiness; não altera o protocolo local ou remoto.
pub enum StartupFailure {
    External,
    Bootstrap,
}

/// Readiness pertence à tarefa que consome o listener. Falhas limpam módulos antes de liberar a reserva.
pub async fn run_reserved(
    startup: Startup,
    modules: Vec<Box<dyn Module>>,
    initial_lock: Option<bool>,
    ready: Option<oneshot::Sender<Result<(), StartupFailure>>>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    let Startup {
        profile: p,
        config: cfg,
        operation,
        mut control,
    } = startup;
    let identity = p.load_identity()?;
    let peers = p.load_peers()?;
    let state = p.load_state()?;
    if let Err(error) = crate::logging::open_file(&p.dir, &cfg.device_name) {
        static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            tracing::warn!(%error, "{}", msg::LOG_NOT_OPENED);
        }
    }

    let mut channels = HashMap::new();
    let mut adapters = HashMap::new();
    let mut runners = Vec::new();
    for m in modules {
        let kind = m.kind();
        if !cfg.modules.contains(kind) || channels.contains_key(&kind) {
            tracing::warn!(
                ?kind,
                "module not started: off in config.toml or duplicated"
            );
            continue;
        }
        let (tx, rx) = mpsc::channel(MODULE_QUEUE);
        adapters.insert(kind, m.adapter_name());
        channels.insert(kind, tx);
        runners.push((m, rx));
    }

    let mesh = Arc::new(Mesh::start(MeshOptions {
        identity,
        device_name: cfg.device_name.clone(),
        listen: cfg.listen,
        modules: channels.clone(),
        peers,
        paused: state.paused,
    })?);
    // v1.16: a peer that removed us (Bye{Revoked}) leaves peers.toml the way a local revoke does.
    let revocations = mesh.revocations().map(|mut rx| {
        let (profile, mesh) = (p.clone(), mesh.clone());
        tokio::spawn(async move {
            while let Some(peer) = rx.recv().await {
                remove_revoked(&profile, &mesh, peer).await;
            }
        })
    });
    let mut tasks = JoinSet::new();
    for (m, rx) in runners {
        // Each module holds its own handle: none may outlive `run` here (see wait_port_released).
        let host = mesh.host();
        tasks.spawn(async move {
            let kind = m.kind();
            if let Err(e) = m.run(host, rx).await {
                tracing::error!(?kind, error = %e, "module stopped");
            }
        });
    }
    if state.paused {
        for tx in channels.values() {
            let _ = tx.send(ModuleInput::Paused(true)).await;
        }
    }
    tracing::info!(evento = "agent.up", descartados = crate::logging::preopen_dropped(), id = %mesh.local_id(), addr = ?mesh.local_addr().ok(), "agent up");

    let handler = Arc::new(Agent {
        profile: p.clone(),
        device_name: cfg.device_name,
        mesh: mesh.clone(),
        channels: channels.clone(),
        adapters,
        seen_blocks: Mutex::new(HashMap::new()),
    });
    // Win+Alt+P (`ModuleHost::request_pause`) takes the control's path: same state, same file.
    let pausing = mesh.pause_requests().map(|mut rx| {
        let handler = handler.clone();
        tokio::spawn(async move {
            while let Some(paused) = rx.recv().await {
                let _ = handler.set_paused(paused).await;
            }
        })
    });
    let bootstrap = async {
        if let Some(locked) = initial_lock {
            if !state.paused {
                return Err(CoreError::Profile(
                    "bootstrap exige pausa persistida".into(),
                ));
            }
            match handler
                .ask_module(ModuleKind::Input, ModuleCommand::InputLock { locked })
                .await
            {
                Ok(ModuleReply::Done) => {}
                _ => return Err(CoreError::Profile("trava de tela não confirmada".into())),
            }
        }
        // A resposta atravessa a mesma fila que recebeu Paused, confirmando seu processamento.
        for kind in channels.keys() {
            match handler.ask_module(*kind, ModuleCommand::Status).await {
                Ok(ModuleReply::Status(_)) => {}
                _ => {
                    return Err(CoreError::Profile(
                        "inicialização de módulo não confirmada".into(),
                    ));
                }
            }
        }
        if !control.is_current() {
            return Err(CoreError::AgentAlreadyRunning);
        }
        Ok(())
    }
    .await;
    let mut operation = Some(operation);
    let served = match bootstrap {
        Ok(()) => {
            if ready.is_some_and(|ready| ready.send(Ok(())).is_err()) {
                // O chamador já encerrou a espera: sucesso tardio não libera a operação.
                Err(CoreError::Profile(
                    "inicialização cancelada antes da confirmação".into(),
                ))
            } else {
                drop(operation.take());
                control.serve_reserved(handler.clone(), shutdown).await
            }
        }
        Err(error) => {
            if let Some(ready) = ready {
                let failure = if matches!(error, CoreError::AgentAlreadyRunning) {
                    StartupFailure::External
                } else {
                    StartupFailure::Bootstrap
                };
                let _ = ready.send(Err(failure));
            }
            Err(error)
        }
    };

    drop(handler);
    for tx in channels.values() {
        let _ = tx.send(ModuleInput::Shutdown).await;
    }
    drop(channels);
    let _ = timeout(MODULE_STOP_GRACE, async {
        while tasks.join_next().await.is_some() {}
    })
    .await;
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    if let Some(r) = revocations {
        r.abort();
    }
    if let Some(p) = pausing {
        p.abort();
    }
    let bound = mesh.local_addr().ok();
    mesh.shutdown().await;
    drop(mesh);
    if let Some(addr) = bound {
        wait_port_released(addr).await;
    }
    drop(control);
    drop(operation);
    served
}

/// The UDP socket belongs to quinn's endpoint driver, which stops only after the last handle to
/// the endpoint is gone, and the tasks that hold one (modules and connection tasks, aborted
/// above) let go of it after `shutdown` returns. So `run` returns only once the port binds
/// again: a restart, another agent or a test can take it at once (CI Windows, 601cd7f; here 3
/// stops in 30 still held it).
async fn wait_port_released(addr: SocketAddr) {
    let deadline = tokio::time::Instant::now() + PORT_RELEASE_WAIT;
    loop {
        match std::net::UdpSocket::bind(addr) {
            Ok(_) => return,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {}
            // Not ours to judge (address gone, permission): nothing to wait for.
            Err(_) => return,
        }
        if tokio::time::Instant::now() >= deadline {
            return tracing::warn!(%addr, "port still bound after the agent stopped");
        }
        tokio::time::sleep(PORT_RELEASE_POLL).await;
    }
}

/// Writes peers.toml without `peer` and reloads the mesh, like `ReloadPeers` after a local
/// revoke. A file that cannot be written still leaves the peer out of the running mesh.
async fn remove_revoked(profile: &Profile, mesh: &Mesh, peer: PeerId) {
    let saved = profile.update_peers(|peers| {
        peers.peer.retain(|e| e.id != peer);
        Ok::<_, CoreError>(peers.clone())
    });
    let peers = match saved {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "peers.toml not written; revocation lasts until restart");
            match profile.load_peers() {
                Ok(mut p) => {
                    p.peer.retain(|e| e.id != peer);
                    p
                }
                Err(_) => return,
            }
        }
    };
    mesh.reload(peers).await;
}

struct Agent {
    profile: Profile,
    device_name: String,
    mesh: Arc<Mesh>,
    channels: HashMap<ModuleKind, mpsc::Sender<ModuleInput>>,
    adapters: HashMap<ModuleKind, String>,
    /// Last `last_block` seen per module: a different one joins `recent_blocks`.
    seen_blocks: Mutex<HashMap<ModuleKind, String>>,
}

impl Agent {
    async fn ask_module(
        &self,
        kind: ModuleKind,
        cmd: ModuleCommand,
    ) -> Result<ModuleReply, String> {
        let Some(tx) = self.channels.get(&kind) else {
            return Ok(ModuleReply::Error(
                clipsync_proto::msg::MODULE_DISABLED.to_string(),
            ));
        };
        let wait = match &cmd {
            ModuleCommand::InputLoopback { seconds, .. } => {
                Duration::from_secs(u64::from(*seconds)) + LOOPBACK_SLACK
            }
            ModuleCommand::ClipSend { .. } | ModuleCommand::ClipSendCurrent { .. } => {
                CLIP_SEND_TIMEOUT
            }
            ModuleCommand::InputOffer { .. } | ModuleCommand::InputOfferV4 { .. } => {
                INPUT_OFFER_TIMEOUT
            }
            ModuleCommand::InputPrepareInventory => INPUT_PREPARE_TIMEOUT,
            ModuleCommand::Status
            | ModuleCommand::ClipInbox
            | ModuleCommand::ClipApply { .. }
            | ModuleCommand::ClipDiscard { .. }
            | ModuleCommand::InputState
            | ModuleCommand::InputReload
            | ModuleCommand::InputLock { .. }
            | ModuleCommand::InputSwitch { .. }
            | ModuleCommand::InputOfferClear
            | ModuleCommand::InputPreviewOrganization(_)
            | ModuleCommand::InputApplyOrganization(_)
            | ModuleCommand::InputRestoreLayoutV1 { .. }
            | ModuleCommand::InputSuspendOrganization
            | ModuleCommand::InputAnswerOfferV4 { .. } => STATUS_TIMEOUT,
        };
        let (reply, rx) = oneshot::channel();
        tx.send(ModuleInput::Command { cmd, reply })
            .await
            .map_err(|_| msg::MODULE_GONE.to_string())?;
        match timeout(wait, rx).await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(_)) => Err(msg::MODULE_GONE.to_string()),
            Err(_) => Err(msg::MODULE_TIMEOUT.to_string()),
        }
    }

    /// Names of the peers authorized for `kind`, in either direction.
    fn authorized(&self, kind: ModuleKind) -> Vec<String> {
        self.mesh
            .peer_statuses()
            .into_iter()
            .filter(|p| match kind {
                ModuleKind::Clipboard => p.perms.clipboard_in || p.perms.clipboard_out,
                ModuleKind::Input => p.perms.input_in,
            })
            .map(|p| p.name)
            .collect()
    }

    async fn module_status(&self, kind: ModuleKind) -> ModuleStatus {
        if !self.channels.contains_key(&kind) {
            return ModuleStatus {
                kind,
                enabled: false,
                adapter: None,
                authorized_peers: Vec::new(),
                last_event: None,
                last_block: None,
            };
        }
        match self.ask_module(kind, ModuleCommand::Status).await {
            Ok(ModuleReply::Status(s)) => {
                self.note_block(kind, s.last_block.as_deref());
                s
            }
            _ => ModuleStatus {
                kind,
                enabled: true,
                adapter: self.adapters.get(&kind).cloned(),
                authorized_peers: self.authorized(kind),
                last_event: None,
                last_block: Some(msg::MODULE_TIMEOUT.to_string()),
            },
        }
    }

    /// A module's `last_block` that differs from the one seen before is a new block. Only the
    /// latest one between two `Status` requests is seen (the UI asks every second).
    fn note_block(&self, kind: ModuleKind, last_block: Option<&str>) {
        let Some(reason) = last_block else { return };
        let mut seen = self
            .seen_blocks
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if seen.get(&kind).map(String::as_str) != Some(reason) {
            seen.insert(kind, reason.to_string());
            self.mesh.record_module_block(kind, reason.to_string());
        }
    }

    async fn status(&self) -> AgentStatus {
        let mut modules = Vec::new();
        for kind in [ModuleKind::Clipboard, ModuleKind::Input] {
            modules.push(self.module_status(kind).await);
        }
        AgentStatus {
            log_path: Some(
                crate::logging::log_path(&self.profile.dir)
                    .display()
                    .to_string(),
            ),
            device_name: self.device_name.clone(),
            id: self.mesh.local_id(),
            listen: self
                .mesh
                .local_addr()
                .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0))),
            paused: self.mesh.paused(),
            peers: self.mesh.peer_statuses(),
            modules,
            recent_blocks: self.mesh.recent_blocks(),
            recent_removals: self.mesh.recent_removals(),
        }
    }

    /// Fails closed (S1-08): the new state applies in memory first, then is persisted. If
    /// state.toml cannot be written, the agent keeps the new state (a pause stays a pause) and
    /// answers with an error saying it will not survive a restart.
    async fn set_paused(&self, paused: bool) -> ControlResponse {
        let _operation = if paused {
            None
        } else {
            match self.profile.reserve_operation(false) {
                Ok(guard) => Some(guard),
                Err(_) => {
                    return ControlResponse::Error {
                        message: "reconfiguração do perfil em andamento".into(),
                    };
                }
            }
        };
        self.mesh.set_paused(paused).await;
        match self.profile.set_paused(paused) {
            Ok(()) => ControlResponse::Ok,
            Err(e) => {
                tracing::error!(paused, error = %e, "{}", msg::STATE_NOT_SAVED);
                ControlResponse::Error {
                    message: msg::STATE_NOT_SAVED.to_string(),
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl ControlHandler for Agent {
    async fn handle(&self, req: ControlRequest) -> ControlResponse {
        match req {
            ControlRequest::Status => ControlResponse::Status(self.status().await),
            ControlRequest::Pause => self.set_paused(true).await,
            ControlRequest::Resume => self.set_paused(false).await,
            ControlRequest::ReloadPeers => match self.profile.load_peers() {
                Ok(peers) => {
                    self.mesh.reload(peers).await;
                    ControlResponse::Ok
                }
                Err(e) => ControlResponse::Error {
                    message: format!("{}: {e}", msg::PEERS_NOT_LOADED),
                },
            },
            ControlRequest::Module { kind, cmd } => {
                let _operation = if matches!(cmd, ModuleCommand::InputLock { .. }) {
                    match self.profile.reserve_operation(false) {
                        Ok(guard) => Some(guard),
                        Err(_) => {
                            return ControlResponse::Error {
                                message: "reconfiguração do perfil em andamento".into(),
                            };
                        }
                    }
                } else {
                    None
                };
                match self.ask_module(kind, cmd).await {
                    Ok(reply) => ControlResponse::Module(reply),
                    Err(message) => ControlResponse::Error { message },
                }
            }
        }
    }
}
