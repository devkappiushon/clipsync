//! Evidência offline de pré-requisitos, com logs sintéticos e sem agente.
#[allow(dead_code)]
#[path = "../src/verificar.rs"]
mod verificar;

use serde_json::{Value, json};
use std::{error::Error, path::Path, process::Command};
use verificar::{Entrada, Estado};

fn event(name: &str, fields: Value) -> Value {
    let mut v = json!({"v":1,"ts":"2026-11-04T14:00:00.000Z","evento":name});
    if let (Some(out), Some(fields)) = (v.as_object_mut(), fields.as_object()) {
        out.extend(fields.clone());
    }
    v
}

fn write_log(path: &Path, events: &[Value]) -> Result<(), Box<dyn Error>> {
    let mut contents = String::new();
    for e in events {
        contents.push_str(&serde_json::to_string(e)?);
        contents.push('\n');
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn entrada(dir: &Path, destination: bool) -> Entrada {
    Entrada {
        origem: dir.join("origem.jsonl"),
        destino: destination.then(|| dir.join("destino.jsonl")),
        sessao: None,
        ultima: true,
        desde: None,
        json: false,
    }
}

fn ready() -> Vec<Value> {
    vec![
        event("inventory.prepared", json!({"monitores":1})),
        event("inventory.sent", json!({"peer":"aa"})),
        event("inventory.received", json!({"peer":"aa", "monitores":1})),
        event(
            "routes.built",
            json!({"rotas":1, "barreiras":1, "pares":["aa"]}),
        ),
        event(
            "capture.ready",
            json!({"barreiras":[{"peer":"aa", "monitor":"m", "borda":"right", "trecho":[0,1]}]}),
        ),
    ]
}

#[test]
fn sem_tentativa_mostra_prerequisitos() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    write_log(
        &e.origem,
        &[
            event("inventory.prepared", json!({"monitores":1})),
            event("inventory.sent", json!({"peer":"aa"})),
            event("routes.empty", json!({"motivo":"no_remote_inventory"})),
        ],
    )?;
    let output = verificar::executar(&e)?;
    assert_eq!(output.quebra.as_deref(), Some("rotas"));
    assert_eq!(output.tentativa, None);
    assert!(output.texto().contains("[falhou] rotas"));
    assert!(output.texto().contains("nenhuma tentativa registrada"));
    assert!(matches!(output.passos[1].estado, Estado::SemDado));
    Ok(())
}

#[test]
fn tudo_pronto_sem_tentativa() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), true);
    write_log(&e.origem, &ready())?;
    write_log(
        e.destino.as_deref().ok_or("destino")?,
        &[
            event("agent.up", json!({"id":"aa"})),
            event("inventory.prepared", json!({"monitores":1})),
        ],
    )?;
    let output = verificar::executar(&e)?;
    assert_eq!(output.passos.len(), 5);
    assert!(
        output.passos[..4]
            .iter()
            .all(|step| matches!(step.estado, Estado::Ok))
    );
    assert!(matches!(output.passos[4].estado, Estado::SemDado));
    assert!(output.passos[4].linha.is_none());
    assert_eq!(output.tentativa, None);
    assert!(output.quebra.is_none());
    assert!(
        output
            .texto()
            .contains("nenhuma tentativa registrada: a origem nunca viu o cursor na barreira")
    );
    Ok(())
}

#[test]
fn destino_ausente_e_sem_dado() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    write_log(&e.origem, &ready())?;
    let output = verificar::executar(&e)?;
    let step = &output.passos[1];
    assert_eq!(step.nome, "inventario_destino");
    assert!(matches!(step.estado, Estado::SemDado));
    assert!(step.motivo.is_some());
    assert!(step.linha.is_none());
    Ok(())
}

#[test]
fn captura_falha_tem_prova_e_motivo() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    let mut events = ready();
    events.pop();
    events.push(event(
        "capture.failed",
        json!({"adaptador":"portal", "motivo":"portal_denied"}),
    ));
    write_log(&e.origem, &events)?;
    let output = verificar::executar(&e)?;
    assert_eq!(output.quebra.as_deref(), Some("captura"));
    assert_eq!(output.passos[3].motivo.as_deref(), Some("portal_denied"));
    assert!(
        output.passos[3]
            .linha
            .as_ref()
            .is_some_and(|v| v.contains("capture.failed"))
    );
    Ok(())
}

