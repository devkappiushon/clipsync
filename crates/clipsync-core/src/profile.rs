//! Profile directory: config, identity, peers and persisted state (contract §3).
use crate::CoreError;
use clipsync_proto::{ModuleSet, PeerId, PeerPerms};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub const CONFIG_VERSION: u32 = 1;
pub const DEFAULT_LISTEN: &str = "0.0.0.0:47800";

const CONFIG_FILE: &str = "config.toml";
const KEY_FILE: &str = "identity.key.pem";
const CERT_FILE: &str = "identity.cert.pem";
const PEERS_FILE: &str = "peers.toml";
const STATE_FILE: &str = "state.toml";
/// Control rendezvous: the Unix socket itself; on Windows, the file that publishes the random pipe name.
#[cfg(unix)]
const SOCKET_FILE: &str = "agent.sock";
#[cfg(windows)]
const SOCKET_FILE: &str = "agent.pipe";

#[derive(Debug, Clone)]
pub struct Profile {
    pub dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InitOptions {
    pub device_name: String,
    pub modules: ModuleSet,
    pub listen: SocketAddr,
    pub fake_adapters: bool,
}

/// Name the self-signed certificate answers to; peers pin the key, never the name.
pub const SERVER_NAME: &str = "clipsync";

/// Device identity: ECDSA P-256 key + self-signed certificate.
/// Files: identity.key.pem (PKCS#8, 0600) and identity.cert.pem. `Debug` never prints the key.
pub struct Identity {
    id: PeerId,
    cert_pem: String,
    key_pem: String,
    cert: CertificateDer<'static>,
    key: PrivatePkcs8KeyDer<'static>,
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identity {{ id: {} }}", self.id.short())
    }
}

impl Clone for Identity {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            cert_pem: self.cert_pem.clone(),
            key_pem: self.key_pem.clone(),
            cert: self.cert.clone(),
            key: self.key.clone_key(),
        }
    }
}

impl Identity {
    /// The exact call of the spike: ECDSA P-256 (ring), the verifier ignores the name.
    pub fn generate() -> Result<Self, CoreError> {
        let ck = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_string()])
            .map_err(|e| CoreError::Identity(e.to_string()))?;
        Self::from_pem(&ck.cert.pem(), &ck.signing_key.serialize_pem())
    }

    /// Parses both files and checks that the key belongs to the certificate.
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self, CoreError> {
        let cert = CertificateDer::from_pem_slice(cert_pem.as_bytes())
            .map_err(|e| CoreError::Identity(format!("certificate: {e}")))?;
        let key = match PrivateKeyDer::from_pem_slice(key_pem.as_bytes())
            .map_err(|e| CoreError::Identity(format!("key: {e}")))?
        {
            PrivateKeyDer::Pkcs8(k) => k,
            _ => return Err(CoreError::Identity("key is not PKCS#8".into())),
        };
        let id = crate::tls::pin_of(&cert)?; // also proves the certificate parses
        let pair =
            rcgen::KeyPair::from_pem(key_pem).map_err(|e| CoreError::Identity(e.to_string()))?;
        let spki = rcgen::PublicKeyData::subject_public_key_info(&pair);
        // a certificate embeds the SPKI of its key verbatim
        if !cert
            .as_ref()
            .windows(spki.len())
            .any(|w| w == spki.as_slice())
        {
            return Err(CoreError::Identity("key does not match certificate".into()));
        }
        Ok(Self {
            id,
            cert_pem: cert_pem.to_string(),
            key_pem: key_pem.to_string(),
            cert,
            key,
        })
    }

    pub fn cert_pem(&self) -> String {
        self.cert_pem.clone()
    }

    /// Secret: write only to a 0600 file.
    pub fn key_pem(&self) -> String {
        self.key_pem.clone()
    }

    pub fn cert_der(&self) -> CertificateDer<'static> {
        self.cert.clone()
    }

    pub fn key_der(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(self.key.clone_key())
    }

    /// `tls::pin_of(cert)`, computed once in `from_pem` (the one function that turns a certificate into a PeerId).
    pub fn id(&self) -> PeerId {
        self.id
    }
}

/// peers.toml (on the GRANTING device; everything false by default, prompt-mestre §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peers {
    pub version: u32,
    #[serde(default)]
    pub peer: Vec<PeerEntry>,
}

impl Default for Peers {
    fn default() -> Self {
        Self {
            version: 1,
            peer: Vec::new(),
        }
    }
}

