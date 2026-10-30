//! S8: fatos sobre a saída local. Configuração não prova a saúde da captura nem a recepção.
use clipsync_input::Layout;
use clipsync_proto::{AgentStatus, InputState, ModuleKind, PeerId, PeerState, PeerStatus};
use serde::{Deserialize, Serialize};

use crate::{Ctx, clean, status::Observed};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TraversalCode {
    NotInitialized,
    AgentStopped,
    InputOff,
    Paused,
    ScreenLocked,
    LayoutError,
    NoLocalLink,
    PeerOffline,
    RemoteInputOff,
    PeerTooOld,
    #[default]
    Checking,
    Unverified,
    Active,
}

impl TraversalCode {
    fn detail(self) -> &'static str {
        match self {
            Self::NotInitialized => {
                "Conclua a configuração deste dispositivo para compartilhar teclado e mouse."
            }
            Self::AgentStopped => {
                "O agente está parado. Inicie o ClipSync para conferir a saída do mouse."
            }
            Self::InputOff => "Teclado e mouse estão desligados ou indisponíveis nesta máquina.",
            Self::Paused => {
                "O ClipSync está pausado. Retome para usar teclado e mouse entre as telas."
            }
            Self::ScreenLocked => {
                "A trava do ClipSync está ligada nesta tela. Destrave para o mouse sair."
            }
            Self::LayoutError => {
                "Não foi possível ler o layout. Confira o Diagnóstico antes de organizar as telas."
            }
            Self::NoLocalLink => {
                "O mouse desta máquina ainda não sai pelas bordas. Organize as telas no computador com o mouse físico. Quem só recebe não precisa criar uma ligação local para receber."
            }
            Self::PeerOffline => {
                "Este dispositivo está desconectado. Abra o ClipSync nele e confira a conexão."
            }
            Self::RemoteInputOff => "Teclado e mouse estão desligados no dispositivo de destino.",
            Self::PeerTooOld => {
                "O protocolo deste dispositivo não permite a configuração por monitor. Atualize os dois apps."
            }
            Self::Checking => {
                "Ainda não foi possível conferir a saída do mouse. Consulte o Diagnóstico."
            }
            Self::Unverified => {
                "Ligação configurada; falta testar a travessia. A permissão para controlar deve ser concedida no dispositivo de destino."
            }
            Self::Active => {
                "O agente anuncia uma sessão de saída. Confirme no destino se o mouse e o teclado chegam."
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraversalPeer {
    pub peer_id: String,
    pub code: TraversalCode,
    pub detail: String,
    pub protocol_warning: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Traversal {
    pub code: TraversalCode,
    pub detail: String,
    pub local_links: usize,
    /// Registro histórico; nunca usado para classificar o estado atual.
    pub last_block: Option<String>,
    pub peers: Vec<TraversalPeer>,
}

impl Default for Traversal {
    fn default() -> Self {
        Self {
            code: TraversalCode::Checking,
            detail: TraversalCode::Checking.detail().into(),
            local_links: 0,
            last_block: None,
            peers: vec![],
        }
    }
}

/// Só configuração e o controle local. Não carrega identidade, chave nem token.
pub(crate) fn observe(ctx: &Ctx, observed: &Observed) -> Traversal {
    let initialized = ctx.profile.is_initialized();
    let input_on = ctx
        .profile
        .load_config()
        .ok()
        .map(|c| c.modules.input && clipsync_input::SUPPORTED);
    let peers = ctx.profile.load_peers();
    let known: Vec<PeerId> = peers
        .as_ref()
        .map(|p| p.peer.iter().map(|p| p.id).collect())
        .unwrap_or_default();
    let layout = Layout::read(&ctx.profile.dir.join(clipsync_input::LAYOUT_FILE));
    evaluate(
        initialized,
        if peers.is_err() { None } else { input_on },
        observed.status.as_ref(),
        observed.input.as_ref(),
        layout.as_ref().map(|l| l.as_ref()).map_err(|_| ()),
        &known,
    )
}

fn protocol_warning(peer: &PeerStatus) -> Option<String> {
    if peer.state != PeerState::Connected {
        return None;
    }
    match peer.remote_version {
        1 => Some("Protocolo 1: sem configuração por monitor. Atualize os dois apps.".into()),
        2 => Some("Protocolo 2: modo de compatibilidade, sem o retorno por trecho da versão atual. Atualize os dois apps.".into()),
        _ => None,
    }
}

fn evaluate(
    initialized: bool,
    input_on: Option<bool>,
    status: Option<&AgentStatus>,
    input: Option<&InputState>,
    layout: Result<Option<&Layout>, ()>,
    known: &[PeerId],
) -> Traversal {
    use TraversalCode as C;
    let links: Vec<_> = match (status, layout.as_ref()) {
        (Some(s), Ok(Some(l))) => l
            .barriers_for(s.id)
            .into_iter()
            .filter(|b| known.contains(&b.peer))
            .collect(),
        _ => vec![],
    };
    let local_links = links.len();
    let gate = if !initialized {
        Some(C::NotInitialized)
    } else if status.is_none() {
        Some(C::AgentStopped)
    } else if input_on == Some(false)
        || status.is_some_and(|s| {
            !s.modules
                .iter()
                .any(|m| m.kind == ModuleKind::Input && m.enabled)
        })
    {
        Some(C::InputOff)
    } else if status.is_some_and(|s| s.paused) {
        Some(C::Paused)
    } else if input.is_some_and(|i| i.screen_locked) {
        Some(C::ScreenLocked)
    } else if layout.is_err() {
        Some(C::LayoutError)
    } else if input_on.is_none() {
        Some(C::Checking)
    } else if local_links == 0 {
        Some(C::NoLocalLink)
    } else if input.is_none() {
        Some(C::Checking)
    } else {
        None
    };
    let peers: Vec<_> = known
        .iter()
        .map(|id| {
            let live = status.and_then(|s| s.peers.iter().find(|p| p.id == *id));
            let linked = links.iter().any(|l| l.peer == *id);
            let code = gate.unwrap_or_else(|| match live {
                None => C::PeerOffline,
                Some(p) if p.state != PeerState::Connected => C::PeerOffline,
                Some(p) if !p.remote_modules.input => C::RemoteInputOff,
                Some(p) if p.remote_version == 1 => C::PeerTooOld,
                Some(_) if !linked => C::NoLocalLink,
                Some(p) if p.remote_version == 0 => C::Checking,
                Some(_) if input.is_some_and(|i| i.active_peer == Some(*id)) => C::Active,
                Some(_) => C::Unverified,
            });
            TraversalPeer {
                peer_id: id.to_hex(),
                code,
                detail: code.detail().into(),
                protocol_warning: live.and_then(protocol_warning),
            }
        })
        .collect();
    // Um par desconectado não esconde outra saída configurada. Só pares ligados contam aqui.
    let code = gate.unwrap_or_else(|| {
        let linked = peers
            .iter()
            .filter(|p| links.iter().any(|l| l.peer.to_hex() == p.peer_id));
        if linked.clone().any(|p| p.code == C::Active) {
            C::Active
        } else if linked.clone().any(|p| p.code == C::Unverified) {
            C::Unverified
        } else {
            linked.map(|p| p.code).next().unwrap_or(C::Checking)
        }
    });
    let last_block = status
        .and_then(|s| {
            s.recent_blocks
                .iter()
                .find(|b| b.module == Some(ModuleKind::Input))
                .map(|b| b.reason.as_str())
                .or_else(|| {
                    s.modules
                        .iter()
                        .find(|m| m.kind == ModuleKind::Input)
                        .and_then(|m| m.last_block.as_deref())
                })
        })
        .map(clean);
    Traversal {
        code,
        detail: code.detail().into(),
        local_links,
        last_block,
        peers,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use clipsync_input::layout::{Endpoint, Link};
    use clipsync_proto::{BlockRecord, Edge, ModuleSet, ModuleStatus, PeerPerms};

    fn state() -> AgentStatus {
        AgentStatus {
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
                    clipboard: false,
                    input: true,
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
    fn input() -> InputState {
        InputState {
            inventories: vec![],
            inventory_problem: None,
            pending_monitor_offer: None,
            local: vec![],
            peers: vec![],
            screen_locked: false,
            active_peer: None,
            pending_offer: None,
            keyboard_layout: None,
            resume_hotkey_refused: false,
            switch_supported: true,
        }
    }
    fn layout(a: PeerId, b: PeerId) -> Layout {
        let endpoint = |peer, edge| Endpoint {
            peer,
            monitor: "fake-0".into(),
            edge,
            from: 0.0,
            to: 1.0,
        };
        Layout {
            links: vec![Link {
                id: "l1".into(),
                a: endpoint(a, Edge::Right),
                b: endpoint(b, Edge::Left),
            }],
            ..Layout::default()
        }
    }
    fn run(s: &AgentStatus, i: Option<&InputState>, l: &Layout) -> Traversal {
        evaluate(
            true,
            Some(true),
            Some(s),
            i,
            Ok(Some(l)),
            &[PeerId([2; 32]), PeerId([3; 32])],
        )
    }
    #[test]
    fn unrelated_and_revoked_links_do_not_count_as_local_output() {
        let s = state();
        let i = input();
        for (a, b) in [(PeerId([2; 32]), PeerId([3; 32])), (s.id, PeerId([9; 32]))] {
            let t = run(&s, Some(&i), &layout(a, b));
            assert_eq!(t.code, TraversalCode::NoLocalLink);
            assert_eq!(t.local_links, 0);
        }
    }
    #[test]
    fn peer_versions_and_connectivity_never_invent_an_application_release() {
        let mut s = state();
        let i = input();
        let l = layout(s.id, s.peers[0].id);
        for (version, want) in [
            (0, TraversalCode::Checking),
            (1, TraversalCode::PeerTooOld),
            (2, TraversalCode::Unverified),
            (3, TraversalCode::Unverified),
        ] {
            s.peers[0].remote_version = version;
            let t = run(&s, Some(&i), &l);
            assert_eq!(t.peers[0].code, want);
            assert_eq!(
                t.peers[0].protocol_warning.is_some(),
                version == 1 || version == 2
            );
            if let Some(warning) = &t.peers[0].protocol_warning {
                assert!(!warning.contains("0.2.1"));
                assert!(!warning.contains("0.3.0"));
            }
        }
        s.peers[0].state = PeerState::Disconnected;
        s.peers[0].remote_modules.input = false;
        assert_eq!(
            run(&s, Some(&i), &l).peers[0].code,
            TraversalCode::PeerOffline
        );
        s.peers[0].state = PeerState::Connected;
        assert_eq!(
            run(&s, Some(&i), &l).peers[0].code,
            TraversalCode::RemoteInputOff
        );
    }
    #[test]
    fn historical_blocks_do_not_override_current_facts_and_are_cleaned() {
        let mut s = state();
        let mut i = input();
        let l = layout(s.id, s.peers[0].id);
        s.recent_blocks.push(BlockRecord {
            occurrences: 1,
            last_unix_ms: 0,
            at_unix_ms: 1,
            peer: None,
            peer_name: None,
            module: Some(ModuleKind::Input),
            reason: "falha\u{1b}[2J antiga".into(),
        });
        let t = run(&s, Some(&i), &l);
        assert_eq!(t.code, TraversalCode::Unverified);
        assert_eq!(t.last_block.as_deref(), Some("falha?[2J antiga"));
        i.active_peer = Some(s.peers[0].id);
        assert_eq!(run(&s, Some(&i), &l).code, TraversalCode::Active);
        s.paused = true;
        assert_eq!(run(&s, Some(&i), &l).code, TraversalCode::Paused);
        s.paused = false;
        i.screen_locked = true;
        assert_eq!(run(&s, Some(&i), &l).code, TraversalCode::ScreenLocked);
    }
    #[test]
    fn missing_input_observation_and_old_json_remain_unverified() {
        let s = state();
        let l = layout(s.id, s.peers[0].id);
        assert_eq!(run(&s, None, &l).code, TraversalCode::Checking);
        let old = serde_json::json!({"initialized":false,"profile_dir":"/perfil","device":null,"agent":"stopped","agent_error":null,"paused":false,"screen_locked":false,"mesh":{"connected":0,"total":0},"modules":{"clipboard":false,"input":false},"input_supported":true,"version":"0.3.0","platform":"linux","features":crate::status::Features::default()});
        let app: crate::status::AppState = serde_json::from_value(old).unwrap();
        assert_eq!(app.traversal.code, TraversalCode::Checking);
    }
    #[test]
    fn local_permission_does_not_authorize_or_block_outgoing_control() {
        let mut s = state();
        let i = input();
        let l = layout(s.id, s.peers[0].id);
        assert_eq!(run(&s, Some(&i), &l).code, TraversalCode::Unverified);
        s.peers[0].perms.input_in = true;
        assert_eq!(run(&s, Some(&i), &l).code, TraversalCode::Unverified);
        let t = evaluate(
            true,
            Some(true),
            Some(&s),
            Some(&i),
            Ok(None),
            &[s.peers[0].id],
        );
        assert_eq!(t.code, TraversalCode::NoLocalLink);
    }
}