#[test]
fn json_cli_diz_os_mesmos_estados_sem_criar_perfil() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    write_log(&e.origem, &ready())?;
    let expected = verificar::executar(&e)?.json();
    let profile = dir.path().join("nao-criar");
    let cli = Command::new(env!("CARGO_BIN_EXE_clipsync"))
        .arg("--profile")
        .arg(&profile)
        .args(["verificar-travessia", "--origem"])
        .arg(&e.origem)
        .arg("--json")
        .output()?;
    assert!(cli.status.success(), "{:?}", cli);
    let output: Value = serde_json::from_slice(&cli.stdout)?;
    assert_eq!(output, expected);
    assert_eq!(output["v"], 1);
    assert_eq!(output["passos"][1]["estado"], "sem_dado");
    assert!(!profile.exists());
    Ok(())
}

#[test]
fn inventario_missing_prova_falha_mesmo_sem_destino() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    let mut events = ready();
    events.retain(|v| v["evento"] != "inventory.received");
    events.push(event("inventory.missing", json!({"peer":"aa"})));
    write_log(&e.origem, &events)?;
    let output = verificar::executar(&e)?;
    assert_eq!(output.quebra.as_deref(), Some("inventario_destino"));
    assert_eq!(
        output.passos[1].motivo.as_deref(),
        Some("a origem não recebeu o inventário do par")
    );
    Ok(())
}

#[test]
fn preparado_no_destino_sem_recebimento_explicita_lacuna() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), true);
    let mut events = ready();
    events.retain(|v| v["evento"] != "inventory.received");
    write_log(&e.origem, &events)?;
    write_log(
        e.destino.as_deref().ok_or("destino")?,
        &[
            event("agent.up", json!({"id":"aa"})),
            event("inventory.prepared", json!({"monitores":1})),
        ],
    )?;
    let output = verificar::executar(&e)?;
    assert_eq!(
        output.passos[1].motivo.as_deref(),
        Some(
            "preparado no destino, não recebido na origem; sem registro: passo não alcançado ou log truncado"
        )
    );
    Ok(())
}

fn timed(mut e: Value, minute: u32, second: u32) -> Value {
    e["ts"] = json!(format!("2026-11-04T14:{minute:02}:{second:02}.000Z"));
    e
}

fn chain(minute: u32, session: u32, attempt: u32) -> (Vec<Value>, Vec<Value>) {
    let mut source: Vec<_> = ready().into_iter().map(|e| timed(e, minute, 0)).collect();
    source.insert(0, timed(event("agent.up", json!({"id":"bb"})), minute, 0));
    for (second, name, data) in [
        (
            1,
            "attempt.begin",
            json!({"attempt":attempt,"origem":"barrier","peer":"aa"}),
        ),
        (
            2,
            "session.open",
            json!({"attempt":attempt,"session":session,"peer":"aa","mensagem":"enter_routes"}),
        ),
        (
            3,
            "session.sent",
            json!({"attempt":attempt,"session":session,"peer":"aa"}),
        ),
        (4, "session.active", json!({"session":session,"peer":"aa"})),
        (6, "datagram.first", json!({"session":session})),
        (9, "return.received", json!({"session":session,"peer":"aa"})),
        (
            10,
            "session.closed",
            json!({"session":session,"motivo":"return","totais":{"enviados":1,"descartados":0}}),
        ),
    ] {
        source.push(timed(event(name, data), minute, second));
    }
    let mut destination: Vec<Value> = [
        (0,"agent.up",json!({"id":"aa"})),
        (0,"inventory.prepared",json!({"monitores":1})),
        (3,"enter.received",json!({"session":session,"peer":"bb","mensagem":"enter_routes"})),
        (4,"emulate.start",json!({"session":session,"adaptador":"fake"})),
        (5,"emulate.positioned",json!({"session":session})),
        (6,"datagram.first",json!({"session":session})),
        (8,"return.sent",json!({"session":session,"totais":{"recebidos":1,"ignorados":0,"perdidos":0,"fora_de_ordem":0}})),
    ].into_iter().map(|(s,n,d)|timed(event(n,d),minute,s)).collect();
    // Como os produtores reais: todo evento da sessão identifica o par remoto.
    for r in &mut source {
        if !r["session"].is_null() {
            r["peer"] = json!("aa");
        }
    }
    for r in &mut destination {
        if !r["session"].is_null() {
            r["peer"] = json!("bb");
        }
    }
    (source, destination)
}