impl Peers {
    pub fn get(&self, id: &PeerId) -> Option<&PeerEntry> {
        self.peer.iter().find(|p| p.id == *id)
    }

    /// Exact name, `short()` or full hex id.
    pub fn find(&self, name_or_short_id: &str) -> Option<&PeerEntry> {
        self.peer.iter().find(|p| {
            p.name == name_or_short_id
                || p.id.short() == name_or_short_id
                || p.id.to_hex() == name_or_short_id
        })
    }

    /// Replaces the entry with the same id, or with the same name (a re-keyed device keeps one entry).
    pub fn upsert(&mut self, entry: PeerEntry) {
        self.peer
            .retain(|p| p.id != entry.id && p.name != entry.name);
        self.peer.push(entry);
    }

    pub fn pins(&self) -> Vec<PeerId> {
        self.peer.iter().map(|p| p.id).collect()
    }
}

// `flatten` and `deny_unknown_fields` do not mix: unknown keys inside a [[peer]] are ignored on read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEntry {
    #[serde(with = "peer_id_hex")]
    pub id: PeerId,
    pub name: String,
    /// IP observed while pairing + listen_port announced by the peer.
    pub addr: Option<SocketAddr>,
    /// RFC 3339, UTC; display only.
    pub paired_at: String,
    /// clipboard_in: the peer may SEND me clipboard · clipboard_out: I may send mine to it ·
    /// input_in: the peer may CONTROL my keyboard and mouse (full trust).
    #[serde(flatten)]
    pub perms: PeerPerms,
}

impl PeerEntry {
    pub fn perms(&self) -> PeerPerms {
        self.perms
    }
}

/// state.toml: persisted by the agent; pause survives restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub version: u32,
    pub paused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            paused: false,
        }
    }
}

mod peer_id_hex {
    use clipsync_proto::PeerId;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(id: &PeerId, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&id.to_hex())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PeerId, D::Error> {
        let s = String::deserialize(d)?;
        PeerId::from_hex(&s).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub version: u32,
    pub device_name: String,
    pub modules: ModuleSet,
    pub listen: SocketAddr,
    /// Raw `[clipboard]` table; the clipboard module parses it.
    pub clipboard: Option<toml::Table>,
    /// Raw `[input]` table; the input module parses it.
    pub input: Option<toml::Table>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModules {
    #[serde(default)]
    input: bool,
    #[serde(default)]
    clipboard: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNetwork {
    listen: SocketAddr,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u32,
    device_name: String,
    modules: RawModules,
    network: RawNetwork,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clipboard: Option<toml::Table>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input: Option<toml::Table>,
}

pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn io_ctx(context: impl std::fmt::Display, e: std::io::Error) -> CoreError {
    CoreError::Io(std::io::Error::new(e.kind(), format!("{context}: {e}")))
}

/// The one rule for device names, shared with the wire (`Hello::validate`): no controls (C0, DEL, C1
/// including U+009B) and no Unicode formatting characters, 1 to 64 bytes.
fn valid_name(name: &str) -> Result<(), CoreError> {
    clipsync_proto::validate_device_name(name)
        .map_err(|e| CoreError::Config(format!("device_name inválido: {e}")))
}

/// Writes `data` to `path` through a private temp file and an atomic rename; mode 0600 on Unix (on Windows
/// the file inherits the ACL of the profile directory, see `Profile::init`).
pub(crate) fn write_private(path: &Path, data: &[u8]) -> Result<(), CoreError> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let res = (|| {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        opts.mode(0o600);
        let mut f = opts.open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res.map_err(|e| io_ctx(format!("escrevendo {}", path.display()), e))
}

fn read_text(path: &Path) -> Result<String, CoreError> {
    fs::read_to_string(path).map_err(|e| io_ctx(format!("lendo {}", path.display()), e))
}

fn default_dir() -> Result<PathBuf, CoreError> {
    if let Some(p) = std::env::var_os("CLIPSYNC_PROFILE").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    platform_default_dir()
}

#[cfg(unix)]
fn platform_default_dir() -> Result<PathBuf, CoreError> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(x).join("clipsync"));
    }
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| CoreError::Config("HOME indefinido".into()))?;
    Ok(PathBuf::from(home).join(".config").join("clipsync"))
}

/// Local, not Roaming: the private key must not travel to a domain server (contract §11 rule 4).
#[cfg(windows)]
fn platform_default_dir() -> Result<PathBuf, CoreError> {
    std::env::var_os("LOCALAPPDATA")
        .filter(|v| !v.is_empty())
        .map(|d| PathBuf::from(d).join("clipsync"))
        .ok_or_else(|| CoreError::Profile("LOCALAPPDATA indefinido: use --profile DIR".into()))
}

