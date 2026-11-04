# Desenvolvimento

Requisitos e comandos para compilar e testar o ClipSync. As regras de trabalho do repositório ficam em [AGENTS.md](../AGENTS.md); este guia concentra o ambiente, os comandos e as armadilhas.

## Requisitos

- Rust 1.99.1 via `rustup`. O `rust-toolchain.toml` fixa a versão, e o `PATH` precisa incluir `~/.cargo/bin`.
- Linux: `build-essential` para a CLI e os crates.
- App Tauri 2: headers e libs do WebKitGTK (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`). Sem eles, compile só a CLI e os crates, com `--exclude clipsync-app`.
- Linux GNOME Wayland para os portais de entrada; XWayland ativo (`DISPLAY` definido) para o clipboard.
- Node para os testes da interface e do simulador. O CI usa Node 24, heap de 512 MiB por processo e concorrência 2.
- O script do AppImage instala por usuário e não chama sudo. O `.deb` e as bibliotecas do sistema podem exigir administrador.

Só a sessão gráfica do Ubuntu 26.04 com GNOME 50.1 foi testada. O binário Linux do release é compilado no `ubuntu-24.04` e o CI usa `ubuntu-latest`; KDE e X11 puro não foram testados.

## Preparar o Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --component clippy,rustfmt --no-modify-path
export PATH="$HOME/.cargo/bin:$PATH"
```

## Comandos

Para a checagem cruzada do Windows, instale o alvo uma vez, se ainda não estiver disponível:

```sh
rustup target add x86_64-pc-windows-gnu
```

```sh
cargo fmt --all --check
cargo clippy --workspace --exclude clipsync-app --all-targets --locked -- -D warnings
cargo test --workspace --exclude clipsync-app --locked
CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true \
  cargo clippy --target x86_64-pc-windows-gnu -p clipsync-input -p clipsync-win --all-targets -- -D warnings
node --max-old-space-size=512 --test --test-concurrency=2 crates/clipsync-app/ui-dev/testes/*.test.mjs
scripts/conferir-unsafe.sh
scripts/conferir-privacidade.sh
python3 -I scripts/servir-ui-dev.py
```

Para gerar o binário da CLI:

```sh
cargo build --release -p clipsync-cli --locked
```

O binário fica em `target/release/clipsync`.

## O que roda onde

- O app Tauri (`crates/clipsync-app`) exige os headers do WebKitGTK. Se eles não estiverem instalados, o app não compila; nesse caso, compile a CLI e os crates com `--exclude clipsync-app`. O app compila e testa no CI.
- Os `default-members` do `Cargo.toml` não incluem o app nem o `clipsync-ui`; por isso os comandos locais são explícitos com `--workspace --exclude clipsync-app`. Sem isso, os testes da lógica das telas não rodam.
- O CI usa `--workspace` sem `--exclude` porque lá o app compila. Não copie o comando do CI para uso local.
- Os testes `cfg(windows)` (hooks, SendInput, desktop seguro) só rodam no CI do Windows.
- A checagem cruzada do Windows compila e passa o clippy para `clipsync-input` e `clipsync-win`; ela não executa testes.
- `node --test` verifica os consumidores e o simulador sem `npm`. O job `ui-node` do CI roda separado, com Node 24. Configuração no YAML não equivale a execução aprovada no GitHub.

## Interface

A interface local fica em `http://127.0.0.1:5174`. O comando é `python3 -I scripts/servir-ui-dev.py`. A tabela de cenários do simulador, com `?cenario=...`, `&tema=` e `&plataforma=windows`, está em [crates/clipsync-app/ui/README.md](../crates/clipsync-app/ui/README.md).

## Documentos

- [Contrato](contrato.md): decisões vinculantes; o §15 registra a S9, o §14 a S8, o §11 a S5, o §12 a interface, o §13 o layout e o §16 a S10.
- [Prompt mestre](prompt-mestre.md): especificação v4 de origem; o anexo detalhado é [prompt-mestre-anexo-original.md](prompt-mestre-anexo-original.md).
- [Handoff](handoff-codex.md): história de S1 a S7, anterior à S8.
- [Decisões](decisoes/): ADRs de stack, transporte QUIC e pareamento.
- [Telas](telas.md): especificação da S6, base do protótipo no Figma.
- [Matriz de compatibilidade](matriz-compatibilidade.md): evidência dos spikes de 2026-10-27.
- [Teste do app real](teste-app-real.md): como dirigir a janela pela acessibilidade (AT-SPI).
- [Notas da v0.3.1](releases/v0.3.1.md) e [notas da v0.4.0](releases/v0.4.0.md): histórico de versões.

## CI

O workflow [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) roda em `ubuntu-latest` e `windows-latest`. O `fmt` roda só no job Linux; `clippy` e `test` rodam nos dois com `--workspace --locked`, junto com os dois scripts de conferência e o job Node no Ubuntu.

## Armadilhas

- Perfil em caminho longo quebra o socket de controle do agente (limite de 108 bytes do `sockaddr_un`). Para testes, use caminhos curtos, como `~/.cache/csrt/t1`.
- Nunca crie `target/` em `/tmp`. Para conferir o lockfile de um commit, use `git archive` numa pasta e `cargo metadata --offline --locked`.
- O portal InputCapture v1 abre um diálogo por sessão; o RemoteDesktop guarda token. Os testes de portal são `#[ignore]` e manuais.
- Teste no Windows que mede handles do processo precisa rodar em série e com aquecimento.