fn run_logs(source: &[Value], destination: &[Value]) -> Result<verificar::Saida, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), true);
    write_log(&e.origem, source)?;
    write_log(e.destino.as_deref().ok_or("destino")?, destination)?;
    Ok(verificar::executar(&e)?)
}

#[test]
fn cadeia_feliz() -> Result<(), Box<dyn Error>> {
    let (source, destination) = chain(0, 101, 1);
    let out = run_logs(&source, &destination)?;
    assert_eq!(out.passos.len(), 11);
    assert!(out.passos.iter().all(|p| p.estado == Estado::Ok));
    assert!(out.quebra.is_none());
    assert!(out.json()["parada"].is_null());
    assert_eq!(out.sessao, Some(101));
    assert_eq!(out.tentativa, Some(1));
    Ok(())
}

#[test]
fn remocoes_sao_lacunas_sem_inventar_quebra() -> Result<(), Box<dyn Error>> {
    for (remote, event_name, wanted) in [
        (false, "inventory.prepared", "inventario_origem"),
        (true, "inventory.prepared", "inventario_destino"),
        (false, "routes.built", "rotas"),
        (false, "capture.ready", "captura"),
        (false, "attempt.begin", "tentativa"),
        (false, "session.sent", "sessao"),
        (true, "enter.received", "entrada_destino"),
        (true, "emulate.positioned", "emulacao"),
        (false, "datagram.first", "datagramas"),
        (true, "datagram.first", "datagramas"),
        (false, "return.received", "retorno"),
        (true, "return.sent", "retorno"),
        (false, "session.closed", "fechamento"),
    ] {
        let (mut source, mut dest) = chain(0, 101, 1);
        let target = if remote { &mut dest } else { &mut source };
        target.retain(|v| v["evento"] != event_name);
        let out = run_logs(&source, &dest)?;
        assert!(out.quebra.is_none(), "{event_name}: {:?}", out.quebra);
        assert_eq!(out.json()["parada"], wanted, "{event_name}");
        assert!(
            out.texto()
                .contains("sem registro: passo não alcançado ou log truncado")
        );
    }
    Ok(())
}

#[test]
fn falhas_explicitas_apontam_primeiro_ponto_com_prova() -> Result<(), Box<dyn Error>> {
    for (remote, old, new, reason, wanted) in [
        (false, "routes.built", "routes.empty", "no_links", "rotas"),
        (
            false,
            "capture.ready",
            "capture.failed",
            "portal_denied",
            "captura",
        ),
        (
            false,
            "attempt.begin",
            "attempt.filtered",
            "paused",
            "tentativa",
        ),
        (false, "session.closed", "session.closed", "error", "sessao"),
        (
            false,
            "session.closed",
            "session.closed",
            "refused",
            "entrada_destino",
        ),
        (
            true,
            "enter.received",
            "enter.refused",
            "monitor_not_found",
            "entrada_destino",
        ),
        (
            true,
            "emulate.positioned",
            "emulate.error",
            "apply_failed",
            "emulacao",
        ),
    ] {
        let (mut source, mut dest) = chain(0, 101, 1);
        let target = if remote { &mut dest } else { &mut source };
        let line = target
            .iter_mut()
            .find(|v| v["evento"] == old)
            .ok_or("evento")?;
        line["evento"] = json!(new);
        line["motivo"] = json!(reason);
        line["erro"] = json!("falha sintética");
        let out = run_logs(&source, &dest)?;
        assert_eq!(out.quebra.as_deref(), Some(wanted), "{reason}");
        let step = out
            .passos
            .iter()
            .find(|p| p.nome == wanted)
            .ok_or("passo")?;
        assert!(step.linha.as_ref().is_some_and(|v| v.contains(reason)));
    }
    Ok(())
}

