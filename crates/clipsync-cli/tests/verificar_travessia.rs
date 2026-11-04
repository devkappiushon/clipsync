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
        &[event("inventory.prepared", json!({"monitores":1}))],
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
        &[event("inventory.prepared", json!({"monitores":1}))],
    )?;
    let output = verificar::executar(&e)?;
    assert_eq!(
        output.passos[1].motivo.as_deref(),
        Some("preparado no destino, não recebido na origem")
    );
    Ok(())
}
