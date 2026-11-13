//! 6 · Diagnóstico (§12.9 item 2, fluxo 6): `support_bundle_preview` has no name, id (64 hex or
//! short) or IP of the peers, and not this device's name; the export writes the same kind of
//! text to the downloads folder. Owned by Rivet.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use clipsync_core::control;
use clipsync_proto::{ControlRequest, ModuleCommand, ModuleKind, PeerPerms};
use common::*;
use serde_json::json;

/// Fixture do controle externo; o caminho Ui -> gather -> redação -> arquivo é real.
struct SupportStatus(clipsync_proto::AgentStatus);

impl control::ControlHandler for SupportStatus {
    fn handle<'life0, 'async_trait>(
        &'life0 self,
        req: ControlRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = clipsync_proto::ControlResponse> + Send + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            match req {
                ControlRequest::Status => clipsync_proto::ControlResponse::Status(self.0.clone()),
                _ => clipsync_proto::ControlResponse::Error {
                    message: "requisição fora desta fixture de suporte".into(),
                },
            }
        })
    }
}

/// I01: deduplicar o ID não pode descartar aliases históricos nem endereços posteriores.
#[tokio::test(flavor = "multi_thread")]
async fn support_preview_and_export_redact_every_observation_of_the_same_peer() {
    use clipsync_proto::{AgentStatus, BlockRecord, ModuleStatus, PeerId, PeerState, PeerStatus};

    let dir = tempfile::tempdir().unwrap();
    let (profile, ours) = init_profile(dir.path(), "suporte-i01", CLIPBOARD_ONLY);
    let paired = PeerId([0x31; 32]);
    let status_only = PeerId([0x52; 32]);
    let history_only = PeerId([0x73; 32]);
    let mut peers = profile.load_peers().unwrap();
    peers.upsert(clipsync_core::PeerEntry {
        id: paired,
        name: "mesa-atual".into(),
        addr: None,
        paired_at: "2026-10-29T12:00:00Z".into(),
        perms: NONE,
    });
    profile.save_peers(&peers).unwrap();
    let peer = |id, name: &str, addr: &str| PeerStatus {
        id,
        name: name.into(),
        addr: Some(addr.parse().unwrap()),
        state: PeerState::Connected,
        perms: NONE,
        remote_modules: CLIPBOARD_ONLY,
        remote_version: 3,
    };
    let block = |peer, name: Option<&str>, reason: String, first, last, count| BlockRecord {
        peer: Some(peer),
        peer_name: name.map(str::to_string),
        module: Some(ModuleKind::Clipboard),
        reason,
        at_unix_ms: first,
        last_unix_ms: last,
        occurrences: count,
    };
    let status = AgentStatus {
        log_path: None,
        device_name: "suporte-i01".into(),
        id: ours,
        listen: profile.load_config().unwrap().listen,
        paused: false,
        peers: vec![
            peer(paired, "status-móvel", "[2001:db8::11]:47800"),
            // O segundo endereço não aparece no Device, que usa a primeira observação.
            peer(paired, "status-móvel", "[2001:db8::99]:47800"),
            peer(status_only, "órfão-status", "[2001:db8::22]:47800"),
        ],
        modules: vec![ModuleStatus {
            kind: ModuleKind::Clipboard,
            enabled: true,
            adapter: None,
            authorized_peers: vec![],
            last_event: Some("endereço observado 2001:db8::99".into()),
            last_block: None,
        }],
        recent_blocks: vec![
            block(
                paired,
                Some("mesa-antiga"),
                format!(
                    "causa-a status-móvel {} {}",
                    paired.to_hex(),
                    paired.short()
                ),
                10,
                30,
                7,
            ),
            block(
                paired,
                Some("portátil\nantigo"),
                "causa-b".into(),
                40,
                50,
                2,
            ),
            block(paired, Some("mesa-antiga"), "causa-c".into(), 60, 60, 1),
            block(paired, Some(""), "causa-d".into(), 70, 0, 1),
            block(
                status_only,
                Some("órfão-antigo"),
                format!(
                    "causa-e órfão-status {} {}",
                    status_only.to_hex(),
                    status_only.short()
                ),
                80,
                90,
                4,
            ),
            block(history_only, None, "causa-f".into(), 100, 0, 1),
            block(
                history_only,
                Some("histórico-único"),
                format!("causa-g {} {}", history_only.to_hex(), history_only.short()),
                110,
                120,
                u64::MAX,
            ),
            block(
                history_only,
                Some("histórico-único"),
                "causa-h".into(),
                130,
                140,
                2,
            ),
        ],
        recent_removals: vec![],
    };
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let socket = profile.socket_path();
    let server = tokio::spawn(async move {
        control::serve(&socket, std::sync::Arc::new(SupportStatus(status)), async {
            let _ = stopped.await;
        })
        .await
    });
    wait_until("fixture de Status responde", || async {
        control::request(&profile, ControlRequest::Status)
            .await
            .is_ok()
    })
    .await;
    let ui = start(dir.path(), "suporte-i01").await;
    let diagnostics = ok(&ui, json!({"cmd":"diagnostics_get"})).await;
    assert_eq!(diagnostics["network"]["peers"][0]["name"], "mesa-atual");
    assert_eq!(diagnostics["blocks"][0]["peer_name"], "mesa-antiga");
    assert_eq!(diagnostics["blocks"][1]["peer_name"], "portátil antigo");
    assert_eq!(diagnostics["blocks"].as_array().unwrap().len(), 8);

    let preview = ok(&ui, json!({"cmd":"support_bundle_preview"})).await;
    let exported = ok(&ui, json!({"cmd":"support_bundle_export"})).await;
    let path = std::path::Path::new(exported["path"].as_str().unwrap());
    let file = std::fs::read_to_string(path).unwrap();
    let text = preview["text"].as_str().unwrap();
    assert_eq!(
        file, text,
        "a exportação deve ser idêntica à prévia aprovada"
    );
    assert_eq!(exported["bytes"], file.len());
    assert_eq!(
        path.parent().unwrap(),
        dir.path().join("suporte-i01-downloads")
    );
    let after = ok(&ui, json!({"cmd":"diagnostics_get"})).await;
    assert_eq!(
        after["blocks"], diagnostics["blocks"],
        "suporte não altera o histórico"
    );
    ui.shutdown().await;
    let _ = stop.send(());
    server.await.unwrap().unwrap();

    let secrets = [
        "mesa-atual".to_string(),
        "status-móvel".into(),
        "mesa-antiga".into(),
        "portátil antigo".into(),
        "órfão-status".into(),
        "órfão-antigo".into(),
        "histórico-único".into(),
        "2001:db8::11".into(),
        "2001:db8::99".into(),
        "suporte-i01".into(),
        paired.to_hex(),
        paired.short(),
        status_only.to_hex(),
        status_only.short(),
        history_only.to_hex(),
        history_only.short(),
    ];
    let leaked = secrets.iter().filter(|s| text.contains(s.as_str())).count();
    assert_eq!(
        leaked, 0,
        "identificadores de fixture vazaram na prévia e no arquivo"
    );
    let blocks = text.split_once("[últimos bloqueios]\n").unwrap().1;
    assert_eq!(
        blocks,
        concat!(
            "primeiro=10 · último=30 · 7 ocorrências observadas · <par-1> · clipboard · causa-a <par-1> <id-par-1> <id-par-1>\n",
            "primeiro=40 · último=50 · 2 ocorrências observadas · <par-1> · clipboard · causa-b\n",
            "primeiro=60 · último=60 · 1 ocorrências observadas · <par-1> · clipboard · causa-c\n",
            "primeiro=70 · último=70 · 1 ocorrências observadas ·  · clipboard · causa-d\n",
            "primeiro=80 · último=90 · 4 ocorrências observadas · <par-2> · clipboard · causa-e <par-2> <id-par-2> <id-par-2>\n",
            "primeiro=100 · último=100 · 1 ocorrências observadas · - · clipboard · causa-f\n",
            "primeiro=110 · último=120 · pelo menos 18446744073709551615 ocorrências observadas · <par-3> · clipboard · causa-g <id-par-3> <id-par-3>\n",
            "primeiro=130 · último=140 · 2 ocorrências observadas · <par-3> · clipboard · causa-h\n",
        )
    );
    assert!(text.contains("endereço observado <ip>"));
    for (kind, count) in [("peer_id", 7), ("peer_name", 9)] {
        assert_eq!(
            preview["redactions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["kind"] == kind)
                .unwrap()["count"],
            count
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_support_bundle_carries_no_identifier() {
    let dir = tempfile::tempdir().unwrap();
    let (pa, ida) = init_profile(dir.path(), "pc-da-ana", CLIPBOARD_ONLY);
    let (pb, idb) = init_profile(dir.path(), "notebook-do-bruno", CLIPBOARD_ONLY);
    // b may send, a does not accept: a records a block naming b.
    let b_grants = PeerPerms {
        clipboard_out: true,
        ..NONE
    };
    pair_files(&pa, &pb, NONE, b_grants);
    let ui_a = start(dir.path(), "pc-da-ana").await;
    let ui_b = start(dir.path(), "notebook-do-bruno").await;
    wait_until("a and b connected", || async {
        ok(&ui_a, json!({ "cmd": "devices_list" })).await["devices"][0]["state"]
            == json!("connected")
    })
    .await;
    let send = ControlRequest::Module {
        kind: ModuleKind::Clipboard,
        cmd: ModuleCommand::ClipSend {
            text: "conteúdo que nunca entra no pacote".into(),
            to: Some(ida),
        },
    };
    control::request(&pb, send).await.unwrap();
    wait_until("a has a block", || async {
        !ok(&ui_a, json!({ "cmd": "diagnostics_get" })).await["blocks"]
            .as_array()
            .unwrap()
            .is_empty()
    })
    .await;

    let d = ok(&ui_a, json!({ "cmd": "diagnostics_get" })).await;
    assert!(!d["items"].as_array().unwrap().is_empty());
    assert!(
        d["network"]["listen"]
            .as_str()
            .unwrap()
            .starts_with("127.0.0.1:")
    );
    assert_eq!(d["network"]["peers"][0]["peer_id"], json!(idb.to_hex()));
    assert_eq!(d["network"]["peers"][0]["state"], json!("connected"));
    let b = &d["blocks"][0];
    assert!(b["reason"].as_str().is_some() && b["at_unix_ms"].as_u64().is_some());
    assert!(b["occurrences"].as_u64().is_some_and(|count| count >= 1));
    assert!(b["last_unix_ms"].as_u64().is_some());

    let p = ok(&ui_a, json!({ "cmd": "support_bundle_preview" })).await;
    let text = p["text"].as_str().unwrap();
    assert!(text.contains("ocorrências observadas") && text.contains("último="));
    let secrets = [
        "notebook-do-bruno".to_string(),
        idb.to_hex(),
        idb.short(),
        "127.0.0.1".to_string(),
        "pc-da-ana".to_string(),
        ida.to_hex(),
        "conteúdo que nunca entra".to_string(),
    ];
    for s in &secrets {
        assert!(!text.contains(s.as_str()), "{s} leaked:\n{text}");
    }
    assert!(text.contains("<par-1>") && text.contains("<ip>"), "{text}");
    let count = |kind: &str| {
        p["redactions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == json!(kind))
            .map_or(0, |r| r["count"].as_u64().unwrap())
    };
    // This device's name is never written; its count may be 0, the others are not.
    for kind in ["peer_id", "peer_name", "ip"] {
        assert!(count(kind) > 0, "{kind}: {}", p["redactions"]);
    }

    let x = ok(&ui_a, json!({ "cmd": "support_bundle_export" })).await;
    let path = std::path::PathBuf::from(x["path"].as_str().unwrap());
    assert_eq!(
        path.parent().unwrap(),
        dir.path().join("pc-da-ana-downloads")
    );
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("clipsync-suporte-") && name.ends_with(".txt") && name.len() == 36,
        "{name}"
    );
    let file = std::fs::read_to_string(&path).unwrap();
    assert_eq!(x["bytes"].as_u64().unwrap() as usize, file.len());
    for s in &secrets {
        assert!(!file.contains(s.as_str()), "{s} leaked in the file");
    }
    ui_a.shutdown().await;
    ui_b.shutdown().await;
}

/// S7 (§13.9): "layout do teclado" comes from the input module's `InputState`, and says
/// "agente parado" with the agent stopped; on Windows "atalhos" too, inactive without a link of
/// this machine in layout.toml. S8 checks the reason and uniqueness without treating
/// configuration as proof that the hooks work.
#[tokio::test(flavor = "multi_thread")]
async fn diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts() {
    let dir = tempfile::tempdir().unwrap();
    init_profile(dir.path(), "a", BOTH);
    let ui = start(dir.path(), "a").await;
    let item = |d: &serde_json::Value, name: &str| {
        d["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["area"] == json!("input") && i["name"] == json!(name))
            .cloned()
    };
    wait_until("input answers", || async {
        app_state(&ui).await["features"]["input"] == json!(true)
    })
    .await;
    let d = ok(&ui, json!({ "cmd": "diagnostics_get" })).await;
    // One item per name: the input crate names them too on Windows (CI 40757283200).
    for name in ["layout do teclado", "atalhos"] {
        let n = d["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["area"] == json!("input") && i["name"] == json!(name))
            .count();
        assert!(n <= 1, "{name} shows {n} times");
    }
    let keyboard = item(&d, "layout do teclado").expect("keyboard layout item");
    assert_ne!(keyboard["detail"], json!("agente parado"), "{keyboard}");
    let shortcuts = item(&d, "atalhos");
    if cfg!(windows) {
        let shortcuts = shortcuts.expect("contextual shortcut item");
        assert_eq!(shortcuts["status"], "warn");
        let detail = shortcuts["detail"].as_str().unwrap();
        assert!(detail.contains("nenhuma ligação desta máquina"), "{detail}");
        assert!(detail.contains("mouse físico"), "{detail}");
        assert!(!detail.starts_with("ativos:"), "{detail}");
    } else {
        assert_eq!(shortcuts, None, "no shortcuts on Linux in this version");
    }

    ok(&ui, json!({ "cmd": "agent_stop" })).await;
    let d = ok(&ui, json!({ "cmd": "diagnostics_get" })).await;
    assert_eq!(
        item(&d, "layout do teclado").unwrap()["detail"],
        json!("agente parado")
    );
    if cfg!(windows) {
        let shortcuts = item(&d, "atalhos").unwrap();
        assert_eq!(shortcuts["status"], "warn");
        let detail = shortcuts["detail"].as_str().unwrap();
        assert!(detail.contains("não conferido"), "{detail}");
        assert!(detail.contains("agente parado"), "{detail}");
        assert!(!detail.starts_with("ativos:"), "{detail}");
        assert_eq!(
            d["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["area"] == "input" && i["name"] == "atalhos")
                .count(),
            1
        );
    }
    ui.shutdown().await;
}

/// Exercita a API e a exportação com JSONL sintético, sem iniciar agente ou adaptador real.
async fn l1_support_rows(rows: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let dir = tempfile::Builder::new().prefix("csl1-").tempdir().unwrap();
    let profile = dir.path().join("p");
    let ui = clipsync_ui::Ui::start(clipsync_ui::UiOptions {
        profile: Some(profile.clone()),
        poll: std::time::Duration::from_secs(600),
        fake_adapters: true,
        downloads_dir: Some(dir.path().join("downloads")),
        autostart_dir: Some(dir.path().join("autostart")),
        ..Default::default()
    })
    .await
    .unwrap();
    let path = clipsync_core::logging::log_path(&profile);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let raw = rows
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, raw + "\n").unwrap();
    let preview = ok(&ui, json!({"cmd":"support_bundle_preview"})).await;
    let exported = ok(&ui, json!({"cmd":"support_bundle_export"})).await;
    let file = std::fs::read_to_string(exported["path"].as_str().unwrap()).unwrap();
    assert_eq!(file, preview["text"].as_str().unwrap());
    assert_eq!(exported["bytes"], file.len());
    ui.shutdown().await;
    let result: Vec<_> = file
        .split_once("\n[log]\n")
        .unwrap()
        .1
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(result.len(), rows.len(), "a cauda perdeu linhas da fixture");
    result
}

#[tokio::test]
async fn l1_cauda_preserva_eis_device_e_contadores() {
    let rows = [json!({
        "evento":"eis.device", "acao":"removed", "origem":"loop",
        "dispositivo":2, "capacidades":"pointer,pointer_absolute,button,scroll,keyboard",
        "contagem":5, "descartados":3
    })];
    let output = l1_support_rows(&rows).await;
    assert_eq!(output[0]["evento"], "eis.device", "evento EIS omitido");
    assert_eq!(
        output, rows,
        "campos estruturados EIS perdidos na exportação"
    );
}

#[tokio::test]
async fn l1_cauda_preserva_fechamento_por_peer_unresponsive() {
    let rows = [json!({"evento":"session.closed", "session":7, "motivo":"peer_unresponsive"})];
    let output = l1_support_rows(&rows).await;
    assert_eq!(
        output[0]["motivo"], "peer_unresponsive",
        "motivo da vigia omitido"
    );
    assert_eq!(output, rows);
}

#[tokio::test]
async fn l1_cauda_preserva_vocabulario_dos_emissores() {
    let mut rows = vec![json!({"evento":"peer.readdressed", "origem":"name"})];
    for (acao, origem) in [
        ("added", "added"),
        ("resumed", "resumed"),
        ("paused", "paused"),
        ("removed", "removed"),
        ("removed", "loop"),
        ("missing", "start"),
        ("discarding", "emit"),
        ("reconciled", "emit"),
        ("reconciled", "resumed"),
        ("reconciled", "release_all"),
    ] {
        rows.push(json!({"evento":"eis.device", "acao":acao, "origem":origem}));
    }
    rows.push(json!({"evento":"eis.device", "acao":"missing", "motivo":"not_ready"}));
    for motivo in [
        "context",
        "handshake",
        "converter",
        "eof",
        "disconnected",
        "closed",
        "runtime",
        "io",
    ] {
        rows.push(json!({"evento":"eis.loop", "motivo":motivo}));
    }
    for evento in ["session.refused", "enter.refused"] {
        rows.push(json!({"evento":evento, "motivo":"apply_failed"}));
    }
    assert_eq!(l1_support_rows(&rows).await, rows);
}

#[tokio::test]
async fn l1_cauda_preserva_combinacoes_canonicas_de_capacidades() {
    // Casos independentes do validador: vazio, cada nome e combinações com lacunas.
    let rows: Vec<_> = [
        "",
        "pointer",
        "pointer_absolute",
        "button",
        "scroll",
        "keyboard",
        "touch",
        "text",
        "pointer,button,scroll",
        "pointer_absolute,keyboard,text",
        "keyboard,touch",
        "pointer,pointer_absolute,button,scroll,keyboard,touch,text",
    ]
    .into_iter()
    .map(|capacidades| json!({"evento":"eis.device", "capacidades":capacidades}))
    .collect();
    assert_eq!(l1_support_rows(&rows).await, rows);
}

#[tokio::test]
async fn l1_cauda_rejeita_capacidades_nao_canonicas_e_tipos_indevidos() {
    let mut rows: Vec<_> = [
        "keyboard,keyboard",
        "keyboard,pointer",
        "text,touch",
        "pointer_absolute,pointer",
        ",keyboard",
        "keyboard,",
        "pointer,,keyboard",
        "keyboard,texto-privado",
        "keyboard-sufixo",
        "keyboard texto-privado",
        " keyboard",
        "keyboard ",
        "Keyboard",
        "keyboard\n",
        "nome-do-dispositivo",
        "keycode",
        "tecla",
    ]
    .into_iter()
    .map(|capacidades| json!({"evento":"eis.device", "capacidades":capacidades}))
    .collect();
    for capacidades in [
        json!(1),
        json!(true),
        json!(null),
        json!(["keyboard"]),
        json!({"nome":"privado"}),
    ] {
        rows.push(json!({"evento":"eis.device", "capacidades":capacidades}));
    }
    for row in l1_support_rows(&rows).await {
        assert!(row.get("capacidades").is_none() || row["capacidades"] == "<texto omitido>");
    }
}

#[tokio::test]
async fn l1_cauda_aceita_dispositivos_apenas_como_inteiros_nao_negativos() {
    let mut rows = Vec::new();
    for n in [0, 1, u64::MAX] {
        rows.push(json!({"evento":"eis.device", "dispositivo":n}));
        rows.push(json!({"evento":"eis.loop", "dispositivos":n}));
    }
    assert_eq!(l1_support_rows(&rows).await, rows);
    let invalid = [
        json!(-1),
        json!(1.5),
        json!(1.0),
        json!("1"),
        json!(true),
        json!(null),
        json!([1]),
        json!({"id":1}),
    ];
    let rows: Vec<_> = invalid
        .into_iter()
        .map(|value| json!({"evento":"eis.device", "dispositivo":value, "dispositivos":value}))
        .collect();
    for row in l1_support_rows(&rows).await {
        assert!(row.get("dispositivo").is_none());
        assert!(row.get("dispositivos").is_none());
    }
}

#[tokio::test]
async fn l1_cauda_preserva_categorias_de_erro_somente_em_eis_loop() {
    let categories = [
        "event",
        "parse",
        "request",
        "handshake",
        "interface_version",
        "io",
    ];
    let rows: Vec<_> = categories
        .iter()
        .map(|erro| json!({"evento":"eis.loop", "erro":erro}))
        .collect();
    assert_eq!(l1_support_rows(&rows).await, rows);
    let mut rows = Vec::new();
    for evento in [
        json!("eis.device"),
        json!("emulate.error"),
        json!("session.closed"),
        json!("eis.loop.sufixo"),
        json!(["eis.loop"]),
        json!(null),
    ] {
        for erro in categories {
            rows.push(json!({"evento":evento, "erro":erro}));
        }
    }
    rows.push(json!({"erro":"io"}));
    rows.push(
        json!({"evento":"eis.loop", "totais":{"evento":"eis.loop", "erro":"io", "enviados":1}}),
    );
    let output = l1_support_rows(&rows).await;
    for row in &output[..output.len() - 1] {
        assert_eq!(row["erro"], "<texto omitido>");
    }
    assert_eq!(output.last().unwrap()["totais"], json!({"enviados":1}));
}

#[tokio::test]
async fn l1_cauda_mantem_texto_livre_e_campos_privados_omitidos() {
    let rows = [
        json!({"evento":"eis.loop", "erro":"io /home/segredo", "message":"texto-privado", "peer":"par-privado", "host":"host-privado", "tecla":"proibida", "keycode":123, "clipboard_text":"segredo", "pending_releases":[123], "nome":"nome-privado"}),
        json!({"evento":"hooks.reinstalled", "origem":"origem-inventada", "acao":"acao-inventada", "motivo":"motivo-inventado", "erro":"event"}),
        json!({"evento":"eis.loop", "erro":"request-sufixo", "acao":42}),
        json!({"evento":"eis.loop", "erro":["io"]}),
        json!({"evento":"eis.loop", "erro":{"categoria":"io", "tecla":"privada"}}),
        json!({"evento":"eis.loop", "erro":123}),
        json!({"evento":"eis.loop", "erro":true}),
    ];
    let output = l1_support_rows(&rows).await;
    for row in &output {
        for key in ["tecla", "keycode", "clipboard_text", "pending_releases"] {
            assert!(row.get(key).is_none());
        }
        assert!(row.get("erro").is_none() || row["erro"] == "<texto omitido>");
    }
    for key in ["evento", "origem", "acao", "motivo"] {
        assert_eq!(output[1][key], "<texto omitido>");
    }
    assert!(output[2].get("acao").is_none());
    assert_eq!(output[0]["peer"], "<id>");
    assert_eq!(output[0]["host"], "<dispositivo>");
    assert_eq!(output[0]["nome"], "<nome>");
    assert_eq!(output[0]["message"], "<texto omitido>");
}