#[test]
fn tentativa_sem_ativacao_e_interrupcao_sao_distintas() -> Result<(), Box<dyn Error>> {
    let (mut source, dest) = chain(0, 101, 1);
    source.retain(|v| v["evento"] == "attempt.begin" || v["session"].is_null());
    source.push(timed(
        event(
            "attempt.filtered",
            json!({"attempt":1,"motivo":"no_activation"}),
        ),
        0,
        3,
    ));
    let out = run_logs(&source, &dest)?;
    assert_eq!(out.quebra.as_deref(), Some("tentativa"));
    for reason in ["pause", "lock", "stop", "peer_down", "switch", "inventory"] {
        let (mut source, mut dest) = chain(0, 101, 1);
        source.retain(|v| {
            !matches!(
                v["evento"].as_str(),
                Some("datagram.first" | "return.received")
            )
        });
        dest.retain(|v| !matches!(v["evento"].as_str(), Some("datagram.first" | "return.sent")));
        let closed = source
            .iter_mut()
            .find(|v| v["evento"] == "session.closed")
            .ok_or("closed")?;
        closed["motivo"] = json!(reason);
        let out = run_logs(&source, &dest)?;
        assert!(out.quebra.is_none());
        assert_eq!(out.json()["parada"], "datagramas");
        assert!(out.texto().contains(reason));
    }
    Ok(())
}

#[test]
fn ultima_e_sessao_nao_misturam_contador_reiniciado() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut e = entrada(dir.path(), true);
    let (old, old_dest) = chain(0, 101, 1);
    let (mut new, mut new_dest) = chain(20, 202, 1);
    new.extend(old);
    new_dest.extend(old_dest); // arquivo fora de ordem: quem decide é o ts
    write_log(&e.origem, &new)?;
    write_log(e.destino.as_deref().ok_or("dest")?, &new_dest)?;
    assert_eq!(verificar::executar(&e)?.sessao, Some(202));
    e.ultima = false;
    e.sessao = Some(101);
    let out = verificar::executar(&e)?;
    assert_eq!(out.sessao, Some(101));
    assert!(
        out.passos[4]
            .linha
            .as_ref()
            .is_some_and(|v| v.contains("14:00:01"))
    );
    Ok(())
}

#[test]
fn tentativa_nova_sem_sessao_nao_herda_sessao_antiga() -> Result<(), Box<dyn Error>> {
    let (mut source, dest) = chain(0, 101, 1);
    source.push(timed(
        event("attempt.begin", json!({"attempt":1,"origem":"test"})),
        20,
        1,
    ));
    source.push(timed(
        event(
            "attempt.filtered",
            json!({"attempt":1,"motivo":"no_activation"}),
        ),
        20,
        2,
    ));
    let out = run_logs(&source, &dest)?;
    assert_eq!(out.sessao, None);
    assert_eq!(out.quebra.as_deref(), Some("tentativa"));
    Ok(())
}

#[test]
fn desde_absoluto_e_duracao_usam_maior_ts_da_origem() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut e = entrada(dir.path(), true);
    let (mut source, mut dest) = chain(0, 101, 1);
    let (new, new_dest) = chain(20, 202, 2);
    source.extend(new);
    dest.extend(new_dest);
    write_log(&e.origem, &source)?;
    write_log(e.destino.as_deref().ok_or("dest")?, &dest)?;
    e.desde = Some("10m".into());
    let relative = verificar::executar(&e)?.json();
    e.desde = Some("2026-11-04T11:10:10-03:00".into());
    let absolute = verificar::executar(&e)?.json();
    assert_eq!(relative, absolute);
    assert_eq!(relative["sessao"], 202);
    assert!(relative["parada"].is_null());
    e.desde = Some("2026-11-04T14:20:02Z".into());
    let cut = verificar::executar(&e)?;
    assert!(cut.passos[1].estado == Estado::SemDado);
    Ok(())
}

