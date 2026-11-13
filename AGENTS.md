# AGENTS.md · ClipSync

Instruções para agentes de código (Codex e outros) neste repositório. Leia inteiro antes da primeira mudança.

## O projeto

ClipSync compartilha teclado, mouse e área de transferência entre os computadores do usuário, num app só, sem servidor central, para substituir o Input Leap. Rust (workspace com 8 crates), app de desktop em Tauri 2, Linux (GNOME Wayland) e Windows. Versão deste checkout: **v0.5.3, publicada em 13/11/2026**. O que ainda está aberto fica em `docs/pendencias-v0.5.0.md`; o contrato §16 registra a S10 e `docs/pendencias-v0.4.0.md` preserva os aceites anteriores.

## Comece por aqui

1. `CONTEXTO.md` (topo): estado atual e pendências. O bloco automático de 07:54 é histórico anterior à S7.
2. `docs/workboard.md`, seção "Estado atual".
3. `docs/contexto-equipe.md`: entrada comum dos andares no Maestri, objetivo atual, fonte de verdade e limites de atuação.
4. `docs/contrato.md` §15 (S9, vigente), §14 (S8, histórico) e a seção da área que você vai tocar (§11 Windows e release, §12 interface, §13 layout de telas). Mudança de comportamento, API pública, protocolo ou dependência é mudança de contrato: registre na seção da fatia, no mesmo trabalho.
5. `docs/handoff-codex.md`: história de S1 a S7, anterior à S8.
6. `docs/seguranca.md` (revisões de segurança) e `docs/verdicts.md` (vereditos de critério): a última rodada de cada um; seção nova por rodada, nunca reescrever as antigas.

## Trabalho no Maestri

1. **Identidade:** rode `maestri list` antes de agir e use o nome que aparece em `You:`. Nome de sessão antiga não vale.
2. **Notas:** as notas do andar (feature-spec, workboard) trazem a tarefa e apontam para `docs/contexto-equipe.md`. Escreva só na sua seção, com `maestri note edit`. Nunca `note write` em nota compartilhada, nunca `note delete`. A fonte de verdade são os arquivos de `docs/`, não as notas.
3. **Terminais:** fale com o seu coordenador por `maestri ask` e acompanhe com `maestri check`, sem reenviar o pedido. Não recrute, não reinicie papéis e não crie andares.
4. **Portais:** a interface só se vê pelos portais do Maestri (`maestri portal ...`) ou por AT-SPI (`docs/teste-app-real.md`). Medida de layout sai de `portal evaluate` (`getBoundingClientRect`, `scrollWidth`), com viewport e cenário anotados. Fluxo que muda o estado do simulador usa portal próprio. Simulador não prova Windows nem Linux físicos.
5. **Runtime:** o Maestro central é dono do runtime. O endereço reservado é `http://127.0.0.1:5174`; confirme o estado atual com o Maestro, pois a existência de um portal não prova servidor ativo. Não suba outro servidor. O `servir-ui-dev.py` é para quem trabalha fora do Maestri.
6. **Um editor por arquivo:** todos os andares usam o mesmo checkout. Combine a posse com o Maestro antes de editar. Arquivo sujo de outro autor: avise o Maestro central antes. Não edite `.maestri/` (gerado) nem as instruções globais de `~/.agents` e `~/.codex`.
7. **Histórico:** `docs/verdicts.md` e `docs/seguranca.md` ganham seção nova por rodada; o contrato muda por entrada nova de versão; nada antigo é reescrito.

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
cargo clippy --workspace --exclude clipsync-app --all-targets --locked -- -D warnings
cargo test --workspace --exclude clipsync-app --locked
# checagem do Windows sem Windows (compila e passa o clippy; não roda teste):
CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true \
  cargo clippy --target x86_64-pc-windows-gnu -p clipsync-input -p clipsync-win --all-targets -- -D warnings
