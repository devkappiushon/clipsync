//! Start with the session (contrato §12.5, `prefs_set {autostart}`). Owned by Flux (§12.1).
//!
//! Interface used by `prefs.rs` (Rivet): `set(enabled, dir)` and `is_enabled(dir)`. `dir` is
//! `UiOptions::autostart_dir`; with `Some`, the file-based entry goes there on every platform
//! (tests never touch the registry). `None` = the platform default. Errors carry a Portuguese
//! reason, shown as is (code `io`). Default off: the user turns it on (explicit consent).
//!
//! Linux: `<$XDG_CONFIG_HOME or ~/.config>/autostart/clipsync.desktop`, written by tmp + rename,
//! with `Exec` = `$APPIMAGE` when set (the AppImage runtime sets it; the mounted executable
//! disappears on exit), else the running executable, plus `--hidden` (§12.7 rule 6).
//! A relative `$APPIMAGE` is ignored (the session would resolve it from another directory); a path
//! that is not UTF-8 or holds a control character (a line break would inject a `.desktop` key) is
//! refused, never written.
//! Windows: value `ClipSync` in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` via `reg.exe`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

pub(crate) mod fsio;

pub const DESKTOP_FILE: &str = "clipsync.desktop";
pub const HIDDEN_FLAG: &str = "--hidden";

// User-facing text (pt-BR); `msg.rs` is Rivet's, so the autostart texts live here.
pub const NO_HOME: &str = "não foi possível achar a pasta de autostart: HOME não está definido";
pub const BAD_PATH: &str = "não dá para ligar o início automático: o caminho do executável tem caractere de controle ou não é texto válido";
fn no_exe(detail: &str) -> String {
    format!("não foi possível descobrir o executável do ClipSync: {detail}")
}
fn io_text(error: &io::Error) -> String {
    // InvalidInput is `check` refusing the path: its text already says everything.
    if error.kind() == io::ErrorKind::InvalidInput {
        return error.to_string();
    }
    format!("não foi possível gravar o autostart: {error}")
}

/// How the app is started at login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    /// The AppImage when `$APPIMAGE` is set, else the executable.
    pub program: PathBuf,
}

impl Launcher {
    pub fn new(program: impl Into<PathBuf>) -> Launcher {
        Launcher {
            program: program.into(),
        }
    }

    /// Reads `$APPIMAGE` and `current_exe()`; the choice itself is `from_env`.
    pub fn detect() -> Result<Launcher, String> {
        Launcher::from_env(env::var_os("APPIMAGE"), env::current_exe())
    }

    /// `$APPIMAGE` (non-empty and absolute) wins over the executable. The chosen path must pass
    /// `check`.
    pub fn from_env(
        appimage: Option<OsString>,
        exe: io::Result<PathBuf>,
    ) -> Result<Launcher, String> {
        let launcher = match appimage
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            Some(appimage) => Launcher::new(appimage),
            None => exe.map(Launcher::new).map_err(|e| no_exe(&e.to_string()))?,
        };
        launcher.check()?;
        Ok(launcher)
    }

    /// The program path must be UTF-8 without control characters (C0, DEL, C1): a line break
    /// would end the `Exec=` line and let the rest become another `.desktop` key.
    pub fn check(&self) -> Result<(), String> {
        match self.program.to_str() {
            Some(text) if !text.chars().any(char::is_control) => Ok(()),
            _ => Err(BAD_PATH.to_string()),
        }
    }

    /// The `Exec=` value: the program quoted by the Desktop Entry rules, then `--hidden`.
    pub fn exec_line(&self) -> String {
        format!(
            "{} {HIDDEN_FLAG}",
            desktop_quote(&self.program.to_string_lossy())
        )
    }

    /// Full `.desktop` file for the autostart directory.
    pub fn desktop_entry(&self) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=ClipSync\n\
             Comment=Teclado, mouse e clipboard entre computadores\n\
             Exec={}\n\
             Icon=clipsync\n\
             Terminal=false\n\
             Categories=Utility;\n\
             X-GNOME-Autostart-enabled=true\n",
            self.exec_line()
        )
    }

    /// Windows `Run` value: the executable quoted, then `--hidden`.
    pub fn run_value(&self) -> String {
        format!("\"{}\" {HIDDEN_FLAG}", self.program.to_string_lossy())
    }
}