#[test]
fn esquema_futuro_invalido_e_arquivo_ausente_param_sem_passos() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    assert!(matches!(
        verificar::executar(&e).err().ok_or("erro")?.tipo,
        verificar::TipoFalha::Arquivo
    ));
    for v in [
        json!({"v":2,"ts":"2026-11-04T14:00:00Z","evento":"futuro"}),
        json!({"v":1,"ts":"inválido","evento":"x"}),
    ] {
        write_log(&e.origem, &[v])?;
        assert!(matches!(
            verificar::executar(&e).err().ok_or("erro")?.tipo,
            verificar::TipoFalha::Esquema
        ));
    }
    std::fs::write(&e.origem, "não é JSON\n")?;
    assert!(verificar::executar(&e).is_err());
    Ok(())
}

#[test]
fn desconhecidos_e_esquema_antigo_nao_inventam_passos() -> Result<(), Box<dyn Error>> {
    let (mut source, dest) = chain(0, 101, 1);
    source.push(timed(event("novo.desconhecido", json!({"v":0})), 0, 11));
    let out = run_logs(&source, &dest)?;
    assert_eq!(out.passos.len(), 11);
    assert!(out.quebra.is_none());
    assert!(out.json()["parada"].is_null());
    Ok(())
}

#[test]
fn destino_sem_sessao_nao_le_rotacao_implicitamente() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), true);
    let (source, dest) = chain(0, 101, 1);
    write_log(&e.origem, &source)?;
    let path = e.destino.as_deref().ok_or("dest")?;
    write_log(
        path,
        &dest
            .iter()
            .filter(|r| r["session"].is_null())
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    write_log(&path.with_extension("jsonl.1"), &dest)?;
    let out = verificar::executar(&e)?;
    assert!(out.quebra.is_none());
    assert_eq!(out.json()["parada"], "entrada_destino");
    assert!(out.texto().contains("rotacionado"));
    Ok(())
}

#[test]
fn cli_codigos_json_e_texto_respeitam_quebra_e_parada() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), true);
    let (mut source, dest) = chain(0, 101, 1);
    write_log(&e.origem, &source)?;
    let destination = e.destino.as_deref().ok_or("dest")?;
    write_log(destination, &dest)?;
    let invoke = |args: &[&str]| -> Result<std::process::Output, std::io::Error> {
        Command::new(env!("CARGO_BIN_EXE_clipsync"))
            .args(["verificar-travessia", "--origem"])
            .arg(&e.origem)
            .arg("--destino")
            .arg(destination)
            .args(args)
            .output()
    };
    assert_eq!(invoke(&[])?.status.code(), Some(0));
    source.retain(|v| v["evento"] != "session.closed");
    write_log(&e.origem, &source)?;
    let j = invoke(&["--json"])?;
    assert_eq!(j.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&j.stdout)?;
    assert_eq!(v["parada"], "fechamento");
    assert!(v["quebra"].is_null());
    let human = String::from_utf8(invoke(&[])?.stdout)?;
    assert!(human.contains("[sem dado] fechamento"));
    assert_eq!(
        invoke(&["--sessao", "101", "--ultima"])?.status.code(),
        Some(2)
    );
    assert_eq!(invoke(&["--desde", "não-data"])?.status.code(), Some(2));
    std::fs::remove_file(destination)?;
    assert_eq!(invoke(&[])?.status.code(), Some(3));
    write_log(destination, &dest)?;
    source[0]["v"] = json!(2);
    write_log(&e.origem, &source)?;
    let future = invoke(&["--json"])?;
    assert_eq!(future.status.code(), Some(3));
    assert!(future.stdout.is_empty());
    Ok(())
}

#[test]
fn sessao_pedida_sem_open_preserva_tentativa_e_prova_a_lacuna() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut e = entrada(dir.path(), true);
    e.ultima = false;
    e.sessao = Some(101);
    let (mut source, dest) = chain(0, 101, 1);
    source.retain(|v| v["evento"] != "session.open");
    write_log(&e.origem, &source)?;
    write_log(e.destino.as_deref().ok_or("destino")?, &dest)?;
    let out = verificar::executar(&e)?;
    assert_eq!(out.tentativa, Some(1));
    assert!(out.passos[4].estado == Estado::Ok);
    assert_eq!(out.json()["parada"], "sessao");
    assert!(out.quebra.is_none());
    Ok(())
}

