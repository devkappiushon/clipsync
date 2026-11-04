//! Log JSONL do agente, com caminho puro e rotação por tamanho.

use crate::CoreError;
use serde_json::{Map, Value};
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::{EnvFilter, Layer, layer::Context, prelude::*};

const LIMIT: u64 = 5 * 1024 * 1024;
const FILE_FILTER: &str = "warn,clipsync_core=info,clipsync_input=info,clipsync_clipboard=info,clipsync_proto=info,clipsync_ui=info,clipsync_cli=info,clipsync_app=info";
static STATE: OnceLock<Arc<FileState>> = OnceLock::new();

fn state() -> &'static Arc<FileState> {
    STATE.get_or_init(|| Arc::new(FileState::default()))
}

/// Caminho previsto, inclusive quando o perfil ou o agente ainda não existem.
pub fn log_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join("logs").join("clipsync.jsonl")
}

/// Instala stderr e arquivo com filtros independentes. Pode ser chamada mais de uma vez.
pub fn init() {
    let stderr = EnvFilter::try_from_env("CLIPSYNC_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let file = EnvFilter::try_from_env("CLIPSYNC_LOG_ARQUIVO")
        .unwrap_or_else(|_| EnvFilter::new(FILE_FILTER));
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(io::stderr)
                .with_filter(stderr),
        )
        .with(FileLayer(Arc::clone(state())).with_filter(file))
        .try_init();
}

/// Abre o arquivo uma única vez, depois da reserva exclusiva do agente no perfil.
pub fn open_file(profile_dir: &Path, device_name: &str) -> Result<(), CoreError> {
    let mut guard = state()
        .sink
        .lock()
        .map_err(|_| io::Error::other("mutex do log indisponível"))?;
    if guard.is_none() {
        *guard = Some(Sink::open(profile_dir, device_name, LIMIT)?);
    }
    Ok(())
}

/// Quantidade limitada de eventos descartados antes da abertura do arquivo.
pub fn preopen_dropped() -> u64 {
    state().dropped.load(Ordering::Relaxed)
}

#[derive(Default)]
struct FileState {
    sink: Mutex<Option<Sink>>,
    dropped: AtomicU64,
    write_warned: AtomicBool,
}

impl FileState {
    fn write_event(&self, mut fields: Map<String, Value>) {
        let Ok(mut guard) = self.sink.lock() else {
            return;
        };
        let Some(sink) = guard.as_mut() else {
            let _ = self
                .dropped
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                    Some((n + 1).min(200))
                });
            return;
        };
        fields.insert("host".into(), Value::String(sink.host.clone()));
        let result = serde_json::to_string(&fields)
            .map_err(io::Error::other)
            .and_then(|line| sink.write_line(&line));
        if result.is_err() && !self.write_warned.swap(true, Ordering::Relaxed) {
            // Não reentra no tracing enquanto o mutex do arquivo está adquirido.
            eprintln!("warn: não foi possível escrever o arquivo de log");
        }
    }
}

struct Sink {
    file: File,
    path: PathBuf,
    host: String,
    size: u64,
    limit: u64,
    rotation_warned: bool,
}

impl Sink {
    fn open(dir: &Path, host: &str, limit: u64) -> io::Result<Self> {
        let mut builder = DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(dir.join("logs"))?;
        let path = log_path(dir);
        let file = Self::append(&path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            file,
            path,
            host: host.to_owned(),
            size,
            limit,
            rotation_warned: false,
        })
    }

    fn append(path: &Path) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)
    }

    fn rotate(&mut self) -> io::Result<()> {
        let first = self.path.with_file_name("clipsync.jsonl.1");
        let second = self.path.with_file_name("clipsync.jsonl.2");
        match std::fs::remove_file(&second) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        match std::fs::rename(&first, &second) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        std::fs::rename(&self.path, &first)?;
        match Self::append(&self.path) {
            Ok(file) => {
                self.file = file;
                self.size = 0;
                Ok(())
            }
            Err(e) => {
                // O handle anterior continua válido mesmo se a criação ou o rollback falhar.
                let _ = std::fs::rename(&first, &self.path);
                Err(e)
            }
        }
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        if self.size > 0
            && self.size.saturating_add(bytes.len() as u64) > self.limit
            && self.rotate().is_err()
            && !self.rotation_warned
        {
            self.rotation_warned = true;
            eprintln!("warn: rotação do log falhou; mantendo o arquivo atual");
        }
        self.file.write_all(&bytes)?;
        self.file.flush()?;
        self.size = self.size.saturating_add(bytes.len() as u64);
        Ok(())
    }
}

struct FileLayer(Arc<FileState>);

impl<S: Subscriber> Layer<S> for FileLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let metadata = event.metadata();
        fields.0.insert("v".into(), Value::from(1));
        fields.0.insert("ts".into(), Value::String(timestamp()));
        fields.0.insert(
            "nivel".into(),
            Value::String(metadata.level().as_str().to_ascii_lowercase()),
        );
        fields
            .0
            .insert("alvo".into(), Value::String(metadata.target().into()));
        fields
            .0
            .entry("evento")
            .or_insert_with(|| Value::String(String::new()));
        self.0.write_event(fields.0);
    }
}

fn timestamp() -> String {
    static FORMAT: OnceLock<Option<Vec<time::format_description::BorrowedFormatItem<'static>>>> =
        OnceLock::new();
    let format = FORMAT.get_or_init(|| {
        time::format_description::parse_borrowed::<2>(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
        )
        .ok()
    });
    format
        .as_ref()
        .and_then(|f| time::OffsetDateTime::now_utc().format(f).ok())
        .unwrap_or_default()
}

