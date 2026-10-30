#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_core::{InitOptions, Profile, agent, control, pairing};
use clipsync_proto::{ControlRequest, ControlResponse, ModuleCommand, ModuleKind, ModuleSet};
use std::time::Duration;

fn fixture() -> (tempfile::TempDir, Profile) {
    let d = tempfile::Builder::new().prefix("csr-").tempdir().unwrap();
    let p = Profile::resolve(Some(d.path().join("p"))).unwrap();
    p.init(InitOptions {
        device_name: "runtime-fake".into(),
        modules: ModuleSet {
            input: false,
            clipboard: true,
        },
        listen: "127.0.0.1:0".parse().unwrap(),
        fake_adapters: true,
    })
    .unwrap();
    (d, p)
}

#[tokio::test]
async fn reserva_recusa_startup_antes_de_publicar_socket() {
    let (_d, p) = fixture();
    let _guard = p.reserve_operation(true).unwrap();
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        agent::run(&p, vec![], std::future::pending()),
    )
    .await;
    assert!(matches!(result, Ok(Err(_))));
    assert!(!p.socket_path().exists());
}

#[tokio::test]
async fn reserva_recusa_host_e_join_sem_abrir_tentativa() {
    let (_d, p) = fixture();
    let _guard = p.reserve_operation(true).unwrap();
    let host = tokio::time::timeout(
        Duration::from_millis(100),
        pairing::host(&p, 0, Duration::from_secs(1), |_| {}, |_| unreachable!()),
    )
    .await;
    assert!(matches!(host, Ok(Err(_))));
    let join = tokio::time::timeout(
        Duration::from_millis(100),
        pairing::join(&p, "127.0.0.1:9".parse().unwrap(), |_| unreachable!()),
    )
    .await;
    assert!(matches!(join, Ok(Err(_))));
}

#[tokio::test]
async fn reserva_permite_pause_mas_recusa_resume_e_inputlock() {
    let (_d, p) = fixture();
    let (stop, rx) = tokio::sync::oneshot::channel();
    let q = p.clone();
    let task = tokio::spawn(async move {
        agent::run(&q, vec![], async {
            let _ = rx.await;
        })
        .await
    });
    for _ in 0..100 {
        if matches!(
            control::request(&p, ControlRequest::Status).await,
            Ok(ControlResponse::Status(_))
        ) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let _guard = p.reserve_operation(true).unwrap();
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await.unwrap(),
        ControlResponse::Ok
    ));
    assert!(matches!(
        control::request(&p, ControlRequest::Resume).await.unwrap(),
        ControlResponse::Error { .. }
    ));
    assert!(p.load_state().unwrap().paused);
    let reply = control::request(
        &p,
        ControlRequest::Module {
            kind: ModuleKind::Input,
            cmd: ModuleCommand::InputLock { locked: true },
        },
    )
    .await
    .unwrap();
    assert!(matches!(reply, ControlResponse::Error { .. }));
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn listener_antigo_nao_remove_socket_de_outro() {
    let (_d, p) = fixture();
    let first = control::bind(&p.socket_path()).await.unwrap();
    std::fs::remove_file(p.socket_path()).unwrap();
    let second = control::bind(&p.socket_path()).await.unwrap();
    drop(first);
    assert!(p.socket_path().exists());
    drop(second);
    assert!(!p.socket_path().exists());
}