#[test]
fn sessao_inexistente_e_uso_invalido_nao_parecem_sucesso() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let e = entrada(dir.path(), false);
    write_log(&e.origem, &ready())?;
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_clipsync"))
            .args(["verificar-travessia", "--origem"])
            .arg(&e.origem)
            .args(args)
            .output()
    };
    let missing = invoke(&["--sessao", "999", "--json"])?;
    assert_eq!(missing.status.code(), Some(1));
    let out: Value = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(out["sessao"], 999);
    assert!(out["tentativa"].is_null());
    assert_eq!(out["passos"].as_array().ok_or("passos")?.len(), 11);
    assert!(!out["parada"].is_null());
    let text = invoke(&["--sessao", "999"])?;
    assert!(String::from_utf8(text.stdout)?.contains("sessão pedida não comprovada"));
    assert_eq!(invoke(&["--sessao", "0"])?.status.code(), Some(2));
    assert_eq!(invoke(&["--ultima"])?.status.code(), Some(0));
    let mut source = ready();
    source.push(event("routes.empty", json!({"motivo":"no_links"})));
    write_log(&e.origem, &source)?;
    assert_eq!(invoke(&["--ultima"])?.status.code(), Some(1));
    Ok(())
}

#[test]
fn rotas_e_captura_de_outro_par_nao_provam_o_par_selecionado() -> Result<(), Box<dyn Error>> {
    for (event_name, step) in [("routes.built", "rotas"), ("capture.ready", "captura")] {
        let (mut source, dest) = chain(0, 101, 1);
        let record = source
            .iter_mut()
            .find(|v| v["evento"] == event_name)
            .ok_or("evento")?;
        if step == "rotas" {
            record["pares"] = json!(["bb"]);
        } else {
            record["barreiras"][0]["peer"] = json!("bb");
        }
        let out = run_logs(&source, &dest)?;
        assert!(
            out.passos
                .iter()
                .find(|p| p.nome == step)
                .ok_or("passo")?
                .estado
                == Estado::SemDado,
            "{step}"
        );
        assert_eq!(out.json()["parada"], step);
        assert!(out.quebra.is_none());
    }
    Ok(())
}

#[test]
fn reinicio_invalida_prerequisitos_da_execucao_anterior() -> Result<(), Box<dyn Error>> {
    let (mut old, mut old_dest) = chain(0, 101, 1);
    old.push(timed(event("agent.up", json!({})), 10, 0));
    let (mut new, new_dest) = chain(20, 202, 1);
    new.retain(|v| {
        !matches!(
            v["evento"].as_str(),
            Some(
                "inventory.prepared"
                    | "inventory.sent"
                    | "inventory.received"
                    | "routes.built"
                    | "capture.ready"
            )
        )
    });
    old.extend(new);
    old_dest.extend(new_dest);
    let out = run_logs(&old, &old_dest)?;
    assert_eq!(out.sessao, Some(202));
    assert_eq!(out.json()["parada"], "inventario_origem");
    assert!(out.passos[..4].iter().all(|p| p.estado == Estado::SemDado));
    assert!(out.quebra.is_none());
    Ok(())
}

#[test]
fn reinicio_do_destino_tambem_invalida_inventario_antigo() -> Result<(), Box<dyn Error>> {
    let (source, mut dest) = chain(0, 101, 1);
    dest.push(timed(event("agent.up", json!({})), 0, 1));
    let out = run_logs(&source, &dest)?;
    assert!(out.passos[1].estado == Estado::SemDado);
    assert_eq!(out.json()["parada"], "inventario_destino");
    assert!(out.quebra.is_none());
    Ok(())
}

#[test]
fn destino_de_outro_peer_nao_confirma_nem_recusa_a_sessao() -> Result<(), Box<dyn Error>> {
    let (source, mut dest) = chain(0, 101, 1);
    for r in &mut dest {
        if !r["session"].is_null() {
            r["peer"] = json!("cc");
        }
    }
    let out = run_logs(&source, &dest)?;
    assert_eq!(out.json()["parada"], "entrada_destino");
    assert!(out.quebra.is_none());
    assert!(out.incompleta());
    let (source, mut dest) = chain(0, 101, 1);
    dest.push(timed(
        event(
            "enter.refused",
            json!({"session":101,"peer":"cc","motivo":"locked"}),
        ),
        0,
        7,
    ));
    let out = run_logs(&source, &dest)?;
    assert!(out.quebra.is_none());
    assert!(out.json()["parada"].is_null());
    assert!(!out.incompleta());
    Ok(())
}