#[derive(Default)]
struct Fields(Map<String, Value>);

impl Fields {
    fn insert(&mut self, field: &Field, value: Value) {
        let name = if field.name() == "message" {
            "mensagem"
        } else {
            field.name()
        };
        // A mensagem de sessão explícita descreve a variante do fio, não o texto livre.
        if field.name() == "message" && self.0.contains_key("mensagem") {
            return;
        }
        self.0.insert(name.to_owned(), value);
    }

    fn string(&mut self, field: &Field, value: &str) {
        let value = if matches!(
            field.name(),
            "barreiras" | "uids" | "pares" | "totais" | "pos" | "trecho"
        ) {
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.into()))
        } else {
            Value::String(value.into())
        };
        self.insert(field, value);
    }
}

impl Visit for Fields {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, value.into());
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, value.into());
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.insert(field, value.into());
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, value.into());
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.string(field, value);
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.string(field, &format!("{value:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn opened(dir: &Path, limit: u64) -> Result<Arc<FileState>, Box<dyn Error>> {
        let state = Arc::new(FileState::default());
        *state.sink.lock().map_err(|_| "mutex")? = Some(Sink::open(dir, "notebook", limit)?);
        Ok(state)
    }

    #[test]
    fn caminho_puro_nao_cria() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        assert_eq!(log_path(tmp.path()), tmp.path().join("logs/clipsync.jsonl"));
        assert!(!tmp.path().join("logs").exists());
        Ok(())
    }

    #[test]
    fn abre_0600_e_uma_linha_por_evento() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let state = opened(tmp.path(), 5 * 1024 * 1024)?;
        let subscriber = tracing_subscriber::registry().with(FileLayer(state));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(evento = "agent.up", descartados = 0_u64, "agente\npronto");
            tracing::warn!(evento = "peer.disconnected", peer = "aa31-537a", "par saiu");
        });
        let data = std::fs::read_to_string(log_path(tmp.path()))?;
        assert!(data.ends_with('\n'));
        assert_eq!(data.lines().count(), 2);
        for line in data.lines() {
            let value: serde_json::Value = serde_json::from_str(line)?;
            assert_eq!(value["v"], 1);
            assert_eq!(value["host"], "notebook");
            let ts = value["ts"].as_str().ok_or("ts ausente")?;
            assert_eq!(ts.len(), 24);
            assert_eq!(&ts[19..20], ".");
            assert!(ts.ends_with('Z'));
            assert!(value["nivel"] == "info" || value["nivel"] == "warn");
            assert!(
                value["alvo"]
                    .as_str()
                    .is_some_and(|v| v.contains("logging"))
            );
            assert!(value["evento"].as_str().is_some());
            assert!(value["mensagem"].as_str().is_some());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(log_path(tmp.path()))?
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(tmp.path().join("logs"))?
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        Ok(())
    }

    #[test]
    fn rotaciona_em_5_mib() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let mut sink = Sink::open(tmp.path(), "notebook", 8)?;
        sink.write_line("1234567")?;
        sink.write_line("segunda")?;
        assert_eq!(std::fs::read_to_string(log_path(tmp.path()))?, "segunda\n");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("logs/clipsync.jsonl.1"))?,
            "1234567\n"
        );
        sink.write_line("terceira")?;
        assert_eq!(std::fs::read_to_string(log_path(tmp.path()))?, "terceira\n");
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("logs/clipsync.jsonl.1"))?,
            "segunda\n"
        );
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("logs/clipsync.jsonl.2"))?,
            "1234567\n"
        );
        Ok(())
    }

    #[test]
    fn rotacao_falha_preserva_linha_corrente() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let mut sink = Sink::open(tmp.path(), "notebook", 8)?;
        sink.write_line("primeira")?;
        std::fs::create_dir(tmp.path().join("logs/clipsync.jsonl.2"))?;
        sink.write_line("segunda")?;
        sink.write_line("terceira")?;
        assert_eq!(
            std::fs::read_to_string(log_path(tmp.path()))?,
            "primeira\nsegunda\nterceira\n"
        );
        Ok(())
    }

    #[test]
    fn descarta_antes_de_abrir_e_conta() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let state = Arc::new(FileState::default());
        let subscriber = tracing_subscriber::registry().with(FileLayer(Arc::clone(&state)));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(evento = "agent.up");
            assert_eq!(state.dropped.load(Ordering::Relaxed), 1);
            for _ in 0..300 {
                tracing::info!(evento = "agent.up");
            }
        });
        assert_eq!(state.dropped.load(Ordering::Relaxed), 200);
        assert!(!tmp.path().join("logs").exists());
        Ok(())
    }

    #[test]
    fn lista_vira_json() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let subscriber = tracing_subscriber::registry().with(FileLayer(opened(tmp.path(), 4096)?));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(evento = "capture.ready", barreiras = %"[{\"peer\":\"aa\"}]", totais = %"{\"enviados\":2}", pos = 0.125_f64, ligado = true);
        });
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(log_path(tmp.path()))?)?;
        assert!(value["barreiras"].is_array());
        assert_eq!(value["barreiras"][0]["peer"], "aa");
        assert_eq!(value["totais"]["enviados"], 2);
        assert_eq!(value["pos"], 0.125);
        assert_eq!(value["ligado"], true);
        Ok(())
    }
}
