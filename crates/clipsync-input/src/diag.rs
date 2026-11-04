//! `diagnose()`: portal versions, RemoteDesktop devices and Input Leap / Barrier conflicts.
use clipsync_proto::{AgentStatus, DiagItem, DiagStatus, ModuleKind, PeerId};
use std::path::Path;

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
                "o logind não respondeu: o controle permanece bloqueado até confirmar o estado da tela",
            )
        });
    }
    #[cfg(windows)]
    {
        // Nothing here installs a hook or moves the cursor: it only names what will run.
        out.push(item(
            "teclado e mouse",
            DiagStatus::Warn,
            "captura e emulação do Windows prontas, ainda sem prova nesta máquina",
        ));
        out.push(item(
            "captura",
            DiagStatus::Ok,
            "win-hooks: ganchos de teclado e mouse só com ligação no layout e o módulo ligado",
        ));
        out.push(item(
            "emulação",
            DiagStatus::Ok,
            "win-sendinput: movimento absoluto no desktop virtual, teclas pela posição física",
        ));
        out.push(item("layout do teclado", DiagStatus::Ok, windows_layout()));
        // `diagnose()` knows neither the profile nor this machine's id, so it cannot tell whether
        // a local link exists (§13.9): it says when they are active, never that they are.
        out.push(item(
            "atalhos",
            DiagStatus::Warn,
            "Win+Alt+1 a 9 (ir para o dispositivo), Win+Alt+L (travar) e Win+Alt+P (pausar): ativos quando há ligação desta máquina no layout; Win+Alt+C e Win+Alt+V: indisponível nesta versão",
        ));
    }
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

/// Diagnóstico somente de leitura com o estado que a CLI/UI recebeu pelo controle local.
/// Não lê chaves nem tokens, e não instala captura para inferir sua saúde.
pub async fn diagnose_for(
    profile_dir: &Path,
    self_id: Option<PeerId>,
    status: Option<&AgentStatus>,
) -> Vec<DiagItem> {
    let mut items = diagnose().await;
    let context = context_item(profile_dir, self_id, status);
    #[cfg(windows)]
    {
        items.retain(|i| i.area != AREA || i.name != "atalhos");
        items.push(item(
            "atalhos",
            context.status,
            format!(
                "{}; Win+Alt+1 a 9, Win+Alt+L e Win+Alt+P dependem da captura",
                context.detail
            ),
        ));
    }
    items.push(context);
    items
}