#[test]
fn destino_fora_da_execucao_ou_janela_nao_prova_sucesso() -> Result<(), Box<dyn Error>> {
    for (minute, reboot) in [(0, false), (0, true), (30, false)] {
        let (source, _) = chain(20, 101, 1);
        let (_, mut dest) = chain(0, 101, 1);
        for r in &mut dest {
            if !r["session"].is_null() {
                *r = timed(r.clone(), minute, 3);
            }
            if r["evento"] == "inventory.prepared" {
                *r = timed(r.clone(), 20, 0);
            }
        }
        if reboot {
            dest.push(timed(event("agent.up", json!({"id":"aa"})), 10, 0));
        }
        let out = run_logs(&source, &dest)?;
        assert_eq!(
            out.json()["parada"],
            "entrada_destino",
            "minuto={minute}, reinício={reboot}"
        );
        assert!(out.quebra.is_none());
    }
    Ok(())
}

#[test]
fn sem_identidade_da_execucao_nao_atribui_eventos_remotos() -> Result<(), Box<dyn Error>> {
    for remote in [false, true] {
        let (mut source, mut dest) = chain(0, 101, 1);
        let target = if remote { &mut dest } else { &mut source };
        target.retain(|v| v["evento"] != "agent.up");
        let out = run_logs(&source, &dest)?;
        assert_eq!(
            out.json()["parada"],
            if remote {
                "inventario_destino"
            } else {
                "entrada_destino"
            }
        );
        assert!(out.quebra.is_none());
        assert!(out.texto().contains("execução não comprovada"));
    }
    Ok(())
}

#[test]
fn troca_preserva_fechamento_anterior_sem_misturar_tentativas() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let mut e = entrada(dir.path(), true);
    e.ultima = false;
    e.sessao = Some(101);
    let (mut source, dest) = chain(0, 101, 1);
    source.retain(|r| {
        !matches!(
            r["evento"].as_str(),
            Some("session.closed" | "return.received")
        )
    });
    source.extend([
        timed(
            event(
                "attempt.begin",
                json!({"attempt":2,"peer":"cc","origem":"switch"}),
            ),
            0,
            7,
        ),
        timed(
            event(
                "session.closed",
                json!({"attempt":1,"session":101,"peer":"aa","motivo":"switch"}),
            ),
            0,
            8,
        ),
        timed(
            event(
                "session.open",
                json!({"attempt":2,"session":202,"peer":"cc"}),
            ),
            0,
            9,
        ),
        timed(
            event(
                "session.sent",
                json!({"attempt":2,"session":202,"peer":"cc"}),
            ),
            0,
            10,
        ),
    ]);
    write_log(&e.origem, &source)?;
    write_log(e.destino.as_deref().ok_or("destino")?, &dest)?;
    let old = verificar::executar(&e)?;
    assert_eq!(old.tentativa, Some(1));
    assert!(old.passos[10].estado == Estado::Ok);
    assert_eq!(old.passos[10].motivo.as_deref(), Some("switch"));
    assert_eq!(old.json()["parada"], "retorno");
    assert!(old.quebra.is_none());
    e.sessao = Some(202);
    let next = verificar::executar(&e)?;
    assert_eq!(next.tentativa, Some(2));
    assert!(next.passos[4].estado == Estado::Ok);
    assert!(next.passos[10].estado == Estado::SemDado);
    assert!(
        !next.passos[10]
            .linha
            .as_ref()
            .is_some_and(|line| line.contains("switch"))
    );
    Ok(())
}

#[test]
fn novo_agent_up_impede_usar_fechamento_de_outra_execucao() -> Result<(), Box<dyn Error>> {
    let (mut source, dest) = chain(0, 101, 1);
    source.retain(|r| r["evento"] != "session.closed");
    source.extend([
        timed(event("agent.up", json!({"id":"bb"})), 0, 10),
        timed(
            event(
                "session.closed",
                json!({"attempt":1,"session":101,"peer":"aa","motivo":"return"}),
            ),
            0,
            11,
        ),
    ]);
    let dir = tempfile::tempdir()?;
    let mut e = entrada(dir.path(), true);
    e.ultima = false;
    e.sessao = Some(101);
    write_log(&e.origem, &source)?;
    write_log(e.destino.as_deref().ok_or("destino")?, &dest)?;
    let out = verificar::executar(&e)?;
    assert!(out.passos[10].estado == Estado::SemDado);
    assert!(out.quebra.is_none());
    Ok(())
}