fn module_table(fake: bool, keys: &[(&str, &str)]) -> toml::Table {
    let mut t = toml::Table::new();
    for (k, v) in keys {
        let v = if fake && matches!(*k, "adapter" | "capture" | "emulate") {
            "fake"
        } else {
            v
        };
        t.insert((*k).into(), toml::Value::String(v.into()));
    }
    t
}

impl Profile {
    /// `--profile` argument, else `$CLIPSYNC_PROFILE`, else `$XDG_CONFIG_HOME/clipsync`, else `~/.config/clipsync`.
    /// On Windows the last two are replaced by `%LOCALAPPDATA%\clipsync`.
    pub fn resolve(arg: Option<PathBuf>) -> Result<Self, CoreError> {
        let dir = match arg {
            Some(p) => p,
            None => default_dir()?,
        };
        let dir = if dir.is_absolute() {
            dir
        } else {
            std::env::current_dir()
                .map_err(|e| io_ctx("diretório atual", e))?
                .join(dir)
        };
        Ok(Self { dir })
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }
    pub fn peers_path(&self) -> PathBuf {
        self.dir.join(PEERS_FILE)
    }
    pub fn state_path(&self) -> PathBuf {
        self.dir.join(STATE_FILE)
    }
    pub fn socket_path(&self) -> PathBuf {
        self.dir.join(SOCKET_FILE)
    }
    pub fn is_initialized(&self) -> bool {
        self.config_path().is_file() && self.dir.join(KEY_FILE).is_file()
    }

    /// Creates DIR (0700), a fresh identity (key 0600) and `config.toml`. An existing profile gets a NEW
    /// identity and config; `peers.toml` and `state.toml` are kept.
    pub fn init(&self, opts: InitOptions) -> Result<Identity, CoreError> {
        self.init_with(opts, Identity::generate()?)
    }

