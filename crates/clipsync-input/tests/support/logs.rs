//! Harness de eventos do contrato §17 para os testes de `clipsync-input`.
//!
//! A camada não filtra: todo evento com o campo `evento` é guardado, inclusive os de produção,
//! para o teste afirmar o que o arquivo de log receberia. Os campos saem como texto, porque é o
//! que o escritor de arquivo grava; listas chegam como o JSON que a emissão mandou.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

/// Uma linha do log: o nome do evento e os campos, sem o `evento` repetido.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evento {
    pub evento: String,
    pub campos: BTreeMap<String, String>,
}

impl Evento {
    pub fn campo(&self, nome: &str) -> Option<&str> {
        self.campos.get(nome).map(String::as_str)
    }
}

/// Assinante de teste. Mantém o `DefaultGuard` vivo enquanto o teste roda, para o `Dispatch`
/// valer nas tarefas do `MemHub` (por isso o `#[tokio::test]` current_thread).
pub struct Captura {
    pub eventos: Arc<Mutex<Vec<Evento>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

struct Camada {
    eventos: Arc<Mutex<Vec<Evento>>>,
}

#[derive(Default)]
struct Campos {
    evento: Option<String>,
    campos: BTreeMap<String, String>,
}

pub fn capturar() -> Captura {
    let eventos = Arc::new(Mutex::new(Vec::new()));
    let camada = Camada {
        eventos: eventos.clone(),
    };
    let assinante = tracing_subscriber::registry().with(camada);
    let guard = tracing::subscriber::set_default(assinante);
    Captura {
        eventos,
        _guard: guard,
    }
}

impl Captura {
    pub fn tem(&self, evento: &str) -> bool {
        self.eventos().iter().any(|e| e.evento == evento)
    }

    pub fn primeiro(&self, evento: &str) -> Option<Evento> {
        self.eventos().iter().find(|e| e.evento == evento).cloned()
    }

    pub fn todos(&self, evento: &str) -> Vec<Evento> {
        self.eventos()
            .iter()
            .filter(|e| e.evento == evento)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    pub fn limpar(&self) {
        self.eventos().clear();
    }

    fn eventos(&self) -> std::sync::MutexGuard<'_, Vec<Evento>> {
        self.eventos.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl<S: tracing::Subscriber> Layer<S> for Camada {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut campos = Campos::default();
        event.record(&mut campos);
        let Some(evento) = campos.evento else {
            return;
        };
        self.eventos
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Evento {
                evento,
                campos: campos.campos,
            });
    }
}

impl Visit for Campos {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "evento" {
            self.evento = Some(value.to_owned());
        } else {
            self.campos
                .insert(field.name().to_owned(), value.to_owned());
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.campos
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.campos
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.campos
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.campos
            .insert(field.name().to_owned(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let texto = format!("{value:?}");
        if field.name() == "evento" {
            self.evento = Some(texto.trim_matches('"').to_owned());
        } else {
            self.campos.insert(field.name().to_owned(), texto);
        }
    }
}
