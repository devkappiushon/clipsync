//! Build do app: declara o comando `clipsync` no manifesto de ACL (contrato §12.7 regra 3). O
//! tauri-build gera daí as permissões `allow-clipsync` e `deny-clipsync`, e a primeira entra em
//! `capabilities/default.json`: com manifesto de app, nenhum comando passa sem permissão.

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&["clipsync"]);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);
    if let Err(error) = tauri_build::try_build(attributes) {
        // Mesmo caminho de `tauri_build::build()`, sem pânico (lints do workspace).
        println!("cargo::error={error:#}");
        std::process::exit(1);
    }
}
