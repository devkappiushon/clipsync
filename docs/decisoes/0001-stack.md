# 0001 · Stack do núcleo

Estado: aceita (2026-10-27) · Autor: Keel · Contrato: `docs/contrato.md` §1 e §2

## Contexto

O ClipSync roda por usuário, sem root, na sessão gráfica. A máquina de desenvolvimento (Ubuntu 26.04, GNOME 50.1, Wayland) não tem sudo e não tem cmake nem nasm. O produto precisa chegar depois a Windows e macOS. A UI Tauri só entra quando o usuário instalar as libs de sistema (prompt mestre §5).

## Decisão

1. Núcleo em Rust estável, toolchain fixado em `rust-toolchain.toml` (1.99.1), edition 2024. Workspace Cargo com 5 crates por fronteira: `clipsync-proto`, `clipsync-core`, `clipsync-cli`, `clipsync-clipboard`, `clipsync-input`. Os módulos dependem só de `proto`, nunca de `core`.
2. Versões fixadas com `=` no `[workspace.dependencies]`, resolvidas no spike do Bolt: tokio 1.54.1, quinn 0.11.12, rustls 0.23.46, rcgen 0.14.10, arboard 3.7.0, x11rb 0.13.2, ashpd 0.13.13, reis 0.7.1, clap 4.6.7, serde 1.0.229, toml 1.1.8, tracing 0.1.44.
3. Criptografia só com o provider **ring**. rustls, quinn e rcgen ficam com `default-features = false`.
4. Clipboard no Linux: arboard sem `wayland-data-control` (cai para X11 sob XWayland) + x11rb com XFixes para observar. Uma versão só de x11rb: a que o arboard usa.
5. Entrada no Linux: ashpd com `input_capture`, `remote_desktop` e `screencast` (o remote_desktop não compila sem screencast) + reis com tokio.
6. Formatos: postcard no fio entre máquinas, serde_json no socket local CLI ↔ agente, TOML versionado para configuração.
7. Hoje não entram Tauri, Node nem libs de sistema.

## Consequências

- Compila sem lib de sistema e sem pkg-config (spike do Bolt). Build frio ~6 min no workspace inteiro, incremental ~30 s. Os builders compartilham o `target/` da raiz.
- O aws-lc fica fora porque exige cmake e nasm. Se um dia precisarmos de FIPS ou do aws-lc, revisamos esta decisão.
- `screencast` entra no binário só por causa do ashpd. Ele não é usado.
- Lints do workspace: `unsafe_code = "forbid"`, `unwrap_used`/`expect_used`/`panic` negados fora de teste. Um adaptador Windows futuro vai precisar de `unsafe` (`SendInput`). Ele ganha exceção por crate, registrada em ADR.
- Licença ainda em aberto. O usuário decide antes do primeiro commit público (ver Decisões no `docs/workboard.md`).