struct BootstrapFailure {
    finish: std::sync::Arc<tokio::sync::Notify>,
    stopping: tokio::sync::oneshot::Sender<()>,
}
#[async_trait::async_trait]
impl clipsync_proto::Module for BootstrapFailure {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Input
    }
    fn adapter_name(&self) -> String {
        "fake-bootstrap".into()
    }
    async fn run(
        self: Box<Self>,
        _host: std::sync::Arc<dyn clipsync_proto::ModuleHost>,
        mut rx: tokio::sync::mpsc::Receiver<clipsync_proto::ModuleInput>,
    ) -> Result<(), clipsync_proto::ModuleError> {
        let mut stopping = Some(self.stopping);
        while let Some(input) = rx.recv().await {
            match input {
                clipsync_proto::ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(clipsync_proto::ModuleReply::Error(
                        "falha fake de bootstrap".into(),
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
    }
}

#[tokio::test]
async fn bootstrap_falho_nao_libera_reserva_antes_da_tarefa_terminar() {
    let (_d, p) = fixture();
    let operation = p.reserve_operation(true).unwrap();
    let mut edit = p
        .begin_config_edit(
            &operation,
            ModuleSet {
                input: false,
                clipboard: true,
            },
        )
        .unwrap();
    edit.set_modules(ModuleSet {
        input: true,
        clipboard: false,
    })
    .unwrap();
    drop(edit);
    p.set_paused(true).unwrap();
    let listener = control::bind(&p.socket_path()).await.unwrap();
    let startup = agent::Startup::with_listener(&p, operation, listener).unwrap();
    let finish = std::sync::Arc::new(tokio::sync::Notify::new());
    let (stopping, stopped) = tokio::sync::oneshot::channel();
    let (ready, readiness) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(agent::run_reserved(
        startup,
        vec![Box::new(BootstrapFailure {
            finish: finish.clone(),
            stopping,
        })],
        Some(true),
        Some(ready),
        std::future::pending(),
    ));
    assert!(readiness.await.unwrap().is_err());
    stopped.await.unwrap();
    assert!(!task.is_finished());
    assert!(p.reserve_operation(true).is_err());
    assert!(p.load_state().unwrap().paused);
    finish.notify_one();
    assert!(task.await.unwrap().is_err());
    assert!(p.reserve_operation(true).is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn startup_recusa_publicacao_substituida() {
    let (_d, p) = fixture();
    let op = p.reserve_operation(true).unwrap();
    let old = control::bind(&p.socket_path()).await.unwrap();
    std::fs::remove_file(p.socket_path()).unwrap();
    let current = control::bind(&p.socket_path()).await.unwrap();
    assert!(agent::Startup::with_listener(&p, op, old).is_err());
    assert!(current.is_current());
}

#[tokio::test]
async fn startup_recusa_listener_de_outro_perfil() {
    let (_a, p) = fixture();
    let (_b, q) = fixture();
    let op = p.reserve_operation(true).unwrap();
    let other = control::bind(&q.socket_path()).await.unwrap();
    assert!(agent::Startup::with_listener(&p, op, other).is_err());
}

#[tokio::test]
async fn startup_reservado_impede_init_e_modulos_ate_readiness() {
    let (_d, p) = fixture();
    let startup = agent::Startup::reserve(&p).await.unwrap();
    assert!(p.reserve_operation(true).is_err());
    assert!(
        p.init(InitOptions {
            device_name: "nao-gravar".into(),
            modules: ModuleSet {
                input: false,
                clipboard: true
            },
            listen: "127.0.0.1:0".parse().unwrap(),
            fake_adapters: true
        })
        .is_err()
    );
    assert_eq!(startup.config().device_name, "runtime-fake");
    drop(startup);
    assert!(p.reserve_operation(true).is_ok());
    assert!(!p.socket_path().exists());
}

struct SlowShutdown {
    finish: std::sync::Arc<tokio::sync::Notify>,
    stopping: tokio::sync::oneshot::Sender<()>,
}
#[async_trait::async_trait]
impl clipsync_proto::Module for SlowShutdown {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Clipboard
    }
    fn adapter_name(&self) -> String {
        "fake-slow-stop".into()
    }
    async fn run(
        self: Box<Self>,
        _host: std::sync::Arc<dyn clipsync_proto::ModuleHost>,
        mut rx: tokio::sync::mpsc::Receiver<clipsync_proto::ModuleInput>,
    ) -> Result<(), clipsync_proto::ModuleError> {
        let mut stopping = Some(self.stopping);
        while let Some(input) = rx.recv().await {
            match input {
                clipsync_proto::ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(clipsync_proto::ModuleReply::Status(
                        clipsync_proto::ModuleStatus {
                            kind: ModuleKind::Clipboard,
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
    }
}

#[tokio::test]
async fn parada_lenta_mantem_endpoint_ate_terminar_modulos() {
    let (_d, p) = fixture();
    let startup = agent::Startup::reserve(&p).await.unwrap();
    let finish = std::sync::Arc::new(tokio::sync::Notify::new());
    let (stopping, stopped) = tokio::sync::oneshot::channel();
    let (ready, readiness) = tokio::sync::oneshot::channel();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(agent::run_reserved(
        startup,
        vec![Box::new(SlowShutdown {
            finish: finish.clone(),
            stopping,
        })],
        None,
        Some(ready),
        async {
            let _ = shutdown.await;
        },
    ));
    assert!(readiness.await.unwrap().is_ok());
    stop.send(()).unwrap();
    stopped.await.unwrap();
    assert!(!task.is_finished());
    let second = control::bind(&p.socket_path()).await;
    finish.notify_one();
    task.await.unwrap().unwrap();
    assert!(matches!(
        second,
        Err(clipsync_core::CoreError::AgentAlreadyRunning)
    ));
    assert!(control::bind(&p.socket_path()).await.is_ok());
}

struct LateBootstrap {
    ready_to_reply: std::sync::Arc<tokio::sync::Notify>,
    entered: tokio::sync::oneshot::Sender<()>,
    finish: std::sync::Arc<tokio::sync::Notify>,
    stopping: tokio::sync::oneshot::Sender<()>,
}
#[async_trait::async_trait]
impl clipsync_proto::Module for LateBootstrap {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Clipboard
    }
    fn adapter_name(&self) -> String {
        "fake-late-bootstrap".into()
    }
    async fn run(
        self: Box<Self>,
        _host: std::sync::Arc<dyn clipsync_proto::ModuleHost>,
        mut rx: tokio::sync::mpsc::Receiver<clipsync_proto::ModuleInput>,
    ) -> Result<(), clipsync_proto::ModuleError> {
        let mut entered = Some(self.entered);
        let mut stopping = Some(self.stopping);
        while let Some(input) = rx.recv().await {
            match input {
                clipsync_proto::ModuleInput::Command { reply, .. } => {
                    if let Some(tx) = entered.take() {
                        let _ = tx.send(());
                    }
                    self.ready_to_reply.notified().await;
                    let _ = reply.send(clipsync_proto::ModuleReply::Status(
                        clipsync_proto::ModuleStatus {
                            kind: ModuleKind::Clipboard,
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
    }
}

#[tokio::test]
async fn readiness_expirada_mantem_reserva_no_sucesso_tardio_e_cleanup() {
    let (_d, p) = fixture();
    let startup = agent::Startup::reserve(&p).await.unwrap();
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let finish = std::sync::Arc::new(tokio::sync::Notify::new());
    let (entered, admitted) = tokio::sync::oneshot::channel();
    let (stopping, stopped) = tokio::sync::oneshot::channel();
    let (ready, readiness) = tokio::sync::oneshot::channel();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(agent::run_reserved(
        startup,
        vec![Box::new(LateBootstrap {
            ready_to_reply: release.clone(),
            entered,
            finish: finish.clone(),
            stopping,
        })],
        None,
        Some(ready),
        async {
            let _ = shutdown.await;
        },
    ));
    admitted.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(20), readiness)
            .await
            .is_err()
    );
    stop.send(()).unwrap();
    assert!(p.reserve_operation(true).is_err());
    release.notify_one();
    stopped.await.unwrap();
    let retained = p.reserve_operation(true).is_err();
    assert!(!task.is_finished());
    finish.notify_one();
    let result = task.await.unwrap();
    assert!(
        retained,
        "sucesso tardio não pode liberar reserva antes do join"
    );
    assert!(
        result.is_err(),
        "readiness cancelada não é startup concluído"
    );
    assert!(p.reserve_operation(true).is_ok());
}
