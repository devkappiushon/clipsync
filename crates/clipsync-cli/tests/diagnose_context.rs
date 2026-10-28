//! S8: diagnose respeita o perfil solicitado e mantém a coleta sem agente.
#[test]
fn diagnose_without_a_profile_preserves_system_checks_and_explains_missing_context() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_clipsync"))
        .args(["--profile", dir.path().to_str().unwrap(), "diagnose"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("sistema"), "{text}");
    assert!(text.contains("travessia de saída"), "{text}");
    assert!(text.contains("não conferido"), "{text}");
    assert!(!dir.path().join("layout.toml").exists());
}
