#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_core::{InitOptions, Profile, agent, control};
use clipsync_proto::{
    ControlRequest, ControlResponse, Module, ModuleCommand, ModuleHost, ModuleInput, ModuleKind,
    ModuleReply, ModuleSet, ModuleStatus,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

// O módulo controlável só fornece respostas; todos os comandos cruzam o socket
// e o dispatch real de Agent. O receiver retido permite observar seu fechamento.
struct InputControlavel(mpsc::UnboundedSender<(ModuleCommand, oneshot::Sender<ModuleReply>)>);
#[async_trait::async_trait]
impl Module for InputControlavel {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Input
    }
    fn adapter_name(&self) -> String {
        "controlavel".into()
    }
    async fn run(
        self: Box<Self>,
        _host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), clipsync_proto::ModuleError> {
        let mut bootstrap = true;
        while let Some(input) = rx.recv().await {
            match input {
                ModuleInput::Command {
                    cmd: ModuleCommand::Status,
                    reply,
                } if bootstrap => {
                    bootstrap = false;
                    let _ = reply.send(ModuleReply::Status(ModuleStatus {
                        kind: ModuleKind::Input,
                        enabled: true,
                        adapter: None,
                        authorized_peers: vec![],
                        last_event: None,
                        last_block: None,
                        control_returned_id: 0,
                    }));
                }
                ModuleInput::Command { cmd, reply } => {
                    let _ = self.0.send((cmd, reply));
                }
                ModuleInput::Shutdown => break,
                _ => {}
            }
        }
        Ok(())
    }
}

async fn request(p: Profile, cmd: ModuleCommand) -> ControlResponse {
    control::request(
        &p,
        ControlRequest::Module {
            kind: ModuleKind::Input,
            cmd,
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn preparacao_interativa_supera_status_e_fecha_reply_no_prazo() {
    let dir = tempfile::Builder::new().prefix("cs10-").tempdir().unwrap();
    let p = Profile::resolve(Some(dir.path().join("p"))).unwrap();
    p.init(InitOptions {
        device_name: "s10-dispatch".into(),
        modules: ModuleSet {
            input: true,
            clipboard: false,
        },
        listen: "127.0.0.1:0".parse().unwrap(),
        fake_adapters: true,
    })
    .unwrap();
    let startup = agent::Startup::reserve(&p).await.unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (ready, readiness) = oneshot::channel();
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(agent::run_reserved(
        startup,
        vec![Box::new(InputControlavel(tx))],
        None,
        Some(ready),
        async {
            let _ = stopped.await;
        },
    ));
    assert!(readiness.await.unwrap().is_ok());

    let prepare = tokio::spawn(request(p.clone(), ModuleCommand::InputPrepareInventory));
    let (_, reply) = rx.recv().await.unwrap();
    let status = tokio::spawn(request(p.clone(), ModuleCommand::Status));
    let (_, status_reply) = rx.recv().await.unwrap();
    assert!(
        matches!(status.await.unwrap(), ControlResponse::Error { message } if message == clipsync_core::msg::MODULE_TIMEOUT)
    );
    assert!(status_reply.is_closed());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !prepare.is_finished(),
        "preparação interativa não deve expirar com Status"
    );
    assert!(reply.send(ModuleReply::Done).is_ok());
    assert!(matches!(
        prepare.await.unwrap(),
        ControlResponse::Module(ModuleReply::Done)
    ));

    let prepare = tokio::spawn(request(p.clone(), ModuleCommand::InputPrepareInventory));
    let (_, mut reply) = rx.recv().await.unwrap();
    let started = tokio::time::Instant::now();
    reply.closed().await;
    assert!(started.elapsed() >= Duration::from_secs(109));
    assert!(started.elapsed() < Duration::from_secs(115));
    assert!(
        matches!(prepare.await.unwrap(), ControlResponse::Error { message } if message == clipsync_core::msg::MODULE_TIMEOUT)
    );
    assert!(
        reply.send(ModuleReply::Done).is_err(),
        "timeout recusa publicação tardia"
    );

    let prepare = tokio::spawn(request(p, ModuleCommand::InputPrepareInventory));
    let (_, reply) = rx.recv().await.unwrap();
    drop(reply); // Cancelamento pelo módulo (autoridade revogada/pausa).
    assert!(
        matches!(prepare.await.unwrap(), ControlResponse::Error { message } if message == clipsync_core::msg::MODULE_GONE)
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}
