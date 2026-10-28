//! `diagnose()`: portal versions, RemoteDesktop devices and Input Leap / Barrier conflicts.
use clipsync_proto::{DiagItem, DiagStatus};

const AREA: &str = "input";
/// Process names (`/proc/*/comm`) of tools that grab input globally.
const CONFLICTS: [&str; 6] = [
    "input-leaps",
    "input-leapc",
    "barriers",
    "barrierc",
    "synergys",
    "deskflow-core",
];

fn item(name: &str, status: DiagStatus, detail: impl Into<String>) -> DiagItem {
    DiagItem {
        area: AREA.into(),
        name: name.into(),
        status,
        detail: detail.into(),
    }
}

pub async fn diagnose() -> Vec<DiagItem> {
    let mut out = Vec::new();
    #[cfg(target_os = "linux")]
    {
        use ashpd::desktop::input_capture::InputCapture;
        use ashpd::desktop::remote_desktop::RemoteDesktop;
        match InputCapture::new().await {
            // Never `supported_capabilities()`: GNOME 50 returns an unknown bit and the parse fails.
            Ok(ic) => out.push(item(
                "portal InputCapture",
                DiagStatus::Ok,
                format!("versão {}", ic.version()),
            )),
            Err(e) => out.push(item(
                "portal InputCapture",
                DiagStatus::Missing,
                format!("indisponível: {e}"),
            )),
        }
        match RemoteDesktop::new().await {
            Ok(rd) => {
                let types = rd
                    .available_device_types()
                    .await
                    .map_or_else(|e| format!("não lido: {e}"), |t| format!("{t:?}"));
                out.push(item(
                    "portal RemoteDesktop",
                    DiagStatus::Ok,
                    format!("versão {}; dispositivos {types}", rd.version()),
                ));
            }
            Err(e) => out.push(item(
                "portal RemoteDesktop",
                DiagStatus::Missing,
                format!("indisponível: {e}"),
            )),
        }
        // The screen lock is the logind's `LockedHint` (§13.6 item 8).
        let watch = crate::adapter::session::LockWatch::start(None).await;
        out.push(if watch.available() {
            item(
                "tela bloqueada",
                DiagStatus::Ok,
                if watch.is_locked() {
                    "bloqueada agora (logind)"
                } else {
                    "desbloqueada (logind)"
                },
            )
        } else {
            item(
                "tela bloqueada",
                DiagStatus::Warn,
                "o logind não respondeu: a tela é tratada como desbloqueada",
            )
        });
    }
    #[cfg(windows)]
    out.push(item(
        "teclado e mouse",
        DiagStatus::Warn,
        "captura e emulação do Windows prontas, ainda sem prova nesta máquina",
    ));
    #[cfg(not(any(target_os = "linux", windows)))]
    out.push(item(
        "teclado e mouse",
        DiagStatus::Missing,
        crate::msg::UNAVAILABLE_ON_PLATFORM,
    ));
    let outputs = crate::adapter::outputs::connected_outputs();
    out.push(item(
        "monitores",
        if outputs.is_empty() {
            DiagStatus::Warn
        } else {
            DiagStatus::Ok
        },
        if outputs.is_empty() {
            "nenhum conector DRM conectado encontrado".to_owned()
        } else {
            outputs.join(", ")
        },
    ));
    let found = running_conflicts();
    if found.is_empty() {
        out.push(item(
            "Input Leap / Barrier",
            DiagStatus::Ok,
            "nenhum processo capturando entrada",
        ));
    } else {
        out.push(item(
            "Input Leap / Barrier",
            DiagStatus::Warn,
            format!(
                "em execução: {}; podem disputar a captura",
                found.join(", ")
            ),
        ));
    }
    out
}

pub(crate) fn running_conflicts() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut found: Vec<String> = dir
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path().join("comm")).ok())
        .map(|c| c.trim().to_owned())
        .filter(|c| CONFLICTS.contains(&c.as_str()))
        .collect();
    found.sort();
    found.dedup();
    found
}
