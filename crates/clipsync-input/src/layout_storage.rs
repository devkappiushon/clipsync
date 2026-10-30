//! Cooperative nonblocking transactions and recoverable migration.
use crate::{Layout, LayoutError, layout_document::LayoutDocument};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct LayoutStorage {
    profile: PathBuf,
}
pub struct LayoutGuard {
    profile: PathBuf,
    _lock: File,
}
impl LayoutStorage {
    pub fn new(profile: &Path) -> Self {
        Self {
            profile: profile.to_owned(),
        }
    }
    pub fn try_lock(&self) -> Result<LayoutGuard, LayoutError> {
        private_dir(&self.profile).map_err(io_error)?;
        let lock = lock_file(&self.profile.join("layout.lock")).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                LayoutError::Busy
            } else {
                io_error(e)
            }
        })?;
        Ok(LayoutGuard {
            profile: self.profile.clone(),
            _lock: lock,
        })
    }
}
impl LayoutGuard {
    pub fn read(&self) -> Result<Option<LayoutDocument>, LayoutError> {
        LayoutDocument::read(&self.profile.join("layout.toml"))
    }
    /// A post-rename sync failure returns PublishedUnconfirmed with the visible
    /// revision. Keep affected routes suspended; do not retry or claim rollback.
    pub fn write(&self, document: &LayoutDocument) -> Result<(), LayoutError> {
        let text = document.to_toml_string()?;
        let path = self.profile.join("layout.toml");
        if let Some(current) = self.read()? {
            if current.organization.is_some() && document.organization.is_none() {
                return Err(LayoutError::InvalidField(
                    "use recuperação explícita para restaurar v1".into(),
                ));
            }
            if current.organization.is_none() && document.organization.is_some() {
                let original = fs::read(&path).map_err(io_error)?;
                let backup = self.profile.join("layout.v1.backup.toml");
                match exclusive_write(&backup, &original) {
                    Ok(()) => {}
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        regular_file(&backup).map_err(io_error)?;
                        if fs::read(&backup).map_err(io_error)? != original {
                            return Err(LayoutError::BackupConflict);
                        }
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            File::open(&backup)
                                .and_then(|f| f.set_permissions(fs::Permissions::from_mode(0o600)))
                                .map_err(io_error)?;
                        }
                        confirm_existing(&backup).map_err(io_error)?;
                    }
                    Err(e) => return Err(io_error(e)),
                }
            }
        }
        atomic_write(&path, text.as_bytes()).map_err(|e| publication_error(e, document.legacy.rev))
    }
    /// PublishedUnconfirmed reports the restored v1 revision (base_rev + 1),
    /// even though durability and subsequent activation remain unconfirmed.
    pub fn restore_v1(&self, base_rev: u64) -> Result<u64, LayoutError> {
        let current = self
            .read()?
            .ok_or_else(|| LayoutError::InvalidField("layout ausente".into()))?;
        if current.legacy.rev != base_rev {
            return Err(LayoutError::StaleRev);
        }
        if current.organization.is_none() {
            return Err(LayoutError::InvalidField("layout já está em v1".into()));
        }
        let backup = self.profile.join("layout.v1.backup.toml");
        regular_file(&backup).map_err(io_error)?;
        let mut legacy = Layout::read(&backup)?
            .ok_or_else(|| LayoutError::InvalidField("backup ausente".into()))?;
        legacy.rev = base_rev
            .checked_add(1)
            .ok_or_else(|| LayoutError::InvalidField("revisão esgotada".into()))?;
        let text = legacy.to_toml_string()?;
        let path = self.profile.join("layout.toml");
        exclusive_write(
            &self
                .profile
                .join(format!("layout.v2-before-recovery.{base_rev}.toml")),
            &fs::read(&path).map_err(io_error)?,
        )
        .map_err(io_error)?;
        atomic_write(&path, text.as_bytes()).map_err(|e| publication_error(e, legacy.rev))?;
        Ok(legacy.rev)
    }
}
/// Publication state is independent of whether the filesystem confirmed durability.
/// Callers must not retry a published write or claim the old bytes survived.
#[derive(Debug)]
pub(crate) enum AtomicWriteError {
    Unpublished(io::Error),
    PublishedUnconfirmed(io::Error),
}
fn publication_error(error: AtomicWriteError, rev: u64) -> LayoutError {
    match error {
        AtomicWriteError::Unpublished(e) => io_error(e),
        AtomicWriteError::PublishedUnconfirmed(_) => LayoutError::PublishedUnconfirmed { rev },
    }
}
fn io_error(_: io::Error) -> LayoutError {
    LayoutError::Io("falha no armazenamento privado do layout".into())
}
pub(crate) fn private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    if !fs::symlink_metadata(path)?.is_dir() {
        return Err(io::Error::other("private directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub(crate) fn regular_file(path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("private regular file"));
    }
    Ok(())
}
fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
pub(crate) fn lock_file(path: &Path) -> io::Result<File> {
    let file = match options().create_new(true).open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            regular_file(path)?;
            options().open(path)?
        }
        Err(e) => return Err(e),
    };
    file.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
        std::fs::TryLockError::Error(e) => e,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}
