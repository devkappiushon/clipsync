# AGENTS.md · ClipSync

Instruções para agentes de código (Codex e outros) neste repositório. Leia inteiro antes da primeira mudança.

## O projeto

ClipSync compartilha teclado, mouse e área de transferência entre os computadores do usuário, num app só, sem servidor central, para substituir o Input Leap. Rust (workspace com 8 crates), app de desktop em Tauri 2, Linux (GNOME Wayland) e Windows. Versão publicada mais nova: **v0.3.1**.

## Comece por aqui

1. `docs/handoff-codex.md`: história de S1 a S7, arquitetura, estado atual, o problema aberto ("o mouse não passa") e os próximos passos. **Leia primeiro.**
2. `docs/contrato.md`: contrato vinculante por fatia (§11 S5 Windows e release, §12 S6 interface, §13 S7 layout de telas). Mudança de comportamento, API pública, protocolo ou dependência é mudança de contrato: registre na seção da fatia, no mesmo trabalho.
3. `docs/workboard.md`: commits, critérios e pendências por fatia.
4. `docs/seguranca.md` (revisões de segurança) e `docs/verdicts.md` (vereditos de critério): uma seção nova por rodada, nunca reescrever as antigas.
5. `docs/contexto-equipe.md`: entrada comum dos andares, objetivo atual, fonte de verdade e limites de atuação. Todos os andares compartilham o mesmo diretório; combine posse de arquivos com o Maestro antes de editar.

## Como o usuário quer que você trabalhe

- **Português em tudo**: resposta, commit, documento e texto de interface. Identificador de código e termo técnico ficam como estão.
- Resposta curta: ação na primeira linha, passos numerados, sem preâmbulo e sem fecho. Uma recomendação, não um cardápio de opções.
- **Verificado não é suposto.** Só diga que funciona se viu funcionar, e cite a evidência (comando e saída). O que não deu para verificar, diga que não verificou e por quê.
- Prefira a solução durável à rápida que vai ser refeita.
- Se o usuário estiver errado, discorde uma vez, com argumento concreto; depois respeite a decisão.
- **Commits**: português, minúscula no início, verbo no imperativo (`adiciona`, `ajusta`, `remove`, `corrige`), dizendo o efeito e não o arquivo. **Nunca** `Co-Authored-By` nem qualquer co-autor.
- **Só commite ou dê push quando o usuário pedir.** Push na `main` roda o CI. Push de tag `v*` publica um release: confirme antes.
- Segredo não entra em log, resposta, commit nem tela. Não leia nem escreva `.env`. O arquivo `~/.config/clipsync/config.json` é de **outro programa** e guarda uma chave: nunca mostre nem copie o conteúdo.
- **Automação de navegador é proibida** em qualquer forma (a biblioteca da Microsoft para isso, por MCP, `npx` ou script, e qualquer equivalente). Para ver a interface, use o simulador no navegador do usuário ou a acessibilidade (AT-SPI) da janela real.
- A conta do git é `montesouzajoao` / `devkappiushon`. Não altere a configuração do git.
- Não há `sudo` nesta máquina (notebook). Instalação de `.deb` fica com o usuário.

## Comandos

```sh
export PATH=$HOME/.cargo/bin:$PATH        # o cargo vem do rustup (toolchain 1.99.1)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings  # sem --workspace: o default-members exclui o app
cargo test                                 # idem
# checagem do Windows sem Windows (compila e passa o clippy; não roda teste):
CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true \
  cargo clippy --target x86_64-pc-windows-gnu -p clipsync-input -p clipsync-win --all-targets -- -D warnings
scripts/conferir-unsafe.sh                 # unsafe só no ffi.rs (também roda no CI)
scripts/conferir-privacidade.sh            # nada de log de tecla no código dos hooks (também roda no CI)
python3 -I scripts/servir-ui-dev.py        # interface com simulador em http://127.0.0.1:5174
```

