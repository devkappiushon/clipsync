//! Caminho do log disponível sem agente e sem criar o arquivo.
use std::error::Error;
use std::process::Command;

#[test]
fn caminho_do_log_e_puro() -> Result<(), Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_clipsync"))
        .arg("--profile")
        .arg(tmp.path())
        .args(["log", "caminho"])
        .output()?;
    assert!(output.status.success(), "{:?}", output);
    assert_eq!(
        String::from_utf8(output.stdout)?.trim(),
        tmp.path().join("logs/clipsync.jsonl").display().to_string()
    );
    assert!(!tmp.path().join("logs").exists());
    Ok(())
}

#[test]
fn status_parado_mostra_log_sem_abrir() -> Result<(), Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let init = Command::new(env!("CARGO_BIN_EXE_clipsync"))
        .arg("--profile")
        .arg(tmp.path())
        .args([
            "init",
            "--name",
            "t",
            "--modules",
            "input",
            "--fake-adapters",
        ])
        .output()?;
    assert!(init.status.success(), "{:?}", init);
    let output = Command::new(env!("CARGO_BIN_EXE_clipsync"))
        .arg("--profile")
        .arg(tmp.path())
        .arg("status")
        .output()?;
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8(output.stdout)?.contains(&format!(
        "log: {}",
        tmp.path().join("logs/clipsync.jsonl").display()
    )));
    assert!(!tmp.path().join("logs").exists());
    Ok(())
}
