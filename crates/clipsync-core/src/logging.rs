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

/// Quantidade limitada de descartes antes da abertura ou por linha excessiva.
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
    fn note_drop(&self) {
        let _ = self
            .dropped
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                Some((n + 1).min(200))
            });
    }

    fn write_event(&self, mut fields: Map<String, Value>) {
        let Ok(mut guard) = self.sink.lock() else {
            return;
        };
        let Some(sink) = guard.as_mut() else {
            self.note_drop();
            return;
        };
        fields.insert("host".into(), Value::String(sink.host.clone()));
        let result = serde_json::to_string(&fields)
            .map_err(io::Error::other)
            .and_then(|line| {
                if line.len().saturating_add(1) <= 64 * 1024 {
                    return sink.write_line(&line);
                }
                self.note_drop();
                fields.retain(|key, _| {
                    matches!(key.as_str(), "v" | "ts" | "nivel" | "host" | "alvo")
                });
                fields.insert("evento".into(), Value::String("log.truncated".into()));
                fields.insert(
                    "bytes".into(),
                    Value::from(line.len().saturating_add(1) as u64),
                );
                let short = serde_json::to_string(&fields).map_err(io::Error::other)?;
                sink.write_line(&short)
            });
        if result.is_err() && !self.write_warned.swap(true, Ordering::Relaxed) {
            // Não reentra no tracing enquanto o mutex do arquivo está adquirido.
            eprintln!("warn: não foi possível escrever o arquivo de log");
        }
    }
}

