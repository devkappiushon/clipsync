//! The agent (`clipsync run`): mesh + modules + control socket. Owned by Rivet.
//!
//! A module the config turns off is never started, never announced and refused at the door.
//! Pause is persisted in state.toml and applied by the mesh through `authorize_*`.

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clipsync_proto::{
    AgentStatus, ControlRequest, ControlResponse, Module, ModuleCommand, ModuleInput, ModuleKind,
    ModuleReply, ModuleStatus,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::control::{self, ControlHandler};
use crate::transport::{Mesh, MeshOptions};
use crate::{CoreError, Profile, State, msg};

const MODULE_QUEUE: usize = 1024;
const MODULE_STOP_GRACE: Duration = Duration::from_secs(3);
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const CLIP_SEND_TIMEOUT: Duration = Duration::from_secs(15);
const LOOPBACK_SLACK: Duration = Duration::from_secs(30);

/// Runs until `shutdown` completes: then modules get `Shutdown`, peers get Bye and the
/// connections close with CloseCode::Shutdown.
pub async fn run(
    p: &Profile,
    modules: Vec<Box<dyn Module>>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    let cfg = p.load_config()?;
    let identity = p.load_identity()?;
    let peers = p.load_peers()?;
    let state = p.load_state()?;
    // Fail before binding the QUIC port if another agent owns this profile.
    let socket = p.socket_path();
    if control::agent_alive(&socket).await {
        return Err(CoreError::AgentAlreadyRunning);
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
    let host = mesh.host();
    let mut tasks = JoinSet::new();
    for (m, rx) in runners {
        let host = host.clone();
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
    tracing::info!(id = %mesh.local_id(), addr = ?mesh.local_addr().ok(), "agent up");

    let handler = Arc::new(Agent {
        profile: p.clone(),
        device_name: cfg.device_name,
        mesh: mesh.clone(),
        channels: channels.clone(),
        adapters,
    });
    let served = control::serve(&socket, handler, shutdown).await;

    for tx in channels.values() {
        let _ = tx.send(ModuleInput::Shutdown).await;
    }
    drop(channels);
    let _ = timeout(MODULE_STOP_GRACE, async {
        while tasks.join_next().await.is_some() {}
    })
    .await;
    tasks.abort_all();
    mesh.shutdown().await;
    served
}

struct Agent {
    profile: Profile,
    device_name: String,
    mesh: Arc<Mesh>,
    channels: HashMap<ModuleKind, mpsc::Sender<ModuleInput>>,
    adapters: HashMap<ModuleKind, String>,
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
            ModuleCommand::ClipSend { .. } => CLIP_SEND_TIMEOUT,
            ModuleCommand::Status | ModuleCommand::ClipInbox => STATUS_TIMEOUT,
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
            Ok(ModuleReply::Status(s)) => s,
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

    async fn status(&self) -> AgentStatus {
        let mut modules = Vec::new();
        for kind in [ModuleKind::Clipboard, ModuleKind::Input] {
            modules.push(self.module_status(kind).await);
        }
        AgentStatus {
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
        }
    }

    /// Fails closed (S1-08): the new state applies in memory first, then is persisted. If
    /// state.toml cannot be written, the agent keeps the new state (a pause stays a pause) and
    /// answers with an error saying it will not survive a restart.
    async fn set_paused(&self, paused: bool) -> ControlResponse {
        self.mesh.set_paused(paused).await;
        match self.profile.save_state(&State { version: 1, paused }) {
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
            ControlRequest::Module { kind, cmd } => match self.ask_module(kind, cmd).await {
                Ok(reply) => ControlResponse::Module(reply),
                Err(message) => ControlResponse::Error { message },
            },
        }
    }
}