    /// `init` with an identity made earlier, so a UI can show its fingerprint before anything is written.
    pub fn init_with(&self, opts: InitOptions, identity: Identity) -> Result<Identity, CoreError> {
        valid_name(&opts.device_name)?;
        if opts.modules.is_empty() {
            return Err(CoreError::Config(
                "pelo menos um módulo precisa estar ligado".into(),
            ));
        }
        let mut dir = fs::DirBuilder::new();
        dir.recursive(true);
        #[cfg(unix)]
        dir.mode(0o700);
        dir.create(&self.dir)
            .map_err(|e| io_ctx(format!("criando {}", self.dir.display()), e))?;
        // Windows has no mode bits: the directory (and the key in it) inherits the ACL of the user
        // profile, which already keeps other users out.
        #[cfg(unix)]
        fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o700))
            .map_err(|e| io_ctx("permissões do perfil", e))?;

        let raw = RawConfig {
            version: CONFIG_VERSION,
            device_name: opts.device_name,
            modules: RawModules {
                input: opts.modules.input,
                clipboard: opts.modules.clipboard,
            },
            network: RawNetwork {
                listen: opts.listen,
            },
            clipboard: opts.modules.clipboard.then(|| {
                module_table(opts.fake_adapters, &[("adapter", "auto"), ("mode", "auto")])
            }),
            input: opts.modules.input.then(|| {
                module_table(
                    opts.fake_adapters,
                    &[("capture", "auto"), ("emulate", "auto")],
                )
            }),
        };
        let config = toml::to_string(&raw).map_err(|e| CoreError::Config(e.to_string()))?;
        // key first: a half-written profile must never hold a config without its identity
        write_private(&self.dir.join(KEY_FILE), identity.key_pem().as_bytes())?;
        write_private(&self.dir.join(CERT_FILE), identity.cert_pem().as_bytes())?;
        write_private(&self.config_path(), config.as_bytes())?;
        if !self.peers_path().exists() {
            self.save_peers(&Peers::default())?;
        }
        Ok(identity)
    }

    pub fn load_config(&self) -> Result<Config, CoreError> {
        let path = self.config_path();
        if !path.is_file() {
            return Err(CoreError::Config(format!(
                "perfil não inicializado: {}",
                self.dir.display()
            )));
        }
        let raw: RawConfig = toml::from_str(&read_text(&path)?)
            .map_err(|e| CoreError::Config(format!("config.toml: {e}")))?;
        if raw.version != CONFIG_VERSION {
            return Err(CoreError::Config(format!(
                "config.toml: versão {} não suportada (esperada {CONFIG_VERSION})",
                raw.version
            )));
        }
        valid_name(&raw.device_name)?;
        let modules = ModuleSet {
            clipboard: raw.modules.clipboard,
            input: raw.modules.input,
        };
        if modules.is_empty() {
            return Err(CoreError::Config(
                "config.toml: pelo menos um módulo precisa estar ligado".into(),
            ));
        }
        Ok(Config {
            version: raw.version,
            device_name: raw.device_name,
            modules,
            listen: raw.network.listen,
            clipboard: raw.clipboard,
            input: raw.input,
        })
    }

    /// Sets `[clipboard] mode` ("auto" or "manual") in config.toml, keeping every other key.
    /// Written through a temp file and a rename; the running agent reads it on restart.
    pub fn set_clipboard_mode(&self, mode: &str) -> Result<(), CoreError> {
        if !matches!(mode, "auto" | "manual") {
            return Err(CoreError::Config(format!(
                "[clipboard] mode inválido: {mode} (use auto ou manual)"
            )));
        }
        self.load_config()?; // the same checks as a normal read
        let mut raw: RawConfig = toml::from_str(&read_text(&self.config_path())?)
            .map_err(|e| CoreError::Config(format!("config.toml: {e}")))?;
        raw.clipboard
            .get_or_insert_with(toml::Table::new)
            .insert("mode".into(), toml::Value::String(mode.into()));
        let text = toml::to_string(&raw).map_err(|e| CoreError::Config(e.to_string()))?;
        write_private(&self.config_path(), text.as_bytes())
    }

    pub fn load_identity(&self) -> Result<Identity, CoreError> {
        self.load_config()?; // also proves the profile exists
        let key_path = self.dir.join(KEY_FILE);
        // Windows: no mode to check; the key is guarded by the ACL of the user profile (see `init`).
        #[cfg(unix)]
        {
            let mode = fs::metadata(&key_path)
                .map_err(|e| io_ctx("chave privada", e))?
                .permissions()
                .mode()
                & 0o777;
            if mode & 0o077 != 0 {
                return Err(CoreError::Identity(format!(
                    "{KEY_FILE} com modo {mode:o}: deve ser 600"
                )));
            }
        }
        Identity::from_pem(
            &read_text(&self.dir.join(CERT_FILE))?,
            &read_text(&key_path)?,
        )
    }

    pub fn load_peers(&self) -> Result<Peers, CoreError> {
        let path = self.peers_path();
        if !path.exists() {
            return Ok(Peers::default());
        }
        let peers: Peers = toml::from_str(&read_text(&path)?)
            .map_err(|e| CoreError::Profile(format!("peers.toml: {e}")))?;
        if peers.version != 1 {
            return Err(CoreError::Profile(format!(
                "peers.toml: versão {} não suportada",
                peers.version
            )));
        }
        // A peer whose name is unusable is quarantined (skipped, logged by id only) instead of making
        // the whole profile unreadable: one bad entry must not take the other pairings down.
        let mut peers = peers;
        peers.peer.retain(|p| {
            let ok = valid_name(&p.name).is_ok();
            if !ok {
                tracing::warn!(peer = %p.id.short(), "peers.toml: entrada com nome inválido ignorada");
            }
            ok
        });
        for (i, p) in peers.peer.iter().enumerate() {
            if peers.peer[..i].iter().any(|q| q.id == p.id) {
                return Err(CoreError::Profile(format!(
                    "peers.toml: id repetido ({})",
                    p.id.short()
                )));
            }
        }
        Ok(peers)
    }

    pub fn save_peers(&self, p: &Peers) -> Result<(), CoreError> {
        let text = toml::to_string(p).map_err(|e| CoreError::Profile(e.to_string()))?;
        write_private(&self.peers_path(), text.as_bytes())
    }

    /// Missing file = not paused.
    pub fn load_state(&self) -> Result<State, CoreError> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(State::default());
        }
        let st: State = toml::from_str(&read_text(&path)?)
            .map_err(|e| CoreError::Config(format!("state.toml: {e}")))?;
        if st.version != 1 {
            return Err(CoreError::Config(format!(
                "state.toml: versão {} não suportada",
                st.version
            )));
        }
        Ok(st)
    }

    pub fn save_state(&self, s: &State) -> Result<(), CoreError> {
        let text = toml::to_string(&State {
            version: 1,
            paused: s.paused,
        })
        .map_err(|e| CoreError::Config(e.to_string()))?;
        write_private(&self.state_path(), text.as_bytes())
    }
}