pub(crate) fn exclusive_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = options().create_new(true).open(path)?;
    if let Err(e) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    #[cfg(test)]
    faults::check(faults::Stage::ConfirmExclusive)?;
    sync_parent(path)
}
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AtomicWriteError> {
    let temp = path.with_extension(format!(
        "{}.tmp",
        clipsync_proto::MonitorUid::random().to_hex()
    ));
    exclusive_write(&temp, bytes).map_err(AtomicWriteError::Unpublished)?;
    let rename = || {
        #[cfg(test)]
        faults::check(faults::Stage::BeforeRename)?;
        fs::rename(&temp, path)
    };
    if let Err(e) = rename() {
        let _ = fs::remove_file(&temp);
        return Err(AtomicWriteError::Unpublished(e));
    }
    confirm_parent(path).map_err(AtomicWriteError::PublishedUnconfirmed)
}
/// Reading matching bytes does not confirm an earlier failed sync. Reuse only
/// after both the file and its directory entry have been confirmed again.
pub(crate) fn confirm_existing(path: &Path) -> io::Result<()> {
    regular_file(path)?;
    options().open(path)?.sync_all()?;
    confirm_parent(path)
}
fn confirm_parent(path: &Path) -> io::Result<()> {
    #[cfg(test)]
    faults::check(faults::Stage::ConfirmPublication)?;
    sync_parent(path)
}
fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path.parent().ok_or_else(|| io::Error::other("parent"))?)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

