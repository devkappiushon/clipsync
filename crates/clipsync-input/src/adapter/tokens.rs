//! `DIR/portal-tokens.toml` (mode 0600): the RemoteDesktop `restore_token`.
use super::InputError;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const MAX_TOKEN_BYTES: usize = 256;

#[derive(Debug, Clone)]
pub struct TokenStore {
    path: PathBuf,
}

fn valid(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= MAX_TOKEN_BYTES
        && token.bytes().all(|b| b.is_ascii_graphic())
}

impl TokenStore {
    pub fn new(profile_dir: &Path) -> Self {
        Self {
            path: profile_dir.join("portal-tokens.toml"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the stored token. A malformed file or token is discarded (file removed).
    pub fn load(&self) -> Option<String> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let token = toml::from_str::<toml::Table>(&text)
            .ok()
            .and_then(|t| t.get("remote_desktop")?.as_str().map(str::to_owned))
            .filter(|t| valid(t));
        if token.is_none() {
            self.discard();
        }
        token
    }

    pub fn save(&self, token: &str) -> Result<(), InputError> {
        if !valid(token) {
            return Err(InputError::Other("invalid restore token".into()));
        }
        let mut table = toml::Table::new();
        table.insert("version".into(), toml::Value::Integer(1));
        table.insert("remote_desktop".into(), toml::Value::String(token.into()));
        let body = toml::to_string(&table).map_err(|e| InputError::Other(e.to_string()))?;
        let tmp = self.path.with_extension("toml.tmp");
        let io = |e: std::io::Error| InputError::Other(e.to_string());
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(io)?;
        f.write_all(body.as_bytes()).map_err(io)?;
        f.sync_all().map_err(io)?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).map_err(io)?;
        std::fs::rename(&tmp, &self.path).map_err(io)
    }

    pub fn discard(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
