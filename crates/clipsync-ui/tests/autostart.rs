//! §12.9 item 4 (Flux): the `.desktop` carries `Exec` = `$APPIMAGE` when set, else the executable,
//! always with `--hidden`; enabling and disabling go through the directory of `UiOptions`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

use clipsync_ui::autostart::{self, DESKTOP_FILE, Launcher};

/// An absolute path on the platform running the test (`/x/y` on Unix, `C:\x\y` on Windows).
fn abs(unix: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", unix.replace('/', "\\")))
    } else {
        PathBuf::from(unix)
    }
}

fn exe() -> io::Result<PathBuf> {
    Ok(abs("/usr/bin/clipsync-app"))
}

#[test]
fn appimage_wins_over_the_executable_when_set_and_non_empty() {
    // `set_var` is unsafe in edition 2024 (forbidden here), so the choice is fed its inputs.
    let appimage = abs("/opt/ClipSync.AppImage");
    let with = Launcher::from_env(Some(appimage.clone().into_os_string()), exe()).unwrap();
    assert_eq!(with.program, appimage);
    let without = Launcher::from_env(None, exe()).unwrap();
    assert_eq!(without.program, abs("/usr/bin/clipsync-app"));
    let empty = Launcher::from_env(Some(OsString::new()), exe()).unwrap();
    assert_eq!(empty.program, abs("/usr/bin/clipsync-app"));
    let neither = Launcher::from_env(None, Err(io::Error::other("sem exe")));
    assert!(neither.is_err());
}

#[test]
fn a_relative_appimage_falls_back_to_the_executable() {
    // the session would resolve a relative path from some other directory
    for relative in ["app.AppImage", "./app.AppImage", "../bin/app.AppImage"] {
        let l = Launcher::from_env(Some(OsString::from(relative)), exe()).unwrap();
        assert_eq!(l.program, abs("/usr/bin/clipsync-app"), "{relative}");
    }
}

#[test]
fn a_path_with_a_control_character_is_refused() {
    for bad in [
        "/tmp/x\nExec=/tmp/evil\nX-Injected=1",
        "/tmp/x\rExec=/tmp/evil",
        "/tmp/x\0y",
        "/tmp/x\ty",
        "/tmp/x\u{7f}y",
        "/tmp/x\u{85}y",
    ] {
        let from_env = Launcher::from_env(Some(OsString::from(abs(bad))), exe());
        assert_eq!(from_env, Err(autostart::BAD_PATH.to_string()), "{bad:?}");
        assert!(Launcher::new(abs(bad)).check().is_err(), "{bad:?}");
        // the executable gets the same check when $APPIMAGE is not used
        let from_exe = Launcher::from_env(None, Ok(abs(bad)));
        assert_eq!(from_exe, Err(autostart::BAD_PATH.to_string()), "{bad:?}");
    }
    assert!(
        Launcher::new(abs("/opt/Clip Sync/çã.AppImage"))
            .check()
            .is_ok()
    );
}

#[test]
fn enabling_with_a_bad_path_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("autostart");
    let bad = Launcher::new(abs("/tmp/x\nExec=/tmp/evil\nX-Injected=1"));
    let error = autostart::enable_in(&target, &bad).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!autostart::is_enabled_in(&target));
    assert!(!target.exists(), "not even the directory");
}

#[test]
fn exec_line_quotes_the_program_and_adds_hidden() {
    let l = Launcher::new("/opt/ClipSync.AppImage");
    assert_eq!(l.exec_line(), "\"/opt/ClipSync.AppImage\" --hidden");
    let spaced = Launcher::new("/home/jo ao/Apps/Clip$ync 100%.AppImage");
    assert_eq!(
        spaced.exec_line(),
        "\"/home/jo ao/Apps/Clip\\$ync 100%%.AppImage\" --hidden"
    );
    let entry = l.desktop_entry();
    assert!(entry.starts_with("[Desktop Entry]\n"));
    assert!(entry.contains("\nType=Application\n"));
    assert!(entry.contains("\nName=ClipSync\n"));
    assert!(entry.contains("\nExec=\"/opt/ClipSync.AppImage\" --hidden\n"));
    assert!(entry.contains("\nTerminal=false\n"));
    assert_eq!(
        Launcher::new(r"C:\Program Files\ClipSync\clipsync-app.exe").run_value(),
        "\"C:\\Program Files\\ClipSync\\clipsync-app.exe\" --hidden"
    );
}

#[test]
fn enable_writes_the_desktop_file_and_disable_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("autostart"); // created on demand
    assert!(!autostart::is_enabled_in(&target));
    let path = autostart::enable_in(&target, &Launcher::new("/opt/ClipSync.AppImage")).unwrap();
    assert_eq!(path, target.join(DESKTOP_FILE));
    assert!(autostart::is_enabled_in(&target));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Exec=\"/opt/ClipSync.AppImage\" --hidden"));
    let leftovers: Vec<_> = std::fs::read_dir(&target).unwrap().flatten().collect();
    assert_eq!(leftovers.len(), 1, "no temporary file left behind");
    autostart::disable_in(&target).unwrap();
    assert!(!autostart::is_enabled_in(&target));
    autostart::disable_in(&target).unwrap(); // already absent is fine
}

#[test]
fn set_and_is_enabled_round_trip_through_the_options_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!autostart::is_enabled(Some(dir.path())));
    autostart::set(true, Some(dir.path())).unwrap();
    assert!(autostart::is_enabled(Some(dir.path())));
    let text = std::fs::read_to_string(dir.path().join(DESKTOP_FILE)).unwrap();
    // without `$APPIMAGE` the entry points at this very test binary; with it, at the AppImage
    let expected = Launcher::detect().unwrap();
    assert!(text.contains(&format!("Exec={}\n", expected.exec_line())));
    assert!(text.contains("--hidden"));
    autostart::set(false, Some(dir.path())).unwrap();
    assert!(!autostart::is_enabled(Some(dir.path())));
}