// Esta checagem recusa redirecionamentos existentes; não elimina a corrida local
// entre consultar o caminho e abri-lo. O perfil privado continua sendo a fronteira.
fn reject_symlink(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "link simbólico no caminho do log",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

trait FileAccess: Send {
    fn append(&self, path: &Path) -> io::Result<File>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
}

struct Disk;
impl FileAccess for Disk {
    fn append(&self, path: &Path) -> io::Result<File> {
        Sink::append(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }
}

struct Sink {
    access: Box<dyn FileAccess>,
    file: File,
    path: PathBuf,
    host: String,
    size: u64,
    limit: u64,
    rotation_warned: bool,
    reopen_pending: bool,
}

impl Sink {
    fn open(dir: &Path, host: &str, limit: u64) -> io::Result<Self> {
        reject_symlink(&dir.join("logs"))?;
        let mut builder = DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(dir.join("logs"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            File::open(dir.join("logs"))?
                .set_permissions(std::fs::Permissions::from_mode(0o700))?;
        }
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
            reopen_pending: false,
            access: Box::new(Disk),
        })
    }

    fn append(path: &Path) -> io::Result<File> {
        if let Some(parent) = path.parent() {
            reject_symlink(parent)?;
        }
        reject_symlink(path)?;
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(file)
    }

    fn rotate(&mut self) -> io::Result<()> {
        let first = self.path.with_file_name("clipsync.jsonl.1");
        let second = self.path.with_file_name("clipsync.jsonl.2");
        match std::fs::remove_file(&second) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        match self.access.rename(&first, &second) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        self.access.rename(&self.path, &first)?;
        match self.access.append(&self.path) {
            Ok(file) => {
                self.file = file;
                self.size = 0;
                Ok(())
            }
            Err(e) => {
                // O handle pode ter ficado em .1: nenhuma rotação pode removê-lo.
                self.reopen_pending = true;
                let _ = self.access.rename(&first, &self.path);
                Err(e)
            }
        }
    }

    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let mut bytes = line.as_bytes().to_vec();
        bytes.push(b'\n');
        if self.reopen_pending {
            let reopened = self.access.append(&self.path).and_then(|file| {
                let size = file.metadata()?.len();
                Ok((file, size))
            });
            if let Ok((file, size)) = reopened {
                self.file = file;
                self.size = size;
                self.reopen_pending = false;
            }
        }
        if !self.reopen_pending
            && self.size > 0
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
        if *event.metadata().level() > tracing::Level::INFO {
            return;
        }
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
            .or_insert_with(|| Value::String("legado".into()));
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

    #[cfg(unix)]
    #[test]
    fn recusa_symlink_de_diretorio_e_arquivo() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir()?;
        let destination = tempfile::tempdir()?;
        symlink(destination.path(), tmp.path().join("logs"))?;
        assert!(Sink::open(tmp.path(), "host", 4096).is_err());
        assert!(!destination.path().join("clipsync.jsonl").exists());
        std::fs::remove_file(tmp.path().join("logs"))?;
        std::fs::create_dir(tmp.path().join("logs"))?;
        let original = destination.path().join("arquivo");
        std::fs::write(&original, "preservar")?;
        symlink(&original, log_path(tmp.path()))?;
        assert!(Sink::open(tmp.path(), "host", 4096).is_err());
        assert_eq!(std::fs::read_to_string(original)?, "preservar");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn corrige_permissoes_dos_objetos_existentes() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("logs");
        std::fs::create_dir(&dir)?;
        std::fs::write(log_path(tmp.path()), "anterior\n")?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))?;
        std::fs::set_permissions(log_path(tmp.path()), std::fs::Permissions::from_mode(0o644))?;
        let mut sink = Sink::open(tmp.path(), "host", 4096)?;
        sink.write_line("novo")?;
        assert_eq!(std::fs::metadata(&dir)?.permissions().mode() & 0o777, 0o700);
        assert_eq!(
            std::fs::metadata(log_path(tmp.path()))?
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::read_to_string(log_path(tmp.path()))?,
            "anterior\nnovo\n"
        );
        Ok(())
    }

    #[test]
    fn linha_excessiva_vira_truncated_e_conta_descarte() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let state = opened(tmp.path(), 5 * 1024 * 1024)?;
        let subscriber = tracing_subscriber::registry().with(FileLayer(Arc::clone(&state)));
        let long = "conteudo-nao-deve-sair".repeat(4096);
        tracing::subscriber::with_default(subscriber, || {
            tracing::warn!(evento = "emulate.error", erro = %long);
            tracing::info!(evento = "agent.up");
        });
        let data = std::fs::read_to_string(log_path(tmp.path()))?;
        assert_eq!(data.lines().count(), 2);
        let first = data.lines().next().ok_or("sem linha")?;
        assert!(first.len() <= 64 * 1024);
        let value: Value = serde_json::from_str(first)?;
        assert_eq!(value["evento"], "log.truncated");
        assert!(value["bytes"].as_u64().is_some_and(|n| n > 64 * 1024));
        assert_eq!(value["v"], 1);
        assert_eq!(value["host"], "notebook");
        assert!(value["ts"].as_str().is_some_and(|v| v.ends_with('Z')));
        assert_eq!(value["nivel"], "warn");
        assert!(value["alvo"].as_str().is_some());
        assert!(!data.contains("conteudo-nao-deve-sair"));
        assert_eq!(state.dropped.load(Ordering::Relaxed), 1);
        Ok(())
    }

    struct FailingAccess {
        rename: bool,
        rollback: bool,
    }
    impl FileAccess for FailingAccess {
        fn append(&self, _path: &Path) -> io::Result<File> {
            Err(io::Error::other("reabertura injetada"))
        }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            if self.rename
                || (self.rollback && to.file_name().is_some_and(|v| v == "clipsync.jsonl"))
            {
                Err(io::Error::other("rename injetado"))
            } else {
                std::fs::rename(from, to)
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn reabertura_e_rollback_falhos_nunca_desvinculam_handle() -> Result<(), Box<dyn Error>> {
        use std::os::unix::fs::MetadataExt;
        for rollback in [false, true] {
            let tmp = tempfile::tempdir()?;
            let mut sink = Sink::open(tmp.path(), "host", 8)?;
            sink.write_line("primeira")?;
            sink.access = Box::new(FailingAccess {
                rename: false,
                rollback,
            });
            for line in ["segunda", "terceira", "quarta"] {
                sink.write_line(line)?;
            }
            assert_eq!(
                sink.file.metadata()?.nlink(),
                1,
                "rollback falha: {rollback}"
            );
            let live = if rollback {
                tmp.path().join("logs/clipsync.jsonl.1")
            } else {
                log_path(tmp.path())
            };
            assert_eq!(
                std::fs::read_to_string(&live)?,
                "primeira\nsegunda\nterceira\nquarta\n"
            );
            sink.access = Box::new(Disk);
            sink.write_line("recuperou")?;
            assert_eq!(sink.file.metadata()?.nlink(), 1);
            assert_eq!(
                std::fs::read_to_string(log_path(tmp.path()))?,
                "recuperou\n"
            );
        }
        Ok(())
    }

    #[test]
    fn rename_falho_preserva_corrente() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let mut sink = Sink::open(tmp.path(), "host", 8)?;
        sink.write_line("primeira")?;
        sink.access = Box::new(FailingAccess {
            rename: true,
            rollback: true,
        });
        sink.write_line("segunda")?;
        assert_eq!(
            std::fs::read_to_string(log_path(tmp.path()))?,
            "primeira\nsegunda\n"
        );
        Ok(())
    }

    #[test]
    fn legado_tem_evento_e_arquivo_nunca_grava_debug() -> Result<(), Box<dyn Error>> {
        let tmp = tempfile::tempdir()?;
        let subscriber = tracing_subscriber::registry()
            .with(FileLayer(opened(tmp.path(), 4096)?).with_filter(EnvFilter::new("trace")));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(evento = "agent.up", "debug");
            tracing::trace!(evento = "agent.up", "trace");
            tracing::info!("aviso legado");
        });
        let data = std::fs::read_to_string(log_path(tmp.path()))?;
        assert_eq!(data.lines().count(), 1);
        let value: Value = serde_json::from_str(&data)?;
        assert_eq!(value["evento"], "legado");
        assert_eq!(value["mensagem"], "aviso legado");
        Ok(())
    }
}
