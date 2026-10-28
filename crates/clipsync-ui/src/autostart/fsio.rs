//! File writes for the files this crate owns (`ui.toml`, the autostart `.desktop`, the support
//! bundle). The temp file is always created new (`O_CREAT | O_EXCL`), which fails on a symlink
//! sitting at that name instead of following it, so a planted link never redirects the write.
//! Lives under `autostart` because `lib.rs` is Rivet's; the module is `pub(crate)` for the others.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Temp names tried per write before giving up.
const ATTEMPTS: u64 = 8;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Creates a file that must not exist yet. `mode` is the exact Unix mode (set after the create, so
/// the umask cannot widen or narrow it); on Windows the file inherits the folder's ACL.
pub(crate) fn create_new(path: &Path, mode: u32) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }
    let file = opts.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(file)
}

fn tmp_path(path: &Path, n: u64) -> PathBuf {
    let name = path.file_name().map_or_else(
        || "arquivo".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{name}.tmp-{}-{n}", std::process::id()))
}

/// Writes `data` to `path` through a new temp file (`mode`), `fsync` and an atomic rename.
pub(crate) fn write_atomic(path: &Path, data: &[u8], mode: u32) -> io::Result<()> {
    write_atomic_from(
        path,
        data,
        mode,
        NEXT.fetch_add(ATTEMPTS, Ordering::Relaxed),
    )
}

/// `write_atomic` with the first temp name chosen by the caller (tests plant links there).
fn write_atomic_from(path: &Path, data: &[u8], mode: u32, first: u64) -> io::Result<()> {
    for n in first..first + ATTEMPTS {
        let tmp = tmp_path(path, n);
        let mut file = match create_new(&tmp, mode) {
            Ok(file) => file,
            // A leftover or a planted link: leave it alone and try the next name.
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let written = file.write_all(data).and_then(|()| file.sync_all());
        drop(file);
        let done = written.and_then(|()| fs::rename(&tmp, path));
        if done.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        return done;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "não sobrou nome livre para o arquivo temporário",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_replaces_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui.toml");
        write_atomic(&path, b"um", 0o600).unwrap();
        write_atomic(&path, b"dois", 0o600).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"dois");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn mode_is_exact_whatever_the_umask() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        for mode in [0o600, 0o644] {
            let path = dir.path().join(format!("f{mode:o}"));
            write_atomic(&path, b"x", mode).unwrap();
            let got = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(got, mode);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_at_the_temp_name_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("vitima.txt");
        fs::write(&victim, b"intacta").unwrap();
        let path = dir.path().join("ui.toml");
        // Links at the first two temp names, one to the victim and one dangling.
        std::os::unix::fs::symlink(&victim, tmp_path(&path, 100)).unwrap();
        std::os::unix::fs::symlink(dir.path().join("nada"), tmp_path(&path, 101)).unwrap();
        write_atomic_from(&path, b"novo", 0o600, 100).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"intacta");
        assert!(!dir.path().join("nada").exists());
        assert_eq!(fs::read(&path).unwrap(), b"novo");
    }

    #[cfg(unix)]
    #[test]
    fn gives_up_when_every_temp_name_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui.toml");
        for n in 200..200 + ATTEMPTS {
            fs::write(tmp_path(&path, n), b"x").unwrap();
        }
        let e = write_atomic_from(&path, b"novo", 0o600, 200).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
        assert!(!path.exists());
    }

    #[test]
    fn create_new_refuses_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a");
        create_new(&path, 0o600).unwrap();
        let e = create_new(&path, 0o600).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
    }
}
