//! Private hardware evidence, never part of a network inventory.
use crate::layout_storage::{
    AtomicWriteError, atomic_write, confirm_existing, lock_file, private_dir, regular_file,
};
use clipsync_proto::{MonitorInfo, MonitorUid, NativeRect};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    path::{Path, PathBuf},
};
#[derive(Clone, PartialEq, Eq)]
pub enum Evidence {
    WindowsPath(String),
    LinuxSingle {
        connector: String,
        edid_sha256: Option<[u8; 32]>,
    },
}
impl fmt::Debug for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Evidence([privado])")
    }
}
#[derive(Clone)]
pub struct Observation {
    pub evidence: Evidence,
    pub label: String,
    pub rect: NativeRect,
    pub scale: f64,
    pub legacy_aliases: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedSnapshot {
    pub rev: u64,
    pub monitors: Vec<MonitorInfo>,
    pub aliases: Vec<IdentifiedAlias>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct IdentifiedAlias {
    pub alias: String,
    pub uid: MonitorUid,
}
// Aliases may contain the same private device path as Evidence. Redact them at
// the leaf types so Debug remains safe when inventories/snapshots compose them.
impl fmt::Debug for Observation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Observation")
            .field("evidence", &self.evidence)
            .field("label", &self.label)
            .field("rect", &self.rect)
            .field("scale", &self.scale)
            .field("legacy_alias_count", &self.legacy_aliases.len())
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for IdentifiedAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentifiedAlias")
            .field("uid", &self.uid)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("identidade de tela ambígua")]
    Ambiguous,
    #[error("identidade de tela indisponível")]
    Unavailable,
    #[error("versão do registro não suportada")]
    UnsupportedVersion,
    #[error("registro de telas em uso")]
    Busy,
    #[error("falha no armazenamento privado de telas")]
    Io,
}
pub struct IdentityStore {
    profile: PathBuf,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
enum PrivateEvidence {
    WindowsPath(String),
    LinuxSingle {
        connector: String,
        edid_sha256: Option<[u8; 32]>,
    },
}
impl From<&Evidence> for PrivateEvidence {
    fn from(e: &Evidence) -> Self {
        match e {
            Evidence::WindowsPath(p) => Self::WindowsPath(p.clone()),
            Evidence::LinuxSingle {
                connector,
                edid_sha256,
            } => Self::LinuxSingle {
                connector: connector.clone(),
                edid_sha256: *edid_sha256,
            },
        }
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
struct Entry {
    uid: MonitorUid,
    evidence: PrivateEvidence,
    present: bool,
    label: String,
    rect: NativeRect,
    scale: f64,
    aliases: Vec<String>,
}
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    rev: u64,
    entries: Vec<Entry>,
}
fn io_error(_: std::io::Error) -> IdentityError {
    IdentityError::Io
}
impl IdentityStore {
    pub fn new(profile: &Path) -> Self {
        Self {
            profile: profile.to_owned(),
        }
    }
    pub fn reconcile(
        &self,
        observations: &[Observation],
    ) -> Result<IdentifiedSnapshot, IdentityError> {
        for (i, o) in observations.iter().enumerate() {
            let usable = match &o.evidence {
                Evidence::WindowsPath(s) => !s.is_empty(),
                Evidence::LinuxSingle { connector, .. } => {
                    !connector.is_empty() && observations.len() == 1
                }
            };
            if !usable || !valid_geometry(o.rect, o.scale) || o.label.is_empty() {
                return Err(IdentityError::Unavailable);
            }
            if observations[..i]
                .iter()
                .any(|other| o.evidence == other.evidence)
            {
                return Err(IdentityError::Ambiguous);
            }
        }
        private_dir(&self.profile).map_err(io_error)?;
        let _guard = lock_file(&self.profile.join("monitor-identities.lock")).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                IdentityError::Busy
            } else {
                IdentityError::Io
            }
        })?;
        let path = self.profile.join("monitor-identities.toml");
        let previous = match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                regular_file(&path).map_err(io_error)?;
                let text = std::fs::read_to_string(&path).map_err(io_error)?;
                #[derive(Deserialize)]
                struct Version {
                    version: u32,
                }
                let version: Version =
                    toml::from_str(&text).map_err(|_| IdentityError::Unavailable)?;
                if version.version != 1 {
                    return Err(IdentityError::UnsupportedVersion);
                }
                let r: Registry = toml::from_str(&text).map_err(|_| IdentityError::Unavailable)?;
                validate_registry(&r)?;
                r
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry {
                version: 1,
                rev: 0,
                entries: vec![],
            },
            Err(e) => return Err(io_error(e)),
        };
        let evidence: Vec<_> = observations
            .iter()
            .map(|o| PrivateEvidence::from(&o.evidence))
            .collect();
        // A new path while a known screen is absent may be a replacement; never guess.
        let has_new = evidence
            .iter()
            .any(|e| !previous.entries.iter().any(|p| &p.evidence == e));
        let missing = previous
            .entries
            .iter()
            .any(|p| !evidence.contains(&p.evidence));
        if has_new && missing {
            return Err(IdentityError::Ambiguous);
        }
        let mut next = previous.clone();
        for entry in &mut next.entries {
            entry.present = false;
        }
        for (o, evidence) in observations.iter().zip(evidence) {
            let mut aliases = o.legacy_aliases.clone();
            aliases.sort();
            aliases.dedup();
            aliases.retain(|a| !a.is_empty());
            if let Some(e) = next.entries.iter_mut().find(|p| p.evidence == evidence) {
                e.present = true;
                e.label = o.label.clone();
                e.rect = o.rect;
                e.scale = o.scale;
                e.aliases = aliases;
            } else {
                next.entries.push(Entry {
                    uid: MonitorUid::random(),
                    evidence,
                    present: true,
                    label: o.label.clone(),
                    rect: o.rect,
                    scale: o.scale,
                    aliases,
                });
            }
        }
        next.entries.sort_by_key(|e| e.uid);
        if next != previous || previous.rev == 0 {
            next.rev = previous
                .rev
                .checked_add(1)
                .ok_or(IdentityError::Unavailable)?;
            let text = toml::to_string(&next).map_err(|_| IdentityError::Unavailable)?;
            atomic_write(&path, text.as_bytes()).map_err(|error| match error {
                AtomicWriteError::Unpublished(e) => io_error(e),
                // Bytes may already be visible; no snapshot is confirmed on this path.
                AtomicWriteError::PublishedUnconfirmed(e) => io_error(e),
            })?;
        } else {
            // Also covers reopening a registry whose previous publication was unconfirmed.
            confirm_existing(&path).map_err(io_error)?;
        }
        let mut result = IdentifiedSnapshot {
            rev: next.rev,
            monitors: vec![],
            aliases: vec![],
        };
        for e in next.entries.iter().filter(|e| e.present) {
            result.monitors.push(MonitorInfo {
                uid: e.uid,
                label: e.label.clone(),
                rect: e.rect,
                scale: e.scale,
            });
            for a in &e.aliases {
                if next
                    .entries
                    .iter()
                    .filter(|other| other.present && other.aliases.contains(a))
                    .count()
                    == 1
                {
                    result.aliases.push(IdentifiedAlias {
                        alias: a.clone(),
                        uid: e.uid,
                    });
                }
            }
        }
        Ok(result)
    }
}
fn validate_registry(r: &Registry) -> Result<(), IdentityError> {
    if r.rev == 0 {
        return Err(IdentityError::Unavailable);
    }
    let mut uids = HashSet::new();
    for (i, e) in r.entries.iter().enumerate() {
        if !uids.insert(e.uid)
            || !valid_geometry(e.rect, e.scale)
            || r.entries[..i].iter().any(|p| p.evidence == e.evidence)
        {
            return Err(IdentityError::Ambiguous);
        }
    }
    Ok(())
}
pub(crate) fn valid_geometry(rect: NativeRect, scale: f64) -> bool {
    rect.width > 0
        && rect.height > 0
        && scale.is_finite()
        && scale > 0.
        && i64::from(rect.x) + i64::from(rect.width) <= i64::from(i32::MAX)
        && i64::from(rect.y) + i64::from(rect.height) <= i64::from(i32::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::layout_storage::faults::{Stage, during};
    fn observation(scale: f64) -> Observation {
        Observation {
            evidence: Evidence::WindowsPath("private-path".into()),
            label: "Tela".into(),
            rect: NativeRect {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            scale,
            legacy_aliases: vec!["alias".into()],
        }
    }
    #[test]
    fn fix1_identity_sync_failures_never_return_a_snapshot() {
        for initially_absent in [true, false] {
            for stage in [Stage::BeforeRename, Stage::ConfirmPublication] {
                let dir = tempfile::tempdir().unwrap();
                let store = IdentityStore::new(dir.path());
                let path = dir.path().join("monitor-identities.toml");
                let previous = if initially_absent {
                    None
                } else {
                    store.reconcile(&[observation(1.)]).unwrap();
                    Some(std::fs::read(&path).unwrap())
                };
                let result = during(stage, || store.reconcile(&[observation(1.25)]));
                assert!(matches!(result, Err(IdentityError::Io)));
                if stage == Stage::BeforeRename {
                    assert_eq!(std::fs::read(&path).ok(), previous);
                } else {
                    let published: Registry =
                        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                    assert_eq!(published.rev, if initially_absent { 1 } else { 2 });
                    assert_eq!(published.entries[0].scale, 1.25);
                    assert_ne!(Some(std::fs::read(&path).unwrap()), previous);
                }
                assert!(!dir.path().join("layout.toml").exists());
            }
        }
    }
    #[test]
    fn fix1_reopening_unconfirmed_registry_requires_successful_sync_before_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            during(Stage::ConfirmPublication, || IdentityStore::new(dir.path())
                .reconcile(&[observation(1.)])),
            Err(IdentityError::Io)
        ));
        let path = dir.path().join("monitor-identities.toml");
        let bytes = std::fs::read(&path).unwrap();
        let published: Registry = toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert!(matches!(
            during(Stage::ConfirmPublication, || IdentityStore::new(dir.path())
                .reconcile(&[observation(1.)])),
            Err(IdentityError::Io)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let confirmed = IdentityStore::new(dir.path())
            .reconcile(&[observation(1.)])
            .unwrap();
        assert_eq!(confirmed.rev, 1);
        assert_eq!(confirmed.monitors[0].uid, published.entries[0].uid);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
