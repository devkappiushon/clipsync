#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_core::{InitOptions, Profile, agent, logging};
use clipsync_proto::{
    Module, ModuleError, ModuleHost, ModuleInput, ModuleKind, ModuleReply, ModuleSet,
};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

fn child(mode: &str) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "cenario_isolado", "--nocapture"])
        .env("CLIPSYNC_TEST_LOG_MODE", mode)
        .env("CLIPSYNC_LOG", "off")
        .env_remove("CLIPSYNC_LOG_ARQUIVO");
    cmd
}
fn scenario(mode: &str) {
    let result = child(mode).output().unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn parada_fecha_log_e_permite_reinicio() {
    scenario("parada");
}
#[test]
fn falha_de_bootstrap_fecha_log() {
    scenario("bootstrap");
}
#[test]
fn falha_antes_da_readiness_fecha_log() {
    scenario("mesh");
}
#[test]
fn cancelamento_fecha_log() {
    scenario("cancelamento");
}
#[test]
fn tomada_externa_nao_recebe_eventos_do_escritor_antigo() {
    scenario("externo");
}

struct FailingModule;
#[async_trait::async_trait]
impl Module for FailingModule {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Input
    }
    fn adapter_name(&self) -> String {
        "falha-controlada".into()
    }
    async fn run(
        self: Box<Self>,
        _host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        while let Some(event) = rx.recv().await {
            match event {
                ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(ModuleReply::Error("bootstrap recusado".into()));
                }
                ModuleInput::Shutdown => break,
                _ => {}
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn cenario_isolado() {
    let Ok(mode) = std::env::var("CLIPSYNC_TEST_LOG_MODE") else {
        return;
    };
    logging::init();
    if mode == "tomar" {
        let profile = Profile::resolve(Some(
            std::env::var_os("CLIPSYNC_TEST_LOG_PROFILE")
                .unwrap()
                .into(),
        ))
        .unwrap();
        let _startup = agent::Startup::reserve(&profile).await.unwrap();
        let _log = logging::open_file(&profile.dir, "externo").unwrap();
        println!("PERFIL_ADQUIRIDO");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        tracing::info!(target: "clipsync_core", evento = "externo.escreveu");
        return;
    }
    let temp = tempfile::Builder::new().prefix("cslog-").tempdir().unwrap();
    let profile = Profile::resolve(Some(temp.path().join("p"))).unwrap();
    let occupied = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    profile
        .init(InitOptions {
            device_name: "ciclo-log".into(),
            modules: ModuleSet {
                input: true,
                clipboard: false,
            },
            listen: if mode == "mesh" {
                occupied.local_addr().unwrap()
            } else {
                "127.0.0.1:0".parse().unwrap()
            },
            fake_adapters: true,
        })
        .unwrap();
    let path = logging::log_path(&profile.dir);
    if mode == "mesh" {
        assert!(agent::run(&profile, vec![], async {}).await.is_err());
    } else {
        let startup = agent::Startup::reserve(&profile).await.unwrap();
        let (ready, readiness) = oneshot::channel();
        let (stop, stopped) = oneshot::channel();
        let modules: Vec<Box<dyn Module>> = if mode == "bootstrap" {
            vec![Box::new(FailingModule)]
        } else {
            vec![]
        };
        let task = tokio::spawn(agent::run_reserved(
            startup,
            modules,
            None,
            Some(ready),
            async {
                let _ = stopped.await;
            },
        ));
        let started = tokio::time::timeout(Duration::from_secs(5), readiness)
            .await
            .unwrap()
            .unwrap();
        if mode == "bootstrap" {
            assert!(started.is_err());
            assert!(task.await.unwrap().is_err());
        } else {
            assert!(started.is_ok());
            tracing::info!(target: "clipsync_ui", evento = "ui.durante");
            assert!(
                std::fs::read_to_string(&path)
                    .unwrap()
                    .contains("ui.durante")
            );
            if mode == "cancelamento" {
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
            } else {
                stop.send(()).unwrap();
                task.await.unwrap().unwrap();
            }
        }
    }
    assert!(!profile.socket_path().exists());
    let before = std::fs::read(&path).unwrap();
    let dropped = logging::preopen_dropped();
    if mode == "externo" {
        let mut process = child("tomar")
            .env("CLIPSYNC_TEST_LOG_PROFILE", &profile.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(process.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert!(output.read_line(&mut line).unwrap() > 0);
            if line.contains("PERFIL_ADQUIRIDO") {
                break;
            }
        }
        tracing::warn!(target: "clipsync_ui", evento = "ui.depois");
        let unchanged = std::fs::read(&path).unwrap() == before;
        process.stdin.take().unwrap().write_all(b"sair\n").unwrap();
        assert!(process.wait().unwrap().success());
        assert!(
            unchanged,
            "o processo antigo escreveu após a tomada externa"
        );
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("externo.escreveu")
        );
    } else {
        let reservation = agent::Startup::reserve(&profile).await.unwrap();
        tracing::warn!(target: "clipsync_ui", evento = "ui.depois");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "o escritor sobreviveu ao agente"
        );
        drop(reservation);
    }
    assert_eq!(logging::preopen_dropped(), (dropped + 1).min(200));
    if mode == "parada" {
        agent::run(&profile, vec![], async {}).await.unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > before.len() as u64);
        let restarted = std::fs::read(&path).unwrap();
        tracing::warn!(target: "clipsync_ui", evento = "ui.depois_do_reinicio");
        assert_eq!(std::fs::read(&path).unwrap(), restarted);
    }
}
