//! System diagnostics: only detects and explains; changes nothing (Input Leap and Barrier are never touched).
use clipsync_proto::{DiagItem, DiagStatus};
use std::fs;
use std::path::Path;

const AREA: &str = "sistema";

fn item(name: &str, status: DiagStatus, detail: impl Into<String>) -> DiagItem {
    DiagItem {
        area: AREA.into(),
        name: name.into(),
        status,
        detail: detail.into(),
    }
}

pub async fn system() -> Vec<DiagItem> {
    tokio::task::spawn_blocking(system_blocking)
        .await
        .unwrap_or_else(|_| {
            vec![item(
                "diagnóstico",
                DiagStatus::Error,
                "a verificação do sistema falhou",
            )]
        })
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn os_pretty_name() -> Option<String> {
    let text = fs::read_to_string("/etc/os-release").ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_owned())
}

fn system_blocking() -> Vec<DiagItem> {
    let mut out = Vec::new();
    out.push(match os_pretty_name() {
        Some(n) => item("sistema operacional", DiagStatus::Ok, n),
        None => item(
            "sistema operacional",
            DiagStatus::Warn,
            "/etc/os-release ilegível",
        ),
    });
    out.push(
        match (env("XDG_SESSION_TYPE"), env("XDG_CURRENT_DESKTOP")) {
            (Some(t), d) => item(
                "sessão gráfica",
                DiagStatus::Ok,
                format!("{t}{}", d.map(|d| format!(" · {d}")).unwrap_or_default()),
            ),
            (None, _) => item(
                "sessão gráfica",
                DiagStatus::Warn,
                "XDG_SESSION_TYPE indefinido (sem sessão gráfica neste terminal?)",
            ),
        },
    );
    out.push(match env("DISPLAY") {
        Some(d) => {
            let n = d
                .rsplit(':')
                .next()
                .and_then(|r| r.split('.').next())
                .unwrap_or("0");
            let socket = format!("/tmp/.X11-unix/X{n}");
            if Path::new(&socket).exists() {
                item(
                    "DISPLAY / XWayland",
                    DiagStatus::Ok,
                    format!("DISPLAY={d}; servidor X11 respondendo no socket"),
                )
            } else {
                item(
                    "DISPLAY / XWayland",
                    DiagStatus::Warn,
                    format!("DISPLAY={d}, mas o socket {socket} não existe"),
                )
            }
        }
        None => item(
            "DISPLAY / XWayland",
            DiagStatus::Missing,
            "DISPLAY indefinido: a ponte de clipboard exige XWayland ativo",
        ),
    });
    let procs = process_names();
    out.push(detect(
        "Input Leap",
        &["input-leap", "inputleap", "input_leap"],
        &procs,
        &["io.github.input_leap.input-leap"],
    ));
    out.push(detect("Barrier", &["barrier"], &procs, &["barrier"]));
    out
}

/// Command lines of this user's processes (`/proc/*/cmdline`, NUL-separated), lowercased.
fn process_names() -> Vec<String> {
    let Ok(dir) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    dir.flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter_map(|e| fs::read(e.path().join("cmdline")).ok())
        .map(|raw| {
            String::from_utf8_lossy(&raw)
                .replace('\0', " ")
                .to_lowercase()
        })
        .filter(|c| !c.trim().is_empty())
        .collect()
}

fn detect(name: &str, needles: &[&str], procs: &[String], flatpak_ids: &[&str]) -> DiagItem {
    // the executable name is the first word; flatpak runs show the app id inside the cmdline
    let hit = |c: &&String| {
        let exe = c.split_whitespace().next().unwrap_or("");
        let exe = exe.rsplit('/').next().unwrap_or(exe);
        needles.iter().any(|n| exe.starts_with(n))
    };
    let running = procs.iter().filter(hit).count();
    let flatpak_hit = procs
        .iter()
        .any(|c| c.contains("flatpak") && flatpak_ids.iter().any(|id| c.contains(id)));
    let flatpak_installed = installed_flatpak(flatpak_ids);
    let how = if flatpak_installed || flatpak_hit {
        " (Flatpak)"
    } else {
        ""
    };
    if running > 0 || flatpak_hit {
        item(
            name,
            DiagStatus::Ok,
            format!(
                "em execução{how}: só detectado; o ClipSync não o altera e as duas ferramentas podem competir pelo mesmo mouse e teclado"
            ),
        )
    } else if flatpak_installed {
        item(name, DiagStatus::Ok, "instalado (Flatpak), parado")
    } else {
        item(name, DiagStatus::Warn, "não detectado")
    }
}

fn installed_flatpak(ids: &[&str]) -> bool {
    let user = env("HOME").map(|h| format!("{h}/.local/share/flatpak/app"));
    ["/var/lib/flatpak/app".to_owned()]
        .into_iter()
        .chain(user)
        .any(|base| ids.iter().any(|id| Path::new(&base).join(id).exists()))
}