// Fault injection is thread-local and compiled only into unit tests. It exercises
// the real file/backup/rename paths without exposing a test API in production.
#[cfg(test)]
pub(crate) mod faults {
    use std::{cell::Cell, io};
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Stage {
        BeforeRename,
        ConfirmPublication,
        ConfirmExclusive,
    }
    thread_local! {static FAILURE:Cell<Option<Stage>>=const {Cell::new(None)};}
    pub(crate) fn check(stage: Stage) -> io::Result<()> {
        FAILURE.with(|slot| {
            if slot.get() == Some(stage) {
                slot.set(None);
                Err(io::Error::other("falha injetada"))
            } else {
                Ok(())
            }
        })
    }
    pub(crate) fn during<T>(stage: Stage, operation: impl FnOnce() -> T) -> T {
        struct Reset(Option<Stage>);
        impl Drop for Reset {
            fn drop(&mut self) {
                FAILURE.with(|slot| slot.set(self.0));
            }
        }
        let _reset = Reset(FAILURE.with(|slot| slot.replace(Some(stage))));
        operation()
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::layout_document::Organization;
    use faults::{Stage, during};
    fn document(rev: u64, v2: bool) -> LayoutDocument {
        LayoutDocument {
            legacy: Layout {
                rev,
                ..Layout::default()
            },
            organization: v2.then(Organization::default),
        }
    }
    #[test]
    fn fix1_write_distinguishes_unpublished_and_published_unconfirmed() {
        for stage in [Stage::BeforeRename, Stage::ConfirmPublication] {
            let dir = tempfile::tempdir().unwrap();
            let guard = LayoutStorage::new(dir.path()).try_lock().unwrap();
            guard.write(&document(7, false)).unwrap();
            let path = dir.path().join("layout.toml");
            let previous = fs::read(&path).unwrap();
            let error = during(stage, || guard.write(&document(8, true))).unwrap_err();
            assert_eq!(
                fs::read(dir.path().join("layout.v1.backup.toml")).unwrap(),
                previous
            );
            if stage == Stage::BeforeRename {
                assert_eq!(fs::read(&path).unwrap(), previous);
                assert_eq!(guard.read().unwrap().unwrap().legacy.rev, 7);
                assert_eq!(error.code(), "io");
            } else {
                assert_eq!(
                    fs::read_to_string(&path).unwrap(),
                    document(8, true).to_toml_string().unwrap()
                );
                assert_eq!(guard.read().unwrap().unwrap().legacy.rev, 8);
                assert_eq!(error.code(), "published_unconfirmed");
                assert_eq!(error, LayoutError::PublishedUnconfirmed { rev: 8 });
            }
        }
    }
    #[test]
    fn fix1_restore_distinguishes_unpublished_and_published_unconfirmed() {
        for stage in [Stage::BeforeRename, Stage::ConfirmPublication] {
            let dir = tempfile::tempdir().unwrap();
            let guard = LayoutStorage::new(dir.path()).try_lock().unwrap();
            guard.write(&document(7, false)).unwrap();
            guard.write(&document(8, true)).unwrap();
            let path = dir.path().join("layout.toml");
            let previous = fs::read(&path).unwrap();
            let error = during(stage, || guard.restore_v1(8)).unwrap_err();
            assert_eq!(
                fs::read(dir.path().join("layout.v2-before-recovery.8.toml")).unwrap(),
                previous
            );
            let current = guard.read().unwrap().unwrap();
            if stage == Stage::BeforeRename {
                assert_eq!(fs::read(&path).unwrap(), previous);
                assert_eq!(current, document(8, true));
                assert_eq!(error.code(), "io");
            } else {
                assert_eq!(
                    fs::read_to_string(&path).unwrap(),
                    document(9, false).to_toml_string().unwrap()
                );
                assert_eq!(current, document(9, false));
                assert_eq!(error.code(), "published_unconfirmed");
                assert_eq!(error, LayoutError::PublishedUnconfirmed { rev: 9 });
            }
        }
    }
    #[test]
    fn fix1_reused_backup_is_synced_before_publishing_the_layout() {
        let dir = tempfile::tempdir().unwrap();
        let guard = LayoutStorage::new(dir.path()).try_lock().unwrap();
        guard.write(&document(7, false)).unwrap();
        let path = dir.path().join("layout.toml");
        let bytes = fs::read(&path).unwrap();
        let backup = dir.path().join("layout.v1.backup.toml");
        let failed_backup =
            during(Stage::ConfirmExclusive, || guard.write(&document(8, true))).unwrap_err();
        assert_eq!(failed_backup.code(), "io");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&backup).unwrap(), bytes);
        let failed_reuse = during(Stage::ConfirmPublication, || {
            guard.write(&document(8, true))
        })
        .unwrap_err();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(failed_reuse.code(), "io");
        guard.write(&document(8, true)).unwrap();
        assert_eq!(guard.read().unwrap(), Some(document(8, true)));
        assert_eq!(fs::read(&backup).unwrap(), bytes);
    }
}