/// Desktop Entry `Exec` quoting: the argument between double quotes; `"`, `` ` `` and `$` escaped
/// with a backslash; a literal backslash needs four (string escape, then quoting escape); `%`
/// doubled so it is not a field code.
fn desktop_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '`' | '$' => {
                out.push('\\');
                out.push(c);
            }
            '\\' => out.push_str("\\\\\\\\"),
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Linux default: `$XDG_CONFIG_HOME/autostart`, else `~/.config/autostart`.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(xdg).join("autostart"));
    }
    env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".config").join("autostart"))
}

/// Writes `clipsync.desktop` in `dir` (new temp file + rename, mode 0644) and returns its path.
pub fn enable_in(dir: &Path, launcher: &Launcher) -> io::Result<PathBuf> {
    launcher
        .check()
        .map_err(|why| io::Error::new(io::ErrorKind::InvalidInput, why))?;
    fs::create_dir_all(dir)?;
    let path = dir.join(DESKTOP_FILE);
    fsio::write_atomic(&path, launcher.desktop_entry().as_bytes(), 0o644)?;
    Ok(path)
}

/// Removes `clipsync.desktop` from `dir`; already absent is fine.
pub fn disable_in(dir: &Path) -> io::Result<()> {
    match fs::remove_file(dir.join(DESKTOP_FILE)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

pub fn is_enabled_in(dir: &Path) -> bool {
    dir.join(DESKTOP_FILE).is_file()
}

fn set_in(dir: &Path, enabled: bool) -> Result<(), String> {
    if enabled {
        let launcher = Launcher::detect()?;
        enable_in(dir, &launcher)
            .map(|_| ())
            .map_err(|e| io_text(&e))
    } else {
        disable_in(dir).map_err(|e| io_text(&e))
    }
}

/// Turns autostart on or off. `dir` = `UiOptions::autostart_dir`; `None` = the platform default.
pub fn set(enabled: bool, dir: Option<&Path>) -> Result<(), String> {
    match dir {
        Some(dir) => set_in(dir, enabled),
        None => platform::set_default(enabled),
    }
}

/// What `prefs_get` reports: read from the system, not from `ui.toml`.
pub fn is_enabled(dir: Option<&Path>) -> bool {
    match dir {
        Some(dir) => is_enabled_in(dir),
        None => platform::is_enabled_default(),
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{NO_HOME, default_dir, is_enabled_in, set_in};

    pub(super) fn set_default(enabled: bool) -> Result<(), String> {
        let dir = default_dir().ok_or_else(|| NO_HOME.to_string())?;
        set_in(&dir, enabled)
    }

    pub(super) fn is_enabled_default() -> bool {
        default_dir().is_some_and(|dir| is_enabled_in(&dir))
    }
}

#[cfg(windows)]
mod platform {
    use std::process::Command;

    use super::Launcher;

    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = "ClipSync";

    fn reg(args: &[&str]) -> Result<(), String> {
        let output = Command::new("reg.exe")
            .args(args)
            .output()
            .map_err(|e| format!("não foi possível executar o reg.exe: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let detail = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "o Windows recusou a alteração do autostart: {}",
                detail.trim()
            ))
        }
    }

    pub(super) fn set_default(enabled: bool) -> Result<(), String> {
        if enabled {
            let value = Launcher::detect()?.run_value();
            reg(&[
                "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &value, "/f",
            ])
        } else if is_enabled_default() {
            reg(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
        } else {
            Ok(())
        }
    }

    pub(super) fn is_enabled_default() -> bool {
        Command::new("reg.exe")
            .args(["query", RUN_KEY, "/v", RUN_VALUE])
            .output()
            .is_ok_and(|output| output.status.success())
    }
}