node --max-old-space-size=512 --test --test-concurrency=2 crates/clipsync-app/ui-dev/testes/*.test.mjs
scripts/conferir-unsafe.sh                 # unsafe só no ffi.rs (também roda no CI)
scripts/conferir-privacidade.sh            # nada de log de tecla no código dos hooks (também roda no CI)
python3 -I scripts/servir-ui-dev.py        # interface com simulador em http://127.0.0.1:5174 (fora do Maestri)
```

- O app Tauri (`crates/clipsync-app`) **não compila aqui** (faltam os headers do WebKitGTK e não há sudo). Ele compila e testa só no CI.
- Os `default-members` do `Cargo.toml` não incluem o app nem o `clipsync-ui`; por isso os comandos locais são explícitos com `--exclude clipsync-app`, senão os testes da lógica das telas não rodam.
- O CI usa `--workspace` sem `--exclude` porque lá o app compila; não copie o comando do CI para uso local.
- `node --test` verifica os consumidores e o simulador sem `npm`. O CI tem um job `ui-node` separado, Node 24, heap 512 MiB por processo e concorrência 2; configuração no YAML não equivale a execução aprovada no GitHub.
- Testes `cfg(windows)` (hooks, SendInput, desktop seguro) só rodam no CI do Windows.
- CI: `.github/workflows/ci.yml` (ubuntu-latest e windows-latest; fmt só no job Linux; clippy e test nos dois com `--workspace --locked`; mais os dois scripts e o job Node no Ubuntu).
- **Nunca crie `target/` em `/tmp`.** Para conferir o lockfile de um commit, use `git archive` numa pasta e `cargo metadata --offline --locked`, que não compila.
- Cenários do simulador da interface: a tabela completa, com `?cenario=...`, `&tema=` e `&plataforma=windows`, está em `crates/clipsync-app/ui/README.md`.

## Invariantes que não podem quebrar

1. **unsafe isolado.** O workspace tem `unsafe_code = "forbid"`. O único lugar com `unsafe` é `crates/clipsync-win/src/ffi.rs`, com o único `#![allow(unsafe_code)]` do repositório, um `// SAFETY:` por bloco e nenhuma `unsafe fn` exportada. O `clipsync-win` tem lints próprios (`unsafe_code`, `unsafe_op_in_unsafe_fn`, `undocumented_unsafe_blocks` e `multiple_unsafe_ops_per_block` em `deny`). O CI falha se isso mudar.
2. **Privacidade dos hooks do Windows** (contrato §13.2 regra 2): hook só com o módulo input ligado **e** pelo menos uma ligação desta máquina no `layout.toml`; evento injetado passa sem ser lido; fora da captura, o hook só confere o cruzamento, o conjunto do que está pressionado agora e os atalhos; nenhum log, `Debug` ou `tracing` com tecla; pausar remove os hooks.
3. **Dependências fixadas com `=`** em `[workspace.dependencies]` do `Cargo.toml` raiz, builds com `--locked`. Feature nova do crate `windows` é mudança de contrato. O `Cargo.lock` muda no mesmo commit da dependência.
4. **Protocolo**: variante nova só no fim do enum (postcard). `PROTOCOL_VERSION = 4`, `MIN_PROTOCOL_VERSION = 1` (S10; variantes v4 exigem par v4, bytes v1–v3 preservados). Mensagem nova confere a versão do par e dá `RemoteTooOld` para par antigo. Campo novo em struct JSON do controle leva `#[serde(default)]`.
5. **Lints do workspace**: `unwrap`, `expect`, `panic`, `todo`, `unimplemented` e `dbg!` proibidos; `clippy -D warnings` nos dois sistemas.
6. **Interface** (`crates/clipsync-app/ui`): sem `innerHTML`, sem `style=`, todo `<script>` com `src`, nenhum travessão em texto, fontes só locais (Inter, Geist, Geist Mono), texto vindo do par só por `textContent`. Recurso futuro sem backend permanece visível e desabilitado. Nos ramos da S10 (§16.1), distinguir suporte local ausente, módulo comprovadamente desligado e estado desconhecido; booleano remoto falso não comprova módulo ausente. O simulador mora em `ui-dev/`, fora do pacote. Crédito discreto "feito por montesouza.dev".
7. **App**: a interface fala com o agente só pelo comando `clipsync` (`Ui::call`) e pelos eventos `clipsync://event`. Capabilities mínimas: `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`. A exceção de zoom S9 (§15.5) permite também `core:webview:allow-set-webview-zoom`, somente para o invocador main e sem acesso remoto; não ampliar outras permissões.

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