- O app Tauri (`crates/clipsync-app`) **não compila aqui** (faltam os headers do WebKitGTK e não há sudo). Ele compila e testa só no CI.
- Testes `cfg(windows)` (hooks, SendInput, desktop seguro) só rodam no CI do Windows.
- CI: `.github/workflows/ci.yml` (ubuntu-latest e windows-latest; fmt, clippy e test com `--workspace --locked`, mais os dois scripts).
- **Nunca crie `target/` em `/tmp`.** Para conferir o lockfile de um commit, use `git archive` numa pasta e `cargo metadata --offline --locked`, que não compila.
- Cenários do simulador da interface: tabela em `crates/clipsync-app/ui/README.md` (`?cenario=padrao`, `vazio`, `erro`, `estados`, `pareamento`, `sem-perfil`, `windows`, `windows-recusa`; `&tema=claro|escuro`; `&plataforma=windows`).

## Invariantes que não podem quebrar

1. **unsafe isolado.** O workspace tem `unsafe_code = "forbid"`. O único lugar com `unsafe` é `crates/clipsync-win/src/ffi.rs`, com o único `#![allow(unsafe_code)]` do repositório, um `// SAFETY:` por bloco e nenhuma `unsafe fn` exportada. O `clipsync-win` tem lints próprios (`unsafe_code`, `unsafe_op_in_unsafe_fn`, `undocumented_unsafe_blocks` e `multiple_unsafe_ops_per_block` em `deny`). O CI falha se isso mudar.
2. **Privacidade dos hooks do Windows** (contrato §13.2 regra 2): hook só com o módulo input ligado **e** pelo menos uma ligação desta máquina no `layout.toml`; evento injetado passa sem ser lido; fora da captura, o hook só confere o cruzamento, o conjunto do que está pressionado agora e os atalhos; nenhum log, `Debug` ou `tracing` com tecla; pausar remove os hooks.
3. **Dependências fixadas com `=`** em `[workspace.dependencies]` do `Cargo.toml` raiz, builds com `--locked`. Feature nova do crate `windows` é mudança de contrato. O `Cargo.lock` muda no mesmo commit da dependência.
4. **Protocolo**: variante nova só no fim do enum (postcard). `PROTOCOL_VERSION = 3`, `MIN_PROTOCOL_VERSION = 1`. Mensagem nova confere a versão do par e dá `RemoteTooOld` para par antigo. Campo novo em struct JSON do controle leva `#[serde(default)]`.
5. **Lints do workspace**: `unwrap`, `expect`, `panic`, `todo`, `unimplemented` e `dbg!` proibidos; `clippy -D warnings` nos dois sistemas.
6. **Interface** (`crates/clipsync-app/ui`): sem `innerHTML`, sem `style=`, todo `<script>` com `src`, nenhum travessão em texto, fontes só locais (Inter, Geist, Geist Mono), texto vindo do par só por `textContent`. Recurso que não existe numa plataforma aparece desabilitado com "indisponível nesta versão". O simulador mora em `ui-dev/`, fora do pacote. Crédito discreto "feito por montesouza.dev".
7. **App**: a interface fala com o agente só pelo comando `clipsync` (`Ui::call`) e pelos eventos `clipsync://event`. Capabilities mínimas: `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`.

## Release

- A versão fica em `[workspace.package]` do `Cargo.toml` raiz. A tag `vX.Y.Z` tem de ser igual a ela.
- `release.yml`: a tag `v*` gera a CLI (tar.gz e zip), o app (AppImage e `.deb` no Linux; `setup.exe` e `.msi` no Windows) e o `SHA256SUMS`, e publica. Disparo manual (`gh workflow run release.yml --ref main`) só empacota, sem publicar: use para testar pacote antes da tag.
- Depois: `gh release download vX.Y.Z`, `sha256sum -c SHA256SUMS`. Linux: `scripts/instalar-app-linux.sh <AppImage>` (sem sudo) ou o `.deb` (o pacote se chama `clip-sync`).

## Armadilhas conhecidas

- Perfil em caminho longo quebra o socket de controle do agente (limite de 108 bytes do `sockaddr_un`). Para testes, use caminhos curtos (ex.: `~/.cache/csrt/t1`).
- O portal InputCapture v1 abre um diálogo por sessão; o RemoteDesktop guarda token. Os testes de portal são `#[ignore]` e manuais. Nunca aprove diálogo de permissão do sistema no lugar do usuário.
- Teste no Windows que mede handles do processo precisa rodar em série e com aquecimento.
- O CI do Windows enfileira; cancele um run antigo quando um mais novo cobre o mesmo código.
- Dá para dirigir a janela do app real no Linux pela acessibilidade (AT-SPI), sem navegador: ver `docs/teste-app-real.md`.