fn context_item(
    profile_dir: &Path,
    self_id: Option<PeerId>,
    status: Option<&AgentStatus>,
) -> DiagItem {
    let name = "travessia de saída";
    let Some(status) = status else {
        return item(
            name,
            DiagStatus::Warn,
            "não conferido: agente parado; inicie o ClipSync para consultar o estado deste perfil",
        );
    };
    let Some(id) = self_id else {
        return item(
            name,
            DiagStatus::Warn,
            "não conferido: identidade desta máquina não informada pelo agente",
        );
    };
    if !status
        .modules
        .iter()
        .any(|m| m.kind == ModuleKind::Input && m.enabled)
    {
        return item(
            name,
            DiagStatus::Warn,
            "inativos: módulo de input desligado",
        );
    }
    if status.paused {
        return item(
            name,
            DiagStatus::Warn,
            "inativos: ClipSync pausado; retome para testar a travessia",
        );
    }
    let layout = match crate::Layout::read(&profile_dir.join(crate::LAYOUT_FILE)) {
        Ok(layout) => layout,
        Err(_) => {
            return item(
                name,
                DiagStatus::Error,
                "não conferido: layout ilegível ou inválido",
            );
        }
    };
    let links = layout.as_ref().map_or(0, |l| {
        l.barriers_for(id)
            .iter()
            .filter(|b| status.peers.iter().any(|p| p.id == b.peer))
            .count()
    });
    if links == 0 {
        return item(
            name,
            DiagStatus::Warn,
            "inativos: nenhuma ligação desta máquina no layout; organize as telas no computador com o mouse físico. Uma máquina que só recebe não precisa de ligação local para receber",
        );
    }
    item(
        name,
        DiagStatus::Warn,
        format!(
            "{links} ligação(ões) local(is) configurada(s); captura e permissão no destino ainda não confirmadas. Teste a travessia no computador com o mouse físico"
        ),
    )
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

/// The keyboard layout of the Windows session, by its KLID (diagnostics only).
#[cfg(windows)]
fn windows_layout() -> String {
    match clipsync_win::keyboard_layout() {
        Some(klid) => {
            let name = match klid.as_str() {
                "00010416" => "Português (Brasil ABNT2)",
                "00000416" => "Português (Brasil ABNT)",
                "00000409" => "EUA",
                _ => "outro",
            };
            format!("{name} ({klid})")
        }
        None => "não foi possível ler o layout".to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod context_tests {
    use super::*;
    use crate::layout::{Endpoint, LayoutDevice, Link};
    use clipsync_proto::Edge;
    use clipsync_proto::{ModuleSet, ModuleStatus, PeerPerms, PeerState, PeerStatus};

    fn status() -> AgentStatus {
        AgentStatus {
            log_path: None,
            device_name: "origem".into(),
            id: PeerId([1; 32]),
            listen: "127.0.0.1:47800".parse().unwrap(),
            paused: false,
            peers: vec![PeerStatus {
                name: "destino".into(),
                id: PeerId([2; 32]),
                addr: None,
                state: PeerState::Connected,
                perms: PeerPerms::default(),
                remote_modules: ModuleSet {
                    input: true,
                    clipboard: false,
                },
                remote_version: 3,
            }],
            modules: vec![ModuleStatus {
                kind: ModuleKind::Input,
                enabled: true,
                adapter: None,
                authorized_peers: vec![],
                last_event: None,
                last_block: None,
            }],
            recent_blocks: vec![],
            recent_removals: vec![],
        }
    }
    fn save_link(dir: &Path, a: PeerId, b: PeerId) {
        let ep = |peer, edge| Endpoint {
            peer,
            monitor: "fake-0".into(),
            edge,
            from: 0.0,
            to: 1.0,
        };
        crate::Layout {
            devices: [a, b]
                .into_iter()
                .enumerate()
                .map(|(n, peer)| LayoutDevice {
                    peer,
                    name: format!("tela{n}"),
                    x: 0,
                    y: 0,
                    hotkey: None,
                })
                .collect(),
            links: vec![Link {
                id: "l1".into(),
                a: ep(a, Edge::Right),
                b: ep(b, Edge::Left),
            }],
            ..crate::Layout::default()
        }
        .write(&dir.join(crate::LAYOUT_FILE))
        .unwrap();
    }
    #[test]
    fn missing_context_does_not_invent_an_active_capture() {
        let dir = tempfile::tempdir().unwrap();
        let s = status();
        assert!(
            context_item(dir.path(), None, None)
                .detail
                .contains("agente parado")
        );
        assert!(
            context_item(dir.path(), None, Some(&s))
                .detail
                .contains("identidade")
        );
        assert!(!dir.path().join(crate::LAYOUT_FILE).exists());
    }
    #[test]
    fn context_distinguishes_paused_disabled_invalid_and_absent_layout() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = status();
        assert!(
            context_item(dir.path(), Some(s.id), Some(&s))
                .detail
                .contains("nenhuma ligação")
        );
        s.paused = true;
        assert!(
            context_item(dir.path(), Some(s.id), Some(&s))
                .detail
                .contains("pausado")
        );
        s.paused = false;
        s.modules[0].enabled = false;
        assert!(
            context_item(dir.path(), Some(s.id), Some(&s))
                .detail
                .contains("desligado")
        );
        s.modules[0].enabled = true;
        std::fs::write(dir.path().join(crate::LAYOUT_FILE), "[invalid").unwrap();
        assert_eq!(
            context_item(dir.path(), Some(s.id), Some(&s)).status,
            DiagStatus::Error
        );
    }
    #[test]
    fn only_trusted_local_links_count_and_configuration_is_not_hook_health() {
        let dir = tempfile::tempdir().unwrap();
        let s = status();
        for (a, b) in [(PeerId([2; 32]), PeerId([3; 32])), (s.id, PeerId([3; 32]))] {
            save_link(dir.path(), a, b);
            assert!(
                context_item(dir.path(), Some(s.id), Some(&s))
                    .detail
                    .contains("nenhuma ligação")
            );
        }
        save_link(dir.path(), s.id, s.peers[0].id);
        let item = context_item(dir.path(), Some(s.id), Some(&s));
        assert_eq!(item.status, DiagStatus::Warn);
        assert!(item.detail.contains("1 ligação"));
        assert!(item.detail.contains("não confirmadas"));
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn contextual_windows_diagnosis_has_one_shortcut_item_without_claiming_hooks_work() {
        let dir = tempfile::tempdir().unwrap();
        let s = status();
        save_link(dir.path(), s.id, s.peers[0].id);
        let items = diagnose_for(dir.path(), Some(s.id), Some(&s)).await;
        let shortcuts: Vec<_> = items
            .iter()
            .filter(|i| i.area == "input" && i.name == "atalhos")
            .collect();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].status, DiagStatus::Warn);
        assert!(shortcuts[0].detail.contains("não confirmadas"));
    }
}
