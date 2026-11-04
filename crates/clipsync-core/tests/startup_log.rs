#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_core::{InitOptions, Profile, agent};
use clipsync_proto::{
    Module, ModuleError, ModuleHost, ModuleInput, ModuleKind, ModuleReply, ModuleSet,
};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, layer::Context, prelude::*};

type Events = Arc<(Mutex<Vec<String>>, Condvar)>;
struct Order(Events);
#[derive(Default)]
struct Name(String);
impl Visit for Name {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "evento" {
            self.0 = value.into();
        }
    }
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
}
impl<S: Subscriber> Layer<S> for Order {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut name = Name::default();
        event.record(&mut name);
        if !matches!(name.0.as_str(), "agent.up" | "inventory.prepared") {
            return;
        }
        let (events, ready) = &*self.0;
        let mut events = events.lock().unwrap();
        if name.0 == "agent.up" {
            // Deixa o produtor disputar a escrita se já tiver sido iniciado.
            // Com o marcador antes dos runners, nenhum produtor existe nesta janela.
            events = ready
                .wait_timeout_while(events, Duration::from_millis(200), |events| {
                    events.is_empty()
                })
                .unwrap()
                .0;
        }
        events.push(name.0);
        ready.notify_all();
    }
}
struct Producer;
#[async_trait::async_trait]
impl Module for Producer {
    fn kind(&self) -> ModuleKind {
        ModuleKind::Input
    }
    fn adapter_name(&self) -> String {
        "produtor-de-teste".into()
    }
    async fn run(
        self: Box<Self>,
        _host: Arc<dyn ModuleHost>,
        mut rx: mpsc::Receiver<ModuleInput>,
    ) -> Result<(), ModuleError> {
        tracing::info!(evento = "inventory.prepared", monitores = 1);
        while let Some(input) = rx.recv().await {
            match input {
                ModuleInput::Command { reply, .. } => {
                    let _ = reply.send(ModuleReply::Error("fim do cenário".into()));
                }
                ModuleInput::Shutdown => break,
                _ => {}
            }
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn marcador_da_execucao_precede_qualquer_produtor() {
    let events: Events = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(Order(events.clone())),
    )
    .unwrap();
    let dir = tempfile::Builder::new().prefix("csup-").tempdir().unwrap();
    let profile = Profile::resolve(Some(dir.path().join("p"))).unwrap();
    profile
        .init(InitOptions {
            device_name: "ordem-log".into(),
            modules: ModuleSet {
                input: true,
                clipboard: false,
            },
            listen: "127.0.0.1:0".parse().unwrap(),
            fake_adapters: true,
        })
        .unwrap();
    assert!(
        agent::run(&profile, vec![Box::new(Producer)], std::future::pending())
            .await
            .is_err()
    );
    assert_eq!(
        *events.0.lock().unwrap(),
        ["agent.up", "inventory.prepared"]
    );
}