#[test]
fn return_failed_do_par_prova_quebra_e_de_outro_par_e_ignorado() -> Result<(), Box<dyn Error>> {
    let (mut source, mut dest) = chain(0, 101, 1);
    source.retain(|r| {
        !matches!(
            r["evento"].as_str(),
            Some("return.received" | "session.closed")
        )
    });
    dest.retain(|r| r["evento"] != "return.sent");
    dest.push(timed(
        event(
            "return.failed",
            json!({"session":101,"peer":"bb","erro":"transport"}),
        ),
        0,
        8,
    ));
    let out = run_logs(&source, &dest)?;
    assert_eq!(out.quebra.as_deref(), Some("retorno"));
    assert_eq!(out.passos[9].motivo.as_deref(), Some("transport"));
    assert!(
        out.passos[9]
            .linha
            .as_ref()
            .is_some_and(|line| line.contains("return.failed"))
    );
    assert!(out.incompleta());
    let (source, mut dest) = chain(0, 101, 1);
    dest.push(timed(
        event(
            "return.failed",
            json!({"session":101,"peer":"cc","erro":"transport"}),
        ),
        0,
        8,
    ));
    let out = run_logs(&source, &dest)?;
    assert!(out.quebra.is_none());
    assert!(!out.incompleta());
    Ok(())
}

#[test]
fn inventario_remoto_exige_identidade_do_par_selecionado() -> Result<(), Box<dyn Error>> {
    for count in [0, 1] {
        let (source, mut destination) = chain(0, 101, 1);
        destination[0]["id"] = json!("cc");
        destination[1]["monitores"] = json!(count);
        let out = run_logs(&source, &destination)?;
        let inventory = &out.passos[1];
        assert!(
            inventory.estado == Estado::SemDado,
            "usou inventário de outro par com {count} monitores"
        );
        assert!(inventory.linha.is_none());
        assert!(
            inventory
                .motivo
                .as_deref()
                .is_some_and(|reason| reason.contains("execução não comprovada"))
        );
        assert!(out.quebra.is_none());
        assert_eq!(out.json()["parada"], "inventario_destino");
        assert!(out.incompleta());
        destination.remove(0);
        let out = run_logs(&source, &destination)?;
        assert!(out.passos[1].estado == Estado::SemDado);
        assert!(out.quebra.is_none());
    }
    Ok(())
}

#[test]
fn tentativa_sem_abertura_nao_herda_fechamento_da_anterior() -> Result<(), Box<dyn Error>> {
    for filtered in [false, true] {
        let (mut source, destination) = chain(0, 101, 1);
        source.retain(|r| {
            r["ts"]
                .as_str()
                .is_some_and(|ts| ts < "2026-11-04T14:00:07.000Z")
        });
        source.extend([
            timed(
                event(
                    "attempt.begin",
                    json!({"attempt":2,"peer":"aa","origem":"switch"}),
                ),
                0,
                7,
            ),
            timed(
                event(
                    "session.closed",
                    json!({"attempt":1,"session":101,"peer":"aa","motivo":"switch"}),
                ),
                0,
                8,
            ),
        ]);
        if filtered {
            source.push(timed(
                event(
                    "attempt.filtered",
                    json!({"attempt":2,"peer":"aa","motivo":"inventory_changed"}),
                ),
                0,
                9,
            ));
        }
        let out = run_logs(&source, &destination)?;
        assert_eq!(out.tentativa, Some(2));
        assert_eq!(
            out.sessao, None,
            "herdou sessão anterior; filtered={filtered}"
        );
        assert!(out.passos[10].estado == Estado::SemDado);
        assert!(out.passos[10].linha.is_none());
        assert!(out.incompleta());
        assert_eq!(out.quebra.as_deref(), filtered.then_some("tentativa"));
    }
    Ok(())
}
