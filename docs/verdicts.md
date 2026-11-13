# Vereditos e achados (dono: Argus · warden)

> **Índice de rodadas** (navegação; o texto abaixo não foi alterado)
> - Verificações planejadas e Verdicts: critérios e vereditos de S0 a S7.
> - Findings: achados consolidados.
> - S8 · diagnóstico e configuração da travessia (2026-10-28).
> - Preparação da v0.3.1: revisão e validação antes do commit.
> - Publicação da v0.3.1 e validação no CI.
> - Alinhamento do contexto entre os andares.

## Verificações planejadas

Responsável: Argus · warden. Plano publicado em 2026-10-27 antes da execução; **resultados reais posteriores estão em Verdicts**. Fonte de verdade: [contrato.md](contrato.md), v1 de Keel, em especial §§1–9; posse e ordem em [workboard.md](workboard.md). Adaptação do roteiro anterior `verificacoes-planejadas.md` aos cinco crates e à CLI efetiva do contrato. Estes comandos são planos, não saídas observadas nem aprovação.

Argus só altera este relatório; não edita código, não roda git e não inicia servidores ou agentes persistentes. Os testes automatizados podem iniciar processos filhos e devem encerrá-los. Itens **M** são executados pelo Maestro/usuário em sessão gráfica, conforme §9; Argus registra a evidência recebida, distinguindo-a da observação própria. Se houver interface web, a URL vem do Maestro e a validação usa exclusivamente o portal do Maestri; nunca Playwright. O contrato atual é uma CLI, sem Tauri ou servidor web exigido.

### Evidência e regra de veredito

- A cada revisão solicitada, reler a fatia da §9 e sua posse no workboard; registrar a versão/hash do contrato e dos arquivos revisados com `sha256sum`, sem git. Mudança concorrente que invalide a evidência exige nova conferência focada.
- Em cada bloco de `Verdicts`: fatia, data/hora, diretório, ambiente relevante, comando literal efetivamente executado, stdout/stderr relevante e código de saída. Para testes, registrar nomes, quantidade executada, aprovada, falha e ignorada. Saída de zero testes ou só ignorados não comprova requisito; inspecionar também as asserções dos testes.
- Os filtros `pair_`, `pairing_`, `mesh_`, `tls_`, `stream_and_datagram`, `pause_` e `clip_` vêm da §9. Conferir primeiro a listagem. O contrato não atribui crate ao teste `stream_and_datagram` nem a `pause_`; os comandos abaixo usam o workspace, sem inventar localização.
- Classificar cada requisito como **implementado e testado aqui**, **implementado, depende de outra máquina/sessão** ou **planejado**. Falha de dependência, ausência de sessão, comando inexistente ou cobertura ausente fica explícita. Testes falsos não comprovam adaptador gráfico real.
- `Passa` exige todos os gates pertinentes e evidência do comportamento. Se um item M obrigatório não foi observado, registrar **validação incompleta**, sem aprovar a fatia inteira. Achados em ordem de severidade: `arquivo:linha`, defeito, impacto, reprodução e evidência. Não corrigir implementação durante a revisão.
- Só usar dados sintéticos. Não transcrever chave privada, certificado privado, clipboard real, tecla ou keycode. Preview sintético de `inbox` é saída da funcionalidade; não autoriza conteúdo em logs. Expurgar eventual vazamento na evidência publicada.
- Executar comandos separadamente e registrar seus resultados, inclusive falhas esperadas. Não tratar o código esperado de `cargo tree -i aws-lc-*`, teste de ausência de perfil ou probe sem DISPLAY como erro inesperado; também não aceitar qualquer falha como prova de ausência.

### Preparação comum

Atualização operacional do Maestro: julgar somente o export limpo informado, atualmente `e611a88`; fontes em `~/.cache/clipsync-verdict/src`, cache em `~/.cache/clipsync-verdict/target`. Não executar gates na árvore compartilhada em edição nem no export antigo de /tmp. Todos os blocos abaixo usam a raiz do export, após carregar o ambiente Cargo:

```sh
cd ~/.cache/clipsync-verdict/src
source ~/.cargo/env
export CARGO_TARGET_DIR="$HOME/.cache/clipsync-verdict/target"
date --iso-8601=seconds
sha256sum docs/contrato.md docs/workboard.md
rustc --version
cargo --version
rustup show active-toolchain
cargo metadata --locked --no-deps --format-version 1
cargo test --workspace -- --list
```

Conferir `clipsync-proto`, `clipsync-core`, `clipsync-cli`, `clipsync-clipboard` e `clipsync-input`, binário `clipsync`, Rust 1.99.1 e os testes existentes. Não executar cedo: esta preparação acompanha a revisão enviada pelo Maestro. Usar o cache de validação indicado pelo Maestro nos gates normais; não mudar configuração versionada de target-dir. Os comandos da §9 abaixo são mantidos sem acrescentar `--locked`; o inventário e a prova fria adicional verificam também o lockfile.

### S0 · Bolt · §9, itens 1–7

```sh
source ~/.cargo/env
cat rust-toolchain.toml
rustup show active-toolchain
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
grep -n 'unwrap_used\|unsafe_code' Cargo.toml
cat clippy.toml
cat .github/workflows/ci.yml
python3 -c "import yaml,sys;yaml.safe_load(open('.github/workflows/ci.yml'))"
cargo tree -i ring
cargo tree -i aws-lc-sys
cargo tree -i aws-lc-rs
cargo tree -d -i x11rb
```

Evidência exigida:

1. Toolchain `1.99.1`, componentes rustfmt e clippy, cinco crates da §1, edition 2024, rust-version 1.99 e `[lints] workspace = true` em cada crate.
2. Build/test/fmt/clippy saem 0. Na entrega S0, um smoke por crate, total 5; se a revisão acontecer após outras fatias, identificar os cinco smoke separadamente e registrar a contagem real, sem exigir que a suíte maior tenha só cinco testes.
3. Lints `unsafe_code = "forbid"` e `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro = "deny"`; exceções de unwrap/expect/panic só em testes no clippy.toml.
4. CI em ubuntu-latest com fmt, clippy `-D warnings` e test workspace; YAML parseado. Se PyYAML estiver ausente, registrar o impedimento; não instalar pacote por conta própria nem chamar o gate de verde.
5. Dependências da §1 fixadas com `=`; ring presente, aws-lc-sys/aws-lc-rs ausentes pelo motivo específico do Cargo, x11rb sem duas versões. Conferir features de ashpd (`screencast` junto de `remote_desktop`), arboard sem wayland-data-control, rustls/quinn sem provider aws-lc.

Prova adicional de instalação e build com artefatos novos, quando S0 estiver entregue. O target temporário vale somente no subshell, preservando o target compartilhado dos builders:

```sh
(
  source ~/.cargo/env
  export CARGO_TARGET_DIR="$(mktemp -d "$HOME/.cache/clipsync-argus-target.XXXXXX")"
  CLIPSYNC_INSTALL_DIR="$(mktemp -d "$HOME/.cache/clipsync-argus-install.XXXXXX")"
  cargo fetch --locked
  cargo build --workspace --all-targets --locked
  cargo install --locked --path crates/clipsync-cli --root "$CLIPSYNC_INSTALL_DIR"
  "$CLIPSYNC_INSTALL_DIR/bin/clipsync" --help
)
```

Registrar o código de **cada comando**, não só o último do subshell. Artefatos novos provam build sem reaproveitar `target`; o cache de download não foi removido. Em S0, o smoke/binário pode ainda não ter a CLI da §6: separar instalação/inicialização do teste funcional da ajuda, que passa a ser obrigatório em S1. O fluxo vivo de desenvolvimento é coberto pelos filhos `clipsync run` de `--test roteiro mesh_`, não apenas por `--help`; Argus não inicia um `run` persistente.

### S1 · Rivet · base, P2 e §9, itens 1–10

```sh
source ~/.cargo/env
cargo test -p clipsync-proto -- --list
cargo test -p clipsync-core -- --list
cargo test -p clipsync-cli --test roteiro -- --list
cargo test -p clipsync-proto
cargo test -p clipsync-cli --test roteiro pair_
cargo test -p clipsync-core pairing_
cargo test -p clipsync-cli --test roteiro mesh_
cargo test -p clipsync-core tls_
cargo test --workspace stream_and_datagram -- --nocapture
cargo test --workspace pause_
cargo run -q -p clipsync-cli -- --help
cargo run -q -p clipsync-cli -- diagnose
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

`--nocapture` só deve revelar medições/metadados permitidos; não publicar payloads ou segredos caso um teste os vaze. Exigir correspondência entre os casos efetivamente executados e:

1. **Proto:** limites antes de alocar; datagrama acima de 1.200 bytes; NaN, pos=1.5, 33 eventos, nome de 65 bytes; Debug redigido; tabela de `authorize_*`; `MemHub` usando as mesmas regras. Ack, Stats, StatsRequest, Leave e ReleaseAll ignoram pausa/permissão, mas exigem módulo local ligado; Offer, Enter e datagramas são bloqueados. Conferir a ordem da primeira falha: saída LocalModuleDisabled → RemoteModuleDisabled → NotAuthorized → Paused; entrada ModuleDisabled → NotAuthorized → Paused, respeitando as exceções por mensagem da §4.
2. **Pareamento:** host/join locais mostram o mesmo `Código: NNN NNN`; ambos confirmam; permissões de `--allow` persistem nos respectivos perfis. Recusa de um não grava em nenhum. Commit/Reveal divergente, conexão extra, nova tentativa após falha e nonces distintos são cobertos; falha encerra host com `PairFailed`, saída 3.
3. **Malha:** dois filhos `run`, `status` com Connected nos dois, módulos/permissões; terceiro desconhecido recusado; reinício reconecta; identidade alterada bloqueia com motivo. Filhos e sockets de teste são encerrados ao terminar.
4. **TLS:** contador de PinVerifier aumenta nos dois lados na conexão e reconexão; sem retomada; certificado desconhecido nunca gera Connected/PeerUp no servidor; `connect()` bem-sucedido sozinho não prova autenticação. CONNECTION_CLOSE recebido só contém token de `CloseCode::reason()`.
5. **P2:** stream uni e datagrama na mesma `Connection`, controle de 65.537 bytes recusado com RESET antes de alocar. Medições novas: datagrama 64 B e fluxo 1 KiB, p95 < 1 ms em ambos; registrar amostras/unidade/método. Números do spike são referência do contrato, não resultados desta revisão. Argus grava aqui e comunica ao Maestro para o workboard.
6. **Pausa:** state.toml guarda paused=true, status reflete, reinício preserva, resume volta sem replay. **Diagnóstico:** sessão, DISPLAY, Input Leap/Barrier e detecção de Flatpak. Conferir o ambiente da execução; não assumir que um processo descrito no contrato continua ativo.

Roteiro de init da §9, apenas em destinos de teste novos. Antes de usar os caminhos literais, conferir ausência; se já existirem, não remover ou sobrescrever: usar diretórios novos sob `mktemp -d /tmp/clipsync-argus-profiles.XXXXXX` e registrar os comandos com os caminhos resolvidos.

```sh
test ! -e /tmp/cs-a
test ! -e /tmp/cs-b
printf 'notebook-a\nn\n' | cargo run -q -p clipsync-cli -- init --profile /tmp/cs-a
stat -c '%a %n' /tmp/cs-a /tmp/cs-a/identity.key.pem
cargo run -q -p clipsync-cli -- status --profile /tmp/cs-a
cargo run -q -p clipsync-cli -- init --profile /tmp/cs-b --name notebook-b --modules input,clipboard --port 47801 --fake-adapters
stat -c '%a %n' /tmp/cs-b /tmp/cs-b/identity.key.pem
cargo run -q -p clipsync-cli -- status --profile /tmp/cs-b
```

Evidência: DIR 0700, chave 0600 (não ler a chave), config version=1, identidade distinta, peers sem pares; resposta `n` mantém clipboard desligado. Conferir também o caso de resposta vazia, com perfil novo e o mesmo prompt respondido por `printf 'notebook-a\n\n'`, e teste que recusa os dois módulos false. Perfil b deve ter clipboard.adapter=fake, input.capture=fake e input.emulate=fake. Status com agente parado retorna 4, config e `agente parado`; isso é o resultado esperado, não falha do init. Padrão de perfil: CLIPSYNC_PROFILE ou XDG_CONFIG_HOME/clipsync, senão ~/.config/clipsync; nunca confundir com /tmp/cs-a.

CLI de pareamento da §6, a ser exercitada pelos testes `pair_` (ou pelo Maestro caso peça reprodução manual), sem Argus subir hosts avulsos:

```sh
clipsync pair host --profile /tmp/cs-a --port 0 --timeout 120s --allow clipboard-in,clipboard-out
clipsync pair join 127.0.0.1:PORTA --profile /tmp/cs-b --allow clipboard-in,clipboard-out
```

`PORTA` é o único marcador de endereço neste bloco: substituir pela porta realmente impressa pelo host e registrar o comando resolvido. Estes perfis são uma fixture independente da prova de init; para clipboard bilateral, ambos precisam de `--modules input,clipboard --fake-adapters` ao serem criados. `--allow` não confirma o código: stdin `s/N` continua obrigatório nas duas pontas. Controle remoto exige a concessão `input-in` no destino e aviso de confiança total; não conceder para teste de clipboard.

Captura P2 opcional, sem sudo nem criptografia desligada, somente se a ferramenta tiver permissão e durante tráfego da fixture de teste identificado pela porta:

```sh
command -v tshark
command -v tcpdump
tshark -i lo -f 'udp and host 127.0.0.1' -a duration:20 -w /tmp/clipsync-argus-p2.pcapng
rg --text --fixed-strings 'clipsync-argus-p2-texto-sintetico' /tmp/clipsync-argus-p2.pcapng
```

Alternativa se só tcpdump estiver disponível: `timeout --signal=INT 20s tcpdump -i lo -U -w /tmp/clipsync-argus-p2.pcap 'udp and host 127.0.0.1'`, seguido de `rg --text --fixed-strings 'clipsync-argus-p2-texto-sintetico' /tmp/clipsync-argus-p2.pcap`. Registrar término por timeout. Durante a captura, a fixture envia essa sentinela pelo `send --stdin` e confirma entrega no inbox. Exigir captura não vazia do tráfego relevante; `rg` sair 1 só demonstra ausência da sentinela em claro. Sem ferramenta/permissão ou tráfego identificável: **não verificado**. Não iniciar pares persistentes para viabilizar a captura.

### S2 · Flux · clipboard/P1/P3 e §9, itens 1–5

```sh
source ~/.cargo/env
cargo test -p clipsync-clipboard -- --list
cargo test -p clipsync-clipboard
cargo clippy -p clipsync-clipboard --all-targets -- -D warnings
cargo fmt -p clipsync-clipboard --check
env -u DISPLAY cargo run -q -p clipsync-cli -- probe clipboard --write
env -u DISPLAY cargo run -q -p clipsync-cli -- diagnose
```

Evidência de `MemHub` + `FakeClipboard`:

- Malha completa A/B/C, uma cópia local em A: B e C recebem um write cada, total duas ofertas, nenhuma reoferta após 1 s. Conteúdo recebido **nunca é encaminhado**; o roteiro anterior de relay A→B→C→A foi substituído pelo contrato §7.
- Mesmo EventId duplicado: uma aplicação e Ack Duplicate. Lamport antigo não sobrescreve (Stale, caixa). X,Y,X gera três ofertas; X,X seguidas gera uma. Registrar a decisão ainda aberta na §10.6, sem afirmar aprovação do usuário.
- Eco de write_text/re-escrita idêntica, owner_takeover do Mutter, clear e vazio não geram oferta. Conferir o livro de escritas de 2 s, hash atual, TTL de 600.000 ms, cache de vistos 1.024 e caixa de no máximo 50 itens; nenhum hash ou conteúdo em log.
- B com módulo desligado produz Rejected(ModuleDisabled). Pausa impede envio/aplicação, resume não reenvia. Modo manual deixa o clipboard intacto e usa InInbox. Debug de ClipSnapshot e InboxItem redigido.
- Sem DISPLAY, write sai 1 com `a ponte de clipboard exige XWayland ativo (DISPLAY)`; diagnose registra Missing. Não inferir código de saída de diagnose, não especificado para esse cenário.

Itens **M**, executados um por vez pelo Maestro/usuário após sessão e ambiente disponíveis:

```sh
source ~/.cargo/env
cargo run -q -p clipsync-cli -- probe clipboard --watch 15s
cargo run -q -p clipsync-cli -- probe clipboard --write
```

Watch: três cópias sintéticas diferentes num app Wayland; `observação: eventos (XFixes via XWayland)`, três linhas com MIME, bytes e latência de leitura, sem conteúdo. Fechar o app de origem não produz nova cópia. Write: `escrita: ok · leitura idêntica: ok · evento da própria escrita: descartado (eco) · nova cópia disparada: não`. Identificar app, backend, sessão e limitações. Inbox falso não prova clipboard real. Falta de sessão deixa P1 pendente, sem transformar fake em prova gráfica.

### S3 · Vector · input/P4 e §9, itens 1–5

```sh
source ~/.cargo/env
cargo test -p clipsync-input -- --list
cargo test -p clipsync-input
cargo clippy -p clipsync-input --all-targets -- -D warnings
cargo fmt -p clipsync-input --check
```

Evidência de `FakeCapture` + `FakeEmulate` + `MemHub`: 200 eventos recebidos, p50 <= p95; destino sem input_in recusa Enter e nunca inicia emulador; Leave e PeerDown soltam tudo (pressed_now=0); excesso de 2.000 datagramas/s por par é descartado e contado; pausa descarta datagramas; Debug de Key/Button/CaptureEvent não mostra códigos; zona 1920×1080 em (0,0) gera barreira x=1920, y=0..539. Conferir também release_all em pausa/ReleaseAll e recusa de sessão desconhecida, conforme §8. Leave/ReleaseAll/StatsRequest só agem sobre sessão do mesmo remetente; Stats só vale para sessão pedida àquele par. Sessão alheia é ignorada e contada.

Itens **M**, um por vez, pelo Maestro/usuário. Primeiro diagnose; Input Leap/Barrier ativo deve ser informado. Não encerrar concorrentes nem iniciar captura concorrente; aguardar a pausa explícita pelo usuário.

```sh
source ~/.cargo/env
cargo run -q -p clipsync-cli -- diagnose
cargo run -q -p clipsync-cli -- probe input --capture
cargo run -q -p clipsync-cli -- probe input --emulate
clipsync probe input --loopback --profile /tmp/cs-a --to notebook-b --seconds 20
```

- **P4a:** InputCapture/consentimento, zona eDP-1, ativação só na metade superior direita, contagens por tipo/latência, Esc libera o cursor. Se o portal não fornecer nome e Vector precisar de `#0`, registrar a limitação permitida pela §10.3 e a evidência da geometria; não afirmar que eDP-1 foi observado.
- **P4b:** emulação só após confirmação explícita e janela de teste aberta pelo usuário. Sem `--yes`: confirmação s/N, espera de 5 s, quadrado e `ção`. Recusa não move/não digita. Janela nativa fora do alcance do portal exige observação assistida, identificada como tal.
- **P4c:** S1 integrado, perfil A com capture=portal e B com emulate=fake, B concedendo input_in a A; ambos iniciados pelo Maestro. Mesmo host para comparar CLOCK_MONOTONIC. Exigir número de amostras, perdas, p50/p95 em ms de captura até recepção. O comando usa o socket do perfil A; não é um loopback isolado que ignora a autorização. Receptor falso evita realimentar captura com emulação real.

No MVP o agente não captura continuamente: captura só via InputLoopback/probes (§8/§10.5). Testes falsos não autorizam declarar captura ou emulação real validada. Windows/macOS, secure desktop e teclados de outras máquinas ficam fora da aprovação local e mantêm classificação honesta na matriz. Argus entrega evidência ao Maestro; não edita matriz/ADRs de outro dono.

### I · Rivet · integração e §9, itens 1–4

```sh
source ~/.cargo/env
cargo test -p clipsync-cli --test roteiro clip_ -- --list
cargo test -p clipsync-cli --test roteiro clip_
cargo test --workspace pause_
cargo test --workspace
cargo run -q -p clipsync-cli -- diagnose
cargo build --release -p clipsync-cli
test -x target/release/clipsync
target/release/clipsync --help
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

A fixture `roteiro` cria perfis A/B novos com `--fake-adapters`, clipboard ligado, portas diferentes e permissões correspondentes (clipboard_out em A e clipboard_in em B). Os testes executam `send`/`inbox` nos filhos; o bloco abaixo documenta os comandos concretos para uma reprodução pelo Maestro com /tmp/cs-a e /tmp/cs-b livres e preparados por essa fixture:

```sh
printf 'olá ção' | clipsync send --stdin --profile /tmp/cs-a
clipsync inbox --profile /tmp/cs-b
clipsync status --profile /tmp/cs-a
clipsync status --profile /tmp/cs-b
clipsync pause --profile /tmp/cs-b
printf 'clipsync-argus-durante-pausa' | clipsync send --stdin --profile /tmp/cs-a
clipsync status --profile /tmp/cs-b
clipsync inbox --profile /tmp/cs-b
clipsync resume --profile /tmp/cs-b
clipsync inbox --profile /tmp/cs-b
printf 'clipsync-argus-depois-da-pausa' | clipsync send --stdin --profile /tmp/cs-a
clipsync inbox --profile /tmp/cs-b
```

Não acrescentar newline ao primeiro printf: `olá ção` tem 10 bytes UTF-8. Exigir send=0, `notebook-b: aplicado`, inbox com bytes=10 e preview sintético idêntico. Com modules.clipboard=false em B e reinício gerenciado pela fixture, repetir: send=3, `recusado: módulo clipboard desligado em notebook-b`, inbox sem item novo; confirmar que módulo desligado não é construído/não anuncia capacidade/não pede permissão. Durante pausa, envio recusado com `pausado`; resume não muda o inbox por replay, só novo envio posterior acrescenta item. Conferir também bloqueio de origem pausada nas regras unitárias.

Diagnose registra InputCapture v1, RemoteDesktop v2, ausência de data-control e Input Leap conforme o ambiente real; divergência do ambiente é descrita, não ocultada. Release gera `target/release/clipsync` no target padrão. Após integração, reportar P1/P2/P3/P4 separadamente, com as limitações reais e os itens de ADR/matriz da §10 ainda pendentes; compilação não comprova todo o MVP.

### Leitura de segurança e drift · todas as fatias

Localizar e ler fontes, sem git, respeitando a posse: S0 raiz/CI/esqueleto; depois S1 proto/core/cli, S2 clipboard, S3 input. A revisão não edita essas fontes.

```sh
rg --files crates .github -g '*.rs' -g 'Cargo.toml' -g '*.yml' -g '*.yaml'
rg -n -g '*.rs' '(trace!|debug!|info!|warn!|error!|println!|eprintln!|dbg!|instrument|impl.*Debug)' crates
rg -n -g '*.rs' '(PinVerifier|ClientCertVerifier|ServerCertVerifier|verify_tls13_signature|NoServerSessionStorage|send_tls13_tickets|Resumption|export_keying_material|CloseCode)' crates
rg -n -g '*.rs' '(authorize_|ModuleDisabled|NotAuthorized|Paused|input_in|clipboard_in|clipboard_out|ReloadPeers)' crates
rg -n -g '*.rs' '(MAX_|with_capacity|read_to_end|read_exact|deserialize|decode|datagram|is_finite|release_all)' crates
rg -n '(clipsync-core|workspace = true)' crates/*/Cargo.toml
```

Ausência de match (`rg`=1) não aprova requisito. Ler os arquivos e seus caminhos de chamada com `nl -ba` no caminho localizado; registrar hashes quando revisar. Conferir:

1. **Sigilo:** Debug manual dos sete tipos da §2, mais CaptureEvent; mensagens de erro, logs de tracing e argumentos instrumentados sem conteúdo/tecla/keycode. Saída permitida de inbox separada de stderr/log. Textos ao usuário em português no msg.rs de cada crate; identificadores/comentários em inglês.
2. **TLS:** verificador único dos dois sentidos; certificado autoassinado validado, prova de posse TLS 1.3, SPKI pinado em malha; bootstrap ALPN_PAIR permite pino novo, sem dispensar assinatura. Retomada desligada nos dois lados/modos; pareamento exige compromisso e confirmação dupla. CloseCode fixo, sem texto de rustls para o par.
3. **Autorização única:** core e MemHub chamam as mesmas funções do proto; nada de política duplicada em cada fatia. Regras por módulo/direção, pausa reavaliada antes de efeito, entrada concedida no destino, módulo desligado sem build de adaptador. Mensagens de encerramento/ack ignoram pausa/permissão, mas exigem módulo local ligado e session/id pertencente ao remetente; conferir precedência das falhas da §4.
4. **Limites antes de alocar/decodificar:** frame de módulo 1.114.112; controle QUIC 65.536; texto 1.048.576; nome 64; MIME 128; motivo 256; datagrama 1.200 bytes/32 eventos; caixa 50; vistos 1.024; TTL 600.000 ms; janela 120 s; taxa 2.000/s/par. Casos max, max+1, truncamento, overflow, UTF-8/MIME e floats finitos. Negociação não amplia limite local.
5. **Drift:** assinaturas/enums da §4, grafo sem ciclo, clipboard/input sem depender de core, deps via workspace. Erros thiserror nas libs, anyhow só cli/probes; sem unwrap/expect fora de teste nem cast que esconda falha. Adaptadores síncronos fora do runtime async. Sem duplicar autorização ou codec nas fatias.

Pendência identificada no próprio contrato para a revisão de limites: o comentário de `MAX_CONTROL_FRAME_BYTES` inclui o socket local (§4/limits.rs), mas §4/control.rs especifica IPC JSON com `MAX_FRAME_BYTES`. Registrar a divergência e pedir ao Maestro/Keel o limite vinculante do IPC antes de julgar esse ponto; os limites do controle QUIC e dos fluxos de módulo estão explícitos. Não alterar o contrato nem escolher silenciosamente um dos limites.

### Registro ao concluir cada revisão

Adicionar um bloco em `## Verdicts`, preservando os anteriores, com:

```text
### <S0/S1/S2/S3/I> · <data/hora> · <Passa / Não passa / Validação incompleta>
Base: versão/hash do contrato e hashes dos arquivos revisados.
Ambiente e diretório: ...
Comando: ...
Saída real relevante: ...
Exit code: ...
Testes: executados / aprovados / falhos / ignorados; casos da §9 cobertos.
Itens M: observados por Argus / evidência recebida de quem / pendentes.
Achados: arquivo:linha, defeito, impacto e reprodução, por severidade.
Limitações e requisitos ainda não comprovados: ...
```

Publicar achados detalhados em `## Findings` e comunicar o veredito ao Maestro por `maestri ask`. Repetir teste só por mudança, falha ou lacuna de evidência. Nenhuma fatia está aprovada por esta publicação do plano.

## Verdicts
### S0 · histórico 2026-10-27 12:04:33–12:04:36 -03:00 · Não passa no checkout atual (substituído pela reemissão abaixo)

Solicitação: revisar os sete critérios da §9 S0; commit informado pelo Maestro: `79be293`. **Não consultei git e não certifiquei o conteúdo desse commit.** A evidência abaixo é do checkout compartilhado, que já contém implementação de S1/S2. As falhas localizadas nesses crates não são atribuídas ao trabalho do Bolt sem comparação com uma cópia estável de S0.

Diretório: `~/clipsync`. Cada comando abaixo foi executado em zsh após `source ~/.cargo/env`. Hashes dos arquivos antes/depois dos 13 comandos foram idênticos; não houve mudança detectada durante essa janela. Os cinco crates têm `[lints] workspace = true`; as 25 dependências de workspace têm versões iniciadas por `=`. Manifestos conferidos por leitura e parse de TOML. Código-fonte não foi editado.

| §9 S0 | Resultado observado |
|---|---|
| 1 · toolchain | Passa: 1.99.1, rustfmt e clippy declarados. |
| 2 · build e crates | Passa: build=0, cinco crates e lints herdados; build emite três warnings no ledger de clipboard. |
| 3 · cinco smoke | Não passa: test=101 na compilação de proto; nenhum teste chegou a executar, cinco smoke aprovados não comprovados. |
| 4 · clippy e fmt | Não passa: clippy=101 pelo módulo testing ausente; fmt=1 pelo mesmo módulo e diferenças de formatação em clipboard. |
| 5 · lints | Passa: unsafe_code=forbid, unwrap_used=deny, demais lints da §2 declarados; três allow-*-in-tests=true. |
| 6 · CI | Passa na verificação local exigida: YAML parseia (0); jobs.check usa ubuntu-latest e declara fmt, clippy -D warnings e test --workspace. Não executei GitHub Actions. |
| 7 · dependências | Passa: 25 versões exatas; ring 0.17.14 presente; aws-lc-sys/aws-lc-rs ausentes com erro esperado 101; x11rb sem duplicação reportada. |

#### Saídas reais

Comando: `cat rust-toolchain.toml`. Início: `2026-10-27T12:04:33.593894-03:00`. Exit code: **0**.

stdout:

```text
[toolchain]
channel = "1.99.1"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Comando: `rustup show active-toolchain`. Início: `2026-10-27T12:04:33.598252-03:00`. Exit code: **0**.

stdout:

```text
1.99.1-x86_64-unknown-linux-gnu (overridden by '~/clipsync/rust-toolchain.toml')
```

Comando: `cargo build --workspace`. Início: `2026-10-27T12:04:33.621930-03:00`. Exit code: **0**.

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/clipsync/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/clipsync/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/clipsync/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/clipsync/crates/clipsync-clipboard)
warning: method `forget` is never used
  --> crates/clipsync-clipboard/src/ledger.rs:30:12
   |
20 | impl WriteLedger {
   | ---------------- method in this implementation
...
30 |     pub fn forget(&mut self, h: &Hash) {
   |            ^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: struct `SeenEvents` is never constructed
  --> crates/clipsync-clipboard/src/ledger.rs:42:12
   |
42 | pub struct SeenEvents {
   |            ^^^^^^^^^^

warning: associated items `new` and `insert_new` are never used
  --> crates/clipsync-clipboard/src/ledger.rs:49:12
   |
48 | impl SeenEvents {
   | --------------- associated items in this implementation
49 |     pub fn new(ttl: Duration) -> Self {
   |            ^^^
...
53 |     pub fn insert_new(&mut self, id: EventId, now: Instant) -> bool {
   |            ^^^^^^^^^^

warning: `clipsync-clipboard` (lib) generated 3 warnings
   Compiling clipsync-cli v0.1.0 (~/clipsync/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.21s
```

Comando: `cargo test --workspace`. Início: `2026-10-27T12:04:34.877864-03:00`. Exit code: **101**.

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/clipsync/crates/clipsync-proto)
error[E0583]: file not found for module `testing`
  --> crates/clipsync-proto/src/lib.rs:16:1
   |
16 | pub mod testing;
   | ^^^^^^^^^^^^^^^^
   |
   = help: to create the module `testing`, create file "crates/clipsync-proto/src/testing.rs" or "crates/clipsync-proto/src/testing/mod.rs"
   = note: if there is a `mod testing` elsewhere in the crate already, import it with `use crate::...` instead

warning: unused import: `testing::*`
  --> crates/clipsync-proto/src/lib.rs:28:9
   |
28 | pub use testing::*;
   |         ^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

For more information about this error, try `rustc --explain E0583`.
warning: `clipsync-proto` (lib) generated 1 warning
error: could not compile `clipsync-proto` (lib) due to 1 previous error; 1 warning emitted
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`. Início: `2026-10-27T12:04:35.547266-03:00`. Exit code: **101**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/clipsync/crates/clipsync-proto)
error[E0583]: file not found for module `testing`
  --> crates/clipsync-proto/src/lib.rs:16:1
   |
16 | pub mod testing;
   | ^^^^^^^^^^^^^^^^
   |
   = help: to create the module `testing`, create file "crates/clipsync-proto/src/testing.rs" or "crates/clipsync-proto/src/testing/mod.rs"
   = note: if there is a `mod testing` elsewhere in the crate already, import it with `use crate::...` instead

error: unused import: `testing::*`
  --> crates/clipsync-proto/src/lib.rs:28:9
   |
28 | pub use testing::*;
   |         ^^^^^^^^^^
   |
   = note: `-D unused-imports` implied by `-D warnings`
   = help: to override `-D warnings` add `#[allow(unused_imports)]`

For more information about this error, try `rustc --explain E0583`.
error: could not compile `clipsync-proto` (lib) due to 2 previous errors
```

Comando: `cargo fmt --all --check`. Início: `2026-10-27T12:04:36.249759-03:00`. Exit code: **1**.

Trecho literal inicial de stdout:

```diff
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:1:
 use super::{ChangeEvent, ClipError, ClipSnapshot, ClipboardAdapter, WatchMode};
-use clipsync_proto::{mono_us, MIME_TEXT_UTF8};
+use clipsync_proto::{MIME_TEXT_UTF8, mono_us};
 use std::collections::VecDeque;
```

Locais reportados por rustfmt (extraídos de stdout, sem inventar diferenças):

```text
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:1:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:25:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:43:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:51:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:95:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:102:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:106:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/mod.rs:30:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:1:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:8:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:11:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:42:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:49:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:61:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:69:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:74:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:133:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:140:
Diff in ~/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:151:
Diff in ~/clipsync/crates/clipsync-clipboard/src/config.rs:1:
Diff in ~/clipsync/crates/clipsync-clipboard/src/config.rs:33:
Diff in ~/clipsync/crates/clipsync-clipboard/src/config.rs:40:
Diff in ~/clipsync/crates/clipsync-clipboard/src/config.rs:48:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:9:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:16:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:22:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:30:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:43:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:56:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:63:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:67:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:79:
Diff in ~/clipsync/crates/clipsync-clipboard/src/diag.rs:98:
Diff in ~/clipsync/crates/clipsync-clipboard/src/ledger.rs:33:
Diff in ~/clipsync/crates/clipsync-clipboard/src/ledger.rs:47:
Diff in ~/clipsync/crates/clipsync-clipboard/src/ledger.rs:54:
Diff in ~/clipsync/crates/clipsync-clipboard/src/lib.rs:6:
Diff in ~/clipsync/crates/clipsync-clipboard/src/msg.rs:14:
Diff in ~/clipsync/crates/clipsync-clipboard/src/probe.rs:1:
Diff in ~/clipsync/crates/clipsync-clipboard/src/probe.rs:41:
Diff in ~/clipsync/crates/clipsync-clipboard/src/probe.rs:54:
Diff in ~/clipsync/crates/clipsync-clipboard/src/probe.rs:78:
```

stderr integral:

```text
Error writing files: failed to resolve mod `testing`: ~/clipsync/crates/clipsync-proto/src/testing.rs does not exist
```

Stdout completo preservado em `/tmp/clipsync-argus-s0-miuoxq64/results.json`; o trecho acima não é a saída completa de fmt.

Comando: `grep -n 'unwrap_used\|unsafe_code' Cargo.toml`. Início: `2026-10-27T12:04:36.314486-03:00`. Exit code: **0**.

stdout:

```text
13:unsafe_code = "forbid"
16:unwrap_used = "deny"
```

Comando: `cat clippy.toml`. Início: `2026-10-27T12:04:36.317643-03:00`. Exit code: **0**.

stdout:

```text
# Testes podem usar unwrap/expect/panic; produção, não (§2.4 do contrato).
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
```

Comando: `python3 -c "import yaml,sys;yaml.safe_load(open('.github/workflows/ci.yml'))"`. Início: `2026-10-27T12:04:36.322298-03:00`. Exit code: **0**.

stdout/stderr vazios.

Comando: `cargo tree -i ring`. Início: `2026-10-27T12:04:36.354791-03:00`. Exit code: **0**.

stdout:

```text
ring v0.17.14
├── quinn-proto v0.11.19
│   └── quinn v0.11.12
│       └── clipsync-core v0.1.0 (~/clipsync/crates/clipsync-core)
│           └── clipsync-cli v0.1.0 (~/clipsync/crates/clipsync-cli)
├── rcgen v0.14.10
│   └── clipsync-core v0.1.0 (~/clipsync/crates/clipsync-core) (*)
├── rustls v0.23.46
│   ├── clipsync-core v0.1.0 (~/clipsync/crates/clipsync-core) (*)
│   ├── quinn v0.11.12 (*)
│   └── quinn-proto v0.11.19 (*)
└── rustls-webpki v0.103.16
    └── rustls v0.23.46 (*)
```

Comando: `cargo tree -i aws-lc-sys`. Início: `2026-10-27T12:04:36.455366-03:00`. Exit code: **101**.

stderr:

```text
error: package ID specification `aws-lc-sys` did not match any packages
```

Comando: `cargo tree -i aws-lc-rs`. Início: `2026-10-27T12:04:36.536871-03:00`. Exit code: **101**.

stderr:

```text
error: package ID specification `aws-lc-rs` did not match any packages
```

Comando: `cargo tree -d -i x11rb`. Início: `2026-10-27T12:04:36.618969-03:00`. Exit code: **0**.

stderr:

```text
warning: nothing to print.

To find dependencies that require specific target platforms, try to use option `--target all` first, and then narrow your search scope accordingly.
```

#### Base da revisão e limites

Hashes SHA-256 registrados antes/depois:

```text
749632b923347fd19aa3ccde1a4315adf68299679cf91491655a9761d79c20fb  Cargo.toml
18fb92a17a34a262de222fedc3343dbe81be50f61b51a8fda1f22b139a3d2fdf  Cargo.lock
b9c8d2d59f6f46474c0cfa0b8b77bea21d94fc73a30ad93d5bf6385c422197d2  rust-toolchain.toml
7e1ef5bd22265ce84691bfc66ba1e4bdf1f077ca570be5d130c1ee41eb69f2ab  clippy.toml
a17fc31e789ac923ae55a5b2e620b7f561bb62d53fa6bebc968e07353d68e69c  rustfmt.toml
23e47e9f8427f5dba0a7989c5eb40b6d9df69c8ad918cf69dffebefc1236db66  .github/workflows/ci.yml
a9ecc94912ead31e1ce3301812c47e3b72f9c0b98df15faa127681c4a42fef73  docs/contrato.md
4b0f4ef8bb42bd81ad2e4a7d15ee5257b969a98b8336a1af5e05f587135d6770  crates/clipsync-proto/Cargo.toml
fe701bf856be6c9f91bc7283d3f372bb810b8b61c3f21e384c03e15e49e2c70b  crates/clipsync-proto/src/lib.rs
192d459fbd5a3179644574fef00565aef0e8b3dd21d159002c6a08ac4e889c7c  crates/clipsync-clipboard/src/adapter/fake.rs
```

Logs integrais e inventários: `/tmp/clipsync-argus-s0-miuoxq64/{results,before,after}.json`. Resultados pertinentes estão transcritos acima para não depender só de arquivos temporários. Não houve instalação fria adicional, início de servidor, validação gráfica ou execução do vertical slice; o pedido desta revisão foi os sete gates S0, e test/clippy/fmt já impedem aprovação. Reconferir esses gates numa base S0 estável ou após os donos concluírem as alterações que bloqueiam o workspace.


### S0 · reemissão 2026-10-27T12:20:54-03:00 · Passa

Base local: export limpo **e611a88**, informado pelo Maestro, em `~/.cache/clipsync-verdict/src`; `CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target`, `source ~/.cargo/env` antes de cada comando. Execução 12:15:05–12:16:07 -03:00. Todos os arquivos inventariados permaneceram idênticos antes/depois e na conferência posterior. Nenhum git foi executado; o hash de commit é a identificação fornecida pelo Maestro, os SHA-256 abaixo são a evidência local.

**Este veredito substitui o “não passa no checkout atual” das 12:04 para a fatia S0.** Aquela observação continua no histórico, mas era da árvore compartilhada em edição. A rodada intermediária em /tmp foi descartada como base de aprovação: houve troca de export e cota de disco; os comandos foram repetidos no novo caminho com sucesso.

| Critério S0 | Evidência e resultado |
|---|---|
| 1 · toolchain | Passa: 1.99.1, rustfmt/clippy. |
| 2 · build/crates | Passa: build=0; cinco crates, todos herdando lints. |
| 3 · testes | Passa: no export atual, 55 aprovados, zero falhos/ignorados; quatro smoke e testes funcionais de proto. Os cinco smoke da entrega S0 foram conferidos no log do CI de 79be293. |
| 4 · clippy/fmt | Passa: ambos 0 no export e no CI de S0. |
| 5 · lints | Passa: unsafe_code forbid, seis lints Clippy deny e três exceções para testes; manifestos lidos/parseados. |
| 6 · CI | Passa: YAML válido localmente; ubuntu-latest e três gates declarados; execução GitHub success no commit S0 exato. |
| 7 · dependências | Passa: 25 versões exatas com =; ring presente; aws-lc-sys/aws-lc-rs ausentes pelo erro esperado; nenhuma duplicação de x11rb reportada. |

A aprovação cobre o scaffold S0; não aprova o comportamento das outras fatias nem os probes gráficos.

#### Evidência externa do commit S0

Consultas somente de leitura, todas exit 0:

```sh
gh run view 40526929698
gh run view 40526929698 --repo devkappiushon/clipsync --json headSha,jobs,conclusion,url,createdAt,updatedAt
gh run view 40526929698 --repo devkappiushon/clipsync --log
```

[CI 40526929698](https://github.com/devkappiushon/clipsync/actions/runs/40526929698): `headSha=79be29397b926a837857eeb852e247f7b6235882`, `conclusion=success`. Job `122268918983`, `fmt + clippy + test`, todos os steps `cargo fmt`, `cargo clippy` e `cargo test` com `conclusion=success`; completou `2026-10-27T15:04:46Z`. O log informa `Image: ubuntu-26.04` e cinco ocorrências de `test smoke ... ok`, cada uma seguida de `1 passed; 0 failed; 0 ignored`. Saída resumida literal de `gh run view`:

```text
✓ main ci · 40526929698
✓ fmt + clippy + test in 2m45s (ID 122268918983)
```

#### Sete critérios, comandos e saídas locais reais

Comando: `cat rust-toolchain.toml`. Início: `2026-10-27T12:15:05.287951-03:00`. Exit code: **0**.

stdout:

```text
[toolchain]
channel = "1.99.1"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Comando: `rustup show active-toolchain`. Início: `2026-10-27T12:15:05.292622-03:00`. Exit code: **0**.

stdout:

```text
1.99.1-x86_64-unknown-linux-gnu (overridden by '~/.cache/clipsync-verdict/src/rust-toolchain.toml')
```

Comando: `cargo build --workspace`. Início: `2026-10-27T12:15:05.313777-03:00`. Exit code: **0**.

Trecho final literal de stderr (logs de compilação completos no results.json):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.38s
```

Comando: `cargo test --workspace`. Início: `2026-10-27T12:15:25.731855-03:00`. Exit code: **0**.

stdout integral:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 11 tests
test i_debug_never_shows_content ... ok
test h_manual_mode_only_fills_the_inbox ... ok
test c_old_lamport_after_new_is_stale_and_goes_to_inbox ... ok
test f_remote_module_disabled_is_reported ... ok
test b_duplicate_event_id_applies_once_and_acks_duplicate ... ok
test d_x_y_x_makes_three_offers_x_x_makes_one ... ok
test k_ttl_counts_from_local_receipt_and_drops_the_text ... ok
test a_mesh_one_copy_two_offers_no_reoffer ... ok
test e_own_write_echo_takeover_and_empty_do_not_offer ... ok
test g_paused_neither_sends_nor_applies_and_resume_does_not_resend ... ok
test j_clipsend_lists_every_target_and_no_ack_is_unreachable ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test inbound_datagram_table ... ok
test inbound_table ... ok
test outbound_datagram_table ... ok
test outbound_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test event_ids_are_random ... ok
test module_set ... ok
test mono_us_is_monotonic ... ok
test peer_id_from_hex_errors ... ok
test peer_id_hex_roundtrip_and_short_form ... ok
test reject_reasons_have_portuguese_text ... ok
test peer_id_is_sha256_of_spki ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test delivers_authorized_offer ... ok
test datagrams_follow_the_same_rules ... ok
test inbound_refusal_becomes_rejected_at_sender ... ok
test link_delivers_peer_up_with_grants_and_remote_modules ... ok
test outbound_rules_return_errors ... ok
test paused_destination_rejects_offer_but_ack_passes ... ok
test invalid_or_oversized_message_is_not_sent ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 5 tests
test control_request_debug_hides_clip_send_text ... ok
test clip_offer_debug_hides_text ... ok
test key_and_button_debug_hide_code ... ok
test inbox_item_debug_hides_preview ... ok
test control_messages_roundtrip_as_json ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 17 tests
test close_codes_are_stable_short_tokens ... ok
test datagram_over_1200_bytes_is_refused ... ok
test encode_refuses_body_over_max ... ok
test datagram_with_32_events_fits_and_roundtrips ... ok
test control_frame_of_64k_plus_one_is_refused ... ok
test garbage_and_trailing_bytes_are_decode_errors ... ok
test every_frame_variant_roundtrips ... ok
test module_msg_kind ... ok
test unreachable_never_goes_on_the_wire ... ok
test validate_device_name_bounds ... ok
test validate_hello_min_version ... ok
test validate_refuses_33_events ... ok
test oversized_frame_is_refused_before_allocating ... ok
test validate_refuses_nan_and_infinite_floats ... ok
test validate_refuses_pos_outside_unit_range ... ok
test write_then_read_over_a_duplex ... ok
test validate_clip_offer_mime_and_size ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Trechos literais de stderr que identificam executáveis e conclusão:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.98s
     Running unittests src/main.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync-be9f8c6ccda39f75)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-ed14c5d2789ba84f)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-92d4afbd93db2a46)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-6d4b6b899601d3c6)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_input-e5c8f10e63c27e5a)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-6f0702f4c653dfde)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (~/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (~/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (~/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (~/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (~/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
   Doc-tests clipsync_clipboard
   Doc-tests clipsync_core
   Doc-tests clipsync_input
   Doc-tests clipsync_proto
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`. Início: `2026-10-27T12:15:36.966694-03:00`. Exit code: **0**.

Trecho final literal de stderr (logs de compilação completos no results.json):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.36s
```

Comando: `cargo fmt --all --check`. Início: `2026-10-27T12:15:50.385755-03:00`. Exit code: **0**.

stdout/stderr vazios.

Comando: `grep -n 'unwrap_used\|unsafe_code' Cargo.toml`. Início: `2026-10-27T12:15:50.464478-03:00`. Exit code: **0**.

stdout:

```text
13:unsafe_code = "forbid"
16:unwrap_used = "deny"
```

Comando: `cat clippy.toml`. Início: `2026-10-27T12:15:50.467965-03:00`. Exit code: **0**.

stdout:

```text
# Testes podem usar unwrap/expect/panic; produção, não (§2.4 do contrato).
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
```

Comando: `python3 -c "import yaml,sys;yaml.safe_load(open('.github/workflows/ci.yml'))"`. Início: `2026-10-27T12:15:50.472712-03:00`. Exit code: **0**.

stdout/stderr vazios.

Comando: `cargo tree -i ring`. Início: `2026-10-27T12:15:50.503830-03:00`. Exit code: **0**.

stdout:

```text
ring v0.17.14
├── quinn-proto v0.11.19
│   └── quinn v0.11.12
│       └── clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
│           └── clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
├── rcgen v0.14.10
│   └── clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core) (*)
├── rustls v0.23.46
│   ├── clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core) (*)
│   ├── quinn v0.11.12 (*)
│   └── quinn-proto v0.11.19 (*)
└── rustls-webpki v0.103.16
    └── rustls v0.23.46 (*)
```

Comando: `cargo tree -i aws-lc-sys`. Início: `2026-10-27T12:15:50.605001-03:00`. Exit code: **101**.

stderr:

```text
error: package ID specification `aws-lc-sys` did not match any packages
```

Comando: `cargo tree -i aws-lc-rs`. Início: `2026-10-27T12:15:50.691132-03:00`. Exit code: **101**.

stderr:

```text
error: package ID specification `aws-lc-rs` did not match any packages
```

Comando: `cargo tree -d -i x11rb`. Início: `2026-10-27T12:15:50.774377-03:00`. Exit code: **0**.

stderr:

```text
warning: nothing to print.

To find dependencies that require specific target platforms, try to use option `--target all` first, and then narrow your search scope accordingly.
```

#### Hashes da base local

```text
5db5a0b6e04351b370ccbde3f9dc9b5fd48bc24eb337bfbda7c94ced319a9720  Cargo.toml
18fb92a17a34a262de222fedc3343dbe81be50f61b51a8fda1f22b139a3d2fdf  Cargo.lock
b9c8d2d59f6f46474c0cfa0b8b77bea21d94fc73a30ad93d5bf6385c422197d2  rust-toolchain.toml
7e1ef5bd22265ce84691bfc66ba1e4bdf1f077ca570be5d130c1ee41eb69f2ab  clippy.toml
a17fc31e789ac923ae55a5b2e620b7f561bb62d53fa6bebc968e07353d68e69c  rustfmt.toml
23e47e9f8427f5dba0a7989c5eb40b6d9df69c8ad918cf69dffebefc1236db66  .github/workflows/ci.yml
512b073c27ebfb1579bb83f4a2226bf24f78cce86e422b3f4915fb40255ae009  docs/contrato.md
```

Logs integrais e inventários: `~/.cache/clipsync-verdict/argus-e611a88-3609dhci/{meta,results,before,after}.json`.

### S2 · v1.3 · 2026-10-27T12:20:54-03:00 · Não passa

Base: export **e611a88** em `~/.cache/clipsync-verdict/src`, cache em `~/.cache/clipsync-verdict/target`. O módulo e os testes de clipboard têm os mesmos hashes da entrega final 18a461c; contrato v1.3 desta base, incluindo regra 6 de TTL e regra 9 de Unreachable. A leitura parcial anterior foi substituída por esta revisão da versão final. Nenhuma fonte do projeto/export foi editada.

**Itens automáticos:** `cargo test -p clipsync-clipboard` = 12 aprovados (11 funcionais + 1 smoke), 0 falhos, 0 ignorados; Clippy com `-D warnings` = 0; fmt do crate = 0. **Veredito de comportamento: não passa**, pelos dois casos adicionais reproduzidos abaixo. **Itens 2–4: pendentes**, conforme ordem do Maestro, até o CLI/manual; não executei probes nem declarei clipboard gráfico validado.

#### Comandos pedidos e saídas reais

Comando: `cargo test -p clipsync-clipboard`. Início: `2026-10-27T12:15:50.866368-03:00`. Exit code: **0**.

stdout integral:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 11 tests
test i_debug_never_shows_content ... ok
test h_manual_mode_only_fills_the_inbox ... ok
test c_old_lamport_after_new_is_stale_and_goes_to_inbox ... ok
test f_remote_module_disabled_is_reported ... ok
test b_duplicate_event_id_applies_once_and_acks_duplicate ... ok
test d_x_y_x_makes_three_offers_x_x_makes_one ... ok
test k_ttl_counts_from_local_receipt_and_drops_the_text ... ok
test a_mesh_one_copy_two_offers_no_reoffer ... ok
test e_own_write_echo_takeover_and_empty_do_not_offer ... ok
test g_paused_neither_sends_nor_applies_and_resume_does_not_resend ... ok
test j_clipsend_lists_every_target_and_no_ack_is_unreachable ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

Trechos literais de stderr que identificam executáveis e conclusão:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.75s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-d5b415483d8ba192)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-a2a6cec29a2a4f52)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-034f1adebe3f47bd)
   Doc-tests clipsync_clipboard
```

Comando: `cargo clippy -p clipsync-clipboard --all-targets -- -D warnings`. Início: `2026-10-27T12:16:03.712556-03:00`. Exit code: **0**.

Trecho final literal de stderr (logs de compilação completos no results.json):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.15s
```

Comando: `cargo fmt -p clipsync-clipboard --check`. Início: `2026-10-27T12:16:06.922770-03:00`. Exit code: **0**.

stdout/stderr vazios.

#### Leitura de contrato e caminhos efetivos

| Checagem | Evidência observada |
|---|---|
| Debug redigido | `clipsync-proto/src/clip.rs:30` usa ByteLen para text; `clipsync-clipboard/src/adapter/mod.rs:31` imprime só tamanho; `clipsync-proto/src/control.rs:147` usa Redacted para preview. Testes `i_debug_never_shows_content`, `clip_offer_debug_hides_text` e `inbox_item_debug_hides_preview` passaram na execução desta base. |
| Texto/hash em log | Busca e leitura de adapter, módulo, probe, diag e msg: hashes só em memória; único tracing do módulo em `module.rs:553` tem peer/error e mensagem fixa. Probes imprimem MIME/tamanho/latência/estados, sem texto ou hash. Nenhum vazamento direto encontrado nas fontes revisadas; prova manual de saída real permanece pendente. |
| Livro de escritas | `ledger.rs:9` define 2 s; register antes de write em `module.rs:480`; poda por Instant em `ledger.rs:35`; eco consultado em `module.rs:379`. Caso oficial de eco/re-escrita/tomada de posse passou. |
| Dedupe por EventId | Caso simples passou, mas o limite de 1.024 entradas falha: **S2-2** abaixo. |
| Vazio | `module.rs:376` retorna sem oferta e sem alterar hash quando snap=None; adaptadores fake/X11 convertem vazio em None. Caso oficial de clear/texto vazio passou. |
| Pausa | Caminhos normais checam estado em `module.rs:285`, `module.rs:387` e `module.rs:453`; teste oficial passou. Uma pausa durante a leitura anterior ao write ainda permite aplicar: **S2-1** abaixo. |
| Unreachable só local | `proto/clip.rs:64` recusa Ack(Unreachable); encode/decode passam por validate em `proto/wire.rs:210` e `:227`. `unreachable_never_goes_on_the_wire` passou no workspace, incluindo encode recusado, decode de postcard forjado recusado e JSON local preservado. `module.rs:156`, `:179` e `:628` usam Unreachable como resultado local; `j_clipsend_lists_every_target_and_no_ack_is_unreachable` passou. |
| Expiração da caixa | `module.rs:194` guarda Instant e Duration por item; `:530` marca instante local, `:531` limita TTL a CLIP_TTL_MS; `:537` compara idade monotônica e apaga preview. A varredura é a cada 5 s (`:20`, `:307`, `:311`), e ClipInbox chama expire_inbox antes de devolver (`:578`). created_unix_ms/received_unix_ms ficam para exibição. `k_ttl_counts_from_local_receipt_and_drops_the_text` passou: TTL 0 expira ao chegar; TTL 800 ms chega InInbox e após 1 s fica Expired com preview vazio. |

A verificação de expiração acima comprova o cenário do teste e o mecanismo lido; não mede uma garantia de remoção instantânea entre ticks de varredura. A leitura e os testes não abrangem operação real de XWayland nesta revisão.

#### Duas reproduções adicionais, sem editar fontes

O diagnóstico usa as bibliotecas já compiladas do export. O teste de capacidade inclui o próprio `ledger.rs` do export por `#[path]`, sem copiar/reimplementar seu algoritmo. O teste de pausa usa `build_with_adapter`, `MemHub` e um wrapper de FakeClipboard que bloqueia deterministicamente `read_text` até o host entrar em pausa; ele então libera a leitura, observa Ack/write e encerra o módulo. Nada toca clipboard real ou inicia servidor.

O harness foi fornecido ao rustc por stdin (nenhum .rs do projeto/export criado ou alterado). Texto exato de reprodução e argv preservados em `probe-stdin.txt` e `boundary-results.json` no diretório de evidência abaixo. Compile exit 0; avisos de dead_code pertencem ao ledger incluído parcialmente pelo harness, não aos gates Clippy do projeto.

Comando de compilação reexecutável, com o mesmo stdin:

```sh
rustc --edition=2024 --test --crate-name argus_s2_boundary - -o ~/.cache/clipsync-verdict/argus-e611a88-3609dhci/boundary-tests -L dependency=~/.cache/clipsync-verdict/target/debug/deps --extern clipsync_clipboard=~/.cache/clipsync-verdict/target/debug/deps/libclipsync_clipboard-ad5d540442636c69.rlib --extern clipsync_proto=~/.cache/clipsync-verdict/target/debug/deps/libclipsync_proto-200667442ae0d934.rlib --extern tokio=~/.cache/clipsync-verdict/target/debug/deps/libtokio-f1ce68d902523d30.rlib --extern sha2=~/.cache/clipsync-verdict/target/debug/deps/libsha2-dd923891c50e1b65.rlib < ~/.cache/clipsync-verdict/argus-e611a88-3609dhci/probe-stdin.txt
```

Comando executado:

```sh
~/.cache/clipsync-verdict/argus-e611a88-3609dhci/boundary-tests --nocapture --test-threads=1
```

Exit code: **101**. Saída real:

```text

running 2 tests
test duplicate_at_capacity_must_still_be_duplicate ... capacity=1024 duplicate_accepted_as_new=true
FAILED
test pause_during_prewrite_read_must_prevent_write ... paused=true writes=1 ack=Applied
FAILED

failures:

failures:
    duplicate_at_capacity_must_still_be_duplicate
    pause_during_prewrite_read_must_prevent_write

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s


thread 'duplicate_at_capacity_must_still_be_duplicate' (383554) panicked at <anon>:22:5:
duplicado ainda dentro do TTL não pode se tornar novo com a tabela cheia
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'pause_during_prewrite_read_must_prevent_write' (383557) panicked at <anon>:72:9:
assertion `left == right` failed: pausa aconteceu antes de write_text; não pode aplicar
  left: 1
 right: 0
```

Os dois testes verificam requisitos de comportamento; falharam com `duplicate_accepted_as_new=true` e `paused=true writes=1 ack=Applied`. Isso não altera a contagem dos 12 testes oficiais aprovados.

#### Hashes e evidência

```text
512b073c27ebfb1579bb83f4a2226bf24f78cce86e422b3f4915fb40255ae009  docs/contrato.md
20abc06d6d42393bef8ba8e0f6ab06dce8667cf7f61bd7a6dd2712c2a80fd401  crates/clipsync-clipboard/src/module.rs
81fdc2f827f98a9716f600d413064f7d2e352ddcb537394796e7d54881e1df53  crates/clipsync-clipboard/src/ledger.rs
52c031aad980322750af3a5c62aa8fd7e70325ceff5cf459f7e179a6d020b839  crates/clipsync-clipboard/tests/module_mem.rs
3d6eaa08715e9c267f8690d8f47768ea0ed3db7f37c5606bd145c4084d8995e1  crates/clipsync-proto/src/clip.rs
14aa4f0445d1dbf0fb4b082d9d1bb5bb8f2d065f88ecbebf097a4f9d7d180e68  crates/clipsync-proto/src/control.rs
```

Inventários antes/depois iguais; conferência posterior também sem alterações. Evidência persistente: `~/.cache/clipsync-verdict/argus-e611a88-3609dhci` (results.json, before.json, after.json, boundary-results.json, probe-stdin.txt e boundary-tests).

### S2 · v1.4 · 2026-10-27T12:28:01-03:00 · Passa no escopo automático

Base informada pelo Maestro: export **7923e20**, em `~/.cache/clipsync-verdict/src`; cache `~/.cache/clipsync-verdict/target`. Gates executados de 12:25:23 a 12:25:30 -03:00, após `source ~/.cargo/env`. **Substitui a reprovação de S2 v1.3 no escopo automático e encerra S2-1/S2-2.** Itens manuais S2.2–S2.4 permanecem pendentes; não há aprovação de clipboard gráfico. S4 parcial (profile/control/diag/init) não foi revisada nem aprovada neste bloco.

Resultado: **15 testes oficiais aprovados** (2 unitários de ledger, 12 de módulo, 1 smoke), zero falhos/ignorados; Clippy e fmt saem 0. As duas reproduções independentes foram recompiladas com as bibliotecas novas, usando o stdin **idêntico byte a byte** ao da reprovação: SHA-256 `ecb744ef315336cdfa81b59db54d78a79651d01b2aee29eddaf688918d224ad7`. Ambas agora passam, sem enfraquecer as asserções.

| Achado | Correção lida e resultado observado |
|---|---|
| S2-1 · encerrado | Worker recebe ModuleHost; `module.rs:84` confere paused() depois da leitura e antes de write_text. WriteResult::Paused vira Rejected(Paused) em `:512`, remove a entrada do ledger e segue para inbox/Ack. Reprodução original: **paused=true writes=0 ack=Rejected(Paused)**. O novo teste oficial `l_pause_between_authorization_and_write_blocks_the_write` também confere caixa com Rejected(Paused) e nenhum write. |
| S2-2 · encerrado | `ledger.rs:61` primeiro remove expirados, depois recusa duplicata, só então libera capacidade para um ID novo. Reprodução original: **capacity=1024 duplicate_accepted_as_new=false**. Os dois unitários novos cobrem duplicata na capacidade e esquecimento após TTL. |

Leitura focada: alterações S2 limitadas a module.rs, ledger.rs, adapter/fake.rs (bloqueio de leitura para teste) e tests/module_mem.rs. Os arquivos de Debug, ClipOffer/InboxItem e o codec de Unreachable mantêm os hashes da revisão anterior. Os testes de Debug, vazio, eco, pausa, Unreachable local e expiração monotônica da caixa foram executados novamente pelo crate. O teste específico de recusa de Unreachable no wire foi executado na revisão anterior; seu código compartilhado relevante não mudou. Nenhum fonte do projeto/export foi editado pelo Argus.

**Ressalva de reprodutibilidade:** durante os gates, somente `Cargo.lock` mudou de hash; os demais arquivos inventariados ficaram idênticos. Os comandos pedidos não usam --locked. Registro sem atribuir defeito a S2 ou afirmar que o lock original foi validado: antes `907ad512ba7c0a719d52064ddddfedcce2dbe6d11f5cdd824a37879dc778afd9`, depois `1402a6f6986f49c16748e0cacc42d6183d0d8ef525e89a6deb063aa3f88f2005`. O Maestro deve reconciliar o lock do export/commit; não alterei o lock manualmente.

#### Saídas reais dos gates

Comando: `cargo test -p clipsync-clipboard`; início `2026-10-27T12:25:23.064064-03:00`; exit **0**.

stdout:

```text

running 2 tests
test ledger::tests::ids_past_the_ttl_are_forgotten ... ok
test ledger::tests::full_cap_repeating_the_oldest_id_is_a_duplicate_not_a_new_event ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 12 tests
test i_debug_never_shows_content ... ok
test h_manual_mode_only_fills_the_inbox ... ok
test c_old_lamport_after_new_is_stale_and_goes_to_inbox ... ok
test f_remote_module_disabled_is_reported ... ok
test b_duplicate_event_id_applies_once_and_acks_duplicate ... ok
test d_x_y_x_makes_three_offers_x_x_makes_one ... ok
test l_pause_between_authorization_and_write_blocks_the_write ... ok
test k_ttl_counts_from_local_receipt_and_drops_the_text ... ok
test a_mesh_one_copy_two_offers_no_reoffer ... ok
test e_own_write_echo_takeover_and_empty_do_not_offer ... ok
test g_paused_neither_sends_nor_applies_and_resume_does_not_resend ... ok
test j_clipsend_lists_every_target_and_no_ack_is_unreachable ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.40s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-d5b415483d8ba192)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-a2a6cec29a2a4f52)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-034f1adebe3f47bd)
   Doc-tests clipsync_clipboard
```

Comando: `cargo clippy -p clipsync-clipboard --all-targets -- -D warnings`; início `2026-10-27T12:25:29.582489-03:00`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.71s
```

Comando: `cargo fmt -p clipsync-clipboard --check`; início `2026-10-27T12:25:30.342140-03:00`; exit **0**.

stdout/stderr vazios.

#### Reexecução dos casos originais

Compilação por stdin (exit 0), texto original preservado em `~/.cache/clipsync-verdict/argus-7923e20-3po5__ky/probe-stdin.txt`; comando completo e bibliotecas em `boundary-results.json`. O harness inclui também dois testes unitários do novo ledger; cada execução abaixo seleciona exatamente o caso original, por isso três testes ficam filtrados.

```sh
~/.cache/clipsync-verdict/argus-7923e20-3po5__ky/boundary-tests --exact duplicate_at_capacity_must_still_be_duplicate --nocapture
```

Exit **0**.

```text

running 1 test
capacity=1024 duplicate_accepted_as_new=false
test duplicate_at_capacity_must_still_be_duplicate ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

```sh
~/.cache/clipsync-verdict/argus-7923e20-3po5__ky/boundary-tests --exact pause_during_prewrite_read_must_prevent_write --nocapture
```

Exit **0**.

```text

running 1 test
paused=true writes=0 ack=Rejected(Paused)
test pause_during_prewrite_read_must_prevent_write ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.10s

```

#### Hashes da base revisada

```text
87221da170a1501c83d6aa31be70b8873ced7432606c687ef6b59c07db692d9a  docs/contrato.md
0f9feec98d357473ebd2cb596ac0463a07733484e4963e496c07bbf0d3378a63  crates/clipsync-clipboard/src/module.rs
95a006d62fac407b50b63b5a5083bd5707cb1616607254e939fb3e6acd240538  crates/clipsync-clipboard/src/ledger.rs
32686501f8185d74c09e33d3dc89fb6febea48ee11b4f86b3d2bb2b0ff87a49b  crates/clipsync-clipboard/src/adapter/fake.rs
1632ebfd7e0bfdc5c2b7aa073ca3471cab5bbadd7ce444fe6e94c8c01903932c  crates/clipsync-clipboard/tests/module_mem.rs
3d6eaa08715e9c267f8690d8f47768ea0ed3db7f37c5606bd145c4084d8995e1  crates/clipsync-proto/src/clip.rs
14aa4f0445d1dbf0fb4b082d9d1bb5bb8f2d065f88ecbebf097a4f9d7d180e68  crates/clipsync-proto/src/control.rs
```

Evidência persistente: `~/.cache/clipsync-verdict/argus-7923e20-3po5__ky` (meta/results/before/after/boundary-results.json, probe-stdin.txt e boundary-tests).

### S3 · Vector · 2026-10-27T12:35:27-03:00 · Não passa

Base informada pelo Maestro: export **3eaa730**, em `~/.cache/clipsync-verdict/src`; `CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target`. §9 S3 itens 1 e 5 executados de **12:28:30 a 12:28:49 -03:00**, após `source ~/.cargo/env`. Os critérios S3 do contrato no export são idênticos aos do contrato atual no repositório (workboard v1.8). Nenhum arquivo do export mudou entre o inventário inicial, os gates e as reproduções, inclusive Cargo.lock.

**Veredito: Não passa**, por S3-1/S3-2 reproduzidos abaixo. Os gates oficiais passam: **16 testes aprovados** (8 de módulo, 7 unitários de integração, 1 smoke), zero falhos, **1 ignorado** (`portal_loopback`, requer sessão gráfica); Clippy e fmt saem 0. A aprovação automática de S2 v1.4 permanece no bloco anterior; este bloco não aprova S1/S4/integração.

#### Achados reproduzidos

1. **S3-1 · P2 · `crates/clipsync-input/src/module.rs:441`** — todo Enter substitui o registro por `Incoming::new(session)`, zerando `window_count` e reiniciando `window_start` (`:72`). O limite de `:460` fica por sessão, embora §8 regra 3 exija limite por par. Reprodução usando módulo real + MemHub/FakeEmulate: mesmo peer envia 1.500 datagramas na sessão 7 e 1.500 na sessão 8; **3.000 datagramas emulados em 9.348 µs**, limite 2.000/s. As seis movimentações de posicionamento dos dois Enter foram descontadas. Impacto: um peer autorizado contorna continuamente o limite abrindo sessões; o teste oficial só cobre uma sessão.
2. **S3-2 · P2 · `crates/clipsync-input/src/module.rs:550`** — `pick_barriers` faz parse direto de u32; o seletor contratual `monitor = "#0"` (§3 e §10 item 3) cai em `ZoneRef::Name("#0")`. Reprodução pelo módulo real com adaptador de inspeção confirma **configured_monitor=#0 adapter_zone=Name("#0")**. `PortalCapture::resolve` (`adapter/portal.rs:138`) busca esse nome literalmente, logo a seleção por índice falha quando a zona tem nome de conector ou nenhum nome. Impacto: fica quebrado o fallback por índice previsto quando o portal não fornece o nome. Nenhum portal foi acionado por esta reprodução.

#### Checagens por leitura e cobertura observada

| Requisito | Evidência e resultado |
|---|---|
| Debug de Key/Button redigido | `clipsync-proto/src/input.rs:32`: os dois braços usam Redacted; InputDatagram imprime só session/seq/contagem. CaptureEvent delega ao Debug de InputEvent. Teste `debug_hides_key_and_button_codes` passa com sentinelas 4660/4661. |
| Nenhum keycode em log | Revisados logs/prints em module.rs, adapter/portal.rs e probe.rs. Não há impressão explícita de code/ev/eventos; o probe conta tipos e usa KEY_ESC só em comparação (`probe.rs:133`). Erros de emulação imprimem capacidade ou erro de I/O, sem interpolar código de tecla. Verificação por leitura, sem afirmar validação gráfica em execução. |
| Dono de Leave/ReleaseAll/StatsRequest/Stats | `module.rs:361`, `:375`, `:386` consultam incoming pelo remetente e conferem session; `:401` exige ctl.to == from e session atual não zero. Falhas incrementam ignored. Os testes oficiais de Leave, ReleaseAll e StatsRequest estrangeiros passam; a checagem de Stats foi conferida por leitura. |
| Limite por peer | O caso oficial de sessão única passa; **S3-1 reprova a continuidade do orçamento entre sessões**. Sessão desconhecida é descartada em `module.rs:451`/`:455`. |
| TokenStore 0600 | `adapter/tokens.rs:54` cria temporário com 0600, aplica chmod 0600 antes de rename; `:32` descarta token malformado. Testes `token_file_is_0600_and_roundtrips` e `invalid_token_is_discarded` passam. Persistência RemoteDesktop usa ExplicitlyRevoked e restore_token (`adapter/portal.rs:656`). Reuso sem diálogo permanece manual. |
| supported_capabilities ausente | A busca no crate encontra só comentários em portal.rs/diag.rs; nenhuma chamada. CreateSession/SelectDevices pedem Keyboard e Pointer diretamente. |
| Barreira direita | `adapter/barrier.rs:28` usa zone.x + zone.width; span arredonda e subtrai 1 no extremo final. Testes confirmam x=1920, y=0..539, outras bordas e offsets. Configuração por índice tem o defeito separado S3-2. |
| Permissão, pausa, liberação e loopback fake | Testes oficiais aprovados: destino sem input_in não inicia emulador; pausa descarta datagramas e exige novo Enter; Leave/PeerDown liberam teclas; loopback recebe 200 eventos com p50 <= p95. |

**Pendentes, sem execução por Argus:** §9 S3 itens **2, 3, 4, 6 e 7**, conforme instrução do Maestro. Captura/emulação reais, loopback com CLI, `portal_loopback --ignored`, reuso de token sem diálogo, teclado e posição após Release aguardam o log do Maestro/matriz. O teste gráfico ignorado no gate não equivale a aprovação. O workboard mantém essas responsabilidades em S3; nenhuma mudança de posse foi realizada.

#### Saídas reais dos gates

Comando: `cargo test -p clipsync-input`; início `2026-10-27T12:28:30.863454-03:00`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test loopback_counts_every_event_and_orders_percentiles ... ok
test rate_limit_drops_the_excess_and_counts_it ... ok
test destination_without_input_in_refuses_enter_and_never_starts_the_emulator ... ok
test release_all_only_from_the_session_owner ... ok
test leave_and_peer_down_release_everything ... ok
test stats_request_for_foreign_session_gets_no_answer ... ok
test paused_destination_drops_datagrams_and_needs_a_new_enter ... ok
test loopback_without_activation_reports_it ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s


running 1 test
test portal_loopback ... ignored, requer sessão gráfica

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test other_edges_and_offsets ... ok
test debug_hides_key_and_button_codes ... ok
test upper_half_of_the_right_edge_is_x_1920_y_0_to_539 ... ok
test config_defaults_and_barriers ... ok
test token_file_is_0600_and_roundtrips ... ok
test config_rejects_bad_input ... ok
test invalid_token_is_discarded ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling rustix v1.1.6
   Compiling linux-raw-sys v0.12.1
   Compiling slab v0.4.12
   Compiling tracing v0.1.44
   Compiling futures-util v0.3.34
   Compiling zbus v5.19.0
   Compiling reis v0.7.1
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling tempfile v3.27.0
   Compiling ashpd v0.13.13
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 10.97s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_input-17510809f8dddc56)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-6791af65086b93b6)
     Running tests/portal_loopback.rs (~/.cache/clipsync-verdict/target/debug/deps/portal_loopback-637131c0ec0c5ce4)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-3665d3e3b17d8b56)
     Running tests/units.rs (~/.cache/clipsync-verdict/target/debug/deps/units-1b23caa983c13b1f)
   Doc-tests clipsync_input
```

Comando: `cargo clippy -p clipsync-input --all-targets -- -D warnings`; início `2026-10-27T12:28:42.958875-03:00`; exit **0**.

stderr:

```text
    Checking linux-raw-sys v0.12.1
    Checking slab v0.4.12
    Checking tracing v0.1.44
    Checking futures-util v0.3.34
    Checking rustix v1.1.6
    Checking zbus v5.19.0
    Checking reis v0.7.1
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking tempfile v3.27.0
    Checking ashpd v0.13.13
    Checking clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.13s
```

Comando: `cargo fmt -p clipsync-input --check`; início `2026-10-27T12:28:49.148611-03:00`; exit **0**.

stdout/stderr vazios.

#### Reproduções independentes, fora dos fontes

Harness compilado por stdin contra as bibliotecas produzidas pelo gate; texto preservado como `s3-probe-stdin.txt` e comando completo em `s3-boundary-results.json`. SHA-256 do stdin: `4e68bae1e65b6a61f23df78b63c0457e707ef9d3f88dc246cd76af4aafd3836b`. Não foi acrescentado nem editado arquivo .rs no repositório/export. O primeiro comando de compilação do harness escolheu um artefato Tokio com features diferentes e falhou; após selecionar as dependências exatas do fingerprint de clipsync-input, compilou com exit 0. O histórico completo está no JSON. Os quatro warnings de helpers não usados são só do harness; o Clippy do crate permanece verde.

Comando: `~/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr/s3-boundary-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **101**.

stdout:

```text

running 1 test
same_peer sessions=2 submitted_datagrams=3000 emulated_datagrams=3000 elapsed_us=9348 limit=2000
test rate_budget_must_survive_new_session_from_same_peer ... FAILED

failures:

failures:
    rate_budget_must_survive_new_session_from_same_peer

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s

```

stderr:

```text

thread 'rate_budget_must_survive_new_session_from_same_peer' (470633) panicked at <anon>:146:5:
a new session must not renew the peer's rate budget
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Comando: `~/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr/s3-boundary-tests --exact hash_zero_monitor_must_be_zone_index_zero --nocapture`; exit **101**.

stdout:

```text

running 1 test
configured_monitor=#0 adapter_zone=Name("#0")
test hash_zero_monitor_must_be_zone_index_zero ... FAILED

failures:

failures:
    hash_zero_monitor_must_be_zone_index_zero

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text

thread 'hash_zero_monitor_must_be_zone_index_zero' (470635) panicked at <anon>:184:5:
contract #0 must select zone index 0
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

#### Hashes da base revisada

```text
87221da170a1501c83d6aa31be70b8873ced7432606c687ef6b59c07db692d9a  docs/contrato.md
c61ece11395b51b30ad72848618edd23b41fdac97f713deed13fb4873c3a8ef3  crates/clipsync-input/src/module.rs
5c82cf48afbb39ff5f77c5544f4ce9f816120acd47609789e084b187ee989fdd  crates/clipsync-input/src/adapter/portal.rs
dd4b79f976853aae21339e388354c06ac561bae8b9f9afcba61bc866b9902e36  crates/clipsync-input/src/adapter/tokens.rs
75d3718b8996bcaabfb8069c18a583514e89bdbbce3b1042e030b81cb553fc20  crates/clipsync-input/src/adapter/barrier.rs
123e5a26e44f4b0d52c94b41873003e64a146f575846b83cce873a49d51525e3  crates/clipsync-proto/src/input.rs
907ad512ba7c0a719d52064ddddfedcce2dbe6d11f5cdd824a37879dc778afd9  Cargo.lock
```

Evidência persistente: `~/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr` (meta/results/before/after.json, s3-probe-stdin.txt, s3-boundary-results.json e s3-boundary-tests). Argus não aplicou correções e não executou git.

### S3 · v1.1 · 2026-10-27T12:42:05-03:00 · Passa no escopo automático

Base informada: **30dcd43**, export `~/.cache/clipsync-verdict/src`, cache `~/.cache/clipsync-verdict/target`. **Encerra S3-1 e S3-2**; substitui a reprovação de 3eaa730 no escopo automático. Gates oficiais: **18 aprovados, 0 falhos, 1 ignorado** (portal_loopback); clippy/fmt saem 0. Inventário antes/depois idêntico. Itens gráficos §9 S3.2/3/4/6/7 continuam pendentes do Maestro.

- S3-1: `module.rs:103` implementa token bucket monotônico, capacidade 2.000 e reposição de 2.000/s; mapa por PeerId separado de Incoming, consultado em `:489`, removido somente em PeerDown (`:363`). Enter não renova o orçamento. Novo teste oficial `rate_limit_survives_a_new_enter` passa.
- A reprodução original, sem alterar o stdin, resultou em **2.022 aceitos em 11.192 µs** e exit 101: a asserção antiga exigia no máximo 2.000, ignorando a reposição contínua do token bucket informado pelo Maestro. Preservei esse resultado. Adaptei somente o teto da asserção para `2000 + ceil(elapsed_seconds * 2000)`, mantendo o mesmo peer, duas sessões e 3.000 envios. Resultado **2.024 aceitos, teto 2.025, 12.152 µs**, exit 0. Isso aprova a semântica de token bucket solicitada (burst 2.000 + reposição), não afirma um teto rígido de 2.000 em toda janela móvel de um segundo.
- S3-2: `config.rs:118` reconhece '#' seguido de dígitos como Index; `module.rs:574` usa essa função. A reprodução original agora imprime **configured_monitor=#0 adapter_zone=Index(0)**, exit 0. Testes novos cobrem #0/#12 e nomes; o restante da geometria permanece coberto pelos gates.

Debug, não impressão de keycodes, checagem do dono de session, TokenStore e ausência de chamada a supported_capabilities mantêm a leitura da revisão anterior; os testes correspondentes foram executados novamente. Sem edição de fonte, execução de git ou portal.

#### Saídas reais

Comando: `cargo test -p clipsync-input`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 9 tests
test loopback_counts_every_event_and_orders_percentiles ... ok
test rate_limit_drops_the_excess_and_counts_it ... ok
test destination_without_input_in_refuses_enter_and_never_starts_the_emulator ... ok
test release_all_only_from_the_session_owner ... ok
test leave_and_peer_down_release_everything ... ok
test stats_request_for_foreign_session_gets_no_answer ... ok
test rate_limit_survives_a_new_enter ... ok
test paused_destination_drops_datagrams_and_needs_a_new_enter ... ok
test loopback_without_activation_reports_it ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s


running 1 test
test portal_loopback ... ignored, requer sessão gráfica

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test debug_hides_key_and_button_codes ... ok
test other_edges_and_offsets ... ok
test config_defaults_and_barriers ... ok
test upper_half_of_the_right_edge_is_x_1920_y_0_to_539 ... ok
test hash_digits_is_a_zone_index_everything_else_a_name ... ok
test config_rejects_bad_input ... ok
test token_file_is_0600_and_roundtrips ... ok
test invalid_token_is_discarded ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.80s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_input-17510809f8dddc56)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-6791af65086b93b6)
     Running tests/portal_loopback.rs (~/.cache/clipsync-verdict/target/debug/deps/portal_loopback-637131c0ec0c5ce4)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-3665d3e3b17d8b56)
     Running tests/units.rs (~/.cache/clipsync-verdict/target/debug/deps/units-1b23caa983c13b1f)
   Doc-tests clipsync_input
```

Comando: `cargo clippy -p clipsync-input --all-targets -- -D warnings`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.27s
```

Comando: `cargo fmt -p clipsync-input --check`; exit **0**.

stdout/stderr vazios.

Comando: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-original-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **101**.

stdout:

```text

running 1 test
same_peer sessions=2 submitted_datagrams=3000 emulated_datagrams=2022 elapsed_us=11192 limit=2000
test rate_budget_must_survive_new_session_from_same_peer ... FAILED

failures:

failures:
    rate_budget_must_survive_new_session_from_same_peer

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s

```

stderr:

```text

thread 'rate_budget_must_survive_new_session_from_same_peer' (527610) panicked at <anon>:146:5:
a new session must not renew the peer's rate budget
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Comando: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-original-tests --exact hash_zero_monitor_must_be_zone_index_zero --nocapture`; exit **0**.

stdout:

```text

running 1 test
configured_monitor=#0 adapter_zone=Index(0)
test hash_zero_monitor_must_be_zone_index_zero ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

Comando: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-bucket-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **0**.

stdout:

```text

running 1 test
same_peer sessions=2 submitted_datagrams=3000 emulated_datagrams=2024 elapsed_us=12152 limit=2000
token_bucket capacity=2000 permitted_with_elapsed_refill=2025 observed=2024
test rate_budget_must_survive_new_session_from_same_peer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.01s

```

Hashes de reprodução (stdin):

```text
4e68bae1e65b6a61f23df78b63c0457e707ef9d3f88dc246cd76af4aafd3836b  s3-original-stdin.txt
c3957b861828cfef5e00f5b238350b5a233ec0b289f48f2f4a322e67731b794f  s3-bucket-stdin.txt
```

Evidências completas e hashes do export: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s3-results.json, s3-boundary-results.json, before.json/after-s3.json).

### S1 · Rivet · 2026-10-27T12:45:23-03:00 · Passa nos testes solicitados; auditoria separada

Base informada: **30dcd43**, export e cache em `~/.cache/clipsync-verdict/{src,target}`. Contrato v1.8: numeração atual S1.1 proto; S1.2 pairing; S1.3 TLS; S1.4 P2; S1.5 malha; S1.6 comportamento exercitado pela CLI; S1.7 lint/fmt. O pedido usava a numeração antiga (incluindo S1.10); esta matriz mapeia pelos critérios e nomes dos comandos, sem omitir TLS/pareamento. **Todos os comandos desta revisão saíram 0.** Inventário do export idêntico antes/depois.

| Comando/critério | Resultado observado |
|---|---|
| `cargo test -p clipsync-proto` | 40 testes aprovados: limites/validação, Debug, autorização, MemHub e Unreachable fora do wire. |
| `cargo test -p clipsync-core tls_` | 5 aprovados: contador nas reconexões, desconhecido, chave trocada, modo de pairing e atualização de pins. |
| `cargo test -p clipsync-core pairing_` | 8 aprovados, incluindo 7 casos de pairing e um teste TLS cujo nome contém pairing_. Commit inválido, conexão extra, janela após falha, nonces, confirmação bilateral e recusa estão cobertos. |
| `cargo test -p clipsync-core mesh_` | 4 casos de malha e 1 TLS aprovados. QUIC real em loopback, Hello/PeerUp, desconhecido, reconexão e chave trocada/Blocked. |
| `cargo test -p clipsync-core stream_and_datagram -- --nocapture` | 2 aprovados: uni + datagrama; controle 65.537 B recusado com STOP/RESET Protocol após ler apenas o prefixo. 300 amostras: RTT fluxo p95 629,432 µs; datagrama p95 405,839 µs. |
| clippy/fmt do workspace | Ambos saem 0, sem warnings/diffs. |
| S1.6 · pairing/agent pela CLI | Em 30dcd43 houve o flake S4-1. **Encerrado na reemissão 796e46c**: roteiro completo e workspace aprovados, conforme bloco S4 mais recente. |

**P2 — complemento para cumprir os tamanhos literais:** o teste do repositório usa StatsRequest/Stats e datagrama de quatro movimentos, sem comprovar 1 KiB/64 B. Harness de revisão compilado por stdin contra o core novo, sem alterar fonte: endpoints QUIC reais com as configurações TLS do core, verificação dos pins das identidades sintéticas, uma mesma Connection, fluxo uni com **1.024 B** e datagrama com **64 B**, eco conferido byte a byte. 20 aquecimentos + 300 RTTs medidos por modalidade em 127.0.0.1, sem perdas nesta execução. Resultado: **fluxo p50 168 µs / p95 432 µs; datagrama p50 117 µs / p95 313 µs**, ambos < 1 ms. Este complemento mede o transporte com essas cargas; o caminho ModuleHost/agent foi exercitado separadamente pelo teste oficial. Valores são RTT, não latência unilateral. Não generalizar para outra máquina/carga.

**Captura de tráfego:** não verificada. tshark ausente, tcpdump presente; o Maestro registra ausência de CAP_NET_RAW em `docs/evidencias/roteiro-2026-10-27.md`. Sem sudo e sem captura de tráfego alheio. As medições acima são comunicadas ao Maestro para o workboard; Argus escreve somente verdicts.

**Limite desta aprovação:** leitura de segurança/cripto do S1 ficou com o auditor recrutado pelo Maestro. Aqui se verificaram execução, cobertura e drift de contrato. Os testes de reason foram avaliados com a exceção v1.7 (strings constantes do rustls no handshake, CloseCode em closes de aplicação). Este bloco não substitui a auditoria nem aprova toda a integração.

#### Saídas reais

Comando: `cargo test -p clipsync-proto`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test inbound_datagram_table ... ok
test outbound_datagram_table ... ok
test inbound_table ... ok
test outbound_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test event_ids_are_random ... ok
test module_set ... ok
test peer_id_from_hex_errors ... ok
test peer_id_hex_roundtrip_and_short_form ... ok
test reject_reasons_have_portuguese_text ... ok
test peer_id_is_sha256_of_spki ... ok
test mono_us_is_monotonic ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test link_delivers_peer_up_with_grants_and_remote_modules ... ok
test inbound_refusal_becomes_rejected_at_sender ... ok
test paused_destination_rejects_offer_but_ack_passes ... ok
test delivers_authorized_offer ... ok
test outbound_rules_return_errors ... ok
test datagrams_follow_the_same_rules ... ok
test invalid_or_oversized_message_is_not_sent ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 5 tests
test control_request_debug_hides_clip_send_text ... ok
test clip_offer_debug_hides_text ... ok
test inbox_item_debug_hides_preview ... ok
test key_and_button_debug_hide_code ... ok
test control_messages_roundtrip_as_json ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 17 tests
test close_codes_are_stable_short_tokens ... ok
test datagram_over_1200_bytes_is_refused ... ok
test datagram_with_32_events_fits_and_roundtrips ... ok
test encode_refuses_body_over_max ... ok
test garbage_and_trailing_bytes_are_decode_errors ... ok
test module_msg_kind ... ok
test control_frame_of_64k_plus_one_is_refused ... ok
test unreachable_never_goes_on_the_wire ... ok
test every_frame_variant_roundtrips ... ok
test oversized_frame_is_refused_before_allocating ... ok
test validate_device_name_bounds ... ok
test validate_hello_min_version ... ok
test validate_refuses_nan_and_infinite_floats ... ok
test validate_refuses_33_events ... ok
test validate_refuses_pos_outside_unit_range ... ok
test write_then_read_over_a_duplex ... ok
test validate_clip_offer_mime_and_size ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling rustix v1.1.6
   Compiling linux-raw-sys v0.12.1
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.02s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-4b49005cfb6ac050)
     Running tests/authorize.rs (~/.cache/clipsync-verdict/target/debug/deps/authorize-951137c2be6334f2)
     Running tests/ids.rs (~/.cache/clipsync-verdict/target/debug/deps/ids-13024bae7b84b7f7)
     Running tests/memhub.rs (~/.cache/clipsync-verdict/target/debug/deps/memhub-7439a3a7f369aeb3)
     Running tests/redact.rs (~/.cache/clipsync-verdict/target/debug/deps/redact-567bd45310785f8c)
     Running tests/wire.rs (~/.cache/clipsync-verdict/target/debug/deps/wire-e2699de60d8c4ef6)
   Doc-tests clipsync_proto
```

Comando: `cargo test -p clipsync-core tls_`; exit **0**.

stdout:

```text

running 5 tests
test tls::tests::tls_pin_set_replace ... ok
test tls::tests::tls_changed_server_key_detected_by_dialer ... ok
test tls::tests::tls_unknown_client_refused_with_fixed_reason ... ok
test tls::tests::tls_pairing_mode_accepts_any_key_but_mesh_does_not ... ok
test tls::tests::tls_verifier_runs_on_every_connection_resumption_is_off ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling rustix v1.1.6
   Compiling rustls v0.23.46
   Compiling quinn-udp v0.5.16
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling tempfile v3.27.0
   Compiling quinn-proto v0.11.19
   Compiling quinn v0.11.12
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.37s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo test -p clipsync-core pairing_`; exit **0**.

stdout:

```text

running 8 tests
test pairing::tests::pairing_commitment_is_domain_separated ... ok
test tls::tests::tls_pairing_mode_accepts_any_key_but_mesh_does_not ... ok
test pairing::tests::pairing_bad_reveal_fails_saves_nothing_and_host_ends ... ok
test pairing::tests::pairing_declined_on_one_side_saves_nothing ... ok
test pairing::tests::pairing_happy_path_same_code_and_both_saved ... ok
test pairing::tests::pairing_code_depends_on_both_nonces ... ok
test pairing::tests::pairing_window_takes_one_attempt_only ... ok
test pairing::tests::pairing_after_a_failure_the_window_is_closed ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 4.10s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo test -p clipsync-core mesh_`; exit **0**.

stdout:

```text

running 1 test
test tls::tests::tls_pairing_mode_accepts_any_key_but_mesh_does_not ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 4 tests
test mesh_connects_after_hello_both_ways ... ok
test mesh_restart_reconnects ... ok
test mesh_unknown_identity_never_connects ... ok
test mesh_changed_key_is_blocked_with_constant_reason ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 4.23s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo test -p clipsync-core stream_and_datagram -- --nocapture`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 2 tests
test stream_and_datagram_control_frame_over_limit_is_reset ... ok
test stream_and_datagram ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 2.51s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
RTT fluxo 1 frame: p50 519.715µs p95 629.432µs · datagrama 4 eventos: p50 177.177µs p95 405.839µs (300 de 300)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.93s
```

Comando: `cargo fmt --all --check`; exit **0**.

stdout/stderr vazios.

Comando: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/p2-exact-tests --nocapture`; exit **0**.

stdout:

```text

running 1 test
same_quic_connection=true loopback=127.0.0.1 measurement=RTT warmup=20 samples=300 stream_payload_bytes=1024 stream_p50_us=168 stream_p95_us=432 datagram_payload_bytes=64 datagram_p50_us=117 datagram_p95_us=313
test p2_exact_payload_sizes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

```

Hashes relevantes:

```text
751f5d6e2f1c4992bec4a7b749a41330e3d26778cb18e4f1c2a31c40a2885de0  docs/contrato.md
c915f90d97c2bc68460be58a977189311c4f98007498ed20687571a7d4681970  crates/clipsync-core/src/tls.rs
4cb61b771887fe3c2034cf7851fddc12f18fc448b82cfbfab15c9af901a0195f  crates/clipsync-core/src/pairing.rs
6f4bb939c97ec1ebf5e27a7580ca5470dabda082fb333e917713510c689c6a3b  crates/clipsync-core/src/transport.rs
279ed2f0eb25087cb5bbf2dad3122ea3ef933c156f8715f31bac29b7d2c20491  crates/clipsync-core/tests/mesh.rs
781aa047350e2b4926333536600a85785329c06491582cc524cf45e62e753158  p2-exact-stdin.txt
```

Evidência integral: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s1-results.json, p2-exact-results.json, p2-exact-stdin.txt e inventários).

### S4 · Flux · 2026-10-27T12:50:23-03:00 · Não passa (flake reproduzido)

Base informada: **30dcd43**, export/cache em `~/.cache/clipsync-verdict/{src,target}`. Numeração atual v1.8: S4.1 perfil, S4.2 controle, S4.3/4 roteiros pair/mesh, S4.5 init, S4.6 pausa, S4.7 diagnose, S4.8 lint/fmt; I.1/I.2 clipboard e pausa. **Roteiro completo saiu 101**; repetição isolada de mesh_ saiu 0. Todos os hashes do export permaneceram idênticos durante a fila.

1. **S4-1 · P2 · `crates/clipsync-cli/tests/roteiro.rs:230` (chamado em `:479`)** — o roteiro completo falha esperando `chave do par mudou` depois de reinicializar a identidade de B. Na execução do Argus, `cargo test -p clipsync-cli --test roteiro` saiu **101**, com **7 aprovados e 1 falho**; último status de B era **desconectado**, não Blocked. A repetição isolada `... roteiro mesh_` saiu **0**, 1 aprovado em 41,21 s. Confirma **flake observado** no caminho de troca de identidade (além do relato anterior do Maestro sob `cargo test --workspace`). Não há evidência para atribuir a causa exclusivamente à concorrência ou ao timeout. O WAIT do helper é 20 s; investigação da sincronização do teste e do estado/backoff do agente cabe a Flux/Rivet. Impacto: gate S4.4 e integração S1.6 não reproduzem o resultado de forma confiável. Nenhuma correção foi aplicada; sucesso isolado não substitui o gate completo falho.

#### Cobertura e drift de contrato

| Critério | Resultado |
|---|---|
| Perfil | `cargo test -p clipsync-core profile_`: 10 aprovados. Permissões, identidade/pin, peers, estado e validação cobertos. |
| Controle | O comando literal `cargo test -p clipsync-core control_` sai 0, mas **não executa o teste do socket**: o nome é `serve_request_roundtrip_stale_socket_and_cleanup` (`tests/control.rs:22`). O único teste selecionado é o RESET QUIC de mesh.rs. Para fechar a evidência, rodei **`cargo test -p clipsync-core --test control`**, 1 aprovado; cobre request/serve, socket velho, agente duplicado, oversized e limpeza. Drift do filtro a corrigir no contrato ou no nome do teste. |
| Init / pairing / pausa / I.1 / I.2 | Os 7 casos não mesh do roteiro completo passam: três init, dois pair, um pause e um clip. O caso clip confere bytes=10, envio aplicado, saída 3 com pausa/módulo desligado e inbox sem replay após resume. |
| Mesh | Falha no gate completo, passa isolado; achado S4-1. O relato anterior de falha sob workspace veio do Maestro; **Argus não rodou workspace nesta rodada**. |
| `--help`, flags e perfil | Comandos da §6 presentes; --profile funciona antes/depois do subcomando. `run --help` não anuncia --listen; `run --listen` sai 2 conforme v1.8. **Drift textual menor:** títulos/ajuda e erros automáticos do clap continuam em inglês (`Usage`, `Commands`, `Options`, `Print help`, `error: unexpected argument`), apesar da exigência geral de saída em português de §2/§6. Saída real abaixo. |
| Diagnose | Executado por Argus: Ubuntu/Wayland, DISPLAY :0, Input Leap Flatpak em execução, Barrier não detectado, XFixes disponível, data-control ausente, InputCapture v1 e RemoteDesktop v2. Exit 0. Sem DISPLAY, marca as dependências do clipboard ausentes e continua com exit 0. |
| Lints/formato | Clippy workspace --all-targets -D warnings e fmt --all --check já executados nesta mesma base na revisão S1, ambos 0; nenhuma mudança de fonte desde então. |

**Códigos observados:** 0 em help/diagnose/init; 1 em módulos vazios, stdin vazio e probe sem DISPLAY; 2 em --listen removido e send sem --stdin; 3 na recusa de pareamento, send com destino pausado/desligado (asserções do roteiro e log manual); 4 em status/pause sem agente. Init com resposta vazia de clipboard grava desligado; `stat` observou diretório 700 e chave 600. Perfis de revisão novos ficaram em `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles`, sem usar os perfis do Maestro.

**Evidência manual incorporada (executada pelo Maestro, não por Argus):** [roteiro-2026-10-27.md](evidencias/roteiro-2026-10-27.md), release **30dcd43**, SHA-256 `b944a4bd391a624a017177adb7436a89a2cefb11ae974fbeadc6c2a4a1422709`. §14 itens 1/2/3/4/4b/6/7 mostram init, pareamento com mesmo código, status conectado, clipboard sintético aplicado/inbox bytes=10, pausa com rc=3, resume sem replay, módulo desligado rc=3 e diagnose rc=0. Complementam S4 e I.1/I.2; esse roteiro não exercita toda a troca de identidade do teste mesh_ e não encerra S4-1. Item 5/input gráfico segue pendente do usuário.

**Atualização de S1.6:** os testes diretos de S1 e P2 continuam aprovados no bloco anterior. A integração exercitada pelo roteiro CLI **não está aprovada** por S4-1; a leitura de segurança permanece com o auditor do Maestro.

#### Saídas reais

Comando: `cargo test -p clipsync-core profile_`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s


running 10 tests
test profile_both_modules_off_and_bad_names_are_refused ... ok
test profile_state_defaults_to_running_and_roundtrips_pause ... ok
test profile_peers_file_has_64_hex_ids_and_flat_permissions ... ok
test profile_key_with_loose_mode_is_refused_and_missing_profile_says_so ... ok
test profile_fake_adapters_are_written_to_the_module_tables ... ok
test profile_identity_id_is_the_single_pin_function ... ok
test profile_peers_upsert_replaces_by_id_or_name_and_rejects_duplicates_on_load ... ok
test profile_init_creates_private_profile_and_reloads_same_identity ... ok
test profile_config_with_unknown_key_or_wrong_version_is_an_error ... ok
test profile_reinit_gets_a_new_identity_but_keeps_peers ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo test -p clipsync-core control_`; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s


running 1 test
test stream_and_datagram_control_frame_over_limit_is_reset ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo test -p clipsync-core --test diag`; exit **0**.

stdout:

```text

running 1 test
test system_diag_reports_os_session_display_and_both_input_tools ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
```

Comando: `cargo test -p clipsync-cli --test roteiro`; exit **101**.

stdout:

```text

running 8 tests
test init_refuses_a_config_with_both_modules_off_and_says_why ... ok
test init_creates_profile_files_with_private_modes ... ok
test init_without_a_clipboard_answer_writes_false_and_flags_variant_works ... ok
test pair_declined_on_one_side_saves_nothing_on_either ... ok
test pair_shows_the_same_code_and_saves_permissions_on_both_sides ... ok
test pause_blocks_and_resume_returns_and_state_survives ... ok
test clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off ... ok
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... FAILED

failures:

---- mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key stdout ----

thread 'mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key' (565008) panicked at crates/clipsync-cli/tests/roteiro.rs:230:9:
status never showed "chave do par mudou"; last: notebook-a · id c9df-b094-01b6-937e · escuta 0.0.0.0:39080 · ativo
Pares:
  notebook-b (3529-e380-26b4-df4f) · desconectado · 127.0.0.1:40958 · módulos do par: nenhum · permissões: clipboard-out
Módulos:
  clipboard: ligado · adaptador fake · último evento: nenhum · último bloqueio: nenhum
  input: ligado · adaptador fake+fake · último evento: nenhum · último bloqueio: nenhum

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key

test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.64s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.17s
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
error: test failed, to rerun pass `-p clipsync-cli --test roteiro`
```

Comando: `cargo test -p clipsync-cli --test roteiro mesh_`; exit **0**.

stdout:

```text

running 1 test
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 41.21s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
```

Comando: `cargo test -p clipsync-core --test control`; exit **0**.

stdout:

```text

running 1 test
test serve_request_roundtrip_stale_socket_and_cleanup ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

```

stderr:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
```

Comando: `cargo run -q -p clipsync-cli -- --help`; exit **0**.

stdout:

```text
Teclado, mouse e clipboard entre os seus computadores

Usage: clipsync [OPTIONS] <COMMAND>

Commands:
  init      Cria a identidade e a configuração deste dispositivo
  status    Mostra o agente, os pares e os módulos
  pair      Pareia este dispositivo com outro
  run       Sobe o agente em primeiro plano (Ctrl-C encerra)
  pause     Pausa o envio e a aplicação
  resume    Retoma sem reenviar o que foi copiado na pausa
  send      Envia o texto da entrada padrão aos pares autorizados
  inbox     Lista a caixa de recebidos do clipboard
  diagnose  Verifica sistema, clipboard e input
  probe     Provas técnicas
  help      Print this message or the help of the given subcommand(s)

Options:
      --profile <DIR>  Pasta do perfil (padrão: $CLIPSYNC_PROFILE, depois ~/.config/clipsync)
  -h, --help           Print help
  -V, --version        Print version
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync run --help`; exit **0**.

stdout:

```text
Sobe o agente em primeiro plano (Ctrl-C encerra)

Usage: clipsync run [OPTIONS]

Options:
      --profile <DIR>  Pasta do perfil (padrão: $CLIPSYNC_PROFILE, depois ~/.config/clipsync)
  -h, --help           Print help
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync diagnose`; exit **0**.

stdout:

```text
[ok] sistema · sistema operacional: Ubuntu 26.04.1 LTS
[ok] sistema · sessão gráfica: wayland · ubuntu:GNOME
[ok] sistema · DISPLAY / XWayland: DISPLAY=:0; servidor X11 respondendo no socket
[ok] sistema · Input Leap: em execução (Flatpak): só detectado; o ClipSync não o altera e as duas ferramentas podem competir pelo mesmo mouse e teclado
[aviso] sistema · Barrier: não detectado
[ok] clipboard · ponte XWayland (DISPLAY): DISPLAY definido; conexão X11 aberta
[ok] clipboard · XFixes: SelectionNotify do CLIPBOARD disponível
[aviso] clipboard · data-control (Wayland): ausente (zwlr/ext_data_control): observação só pela ponte XWayland
[ok] input · portal InputCapture: versão 1
[ok] input · portal RemoteDesktop: versão 2; dispositivos BitFlags<DeviceType>(0b111, Keyboard | Pointer | Touchscreen)
[ok] input · monitores: eDP-1
[aviso] input · Input Leap / Barrier: em execução: input-leapc; podem disputar a captura
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync init --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a` · stdin nome + resposta vazia de clipboard; exit **0**.

stdout:

```text
Módulos do ClipSync (nada é transmitido agora, e cada permissão é concedida depois, no pareamento):
  • input: este computador pode enviar teclado e mouse a um par, e receber os de um par que você autorizar.
  • clipboard: textos copiados aqui podem ser enviados a pares autorizados, e os deles chegam aqui.
Nome deste dispositivo [notebook-linux]: Ligar o módulo de clipboard? [s/N] Perfil criado em ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a
  dispositivo: argus-fixture
  id: 03fd-b3c5-bb4c-a2d5
  escuta: 0.0.0.0:47800
  input: ligado · clipboard: desligado
Nada foi transmitido.
Próximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.
```

Comando: `stat -c '%a %n' ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a/identity.key.pem`; exit **0**.

stdout:

```text
700 ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a
600 ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a/identity.key.pem
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync status --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **4**.

stdout:

```text
argus-fixture · escuta 0.0.0.0:47800 · input ligado · clipboard desligado · agente parado (suba com `clipsync run`)
Pares:
  nenhum par pareado
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a pause`; exit **4**.

stderr:

```text
erro: agente parado (suba com `clipsync run`)
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync init --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/off --name off --modules ''`; exit **1**.

stderr:

```text
erro: pelo menos um módulo precisa estar ligado
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync run --listen 127.0.0.1:0`; exit **2**.

stderr:

```text
error: unexpected argument '--listen' found

Usage: clipsync run [OPTIONS]

For more information, try '--help'.
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync send --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **2**.

stderr:

```text
error: the following required arguments were not provided:
  --stdin

Usage: clipsync send --stdin --profile <DIR>

For more information, try '--help'.
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync send --stdin --profile ~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **1**.

stderr:

```text
erro: nada para enviar: a entrada está vazia
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync probe clipboard --write` · env -u DISPLAY; exit **1**.

stderr:

```text
erro: a ponte de clipboard exige XWayland ativo (DISPLAY)
```

Comando: `~/.cache/clipsync-verdict/target/debug/clipsync diagnose` · env -u DISPLAY; exit **0**.

stdout:

```text
[ok] sistema · sistema operacional: Ubuntu 26.04.1 LTS
[ok] sistema · sessão gráfica: wayland · ubuntu:GNOME
[ausente] sistema · DISPLAY / XWayland: DISPLAY indefinido: a ponte de clipboard exige XWayland ativo
[ok] sistema · Input Leap: em execução (Flatpak): só detectado; o ClipSync não o altera e as duas ferramentas podem competir pelo mesmo mouse e teclado
[aviso] sistema · Barrier: não detectado
[ausente] clipboard · ponte XWayland (DISPLAY): a ponte de clipboard exige XWayland ativo (DISPLAY)
[ausente] clipboard · XFixes: sem DISPLAY não há como verificar
[aviso] clipboard · data-control (Wayland): ausente (zwlr/ext_data_control): observação só pela ponte XWayland
[ok] input · portal InputCapture: versão 1
[ok] input · portal RemoteDesktop: versão 2; dispositivos BitFlags<DeviceType>(0b111, Keyboard | Pointer | Touchscreen)
[ok] input · monitores: eDP-1
[aviso] input · Input Leap / Barrier: em execução: input-leapc; podem disputar a captura
```

Evidência integral: `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s4-results.json, s4-mesh-isolated.json, s4-extra-results.json, cli-binary.sha256 e after-all.json). Nenhum fonte foi editado, nenhum git executado.

### S2 · adendo manual · 2026-10-27T12:52:13-03:00 · Evidências M incorporadas

Os itens S2.2/S2.3 deixam de aguardar a execução dos cenários abaixo: o Maestro os executou com o **release 30dcd43** e forneceu [docs/evidencias/roteiro-2026-10-27.md](evidencias/roteiro-2026-10-27.md). Argus leu o arquivo; **não executou esses probes gráficos**. SHA-256 da evidência lida: `b944a4bd391a624a017177adb7436a89a2cefb11ae974fbeadc6c2a4a1422709`. Cópia preservada no diretório de evidências da revisão 30dcd43.

| Critério | Evidência e conclusão |
|---|---|
| S2.2 · observação real | Maestro usou `probe clipboard --watch 12s` com três cópias Wayland via wl-copy: três linhas de mime/tamanho, bytes=25, latências 24/17/26 ms, rc=0. **Passa no cenário registrado**. A janela foi 12 s, não os 15 s do comando de referência. O log não separa uma ação de fechar o app e observar ausência de evento; esse subcaso não é inferido da saída (o teste fake owner_takeover já passou na revisão automática). |
| S2.3 · escrita/eco | Saída literal abaixo confirma escrita, releitura idêntica, eco descartado e nenhuma nova cópia; rc=0. **Passa**, conforme execução externa. O log também contém um warning arboard ao entregar ao clipboard manager; ele não mudou o rc nem as verificações impressas. Não converter esse warning em falha sem evidência. |
| S2.4 · sem DISPLAY | **Executado por Argus no binário debug compilado do export 30dcd43**: `env -u DISPLAY clipsync probe clipboard --write` sai 1 e imprime a mensagem exigida; diagnose marca ponte/XFixes ausentes. Nenhuma escrita de clipboard real ocorreu nessa execução sem DISPLAY. |

Saídas externas relevantes (texto sintético, sem clipboard do usuário):

```text
observação: eventos (XFixes via XWayland)
mime=text/plain;charset=utf-8 bytes=25 latência_leitura=24 ms
mime=text/plain;charset=utf-8 bytes=25 latência_leitura=17 ms
mime=text/plain;charset=utf-8 bytes=25 latência_leitura=26 ms
rc=0
escrita: ok · leitura idêntica: ok · evento da própria escrita: descartado (eco) · nova cópia disparada: não
2026-10-27T15:41:35.396038Z  WARN arboard::platform::linux::x11: Could not hand the clipboard contents over to the clipboard manager. The request timed out.
rc=0
clipboard restaurado
```

Saída direta de Argus para S2.4 (exit 1):

```text
erro: a ponte de clipboard exige XWayland ativo (DISPLAY)
```

A aprovação automática de S2 v1.4 consta no bloco anterior (15 testes e duas reproduções corrigidas). O inventário confirma que **nenhum arquivo de clipsync-clipboard mudou de 7923e20 para 30dcd43**; o roteiro CLI de clipboard/I.1/I.2 também passou na revisão 30dcd43. Este adendo acrescenta as observações manuais; não afirma nova execução dos gates S2.1/S2.5 nesse commit. Input gráfico/item 5 do roteiro permanece pendente do usuário e pertence a S3.

Evidências diretas de S2.4 e diagnose em `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s4-extra-results.json`; evidência manual integral no arquivo do Maestro e em `~/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/manual-evidence-copy.md`.

### S4 · reemissão 796e46c · 2026-10-27T12:56:09-03:00 · Passa — S4-1 e S1.6 encerrados

Base informada pelo Maestro: **796e46c**, export `~/.cache/clipsync-verdict/src`, `CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target`. **Substitui a reprovação de S4 em 30dcd43 e encerra S4-1 e o critério de integração S1.6.** Nenhum arquivo do export mudou durante a revisão.

| Verificação fresca | Resultado |
|---|---|
| `cargo test -p clipsync-cli --test roteiro` | **8 aprovados**, 0 falhos; mesh incluído. Duração dos testes 45,01 s. |
| `cargo test --workspace` | **113 aprovados**, 0 falhos, **1 ignorado** (`portal_loopback`). Roteiro: 8 aprovados em 43,74 s. Duração do comando 71,39 s, incluindo compilação. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| `cargo fmt --all --check` | Exit 0, saída vazia. |
| `CLIPSYNC_PERF=1 cargo test -p clipsync-core --test mesh stream_and_datagram -- --exact --nocapture` | 1 aprovado; RTT fluxo p95 **703,969 µs**, datagrama p95 **452,347 µs**, 300/300 na primeira tentativa, zero reenvios. |

**Correção conferida por leitura:** os oito testes do roteiro adquirem o mesmo Mutex estático em `serial()` (`tests/roteiro.rs:18`); WAIT passou de 20 para 60 s (`:15`). Entre 30dcd43 e 796e46c, os únicos arquivos de código alterados são `crates/clipsync-cli/tests/roteiro.rs` e `crates/clipsync-core/tests/mesh.rs`; implementação de core/CLI/input/clipboard permanece idêntica. O cenário anteriormente falho passou tanto no roteiro completo quanto dentro do workspace. O Maestro também informou três execuções verdes do workspace; **Argus executou uma**, registrada integralmente abaixo, além do roteiro separado. O encerramento é baseado nessas observações, sem prometer ausência de qualquer flake futuro.

**Transporte:** o teste novo mantém entrega/ordem/integridade como asserções normais, permite até três tentativas de datagrama e condiciona o teto p95 < 1 ms a CLIPSYNC_PERF=1. Por isso executei esse modo explicitamente depois do workspace. A saída acima corresponde a StatsRequest/Stats e datagrama de quatro eventos; as medições literais 1.024 B/64 B estão no complemento P2 do veredito S1 de 30dcd43, com a mesma implementação de transporte/TLS (somente o teste mudou).

**S1.6 encerrado:** pairing, reconexão, recusa de desconhecido, troca de chave/Blocked, pausa e clipboard pela CLI passaram no roteiro completo. A auditoria de segurança S1 permanece com o auditor recrutado pelo Maestro; não é substituída por este fechamento de integração.

**Drifts aceitos como pós-MVP, sem bloquear por decisão explícita do Maestro:** ajuda/erros padrão do clap em inglês; filtro `control_` não seleciona o teste do socket. Ambos continuam registrados. O workspace executou o teste real `serve_request_roundtrip_stale_socket_and_cleanup`, aprovado; a revisão anterior também o executou por `--test control`.

**Evidências preservadas de S2/S4:** ajuda, diagnose, exit codes e permissões dos perfis foram verificados em 30dcd43; seus fontes não mudaram. Evidência manual release do Maestro em [roteiro-2026-10-27.md](evidencias/roteiro-2026-10-27.md) foi incorporada aos blocos S2/S4. O workspace desta reemissão também repetiu os **15 testes de clipboard** e **18 testes não gráficos de input**, todos aprovados. **Input gráfico/§14 item 5 e portal_loopback continuam pendentes do usuário**. Nenhuma nova captura/emulação gráfica foi executada pelo Argus.

#### Saídas reais da reemissão

Comando: `cargo test -p clipsync-cli --test roteiro`; duração 48.335 s; exit **0**.

stdout:

```text

running 8 tests
test clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off ... ok
test init_refuses_a_config_with_both_modules_off_and_says_why ... ok
test init_creates_profile_files_with_private_modes ... ok
test init_without_a_clipboard_answer_writes_false_and_flags_variant_works ... ok
test pair_declined_on_one_side_saves_nothing_on_either ... ok
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... ok
test pause_blocks_and_resume_returns_and_state_survives ... ok
test pair_shows_the_same_code_and_saves_permissions_on_both_sides ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 45.01s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.24s
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
```

Comando: `cargo test --workspace`; duração 71.39 s; exit **0**.

stdout:

```text

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test init_creates_profile_files_with_private_modes ... ok
test clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off ... ok
test init_refuses_a_config_with_both_modules_off_and_says_why ... ok
test pair_declined_on_one_side_saves_nothing_on_either ... ok
test pair_shows_the_same_code_and_saves_permissions_on_both_sides ... ok
test init_without_a_clipboard_answer_writes_false_and_flags_variant_works ... ok
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... ok
test pause_blocks_and_resume_returns_and_state_survives ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.74s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test ledger::tests::ids_past_the_ttl_are_forgotten ... ok
test ledger::tests::full_cap_repeating_the_oldest_id_is_a_duplicate_not_a_new_event ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 12 tests
test i_debug_never_shows_content ... ok
test h_manual_mode_only_fills_the_inbox ... ok
test c_old_lamport_after_new_is_stale_and_goes_to_inbox ... ok
test f_remote_module_disabled_is_reported ... ok
test b_duplicate_event_id_applies_once_and_acks_duplicate ... ok
test d_x_y_x_makes_three_offers_x_x_makes_one ... ok
test l_pause_between_authorization_and_write_blocks_the_write ... ok
test k_ttl_counts_from_local_receipt_and_drops_the_text ... ok
test a_mesh_one_copy_two_offers_no_reoffer ... ok
test e_own_write_echo_takeover_and_empty_do_not_offer ... ok
test g_paused_neither_sends_nor_applies_and_resume_does_not_resend ... ok
test j_clipsend_lists_every_target_and_no_ack_is_unreachable ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 12 tests
test pairing::tests::pairing_commitment_is_domain_separated ... ok
test tls::tests::tls_pin_set_replace ... ok
test tls::tests::tls_changed_server_key_detected_by_dialer ... ok
test tls::tests::tls_unknown_client_refused_with_fixed_reason ... ok
test tls::tests::tls_pairing_mode_accepts_any_key_but_mesh_does_not ... ok
test tls::tests::tls_verifier_runs_on_every_connection_resumption_is_off ... ok
test pairing::tests::pairing_code_depends_on_both_nonces ... ok
test pairing::tests::pairing_declined_on_one_side_saves_nothing ... ok
test pairing::tests::pairing_bad_reveal_fails_saves_nothing_and_host_ends ... ok
test pairing::tests::pairing_happy_path_same_code_and_both_saved ... ok
test pairing::tests::pairing_window_takes_one_attempt_only ... ok
test pairing::tests::pairing_after_a_failure_the_window_is_closed ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.12s


running 1 test
test serve_request_roundtrip_stale_socket_and_cleanup ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 1 test
test system_diag_reports_os_session_display_and_both_input_tools ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 6 tests
test stream_and_datagram_control_frame_over_limit_is_reset ... ok
test mesh_connects_after_hello_both_ways ... ok
test stream_and_datagram ... ok
test mesh_unknown_identity_never_connects ... ok
test mesh_restart_reconnects ... ok
test mesh_changed_key_is_blocked_with_constant_reason ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.32s


running 10 tests
test profile_both_modules_off_and_bad_names_are_refused ... ok
test profile_identity_id_is_the_single_pin_function ... ok
test profile_state_defaults_to_running_and_roundtrips_pause ... ok
test profile_peers_file_has_64_hex_ids_and_flat_permissions ... ok
test profile_key_with_loose_mode_is_refused_and_missing_profile_says_so ... ok
test profile_fake_adapters_are_written_to_the_module_tables ... ok
test profile_peers_upsert_replaces_by_id_or_name_and_rejects_duplicates_on_load ... ok
test profile_init_creates_private_profile_and_reloads_same_identity ... ok
test profile_config_with_unknown_key_or_wrong_version_is_an_error ... ok
test profile_reinit_gets_a_new_identity_but_keeps_peers ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 9 tests
test loopback_counts_every_event_and_orders_percentiles ... ok
test rate_limit_drops_the_excess_and_counts_it ... ok
test destination_without_input_in_refuses_enter_and_never_starts_the_emulator ... ok
test release_all_only_from_the_session_owner ... ok
test leave_and_peer_down_release_everything ... ok
test stats_request_for_foreign_session_gets_no_answer ... ok
test rate_limit_survives_a_new_enter ... ok
test paused_destination_drops_datagrams_and_needs_a_new_enter ... ok
test loopback_without_activation_reports_it ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s


running 1 test
test portal_loopback ... ignored, requer sessão gráfica

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test debug_hides_key_and_button_codes ... ok
test config_defaults_and_barriers ... ok
test other_edges_and_offsets ... ok
test hash_digits_is_a_zone_index_everything_else_a_name ... ok
test config_rejects_bad_input ... ok
test upper_half_of_the_right_edge_is_x_1920_y_0_to_539 ... ok
test token_file_is_0600_and_roundtrips ... ok
test invalid_token_is_discarded ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test inbound_datagram_table ... ok
test outbound_datagram_table ... ok
test outbound_table ... ok
test inbound_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test module_set ... ok
test reject_reasons_have_portuguese_text ... ok
test peer_id_from_hex_errors ... ok
test mono_us_is_monotonic ... ok
test peer_id_is_sha256_of_spki ... ok
test peer_id_hex_roundtrip_and_short_form ... ok
test event_ids_are_random ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test link_delivers_peer_up_with_grants_and_remote_modules ... ok
test paused_destination_rejects_offer_but_ack_passes ... ok
test inbound_refusal_becomes_rejected_at_sender ... ok
test delivers_authorized_offer ... ok
test outbound_rules_return_errors ... ok
test datagrams_follow_the_same_rules ... ok
test invalid_or_oversized_message_is_not_sent ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 5 tests
test key_and_button_debug_hide_code ... ok
test clip_offer_debug_hides_text ... ok
test inbox_item_debug_hides_preview ... ok
test control_request_debug_hides_clip_send_text ... ok
test control_messages_roundtrip_as_json ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 17 tests
test datagram_over_1200_bytes_is_refused ... ok
test garbage_and_trailing_bytes_are_decode_errors ... ok
test module_msg_kind ... ok
test encode_refuses_body_over_max ... ok
test datagram_with_32_events_fits_and_roundtrips ... ok
test every_frame_variant_roundtrips ... ok
test validate_hello_min_version ... ok
test validate_refuses_nan_and_infinite_floats ... ok
test validate_refuses_33_events ... ok
test validate_refuses_pos_outside_unit_range ... ok
test validate_device_name_bounds ... ok
test close_codes_are_stable_short_tokens ... ok
test unreachable_never_goes_on_the_wire ... ok
test oversized_frame_is_refused_before_allocating ... ok
test control_frame_of_64k_plus_one_is_refused ... ok
test write_then_read_over_a_duplex ... ok
test validate_clip_offer_mime_and_size ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 11.54s
     Running unittests src/main.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync-0967276870f4a82a)
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-cd5a0289063c2691)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-c72612d8116dfe30)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-832e8e82e2bb6e1a)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-312579bf9b7d9ca5)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-cc539867a6e3d6bf)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-a9645200717e2296)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-d468434fb273c2cd)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-5d9c5b98048de6d8)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_input-b172c508683df1d7)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-d213bec33e27bc54)
     Running tests/portal_loopback.rs (~/.cache/clipsync-verdict/target/debug/deps/portal_loopback-b8a062dca1ade761)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-8ffafc657e108765)
     Running tests/units.rs (~/.cache/clipsync-verdict/target/debug/deps/units-8bfc9ad83c6aad0a)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (~/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (~/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (~/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (~/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (~/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
   Doc-tests clipsync_clipboard
   Doc-tests clipsync_core
   Doc-tests clipsync_input
   Doc-tests clipsync_proto
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; duração 1.363 s; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.27s
```

Comando: `cargo fmt --all --check`; duração 0.155 s; exit **0**.

stdout/stderr vazios.

Comando: `CLIPSYNC_PERF=1 cargo test -p clipsync-core --test mesh stream_and_datagram -- --exact --nocapture`; duração 4.097 s; exit **0**.

stdout:

```text

running 1 test
test stream_and_datagram ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 2.54s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.52s
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
RTT fluxo 1 frame: p50 556.598µs p95 703.969µs · datagrama 4 eventos: p50 258.39µs p95 452.347µs (1ª tentativa 300 de 300, reenvios 0)
```

Hashes relevantes:

```text
751f5d6e2f1c4992bec4a7b749a41330e3d26778cb18e4f1c2a31c40a2885de0  docs/contrato.md
6ee61f12e6ea4f2314736f58a9df29c18f4ce593c234749fba7862ad5a19b7ff  crates/clipsync-cli/tests/roteiro.rs
7bf7db7c6f3056bdfaf041c8be1f3c40596c2f94eb81da950848f96fb2c59efe  crates/clipsync-core/tests/mesh.rs
6f4bb939c97ec1ebf5e27a7580ca5470dabda082fb333e917713510c689c6a3b  crates/clipsync-core/src/transport.rs
907ad512ba7c0a719d52064ddddfedcce2dbe6d11f5cdd824a37879dc778afd9  Cargo.lock
```

Evidência completa: `~/.cache/clipsync-verdict/argus-796e46c-bf8p000u` (meta/results/before/after.json). Nenhum fonte foi editado pelo Argus e nenhum comando git foi executado.

### Gates finais do MVP · c1ed65d · Passa

Executado por **Argus** de **2026-10-27T13:08:12.829380-03:00** a **2026-10-27T13:11:25.501762-03:00**, no export informado pelo Maestro como **c1ed65d**: `~/.cache/clipsync-verdict/src`. Ambiente: `source ~/.cargo/env`; `CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target`; toolchain **1.99.1-x86_64-unknown-linux-gnu**, rustc 1.99.1 (58cf8f9c3), cargo 1.99.1 (5f94df478).

**Todos os cinco gates solicitados saíram 0.** Os hashes de todos os arquivos do export são idênticos antes/depois, incluindo Cargo.lock. Não houve falha de espaço/cota.

| Gate | Resultado | Duração do comando |
|---|---|---|
| `cargo test --workspace` | **123 aprovados, 0 falhos, 1 ignorado** (`portal_loopback`, requer sessão gráfica). Inclui o roteiro 8/8. | 72.532 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; sem warnings. | 2.196 s |
| `cargo fmt --all --check` | Exit 0; saída vazia. | 0.185 s |
| `cargo build --release -p clipsync-cli` | Exit 0; binário otimizado gerado. | 68.793 s |
| `cargo test -p clipsync-cli --test roteiro` | **8/8 aprovados** em execução separada, 0 falhos/ignorados; testes em 43,60 s. | 48.942 s |

Os novos testes de nomes hostis/controles/bidi, socket privado e pausa quando a persistência falha também aparecem aprovados na saída do workspace. Este bloco registra os gates finais; não substitui o relatório da auditoria de segurança.

**Artefato release:** `~/.cache/clipsync-verdict/target/release/clipsync` · **16.396.096 bytes**.

```text
SHA-256 0670fd6360edeca8ac95ee7999cdbacc71f787a55c22631885a7fbad91841bf1
```

**Pendência mantida:** input gráfico/§14 item 5 e `portal_loopback` continuam dependentes do usuário; o ignorado não representa aprovação gráfica. Os drifts de ajuda do clap e filtro control_ permanecem pós-MVP por decisão anterior do Maestro. O teste real do socket foi executado dentro do workspace. Nenhum fonte foi editado, nenhum git foi executado e nenhum agente de teste deste cache ficou rodando.

#### Saídas reais dos gates finais

Comando: `cargo test --workspace`; início `2026-10-27T13:08:12.830401-03:00`; exit **0**.

stdout:

```text

running 3 tests
test text::tests::a_hostile_peer_name_never_reaches_the_delivery_line_raw ... ok
test pair::tests::hostile_name_is_cleaned_and_the_code_comes_right_before_the_question ... ok
test text::tests::clean_replaces_controls_c1_and_bidi_and_keeps_text ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test init_refuses_a_config_with_both_modules_off_and_says_why ... ok
test clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off ... ok
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... ok
test pair_shows_the_same_code_and_saves_permissions_on_both_sides ... ok
test init_creates_profile_files_with_private_modes ... ok
test pair_declined_on_one_side_saves_nothing_on_either ... ok
test init_without_a_clipboard_answer_writes_false_and_flags_variant_works ... ok
test pause_blocks_and_resume_returns_and_state_survives ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.78s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test ledger::tests::ids_past_the_ttl_are_forgotten ... ok
test ledger::tests::full_cap_repeating_the_oldest_id_is_a_duplicate_not_a_new_event ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 12 tests
test i_debug_never_shows_content ... ok
test h_manual_mode_only_fills_the_inbox ... ok
test c_old_lamport_after_new_is_stale_and_goes_to_inbox ... ok
test f_remote_module_disabled_is_reported ... ok
test b_duplicate_event_id_applies_once_and_acks_duplicate ... ok
test d_x_y_x_makes_three_offers_x_x_makes_one ... ok
test l_pause_between_authorization_and_write_blocks_the_write ... ok
test k_ttl_counts_from_local_receipt_and_drops_the_text ... ok
test a_mesh_one_copy_two_offers_no_reoffer ... ok
test e_own_write_echo_takeover_and_empty_do_not_offer ... ok
test g_paused_neither_sends_nor_applies_and_resume_does_not_resend ... ok
test j_clipsend_lists_every_target_and_no_ack_is_unreachable ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 14 tests
test pairing::tests::pairing_commitment_is_domain_separated ... ok
test pairing::tests::pairing_save_refuses_a_bad_name ... ok
test tls::tests::tls_pin_set_replace ... ok
test tls::tests::tls_changed_server_key_detected_by_dialer ... ok
test tls::tests::tls_pairing_mode_accepts_any_key_but_mesh_does_not ... ok
test tls::tests::tls_unknown_client_refused_with_fixed_reason ... ok
test tls::tests::tls_verifier_runs_on_every_connection_resumption_is_off ... ok
test pairing::tests::pairing_malicious_peer_name_fails_and_saves_nothing ... ok
test pairing::tests::pairing_bad_reveal_fails_saves_nothing_and_host_ends ... ok
test pairing::tests::pairing_declined_on_one_side_saves_nothing ... ok
test pairing::tests::pairing_code_depends_on_both_nonces ... ok
test pairing::tests::pairing_happy_path_same_code_and_both_saved ... ok
test pairing::tests::pairing_window_takes_one_attempt_only ... ok
test pairing::tests::pairing_after_a_failure_the_window_is_closed ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.10s


running 2 tests
test socket_is_created_private_and_a_live_agent_is_never_unlinked ... ok
test serve_request_roundtrip_stale_socket_and_cleanup ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 1 test
test system_diag_reports_os_session_display_and_both_input_tools ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 7 tests
test stream_and_datagram_control_frame_over_limit_is_reset ... ok
test pause_fails_closed_when_state_cannot_be_saved ... ok
test mesh_connects_after_hello_both_ways ... ok
test stream_and_datagram ... ok
test mesh_unknown_identity_never_connects ... ok
test mesh_changed_key_is_blocked_with_constant_reason ... ok
test mesh_restart_reconnects ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.15s


running 12 tests
test profile_both_modules_off_and_bad_names_are_refused ... ok
test profile_device_name_with_formatting_characters_is_refused_at_init ... ok
test profile_identity_id_is_the_single_pin_function ... ok
test profile_key_with_loose_mode_is_refused_and_missing_profile_says_so ... ok
test profile_fake_adapters_are_written_to_the_module_tables ... ok
test profile_state_defaults_to_running_and_roundtrips_pause ... ok
test profile_peers_file_has_64_hex_ids_and_flat_permissions ... ok
test profile_peers_upsert_replaces_by_id_or_name_and_rejects_duplicates_on_load ... ok
test profile_config_with_unknown_key_or_wrong_version_is_an_error ... ok
test profile_peer_with_a_hostile_name_is_quarantined_and_the_rest_loads ... ok
test profile_init_creates_private_profile_and_reloads_same_identity ... ok
test profile_reinit_gets_a_new_identity_but_keeps_peers ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 9 tests
test loopback_counts_every_event_and_orders_percentiles ... ok
test rate_limit_drops_the_excess_and_counts_it ... ok
test destination_without_input_in_refuses_enter_and_never_starts_the_emulator ... ok
test release_all_only_from_the_session_owner ... ok
test leave_and_peer_down_release_everything ... ok
test stats_request_for_foreign_session_gets_no_answer ... ok
test rate_limit_survives_a_new_enter ... ok
test paused_destination_drops_datagrams_and_needs_a_new_enter ... ok
test loopback_without_activation_reports_it ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s


running 1 test
test portal_loopback ... ignored, requer sessão gráfica

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 8 tests
test debug_hides_key_and_button_codes ... ok
test hash_digits_is_a_zone_index_everything_else_a_name ... ok
test other_edges_and_offsets ... ok
test upper_half_of_the_right_edge_is_x_1920_y_0_to_539 ... ok
test config_defaults_and_barriers ... ok
test config_rejects_bad_input ... ok
test token_file_is_0600_and_roundtrips ... ok
test invalid_token_is_discarded ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 4 tests
test inbound_datagram_table ... ok
test outbound_datagram_table ... ok
test outbound_table ... ok
test inbound_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test module_set ... ok
test event_ids_are_random ... ok
test peer_id_from_hex_errors ... ok
test peer_id_hex_roundtrip_and_short_form ... ok
test mono_us_is_monotonic ... ok
test peer_id_is_sha256_of_spki ... ok
test reject_reasons_have_portuguese_text ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
test link_delivers_peer_up_with_grants_and_remote_modules ... ok
test inbound_refusal_becomes_rejected_at_sender ... ok
test paused_destination_rejects_offer_but_ack_passes ... ok
test delivers_authorized_offer ... ok
test datagrams_follow_the_same_rules ... ok
test outbound_rules_return_errors ... ok
test invalid_or_oversized_message_is_not_sent ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 5 tests
test inbox_item_debug_hides_preview ... ok
test control_request_debug_hides_clip_send_text ... ok
test clip_offer_debug_hides_text ... ok
test key_and_button_debug_hide_code ... ok
test control_messages_roundtrip_as_json ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 18 tests
test datagram_over_1200_bytes_is_refused ... ok
test garbage_and_trailing_bytes_are_decode_errors ... ok
test datagram_with_32_events_fits_and_roundtrips ... ok
test module_msg_kind ... ok
test encode_refuses_body_over_max ... ok
test validate_hello_min_version ... ok
test validate_device_name_rejects_control_and_bidi ... ok
test every_frame_variant_roundtrips ... ok
test validate_refuses_nan_and_infinite_floats ... ok
test validate_refuses_33_events ... ok
test validate_refuses_pos_outside_unit_range ... ok
test validate_device_name_bounds ... ok
test close_codes_are_stable_short_tokens ... ok
test unreachable_never_goes_on_the_wire ... ok
test oversized_frame_is_refused_before_allocating ... ok
test control_frame_of_64k_plus_one_is_refused ... ok
test write_then_read_over_a_duplex ... ok
test validate_clip_offer_mime_and_size ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 12.64s
     Running unittests src/main.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync-0967276870f4a82a)
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-cd5a0289063c2691)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-c72612d8116dfe30)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_core-e47edfcf9db606a7)
     Running tests/control.rs (~/.cache/clipsync-verdict/target/debug/deps/control-be77a47c4b2e6213)
     Running tests/diag.rs (~/.cache/clipsync-verdict/target/debug/deps/diag-5be713f23f06ba4b)
     Running tests/mesh.rs (~/.cache/clipsync-verdict/target/debug/deps/mesh-74da5007e3e212f2)
     Running tests/profile.rs (~/.cache/clipsync-verdict/target/debug/deps/profile-49bad18a8c67f2a8)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-fddeda6a0139483a)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_input-b172c508683df1d7)
     Running tests/module_mem.rs (~/.cache/clipsync-verdict/target/debug/deps/module_mem-d213bec33e27bc54)
     Running tests/portal_loopback.rs (~/.cache/clipsync-verdict/target/debug/deps/portal_loopback-b8a062dca1ade761)
     Running tests/smoke.rs (~/.cache/clipsync-verdict/target/debug/deps/smoke-8ffafc657e108765)
     Running tests/units.rs (~/.cache/clipsync-verdict/target/debug/deps/units-8bfc9ad83c6aad0a)
     Running unittests src/lib.rs (~/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (~/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (~/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (~/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (~/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (~/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
   Doc-tests clipsync_clipboard
   Doc-tests clipsync_core
   Doc-tests clipsync_input
   Doc-tests clipsync_proto
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; início `2026-10-27T13:09:25.365969-03:00`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.13s
```

Comando: `cargo fmt --all --check`; início `2026-10-27T13:09:27.562692-03:00`; exit **0**.

stdout/stderr vazios.

Comando: `cargo build --release -p clipsync-cli`; início `2026-10-27T13:09:27.747954-03:00`; exit **0**.

stderr:

```text
   Compiling proc-macro2 v1.0.107
   Compiling quote v1.0.47
   Compiling unicode-ident v1.0.27
   Compiling libc v0.2.191
   Compiling cfg-if v1.0.5
   Compiling serde_core v1.0.229
   Compiling pin-project-lite v0.2.17
   Compiling serde v1.0.229
   Compiling once_cell v1.21.4
   Compiling winnow v1.0.4
   Compiling hashbrown v0.17.1
   Compiling equivalent v1.0.3
   Compiling rustix v1.1.6
   Compiling find-msvc-tools v0.1.15
   Compiling tracing-core v0.1.36
   Compiling toml_datetime v1.1.2+spec-1.1.0
   Compiling linux-raw-sys v0.12.1
   Compiling rand_core v0.10.1
   Compiling bitflags v2.13.2
   Compiling shlex v2.0.1
   Compiling bytes v1.12.1
   Compiling getrandom v0.4.3
   Compiling cc v1.10.0
   Compiling thiserror v2.0.21
   Compiling indexmap v2.14.2
   Compiling log v0.4.34
   Compiling syn v3.0.9
   Compiling syn v2.0.119
   Compiling toml_parser v1.1.5+spec-1.1.0
   Compiling socket2 v0.6.5
   Compiling errno v0.3.14
   Compiling mio v1.2.4
   Compiling signal-hook-registry v1.4.8
   Compiling typenum v1.20.1
   Compiling toml_edit v0.25.17+spec-1.1.0
   Compiling cpufeatures v0.3.1
   Compiling ring v0.17.14
   Compiling futures-core v0.3.34
   Compiling chacha20 v0.10.2
   Compiling getrandom v0.2.17
   Compiling hybrid-array v0.4.15
   Compiling zmij v1.0.23
   Compiling proc-macro-crate v3.5.0
   Compiling cfg_aliases v0.2.2
   Compiling parking v2.2.1
   Compiling slab v0.4.12
   Compiling untrusted v0.9.0
   Compiling zeroize v1.9.1
   Compiling block-buffer v0.12.1
   Compiling crypto-common v0.2.2
   Compiling rustls-pki-types v1.15.1
   Compiling rand v0.10.3
   Compiling hex v0.4.3
   Compiling utf8parse v0.2.2
   Compiling serde_json v1.0.154
   Compiling const-oid v0.10.2
   Compiling endi v1.1.1
   Compiling anstyle-parse v1.0.0
   Compiling event-listener v5.4.2
   Compiling memchr v2.8.3
   Compiling is_terminal_polyfill v1.70.2
   Compiling anstyle-query v1.1.5
   Compiling anstyle v1.0.14
   Compiling rustls v0.23.46
   Compiling smallvec v1.16.4
   Compiling digest v0.11.3
   Compiling parking_lot_core v0.9.12
   Compiling colorchoice v1.0.5
   Compiling itoa v1.0.18
   Compiling anstream v1.0.0
   Compiling sha2 v0.11.1
   Compiling event-listener-strategy v0.5.4
   Compiling quinn-udp v0.5.16
   Compiling serde_spanned v1.1.2
   Compiling scopeguard v1.2.0
   Compiling deranged v0.5.9
   Compiling toml_writer v1.1.3+spec-1.1.0
   Compiling futures-io v0.3.34
   Compiling time-core v0.1.9
   Compiling anyhow v1.0.104
   Compiling strsim v0.11.1
   Compiling powerfmt v0.2.1
   Compiling clap_lex v1.1.1
   Compiling futures-task v0.3.34
   Compiling num-conv v0.2.2
   Compiling subtle v2.6.1
   Compiling fastrand v2.5.0
   Compiling heck v0.5.0
   Compiling futures-lite v2.6.1
   Compiling time v0.3.55
   Compiling clap_builder v4.6.7
   Compiling serde_derive v1.0.229
   Compiling enumflags2_derive v0.7.12
   Compiling tracing-attributes v0.1.31
   Compiling thiserror-impl v2.0.21
   Compiling tokio-macros v2.7.2
   Compiling async-trait v0.1.92
   Compiling futures-macro v0.3.34
   Compiling tokio v1.54.1
   Compiling tracing v0.1.44
   Compiling async-recursion v1.2.0
   Compiling futures-util v0.3.34
   Compiling serde_repr v0.1.21
   Compiling clap_derive v4.6.7
   Compiling cobs v0.3.0
   Compiling gethostname v1.1.0
   Compiling toml v1.1.8+spec-1.1.0
   Compiling lock_api v0.4.14
   Compiling async-broadcast v0.7.2
   Compiling quinn v0.11.12
   Compiling uuid v1.29.0
   Compiling ordered-stream v0.2.0
   Compiling rand_pcg v0.10.2
   Compiling base64 v0.23.1
   Compiling x11rb-protocol v0.13.2
   Compiling tinyvec v1.13.3
   Compiling regex-syntax v0.8.11
   Compiling rustc-hash v2.1.3
   Compiling lru-slab v0.1.3
   Compiling zvariant_utils v4.2.0
   Compiling enumflags2 v0.7.12
   Compiling zcheapstr v1.1.0
   Compiling postcard v1.1.3
   Compiling pem v4.0.0
   Compiling clap v4.6.7
   Compiling parking_lot v0.12.5
   Compiling yasna v0.6.0
   Compiling zvariant_derive v5.15.1
   Compiling regex-automata v0.4.18
   Compiling zvariant v5.15.1
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling lazy_static v1.5.1
   Compiling percent-encoding v2.3.2
   Compiling zbus_names v4.3.5
   Compiling humantime v2.4.0
   Compiling zbus_macros v5.19.0
   Compiling rustls-webpki v0.103.16
   Compiling x11rb v0.13.2
   Compiling rcgen v0.14.10
   Compiling matchers v0.2.0
   Compiling sharded-slab v0.1.7
   Compiling zbus v5.19.0
   Compiling reis v0.7.1
   Compiling tracing-log v0.2.0
   Compiling arboard v3.7.0
   Compiling thread_local v1.1.10
   Compiling nu-ansi-term v0.50.3
   Compiling tracing-subscriber v0.3.23
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling quinn-proto v0.11.19
   Compiling ashpd v0.13.13
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `release` profile [optimized] target(s) in 1m 08s
```

Comando: `cargo test -p clipsync-cli --test roteiro`; início `2026-10-27T13:10:36.541165-03:00`; exit **0**.

stdout:

```text

running 8 tests
test clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off ... ok
test init_creates_profile_files_with_private_modes ... ok
test init_refuses_a_config_with_both_modules_off_and_says_why ... ok
test init_without_a_clipboard_answer_writes_false_and_flags_variant_works ... ok
test mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key ... ok
test pair_declined_on_one_side_saves_nothing_on_either ... ok
test pair_shows_the_same_code_and_saves_permissions_on_both_sides ... ok
test pause_blocks_and_resume_returns_and_state_survives ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.60s

```

stderr:

```text
   Compiling clipsync-proto v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.26s
     Running tests/roteiro.rs (~/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
```

Hashes da base revisada:

```text
44b4db5574dcab4903fec83c18526c942dfd87d0f3f05ee5d7eae0d3f3286750  Cargo.lock
fa8d2f618b98da815eb0ed7a3a492ef74a74751abce4bf95a936ebb6078bdf1f  docs/contrato.md
5be140cb050cd6b56b75d3615af004426842a68d78c928c53eb66518694163d9  crates/clipsync-core/src/agent.rs
bb63efb2bf50ff0f3269a72ee23026de507137a6f683d20071918f8e22c3eaf9  crates/clipsync-core/src/control.rs
061a9fc13563c3f65c9bab0320234276040195ce4da352d9ee40a53bfbb58ecd  crates/clipsync-core/src/pairing.rs
efb77392e4f269ddbf8c160439e8362cea1ac073a93b97b5454b1a1e79d85143  crates/clipsync-core/src/profile.rs
818cdd5148036aa121b4186ab204473abb36e4f1a991928e290af79ee1f17ca2  crates/clipsync-cli/src/pair.rs
339a5e258fe03eaa29a511b0f22fd5fe7f31cdd55bd9bcfcae6913863afe70f6  crates/clipsync-cli/src/text.rs
9e19030ee97cbadb75d624a60c23597f8eb554cd65cf22fd3d832ec8627bb717  crates/clipsync-proto/src/wire.rs
```

Evidência completa: `~/.cache/clipsync-verdict/argus-c1ed65d-final-68hp_drk` — meta.json, results.json (stdout/stderr/código/duração de cada comando), before.json, after.json, release-binary.json e toolchain.txt.

### S5 · Windows e release · b66df03 · Falha

Revisão de Argus em 2026-10-27, iniciada às 16:39:50 -03:00. Contrato v1.12, §9 S5 e §11. Base: `b66df034996e20e4d1ce2a369a8dcb629d098c41`, HEAD no início da revisão, exportado com `git archive` para `~/.cache/clipsync-verdict/src`. O checkout estava limpo na exportação; avançou durante a revisão. Os **103 arquivos do export permaneceram idênticos** entre início e fim. Nenhuma fonte foi editada.

**Resultado histórico:** gates automáticos verdes, mas três divergências de contrato encontradas nesta base, descritas abaixo. O Maestro informou correções e uma decisão de contrato durante a consolidação; o adendo de 707c780 registra a reavaliação. Não estender este resultado aos commits posteriores.

Ambiente dos comandos: `PATH="$HOME/.cargo/bin:$PATH"`, `CARGO_TARGET_DIR="$HOME/.cache/clipsync-verdict/target"`, `TMPDIR="$HOME/.cache/clipsync-verdict/tmp"`; cwd no export. Fonte e target fora de `/tmp`. O compilador falso e os perfis `/tmp/cs-a`/`cs-b` usam os caminhos explicitamente pedidos pelo Maestro.

#### 1–4 e 7 · comandos e saídas reais resumidas

```text
$ cargo fmt --all -- --check
[sem saída]
[exit 0; 0.143 s]
```

```text
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.51s
[exit 0; 2.568 s]
```

```text
$ cargo test --workspace --locked
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.22s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.17s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.22s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test portal_loopback ... ignored, requer sessão gráfica
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
[soma dos resultados acima: 129 aprovados; 0 falhos; 1 ignorado de portal; roteiro da CLI 8/8]
[exit 0; 71.248 s]
```

```text
$ CC_x86_64_pc_windows_gnu=/tmp/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo check --workspace --locked --target x86_64-pc-windows-gnu
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.56s
[exit 0; 10.608 s]
```

```text
$ CC_x86_64_pc_windows_gnu=/tmp/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-gnu -- -D warnings
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.46s
[exit 0; 2.516 s]
```

```text
$ cargo build --release -p clipsync-cli --locked
   Compiling clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `release` profile [optimized] target(s) in 1m 43s
[exit 0; 103.499 s]
```

O stub de CC fornecido apenas cria arquivos de objeto vazios. Check/clippy Windows verificam Rust e cfg, sem linkar, executar testes Windows ou validar a criptografia C do ring. Não produzem um `.exe` utilizável.

Binário Linux em `~/.cache/clipsync-verdict/target/release/clipsync`: **8.754.744 bytes (8,35 MiB)**. `file` identificou ELF x86-64 PIE, dinamicamente ligado, **stripped**; `--version` imprimiu `clipsync 0.1.0`. SHA-256: `24711e20543df5b37e1d0bcce2911ad33a6ac60186705f1d24b41499b9c562b2`. Perfil lido do Cargo.toml: `lto = true`, `codegen-units = 1`, `strip = true`, `opt-level = 3`. Este build local em Ubuntu 26.04 não prova a glibc mínima do artefato que será gerado em ubuntu-24.04.

#### 5 · grafos por plataforma

```text
$ cargo tree --target x86_64-pc-windows-gnu -e normal | grep -c -E "ashpd|reis|x11rb|rustix"
0
[cargo exit 0; grep exit 1, esperado: nenhuma ocorrência]
$ cargo tree --target x86_64-unknown-linux-gnu -e normal | grep -c -E "x11rb"
3
[cargo exit 0; grep exit 0]
$ grep -rn unsafe_code crates
[sem saída; exit 1: nenhum crate afrouxa o lint]
```

#### 6 · release.yml e ci.yml

YAML dos dois arquivos válido. Leitura e execução local do trecho que confere a tag:

```text
release_actions: actions/checkout@v7, actions/upload-artifact@v4, actions/download-artifact@v4
only_actions: true
no_cache_actions: true
linux_runner: ubuntu-24.04
windows_target: x86_64-pc-windows-msvc
windows_RUSTFLAGS: ${{ matrix.os == 'windows-latest' && '-C target-feature=+crt-static' || '' }}
release_build: cargo build --release -p clipsync-cli --locked --target ${{ matrix.target }}
TAG=v0.1.0: exit 0; sem saída
TAG=v0.0.0-argus-invalid: exit 1
tag v0.0.0-argus-invalid difere da versão 0.1.0 do Cargo.toml
$ grep -c rust-cache .github/workflows/release.yml
0
[exit 1, esperado: nenhuma ocorrência]
windows_ci_runner: windows-latest
windows_ci_commands:
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo build --release -p clipsync-cli --locked
  cargo test --workspace --locked
```

Referências no export: `release.yml:21` fixa Ubuntu, `:43` confere versão, `:59` define CRT estática, `:62`/`:74` empacotam binário + README, `:106` produz SHA256SUMS; `ci.yml:41` define o job Windows. Trigger `v*`, nenhuma action de cache no release, e todos os `uses` são `actions/*`. Conferência estática aprovada; não houve push, tag ou publicação, nem consulta conclusiva de CI deste hash.

#### 8 · roteiro do README com o binário release

Dois perfis novos `/tmp/cs-a` e `/tmp/cs-b`, portas **47900/47901**, `--fake-adapters`. Pareamento real por loopback; comparei os códigos antes de confirmar `s` nos dois lados. Foram usados exclusivamente textos sintéticos. Abaixo, saídas reais resumidas dos comandos (o executável em todos eles foi o release acima):

```text
$ clipsync --version
clipsync 0.1.0
[exit 0]
```

```text
$ clipsync init --profile /tmp/cs-a --name notebook-a --modules input,clipboard --port 47900 --fake-adapters
Perfil criado em /tmp/cs-a
  dispositivo: notebook-a
  id: afb6-dd35-c0e8-253e
  escuta: 0.0.0.0:47900
  input: ligado · clipboard: ligado
  adaptadores: falsos (apenas teste)
Nada foi transmitido.
Próximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.
[exit 0]
```

```text
$ clipsync init --profile /tmp/cs-b --name notebook-b --modules input,clipboard --port 47901 --fake-adapters
Perfil criado em /tmp/cs-b
  dispositivo: notebook-b
  id: e4a9-1244-e066-58d2
  escuta: 0.0.0.0:47901
  input: ligado · clipboard: ligado
  adaptadores: falsos (apenas teste)
Nada foi transmitido.
Próximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.
[exit 0]
```

```text
$ clipsync pair host --profile /tmp/cs-a --port 0 --timeout 45s --allow clipboard-in,clipboard-out,input-in
Aguardando em 0.0.0.0:48064 (expira em 45 s)
Par: notebook-b (id e4a9-1244-e066-58d2) · módulos: clipboard, input
Código: 377 566
O código acima é o mesmo no outro computador? [s/N] Pareado com notebook-b (id e4a9-1244-e066-58d2). Permissões concedidas por você: clipboard-in, clipboard-out, input-in
[exit 0]
```

```text
$ clipsync pair join --profile /tmp/cs-b 127.0.0.1:48064 --allow clipboard-in,clipboard-out
Par: notebook-a (id afb6-dd35-c0e8-253e) · módulos: clipboard, input
Código: 377 566
O código acima é o mesmo no outro computador? [s/N] Pareado com notebook-a (id afb6-dd35-c0e8-253e). Permissões concedidas por você: clipboard-in, clipboard-out
[exit 0]
```

```text
$ clipsync status --profile /tmp/cs-a
notebook-a · id afb6-dd35-c0e8-253e · escuta 0.0.0.0:47900 · ativo
Pares:
  notebook-b (e4a9-1244-e066-58d2) · conectado · 127.0.0.1:47901 · módulos do par: clipboard, input · permissões: clipboard-in, clipboard-out, input-in
Módulos:
  clipboard: ligado · adaptador fake · último evento: nenhum · último bloqueio: nenhum
  input: ligado · adaptador fake+fake · último evento: nenhum · último bloqueio: nenhum
[exit 0]
```

```text
$ clipsync status --profile /tmp/cs-b
notebook-b · id e4a9-1244-e066-58d2 · escuta 0.0.0.0:47901 · ativo
Pares:
  notebook-a (afb6-dd35-c0e8-253e) · conectado · 127.0.0.1:47900 · módulos do par: clipboard, input · permissões: clipboard-in, clipboard-out
Módulos:
  clipboard: ligado · adaptador fake · último evento: nenhum · último bloqueio: nenhum
  input: ligado · adaptador fake+fake · último evento: nenhum · último bloqueio: nenhum
[exit 0]
```

```text
$ clipsync send --stdin --profile /tmp/cs-a
[stdin sintético: olá ção]
notebook-b: aplicado
[exit 0]
```

```text
$ clipsync inbox --profile /tmp/cs-b
57adeaf7 · de notebook-a · 19:44:46 UTC · bytes=10 · aplicado · olá ção
[exit 0]
```

```text
$ clipsync pause --profile /tmp/cs-b
pausado: nada é enviado nem aplicado até `clipsync resume`
[exit 0]
```

```text
$ clipsync send --stdin --profile /tmp/cs-a
[stdin sintético: argus-durante-pausa]
notebook-b: recusado: pausado
[exit 3]
```

```text
$ clipsync resume --profile /tmp/cs-b
retomado: o que foi copiado durante a pausa não será enviado
[exit 0]
```

```text
$ clipsync inbox --profile /tmp/cs-b
57adeaf7 · de notebook-a · 19:44:46 UTC · bytes=10 · aplicado · olá ção
[exit 0]
```

```text
$ clipsync probe input --loopback --to notebook-b --seconds 10 --profile /tmp/cs-a
erro: a barreira não foi ativada no tempo do teste
[exit 1]
```

```text
$ clipsync diagnose --profile /tmp/cs-a
[ok] sistema · sistema operacional: Ubuntu 26.04.1 LTS
[ok] sistema · sessão gráfica: wayland · ubuntu:GNOME
[ok] sistema · DISPLAY / XWayland: DISPLAY=:0; servidor X11 respondendo no socket
[ok] sistema · Input Leap: em execução (Flatpak): só detectado; o ClipSync não o altera e as duas ferramentas podem competir pelo mesmo mouse e teclado
[aviso] sistema · Barrier: não detectado
[ok] clipboard · ponte XWayland (DISPLAY): DISPLAY definido; conexão X11 aberta
[ok] clipboard · XFixes: SelectionNotify do CLIPBOARD disponível
[aviso] clipboard · data-control (Wayland): ausente (zwlr/ext_data_control): observação só pela ponte XWayland
[ok] input · portal InputCapture: versão 1
[ok] input · portal RemoteDesktop: versão 2; dispositivos BitFlags<DeviceType>(0b111, Keyboard | Pointer | Touchscreen)
[ok] input · monitores: eDP-1
[aviso] input · Input Leap / Barrier: em execução: input-leapc; podem disputar a captura
[exit 0]
```

```text
$ clipsync run --profile /tmp/cs-b
Agente de notebook-b no ar, escutando em 0.0.0.0:47901 · módulos: clipboard, input. Ctrl-C encerra.
agente encerrado
[exit 0]
```

```text
$ clipsync status --profile /tmp/cs-a
notebook-a · id afb6-dd35-c0e8-253e · escuta 0.0.0.0:47900 · ativo
Pares:
  notebook-b (e4a9-1244-e066-58d2) · conectado · 127.0.0.1:47901 · módulos do par: input · permissões: clipboard-in, clipboard-out, input-in
Módulos:
  clipboard: ligado · adaptador fake · último evento: nenhum · último bloqueio: notebook-b: recusado
  input: ligado · adaptador fake+fake · último evento: nenhum · último bloqueio: a barreira não foi ativada no tempo do teste
[exit 0]
```

```text
$ clipsync send --stdin --profile /tmp/cs-a
[stdin sintético: argus-modulo-desligado]
notebook-b: recusado: módulo clipboard desligado em notebook-b
[exit 3]
```

```text
$ clipsync run --profile /tmp/cs-b
Agente de notebook-b no ar, escutando em 0.0.0.0:47901 · módulos: input. Ctrl-C encerra.
agente encerrado
[exit 0]
```

```text
$ clipsync run --profile /tmp/cs-a
Agente de notebook-a no ar, escutando em 0.0.0.0:47900 · módulos: clipboard, input. Ctrl-C encerra.
agente encerrado
[exit 0]
```

Passaram inicialização, pareamento com código igual, ambos os agentes conectados, envio de UTF-8 (10 bytes incluindo a quebra de linha usada pelo executor), inbox, pausa com recusa de envio, retomada sem reaplicar o envio recusado, diagnóstico e recusa com clipboard desligado no destino. Os agentes foram encerrados com saída 0.

**Roteiro parcialmente executado:** `probe clipboard --watch/--write` e `probe input --capture/--emulate` usam os adaptadores reais, mesmo nos perfis fake, e dependem de interação gráfica; ficaram pendentes. O loopback com o fake expirou sem ativação da barreira (exit 1); não valida o item gráfico nem demonstra regressão nele. A pergunta sobre disponibilidade do usuário para os passos gráficos não recebeu resposta durante esta execução. O teste automático `loopback_counts_every_event_and_orders_percentiles` passou no workspace, sem substituir o teste gráfico. Não declarar o roteiro completo aprovado.

Após o roteiro, mover os perfis por `rename` entre tmpfs e home retornou EXDEV no executor de evidência. Isso ocorreu depois de todos os processos encerrarem com exit 0; a limpeza foi concluída com `shutil.move`, sem repetir nem alterar resultados da aplicação. Perfis arquivados em `roteiro-profiles` junto à evidência, sem publicar chaves; `/tmp/cs-a` e `/tmp/cs-b` liberados.

#### Divergências encontradas em b66df03

1. **S5-1 · P2 · CLI aceita configuração sem módulo disponível.** `crates/clipsync-cli/src/init.rs:65` mantém `input = true` quando fornecido por `--modules input` fora do Linux; apenas o ramo interativo usa `cfg!(target_os = "linux")`. `crates/clipsync-cli/src/agent.rs:20` omite input fora do Linux e `:37` chama o core mesmo com o vetor vazio, sem sair 1 como exigido em §11.8. O teste puro por plataforma exigido em §9 S5.7 está ausente. Reprodução auxiliar no Linux, fora dos fontes, chamou o core real com config input=true e vetor vazio: `configured_input=true provided_modules=0 control_responded=true enabled_modules=Some(0) agent_stayed_alive=true shutdown_result=Ok(())` (1 teste aprovado que constata o defeito, exit 0). É leitura do caminho não-Linux somada à reprodução do core, não execução de um binário Windows.
2. **S5-2 · P2 na versão do contrato revisada · probe indisponível sai 3, embora §11.7 diga 1.** `crates/clipsync-cli/src/main.rs:147` converte `Unavailable` em `Fail::refused`, que define 3 em `fail.rs:18`; `crates/clipsync-input/src/probe.rs:46`/`:51` retornam esse erro fora do Linux. **Encerrado por decisão explícita do Maestro durante a revisão:** 3 = recusado é o comportamento desejado, consistente com probe clipboard fora do Linux; Keel atualizará o contrato. Não é bloqueio do adendo.
3. **S5-3 · P2 · README não identifica o clipboard Windows como experimental.** `README.md:33` diz `sim (só texto)`, sem polling de 250 ms e sem aviso de teste em máquina real pendente, exigidos em §11.6/§11.11. O diagnóstico já contém o aviso em `crates/clipsync-clipboard/src/diag.rs:54`. Separadamente, `grep -n experimental crates/clipsync-clipboard/src/msg.rs` retornou sem saída/exit 1: a localização literal prevista em §9 S5.5 diverge, embora o aviso exista em diag.rs.

Limites: execução e linkedição Windows reais, testes de named pipe/mesh no runner Windows, artefatos publicados e verificação dos SHA256SUMS baixados permanecem sem prova nesta revisão local. O `.exe` real depende do CI e depois da prova M em máquina Windows. Os passos gráficos locais seguem pendentes.

Evidência integral local: `~/.cache/clipsync-verdict/argus-s5-3hmvsylg` (`gates.json`, `trees.json`, `static.json`, `binary.json`, `roteiro-release.json`, reproduções e hashes before/after). SHA-256 do archive: `093886f0e0d2e4a1f75d9686eb072b8ccb91e8b22f23290e56c482c4c703bde2`.

### S5 · adendo · 707c780 · Passa

Argus, 2026-10-27. **Passa no escopo solicitado para o adendo:** fmt, clippy Linux/Windows com stub, testes do workspace e leitura das recusas/aviso experimental. Novo export limpo de `707c780d8ba123dd216a5efb04614d8b53bbcfb7` via `git archive`, em `~/.cache/clipsync-verdict/src`; target em `~/.cache/clipsync-verdict/target`, PATH com `$HOME/.cargo/bin`. Os 103 arquivos do export não mudaram durante os gates. Nenhuma fonte foi editada. Esta reavaliação não prova execução no Windows.

Saídas reais resumidas:

```text
$ cargo fmt --all -- --check
[sem saída]
[exit 0; 0.156 s]
```

```text
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.87s
[exit 0; 2.936 s]
```

```text
$ CC_x86_64_pc_windows_gnu=/tmp/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-gnu -- -D warnings
    Checking clipsync-cli v0.1.0 (~/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.30s
[exit 0; 2.370 s]
```

```text
$ cargo test --workspace --locked
test agent::tests::plan_for_this_platform_matches_its_support ... ok
test agent::tests::plan_with_input_support_builds_what_is_enabled ... ok
test agent::tests::plan_without_input_support_and_clipboard_off_is_an_error ... ok
test agent::tests::plan_without_input_support_skips_input_and_keeps_clipboard ... ok
test init::tests::init_with_input_support_accepts_input_alone_but_not_nothing ... ok
test init::tests::init_without_input_support_accepts_clipboard ... ok
test init::tests::init_without_input_support_refuses_clipboard_off ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.87s
[demais suítes: 118 aprovados; 0 falhos; 1 ignorado]
test portal_loopback ... ignored, requer sessão gráfica
[soma de todas as suítes: 136 aprovados; 0 falhos; 1 ignorado; roteiro CLI 8/8]
[exit 0; 72.203 s]
```

```text
$ grep -n experimental README.md
32:3. Em máquina com mais de um usuário (compartilhada, RDP), o Windows ainda é experimental. Pare o agente com Ctrl+C; se fechar a janela, rode `clipsync run` de novo antes de usar `clipsync send`.
35:6. O clipboard do Windows é experimental: polling de 250 ms, só texto, ainda não testado em máquina real.
43:| Teclado e mouse | experimental | ainda não disponível |
[exit 0]
```

```text
$ grep -n -E 'RUN_NO_MODULE|INIT_NO_USABLE_MODULE|check_usable\(modules|let plan = plan' crates/clipsync-cli/src/agent.rs crates/clipsync-cli/src/init.rs
crates/clipsync-cli/src/agent.rs:29:        Err(Fail::error(msg::RUN_NO_MODULE))
crates/clipsync-cli/src/agent.rs:39:    let plan = plan(cfg.modules, INPUT_SUPPORTED)?;
crates/clipsync-cli/src/agent.rs:86:        assert_eq!((e.code, e.message.as_str()), (1, msg::RUN_NO_MODULE));
crates/clipsync-cli/src/init.rs:41:        Err(Fail::refused(msg::INIT_NO_USABLE_MODULE))
crates/clipsync-cli/src/init.rs:82:    check_usable(modules, INPUT_SUPPORTED)?;
crates/clipsync-cli/src/init.rs:116:                (3, msg::INIT_NO_USABLE_MODULE)
[exit 0]
```

- **S5-1:** `run` calcula o plano antes de construir módulos; input indisponível com clipboard desligado resulta em erro/1. Com clipboard ligado, omite input e imprime o aviso. `init` verifica disponibilidade antes de gravar o perfil e recusa configuração sem módulo utilizável com código 3. Funções puras recebem `input_supported`; os sete testes novos passaram. O default interativo de input é falso fora do Linux.
- **S5-2:** encerrado por decisão explícita do Maestro: probe indisponível retorna **3 = recusado**, igual ao probe clipboard fora do Linux. O contrato exportado ainda diz 1 em `docs/contrato.md:835`; Keel atualizará o texto. Não exigir alteração de código para voltar a 1.
- **S5-3:** aviso explícito em `README.md:35`: experimental, polling de 250 ms, só texto e ausência de teste em máquina real, junto dos seis avisos de segurança. A tabela `README.md:42` ainda resume `sim (só texto)`; harmonizar sua redação com o aviso. O diagnóstico continua com o texto em `diag.rs`, divergindo do grep literal em `msg.rs`. Essas diferenças editoriais ficam registradas; o aviso e os gates pedidos neste adendo passaram.

**Roteiro reaproveitado:** comparação SHA-256 confirma **zero mudanças em `crates/clipsync-core/`**, em Cargo.toml/Cargo.lock e no roteiro da CLI entre b66df03 e 707c780. Por instrução do Maestro, não repeti o roteiro release: cite-se a evidência de b66df03 acima para pareamento, conexão, send/inbox, pausa/retomada, diagnóstico e módulo desligado. O roteiro automático **8/8** foi reexecutado em 707c780 dentro do workspace. Os probes gráficos e o loopback real continuam pendentes; a expiração do loopback fake não foi tratada como aprovação gráfica.

O release de **8.754.744 bytes** pertence a **b66df03**, não a um rebuild de 707c780. `.exe`, linkedição/CRT real, named pipe e testes Windows dependem do CI; prova M em máquina Windows e verificação dos artefatos publicados seguem pendentes. Nenhum push/tag/release foi feito por Argus.

Evidência integral: `~/.cache/clipsync-verdict/argus-s5-adendo-p2qa07qp` (`meta.json`, `gates.json`, `reads.json`, hashes before/after). SHA-256 do archive: `8cd4518ad46272f92250ac4b070cbe0880e378ac75c8b073e058c19fee644119`.

### S6 · parcial · 5a55688

**Passa no escopo solicitado:** §12.9 item 1 (a–g), item 4 (clipboard, autostart e clippy do app no CI informado) e somente os greps do item 5. Verificado por Argus em **2026-10-28**, com contrato v1.15. **Não encerra S6:** telas, temas, fontes, rede no portal, cenários visuais e app real (item 5 completo e item 6) não foram avaliados nesta rodada; itens 2, 3 e 7 também estão fora deste veredito.

Base local: `5a55688d19c9c55f93a87daf1c00caee819327f4`. O checkout compartilhado tinha alterações de outros agentes; os testes e greps abaixo rodaram no conteúdo de `git archive 5a55688`, exportado para `~/.cache/argus-s6-5a55688-tufou2ei/src`. Os **205 arquivos** do export, inclusive `Cargo.lock`, mantiveram os mesmos SHA-256 antes e depois. Archive SHA-256: `af2e1fcad93afa409c5534fba54bcd56f782807ca5df3057bc03880283dd6206`. Ambiente: Linux x86_64, rustc/cargo **1.99.1**; cache Cargo reaproveitado, sem alegação de build frio.

Preparação aplicada antes dos quatro comandos, na raiz do export:

```sh
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target
```

Nenhum servidor foi iniciado, nenhum código-fonte foi editado e nenhum comando Cargo com `--workspace` foi executado localmente.

#### 1 · Rivet, proto e core — Passa

| Comando executado | Saída resumida (soma das suítes) | Exit |
| --- | --- | --- |
| `cargo test -p clipsync-proto` | 48 passed; 0 failed; 0 ignored; `tests/s6.rs`: 6/6 | 0 |
| `cargo test -p clipsync-core` | 44 passed; 0 failed; 0 ignored; `tests/mesh.rs`: 11/11; `tests/profile.rs`: 14/14 | 0 |

As asserções foram lidas no export, além da execução. Cada teste citado abaixo saiu `... ok`:

| Item | Veredito | Teste e evidência |
| --- | --- | --- |
| (a) Round-trip postcard | **Passa** | `crates/clipsync-proto/tests/s6.rs:106`, `zones_layout_offer_and_enter_at_roundtrip_through_postcard`: codifica/decodifica o frame e compara zonas, oferta e os campos de `EnterAt`. |
| (b) Validação | **Passa** | `crates/clipsync-proto/tests/s6.rs:157`, `validate_refuses_17_zones_nan_scale_pos_out_of_range_and_empty_segment`: recusa 17 zonas, escala NaN, posição 1.5 e segmentos com `from == to` ou `from > to`; 16 zonas e posição 1.0 são aceitas. |
| (c) `authorize_*` | **Passa** | `crates/clipsync-proto/tests/s6.rs:230`, `authorize_table_for_zones_enter_at_and_layout_offer`: `Zones` exige `input_in` na saída e passa em pausa; `EnterAt`/`LayoutOffer` exigem `input_in` na entrada; pausa bloqueia `EnterAt`. Também cobre módulos desligados e a entrada de `Zones` em pausa. |
| (d) Versão 1 | **Passa** | `crates/clipsync-proto/tests/s6.rs:287`, `memhub_refuses_zones_to_a_version_1_peer_and_nothing_leaves`: `RemoteTooOld`, `sent_count == 0` e receptor vazio. `crates/clipsync-core/tests/mesh.rs:649`, `mesh_never_sends_a_protocol_2_message_to_a_version_1_peer`: repete em QUIC real com Hello v1, sem abrir stream uni durante a janela de 1 s; mensagem antiga ainda chega. Ambos cobrem também `LayoutOffer`/`EnterAt`. |
| (e) Hello v2 compatível | **Passa** | `crates/clipsync-core/tests/mesh.rs:621`, `hello_v2_passes_the_0_1_0_compatibility_rule`: Hello 2/min 1 satisfaz a regra do 0.1.0 (`version >= 1 && 1 >= min_version`); o core aceita Hello 1/min 1 e recusa 3/min 3. Prova da regra, sem execução de binário antigo. |
| (f) Identidade de `init_with` | **Passa** | `crates/clipsync-core/tests/profile.rs:45`, `profile_init_with_writes_the_identity_it_was_given_and_nothing_on_a_bad_name`: id retornado e id recarregado do disco iguais ao id fornecido; nome inválido não inicializa o perfil. |
| (g) `recent_blocks` | **Passa** | `crates/clipsync-core/tests/mesh.rs:388`, `mesh_inbound_refusals_are_recorded_newest_first_and_capped`: envia 23 ofertas recusadas, aguarda 20 registros e verifica timestamps em ordem decrescente, origem/motivo e ausência de conteúdo. Conferido `MAX_RECENT_BLOCKS = 20` em `crates/clipsync-proto/src/limits.rs:22` e `push_front` + `truncate(MAX_RECENT_BLOCKS)` em `crates/clipsync-core/src/transport.rs:391`. |

#### 4 · Flux — Passa nos três itens pedidos

**Clipboard — Passa.** `cargo test -p clipsync-clipboard` saiu **0**, com **25 passed; 0 failed; 0 ignored** (7 unitários, 17 em `module_mem`, 1 smoke). Em `crates/clipsync-clipboard/tests/module_mem.rs`, todos estes testes saíram `... ok`:

| Requisito | Veredito | Teste e observação |
| --- | --- | --- |
| `ClipApply` | **Passa** | `m_clipapply_writes_an_inbox_item_without_a_new_offer` (`:473`): fake recebe uma escrita, estado vira Applied, `sent_count` ao terceiro par permanece 0 e segunda aplicação dá `WRONG_STATE`. `o_clipapply_refuses_an_expired_item_because_the_ttl_dropped_the_text` (`:543`): item expirado dá `WRONG_STATE`, sem escrita. `n_clipapply_works_while_paused_and_a_stale_item_qualifies` (`:509`) também passou. |
| `ClipDiscard` | **Passa** | `p_clipdiscard_removes_the_item_and_its_text` (`:559`): caixa fica vazia; reaplicar ou descartar novamente dá `NO_ITEM`; fake continua vazio. Leitura complementar de `src/module.rs:742`: `self.inbox.remove(index)` remove a entrada que possui texto e preview. |
| `ClipSendCurrent` | **Passa** | `q_clipsendcurrent_sends_the_clipboard_to_clipboard_out_peers` (`:582`): conteúdo atual chega ao fake autorizado; broadcast lista só o par com `clipboard_out`; envio explícito ao par sem permissão retorna NotAuthorized; clipboard vazio retorna `EMPTY_CLIPBOARD`. |
| TTL da caixa | **Passa** | `k_ttl_counts_from_local_receipt_and_drops_the_text` (`:397`): TTL zero expira na chegada; após 1 s, item com TTL de 800 ms está Expired e sem preview. O teste `o_clipapply_...` comprova que já não pode ser aplicado. Leitura complementar de `src/module.rs:590`: expiração substitui preview por string vazia e texto por `None`. |
| Debug sem conteúdo | **Passa** | `i_debug_never_shows_content` (`:320`): Debug de `ClipSnapshot` e `InboxItem` não contém o segredo sintético. Em proto, `clip_offer_debug_hides_text`, `inbox_item_debug_hides_preview` e `control_request_debug_hides_clip_send_text` também passaram. |

**Autostart — Passa.** `cargo test -p clipsync-ui --test autostart` saiu **0**, com **4 passed; 0 failed; 0 ignored**. Em `crates/clipsync-ui/tests/autostart.rs`:

- `appimage_wins_over_the_executable_when_set_and_non_empty`: escolhe `$APPIMAGE` não vazio; ausente/vazio usa o executável.
- `exec_line_quotes_the_program_and_adds_hidden`: `Exec="/opt/ClipSync.AppImage" --hidden`, inclusive escape de espaços, `$` e `%`.
- `enable_writes_the_desktop_file_and_disable_removes_it`: cria `.desktop` em diretório temporário com esse Exec, sem arquivo temporário restante; desativar remove a entrada.
- `set_and_is_enabled_round_trip_through_the_options_dir`: consulta/ativação/desativação coerentes via diretório das opções e linha Exec igual à detectada.

**Clippy do app nos dois runners — Passa na run solicitada.** `gh run view 40625572835 --log` saiu **0** (4.368 linhas). `gh run view 40625572835 --json headSha,conclusion,jobs,url,name` saiu **0**: commit `68cd28a95434fd2491e635e9828c58b8d26cb5fd`, conclusão geral e os dois jobs/steps de clippy `success`. Evidência da [run 40625572835](https://github.com/devkappiushon/clipsync/actions/runs/40625572835), nas etapas `cargo clippy`:

| Runner/job | Comando no log | Evidência de inclusão do app e término |
| --- | --- | --- |
| Windows Server 2025 / `windows build + test` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `03:20:45Z Compiling clipsync-app v0.1.0 (D:\a\clipsync\clipsync\crates\clipsync-app)`; `03:21:43Z Finished dev profile ... in 6m 07s`; step `success`. |
| Ubuntu / `fmt + clippy + test` | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `03:18:50Z Compiling clipsync-app v0.1.0 (/home/runner/work/clipsync/clipsync/crates/clipsync-app)`; `03:19:00Z Finished dev profile ... in 2m 38s`; step `success`. |

O `--workspace` do CI inclui o app, com os mesmos `--all-targets --locked -- -D warnings` exigidos pelo critério. Essa evidência remota pertence a **68cd28a**, não a uma nova run de 5a55688. `git diff 68cd28a 5a55688 -- .github/workflows/ci.yml crates/clipsync-app/Cargo.toml crates/clipsync-app/src Cargo.toml` saiu **0**, sem diferenças nesses caminhos.

#### 5 · Pixel, somente greps — Passa

Comandos executados na raiz do export de 5a55688; exit **1** de grep/rg abaixo significa nenhuma correspondência, sem stderr:

| Verificação | Comando | Saída / exit | Veredito |
| --- | --- | --- | --- |
| `innerHTML` e `style=` | `grep -rnE 'innerHTML\|style=' crates/clipsync-app/ui` | sem saída; 1 | **Passa** |
| Travessão em texto de interface | `grep -rn '—' crates/clipsync-app/ui` | sem saída em toda a árvore; 1 | **Passa** |
| Inventário de scripts | `rg -n -i '<script\b' crates/clipsync-app/ui` | 4 tags, todas com `src`; 0 | **Passa** |
| Script sem `src` | `rg -n -U -P -i '<script\b(?![^>]*\bsrc\s*=)[^>]*>' crates/clipsync-app/ui` | sem saída; 1 | **Passa** |

As quatro tags são `componentes.html:10` (`js/tema-inicial.js`), `componentes.html:19` (`js/galeria.js`), `index.html:11` (`js/tema-inicial.js`) e `index.html:72` (`js/app.js`). Não há aprovação visual implícita nesses greps.

Evidência integral local: `~/.cache/argus-s6-5a55688-tufou2ei/`, com `logs/{proto,core,clipboard,autostart,ci}.log`, `logs/ci.json`, `results.json`, `greps.json`, `meta.json` e inventários SHA-256 `before.json`/`after.json`. **121 testes aprovados, zero falhos, zero ignorados; nenhum achado bloqueante no recorte solicitado.**

### S6 · critérios 2 a 4 · 415caf2

**Critério 2: Passa. Critério 3: Passa nos testes automáticos, mas não passa integralmente enquanto faltar a prova M com `layout.toml`. Critério 4: Passa nos testes locais e no último CI verde disponível, identificado abaixo; o CI de 415caf2 ainda estava em execução na consulta.** Critérios 5 (telas) e 6 (app real) ficam fora desta revisão. O critério 1 permanece na revisão anterior, commit c74f1c0.

Argus, **2026-10-28**, contrato **v1.17**, §12.9. Base exata: `415caf2d8fa235fffc669ce7b9552f24a7ab413a`, via `git archive 415caf2`, em `~/.cache/argus-s6-415caf2-wzp4if22/src`. Os **269 arquivos** do export, inclusive `Cargo.lock`, mantiveram os SHA-256 antes/depois; alterações concorrentes da main não entraram na revisão. SHA-256 do archive: `16b1b15b5896e99be5090cdcf65a69e666fa9080303c37ff742fdd922da4eca1`. Ambiente: Linux x86_64, rustc/cargo **1.99.1**, cache Cargo reaproveitado. Preparação aplicada na raiz do export:

```sh
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=~/.cache/clipsync-verdict/target
```

Não houve comando Cargo local com `--workspace`, servidor iniciado ou edição de código-fonte. As suítes usam perfis temporários, agentes reais em `127.0.0.1` e adaptadores fake; testes específicos de recusa de controle usam `FakeAgent`. Leitura de `crates/clipsync-ui/tests/common/mod.rs` confirmou essas condições.

#### Comandos e repetição das suítes

Todos os comandos locais abaixo saíram **0**. `--no-fail-fast` apenas mantém as outras suítes em execução se alguma falhar; não altera as asserções. Cada rodada completa de UI executou todos os testes habilitados nesta plataforma. Além disso, os comandos específicos de layout e autostart foram executados três vezes cada.

| Comando | Rodadas | Saída resumida por rodada |
| --- | --- | --- |
| `cargo test -p clipsync-ui --no-fail-fast` | 1, 2, 3 | **89 passed; 0 failed; 0 ignored**, nas três; durações totais 96,233 s / 46,615 s / 62,504 s, incluindo compilação/espera |
| `cargo test -p clipsync-input` | 1 | **64 passed; 0 failed; 1 ignored** (`portal_loopback`) |
| `cargo test -p clipsync-ui --test layout` | 1, 2, 3 | **13 passed; 0 failed; 0 ignored**, nas três |
| `cargo test -p clipsync-clipboard` | 1 | **33 passed; 0 failed; 1 ignored** (`platform_adapter_round_trip`, requer sessão gráfica) |
| `cargo test -p clipsync-ui --test autostart` | 1, 2, 3 | **7 passed; 0 failed; 0 ignored**, nas três |
| `cargo test -p clipsync-core --test mesh a_revoked_peer_removes_the_revoker_itself -- --exact` | 1 | **1 passed; 0 failed; 0 ignored; 14 filtered out**; complemento da revogação offline do fluxo 2 |

Distribuição do `clipsync-ui`, igual nas três rodadas: unitários **42**; `agent` **5**; `autostart` **7**; `clipboard` **2**; `devices` **4**; `diag` **1**; `events` **1**; `fixtures` **1**; `layout` **13**; `onboarding` **6**; `pairing` **2**; `prefs` **2**; `seguranca_s6` **3**. `layout_windows` e doc-tests executaram **0** testes; o primeiro é excluído por `cfg` no Linux e não foi tratado como prova Windows.

**Nenhuma falha isolada se reproduziu:** não houve teste com resultado FAILED nem panic nas três rodadas completas ou nas repetições específicas. Isso registra a amostra observada, sem afirmar ausência definitiva de flake. Total local, contando repetições e o complemento do core: **425 execuções aprovadas, zero falhas e dois testes gráficos ignorados**.

#### 2 · Rivet, clipsync-ui — Passa

Cada teste de UI citado saiu `... ok` nas três rodadas completas. Os caminhos desta tabela são relativos a `crates/clipsync-ui/`, salvo indicação do core.

| Fluxo | Veredito | Teste que prova o requisito e asserções conferidas |
| --- | --- | --- |
| (1) Onboarding | **Passa** | `tests/onboarding.rs:24`, `create_writes_the_identity_shown_by_start_and_starts_the_agent`: impressão digital do start igual à do create e do agente. `:70`, `an_existing_profile_needs_confirmation_and_recreate_changes_the_identity`: perfil existente sem confirmação dá `needs_confirmation` e mantém a identidade; recriação troca a identidade e apaga os pareamentos antigos. |
| (2) Dispositivos | **Passa**, com complemento do core para offline | `tests/devices.rs:52`, `perms_need_full_trust_for_input_and_revoke_needs_the_typed_name`: sem confirmação de `input_in` dá `needs_confirmation` sem gravar; com confirmação grava grants, agente os relê e o clipboard antes recusado passa. Nome errado no revoke dá `name_mismatch`; nome certo remove o par de arquivo/status/lista dos dois lados. B emite `peer_disconnected` e exatamente uma notificação `device_removed`, corpo `a removeu este dispositivo`, sem `peer_blocked`. A espera pela remoção no arquivo de B é limitada por asserção de 3 s. O caso B desligado não está nessa suíte: foi executado separadamente `crates/clipsync-core/tests/mesh.rs:956`, `a_revoked_peer_removes_the_revoker_itself`, que cobre remoção online em até 3 s e, depois, B subindo após o revoke e mantendo A no arquivo, sem conexão ou aviso de remoção. |
| (3) Pareamento | **Passa** | `tests/pairing.rs:42`, `both_sides_see_the_same_code_and_save_exactly_their_grants`: códigos iguais nas duas Ui; finish antes de code_matches dá `wrong_step`; defaults todos false; arquivos e `pair_result` paired refletem exatamente os grants. `:215`, `cancel_ends_the_attempt_and_saves_nothing`: cancelamento durante escuta e na tela do código, resultado cancelled, sem gravação. Também recusa cancelar depois de enviar a decisão. |
| (4) Layout | **Passa** | `tests/layout.rs:99`, `get_lists_this_device_and_the_paired_ones_with_their_monitors`: `self_id` e dispositivo `is_self=true`, nome e monitor fake próprios; também lista o par e preserva seus monitores como cached após desconectar. |
| (5) Clipboard | **Passa** | `tests/clipboard.rs:20`, `a_manual_box_receives_previews_applies_and_discards`: B manual recebe `clip_received` sem texto/preview; `clipboard_get` também não expõe preview; chamada explícita retorna o texto; copy_item escreve no fake e vira applied; discard remove; repetir aplicação/remoção dá os erros esperados. |
| (6) Diagnóstico | **Passa** | `tests/diag.rs:16`, `the_support_bundle_carries_no_identifier`: cria um bloqueio real e verifica que a prévia não contém nome do par, id completo/curto, IP, nome local, id local completo nem conteúdo sintético; verifica substitutos e contagens de redação. |
| (7) Preferências | **Passa** | `tests/prefs.rs:14`, `prefs_set_writes_ui_toml_and_a_new_ui_reads_it_back`: grava arquivo, emite prefs_changed e nova Ui relê valores iguais. `:72`, `prefs_set_with_an_unknown_field_is_bad_request_and_changes_nothing`: campo desconhecido na raiz ou em appearance dá `bad_request` e não grava. |

| Itens do “Também” | Veredito | Teste e evidência |
| --- | --- | --- |
| Toda fixture JSON desserializa no tipo | **Passa** | `tests/fixtures.rs:68`, `every_fixture_fits_its_type_and_round_trips`: percorre os **43 JSON** do diretório, desserializa no tipo correspondente e serializa de volta sem diferenças; também verifica cobertura de comandos/eventos previstos pelo teste. |
| JSON ruim / comando desconhecido | **Passa** | `tests/agent.rs:22`, `bad_json_is_bad_request_and_unknown_cmd_is_unknown_command`: respostas com `bad_request` e `unknown_command`, respectivamente. |
| `peer_connected` em até 3 s | **Passa** | `tests/devices.rs:52`, `perms_need_full_trust_for_input_and_revoke_needs_the_typed_name`: cronômetro começa antes de iniciar B, recebe o evento de B e exige `elapsed <= 3 s`. |
| Outro agente no mesmo perfil | **Passa** | `tests/agent.rs:178`, `an_agent_already_on_the_profile_makes_the_app_a_client`: sobe outro `agent::run` real, observa `app_state.agent = external`, `agent_start = external` e `agent_stop` recusado com `agent_external`; a Ui continua como cliente. |

#### 3 · Vector — automáticos passam; M impede o fechamento integral

`cargo test -p clipsync-input`: **35** unitários, **10** `layout_capture`, **9** `module_mem`, **1** `self_test`, **1** smoke e **8** `units`, todos aprovados. `portal_loopback` saiu `ignored, requer sessão gráfica`. `cargo test -p clipsync-ui --test layout`: **13/13** nas três execuções específicas e nas três rodadas completas da UI.

| Item | Veredito | Teste e evidência |
| --- | --- | --- |
| Barreiras nos dois lados e mapeamento | **Passa** | `crates/clipsync-input/src/layout.rs:871`, `layout::tests::barriers_are_derived_on_both_sides_of_a_link`: compara monitor, borda, trechos local/remoto e `peer_from`/`peer_to` nos dois sentidos. `:901`, `layout::tests::position_maps_linearly_between_the_two_segments`: pontos inicial, médio, final e clamp. `tests/layout_capture.rs:275`, `enter_carries_the_mapped_position_and_the_monitor_only_for_a_v2_peer`: posição 0.75 no segmento local chega como 0.5 no remoto; v2 usa EnterAt com monitor, v1 usa Enter. |
| Barreira manual com ligação | **Passa** | `crates/clipsync-input/tests/layout_capture.rs:192`, `manual_barrier_and_link_together_refuse_to_build`: `ModuleError::Config`, motivo menciona `[[input.barrier]]` e `layout.toml`. `crates/clipsync-ui/tests/layout.rs:756`, `a_manual_barrier_and_a_link_together_are_refused_and_nothing_is_written`: recusa pela Ui e não grava. |
| Zones sem permissão / versão 1 | **Passa** | `crates/clipsync-input/tests/layout_capture.rs:369`, `zones_go_only_to_v2_peers_that_may_control_me`: sem `input_in` ou com versão 1, receptor vazio e `sent_count == 0`; par v2 autorizado recebe exatamente uma Zones. |
| LayoutOffer sem `input_in` | **Passa** | `crates/clipsync-input/tests/layout_capture.rs:441`, `layout_offer_needs_input_in_and_waits_as_pending_until_cleared`: NotAuthorized e nenhuma oferta pendente; autorizado, mantém a oferta até a resposta explícita. |
| InputLock / InputSwitch | **Passa** | `crates/clipsync-input/tests/layout_capture.rs:314`, `lock_ignores_an_activation_and_switch_opens_a_session_on_the_linked_peer`: ativação travada não envia Enter; switch envia EnterAt com monitor e posição corretos. `crates/clipsync-ui/tests/layout.rs:785`, `the_lock_and_the_switch_go_through_the_agent`, cobre os comandos pela Ui. |
| Ordem de validação / stale_rev | **Passa**, com limite de cobertura descrito aqui | `crates/clipsync-ui/tests/layout.rs:202`, `set_link_validates_in_the_documented_order`: erros combinados preservam prioridade de same_device, empty_segment, segment_out_of_range e segment_overlap sobre stale_rev; revisões e nenhuma gravação indevida conferidas. `:455`, `links_can_be_edited_and_removed_and_a_stale_screen_is_refused`, e `:554`, `two_saves_from_the_same_revision_do_not_both_win`, cobrem revisão obsoleta, id ausente e disputa de gravação. A borda interna usa testes unitários `layout::tests::{side_by_side_monitors_lock_the_shared_edge_on_both,stacked_monitors_lock_bottom_and_top_and_a_gap_locks_nothing,a_link_on_a_locked_edge_is_detected_only_where_it_overlaps}`; a posição de `internal_edge_locked` na ordem foi conferida em `src/layout.rs:851–909`, não por cenário integrado com dois monitores (o fake integrado tem um). |
| `layout.toml` relido igual | **Passa** | `crates/clipsync-ui/tests/layout.rs:389`, `a_saved_link_reads_back_the_same_and_survives_a_restart`: lê arquivo salvo, compara round-trip de todos os campos, reabre Ui e compara rev/links/opções/posições. Também passa `clipsync-input` `layout::tests::roundtrip_keeps_every_field`. |
| M: portal_loopback com ligação do layout | **Não comprovado; impede aprovação integral** | O comando padrão ignorou o teste gráfico. `cargo test -p clipsync-input -- --ignored portal_loopback` não foi executado nesta rodada. Além da evidência gráfica ausente, o roteiro desse commit ainda usa a configuração padrão e não lê `layout.toml` (achado abaixo). O teste `loopback_uses_the_link_barrier_and_maps_the_position` passou com fake, mas não substitui M. |

**Achado:**

1. **P2 · S6-ARGUS-1 · `crates/clipsync-input/tests/portal_loopback.rs:53`** — o teste gráfico instancia `build_with_adapters(InputConfig::default(), PortalCapture, ...)`; o arquivo inteiro não cria/lê `layout.toml` nem usa `build_with_layout`. Assim, mesmo uma execução verde desse roteiro em 415caf2 não provaria o requisito M de §12.9.3, que exige ligação do layout no lugar da barreira padrão. Falta adaptar o roteiro e registrar a execução gráfica. Observação do archive pedido, sem atribuir o mesmo estado a correções posteriores e sem modificar código. Durante a revisão, a main recebeu `4b174d7`, que acrescenta `portal_loopback_from_layout`; esse commit posterior não integra a base 415caf2 nem foi executado neste veredito.

#### 4 · Flux — testes locais passam; CI identificado pela run

**Clipboard — Passa.** `cargo test -p clipsync-clipboard`: 33 aprovados, zero falhos, um teste gráfico ignorado. Em `crates/clipsync-clipboard/tests/module_mem.rs`, as asserções dos testes abaixo foram relidas e todos saíram `... ok`:

| Requisito | Teste e resultado |
| --- | --- |
| ClipApply in_inbox, sem oferta nova; applied/expired recusados | `m_clipapply_writes_an_inbox_item_without_a_new_offer` (`:473`): uma escrita no fake, Applied, zero envio ao terceiro par e segunda aplicação recusada. `o_clipapply_refuses_an_expired_item_because_the_ttl_dropped_the_text` (`:543`): expirado dá WRONG_STATE sem escrita. `n_clipapply_works_while_paused_and_a_stale_item_qualifies` também passou. |
| ClipDiscard elimina item/texto | `p_clipdiscard_removes_the_item_and_its_text` (`:559`): caixa vazia, aplicar/descartar novamente dá NO_ITEM, sem escrita no fake. Implementação `src/module.rs:745` remove a entrada que possui texto e preview. |
| ClipSendCurrent e permissão / vazio | `q_clipsendcurrent_sends_the_clipboard_to_clipboard_out_peers` (`:582`): conteúdo atual chega ao fake autorizado; broadcast exclui par sem clipboard_out; envio explícito sem permissão é NotAuthorized; vazio dá EMPTY_CLIPBOARD. |
| TTL descarta texto da caixa | `k_ttl_counts_from_local_receipt_and_drops_the_text` (`:397`): TTL zero expira na chegada; 800 ms expira após a espera, com preview vazio; teste de aplicação expirada prova recusa de reutilização. Implementação limpa preview e define texto como None. |
| Debug sem conteúdo | `i_debug_never_shows_content` (`:320`): Debug de ClipSnapshot e InboxItem não contém o segredo sintético. O teste `tests/self_test.rs:163`, `the_outcome_never_carries_clipboard_content`, também passou. |

O ignorado `platform_adapter_round_trip` não foi usado como evidência de clipboard real. Os oito testes fake de `self_test` passaram, inclusive restauração do conteúdo anterior e falhas de leitura/escrita/restauração.

**Autostart — Passa.** `cargo test -p clipsync-ui --test autostart`: **7/7 em cada uma das três rodadas** (também 7/7 nas três rodadas completas). Em `crates/clipsync-ui/tests/autostart.rs`: `appimage_wins_over_the_executable_when_set_and_non_empty`, `exec_line_quotes_the_program_and_adds_hidden`, `enable_writes_the_desktop_file_and_disable_removes_it` e `set_and_is_enabled_round_trip_through_the_options_dir` comprovam `$APPIMAGE` absoluto no Exec e `--hidden`, criação/remoção e leitura pelas opções. `a_relative_appimage_falls_back_to_the_executable`, `a_path_with_a_control_character_is_refused` e `enabling_with_a_bad_path_writes_nothing` também passaram.

**CI — Passa no último verde disponível, não certifica ainda a HEAD nova.** Às **02:03:29 -03:00**, `gh run view 40634953651 --json headSha,status,conclusion,jobs,url,name` retornou `headSha=415caf2...`, `status=in_progress`: Linux success e Windows ainda executando testes. Os steps de clippy dos dois runners já constavam success na consulta das 02:00:12, mas a run não havia terminado. Como solicitado, a referência verde usada é a [run 40628041669, commit 5a55688](https://github.com/devkappiushon/clipsync/actions/runs/40628041669), a mais recente em `gh run list --workflow ci --branch main --status success --limit 3 --json databaseId,headSha,conclusion,createdAt,url,displayTitle`.

`gh run view 40628041669 --json headSha,status,conclusion,jobs,url,name` saiu **0**, run e ambos os jobs com `success`. `gh run view 40628041669 --log` retornou **0 sem conteúdo**, inclusive com `rtk bypass`; por isso a evidência foi recuperada pela API oficial: `gh api repos/devkappiushon/clipsync/actions/runs/40628041669/logs`, exit **0**, ZIP de **57.854 bytes** com os logs completos dos dois jobs. Leitura dos trechos de clippy:

| Runner | Comando no log | App e término do clippy |
| --- | --- | --- |
| Windows | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `03:42:13Z Compiling clipsync-app v0.1.0 (D:\a\clipsync\clipsync\crates\clipsync-app)`; `03:42:34Z Finished dev profile ... in 38.90s`; step success. |
| Linux | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `03:42:40Z Compiling clipsync-app v0.1.0 (/home/runner/work/clipsync/clipsync/crates/clipsync-app)`; `03:42:51Z Finished dev profile ... in 12.00s`; step success. |

O app foi incluído por `--workspace`, com os flags exigidos pelo critério. Esse resultado é histórico de **5a55688**; não foi extrapolado para as mudanças posteriores do app/UI em 415caf2.

Evidência integral: `~/.cache/argus-s6-415caf2-wzp4if22/`, com `logs/ui-{1,2,3}.log`, `logs/layout-{1,2,3}.log`, `logs/autostart-{1,2,3}.log`, logs de input/clipboard/core-revoke/CI, `results.json`, `summary.json`, `core-revoke.json`, `meta.json` e inventários `before.json`/`after.json`.

## Findings

### Pendências pós-MVP aceitas pelo Maestro

- **Ajuda/erros nativos do clap em inglês:** registrar localização para depois do MVP; não bloqueia o veredito atual por decisão explícita do Maestro em 796e46c.
- **Filtro `control_`:** o comando contratual não seleciona `serve_request_roundtrip_stale_socket_and_cleanup`. Ajustar nome/filtro depois do MVP. Cobertura real obtida por `--test control` e `cargo test --workspace`; não bloqueia por decisão explícita do Maestro.


### S4 · 30dcd43 · histórico — S4-1 encerrado na reemissão 796e46c

1. **S4-1 · P2 · `crates/clipsync-cli/tests/roteiro.rs:230` (chamado em `:479`)** — o roteiro completo falha esperando `chave do par mudou` depois de reinicializar a identidade de B. Na execução do Argus, `cargo test -p clipsync-cli --test roteiro` saiu **101**, com **7 aprovados e 1 falho**; último status de B era **desconectado**, não Blocked. A repetição isolada `... roteiro mesh_` saiu **0**, 1 aprovado em 41,21 s. Confirma **flake observado** no caminho de troca de identidade (além do relato anterior do Maestro sob `cargo test --workspace`). Não há evidência para atribuir a causa exclusivamente à concorrência ou ao timeout. O WAIT do helper é 20 s; investigação da sincronização do teste e do estado/backoff do agente cabe a Flux/Rivet. Impacto: gate S4.4 e integração S1.6 não reproduzem o resultado de forma confiável. Nenhuma correção foi aplicada; sucesso isolado não substitui o gate completo falho.


### S3 · 3eaa730 · histórico — encerrados na v1.1 (30dcd43)

1. **S3-1 · P2 · `crates/clipsync-input/src/module.rs:441`** — todo Enter substitui o registro por `Incoming::new(session)`, zerando `window_count` e reiniciando `window_start` (`:72`). O limite de `:460` fica por sessão, embora §8 regra 3 exija limite por par. Reprodução usando módulo real + MemHub/FakeEmulate: mesmo peer envia 1.500 datagramas na sessão 7 e 1.500 na sessão 8; **3.000 datagramas emulados em 9.348 µs**, limite 2.000/s. As seis movimentações de posicionamento dos dois Enter foram descontadas. Impacto: um peer autorizado contorna continuamente o limite abrindo sessões; o teste oficial só cobre uma sessão.
2. **S3-2 · P2 · `crates/clipsync-input/src/module.rs:550`** — `pick_barriers` faz parse direto de u32; o seletor contratual `monitor = "#0"` (§3 e §10 item 3) cai em `ZoneRef::Name("#0")`. Reprodução pelo módulo real com adaptador de inspeção confirma **configured_monitor=#0 adapter_zone=Name("#0")**. `PortalCapture::resolve` (`adapter/portal.rs:138`) busca esse nome literalmente, logo a seleção por índice falha quando a zona tem nome de conector ou nenhum nome. Impacto: fica quebrado o fallback por índice previsto quando o portal não fornece o nome. Nenhum portal foi acionado por esta reprodução.


### S2 v1.3 · e611a88 · histórico — S2-1/S2-2 encerrados na v1.4 (7923e20)

1. **S2-1 · P2 · `crates/clipsync-clipboard/src/module.rs:79`** — o worker chama `write_text` após a leitura do clipboard sem reavaliar a pausa. A autorização em `on_offer` (`:453`) ocorre antes de enfileirar Cmd::Write; enquanto o worker lê, o host pode ser pausado. Reproduzido deterministicamente com MemHub/FakeClipboard: bloquear a leitura anterior ao write, `set_paused(true)`, liberar a leitura; resultado **`paused=true writes=1 ack=Applied`**, embora o contrato §7.7/§7.10 exija não aplicar em pausa. O teste existente só pausa antes de enviar a oferta e não cobre essa janela. Impacto: conteúdo remoto ainda altera o clipboard depois da pausa entrar em vigor.
2. **S2-2 · P2 · `crates/clipsync-clipboard/src/ledger.rs:67`** — `insert_new` remove a entrada mais antiga quando len>=SEEN_EVENTS_CAP antes de conferir se o ID recebido já está no mapa (`:74`). Com 1.024 IDs ainda válidos, repetir o mais antigo o apaga e reinsere como novo. Reproduzido no código exato de SeenEvents, no mesmo Instant (sem expiração): **`capacity=1024 duplicate_accepted_as_new=true`**. Viola §7.5 (repetido deve gerar Ack Duplicate) e deixa o resultado dependente do cache cheio. O caso oficial só cobre duplicado com a tabela praticamente vazia.

Argus não aplicou correções. Flux corrigiu os dois achados na v1.4; a reemissão acima registra as reproduções originais aprovadas. Os itens manuais S2.2–S2.4 continuam pendentes.

### S0 · histórico do checkout em edição às 12:04 (encerrado na reemissão do export)

1. **P1 · `crates/clipsync-proto/src/lib.rs:16`** declara `pub mod testing` com feature testing, mas o arquivo `testing.rs`/`testing/mod.rs` estava ausente. A dev-dependency da própria crate em `crates/clipsync-proto/Cargo.toml:26` habilita a feature. `cargo test --workspace` e `cargo clippy --workspace --all-targets -- -D warnings` retornaram 101 com E0583; nenhum smoke foi executado. Bloqueia critérios S0.3/S0.4 e a revisão dos módulos dependentes. Posse atual: Rivet/S1; sem atribuição ao commit S0 informado, que não foi inspecionado.
2. **P2 · `crates/clipsync-clipboard/src/adapter/fake.rs:2` e demais locais na saída de fmt** divergem de rustfmt (primeiro exemplo: ordenação do import de `mono_us`/`MIME_TEXT_UTF8`). `cargo fmt --all --check` retornou 1 e também informou o módulo testing ausente. Bloqueia S0.4 no checkout compartilhado; diferenças de clipboard pertencem à fatia S2 ainda em trabalho. Não rodei fmt que escreve arquivos.

Achados são observações do estado/hash revisado; não constituem pedido para interromper builders ou corrigir código de outro dono.

### S6 · critério 5 · e40849c

**Passa.** Argus, **2026-10-28**, contrato **v1.22**, §12.9 item 5. Base examinada: `e40849c52fd7c07f3278aa5a67f15f2920dac0de`, na `main`. **As nove telas passam nos dois temas, e os cenários vazio e erro foram conferidos nas nove telas.** Este veredito cobre o frontend com o mock e não substitui a prova do app Tauri real do item 6, o build ou os demais critérios da S6.

#### Base e método

- Lidos `docs/contrato.md`, `docs/telas.md`, os vereditos anteriores e as atribuições do `docs/workboard.md`. Conferidos `git status`, `git diff`, histórico da rodada visual e os arquivos de UI correspondentes. Não havia alteração rastreada sobre a HEAD solicitada; `CONTEXTO.md` já estava não rastreado e não foi tocado.
- Inspeção visual individual dos **18 pares** em `docs/telas/comparacao/<tela>-<tema>.webp`, usando os PNG equivalentes em `~/.cache/clipsync-telas-ref/pares-final/`: app à esquerda, Figma à direita. São montagens reduzidas de duas vistas de 1280×800; o arquivo composto mede 1693×572, não 2560×800.
- Portal próprio **Telas 1280 · Argus**, criado com `maestri portal create "http://127.0.0.1:5174/?cenario=padrao" "Telas 1280 · Argus" --size 1280x800`. O portal inicialmente informou 1280×792; aplicado `maestri portal resize "Telas 1280 · Argus" 1280 800` após as navegações. **Todas as 41 coletas finais de DOM registram `innerWidth=1280` e `innerHeight=800`.** As coletas preliminares a 792 px foram substituídas.
- Servidor já existente em `127.0.0.1:5174`, sem iniciar outro processo de servidor. Os **43 recursos/URLs HTTP verificados** (42 recursos carregados pelo navegador mais o documento HTML) coincidem byte a byte com os respectivos arquivos de `e40849c`, incluindo o mock servido sobre `ui-dev/`.
- Evidência bruta local: `~/.cache/argus-s6-e40849c/`. Cada coleta tem `.json` com texto, controles, retângulos, grades, rolagem, fontes e recursos, `.snapshot.txt` e `.logs.txt`. `static.json` registra os greps e os scripts; `served-head.json`, a correspondência HTTP/HEAD. `sweep.py`, `interactions.py` e `supplement.py` registram os comandos de portal usados. Nenhum arquivo de código-fonte foi alterado.

#### Comparação por tela em 1280×800

O julgamento é da estrutura e hierarquia previstas na v1.22, não de igualdade de texto ou de dados do protótipo. Foram aceitos os avisos de recurso indisponível, permissões iniciais desligadas, identificadores/endereços diferentes e detalhes mais precisos do backend. As colunas abaixo são medidas reais de `getComputedStyle(...).gridTemplateColumns`, complementadas por `getBoundingClientRect()`.

| Tela | Claro | Escuro | Evidência estrutural |
| --- | --- | --- | --- |
| Onboarding 1 · Esta máquina | [Passa](telas/comparacao/onboarding1-claro.webp) | [Passa](telas/comparacao/onboarding1-escuro.webp) | Cartão central sem barra lateral, marca e três passos no topo, título, nome, impressão digital e Continuar no rodapé à direita, na mesma ordem do quadro. |
| Onboarding 2 · O que sincronizar | [Passa](telas/comparacao/onboarding2-claro.webp) | [Passa](telas/comparacao/onboarding2-escuro.webp) | Dois cartões de módulos lado a lado, **327 + 327 px**; aviso abaixo; Voltar à esquerda e ação de criação à direita. Não empilha os módulos. |
| Onboarding 3 · Teste | [Passa](telas/comparacao/onboarding3-claro.webp) | [Passa](telas/comparacao/onboarding3-escuro.webp) | Mesmo cartão central: título, aviso, ação do teste, resultados com selo por módulo e rodapé Voltar/Concluir. Os resultados correspondem aos módulos realmente testados; não foi exigida a linha fictícia de teste de rede do protótipo. |
| Dispositivos | [Passa](telas/comparacao/dispositivos-claro.webp) | [Passa](telas/comparacao/dispositivos-escuro.webp) | Tabela e painel de permissões lado a lado, **656 + 296 px**; primeira linha selecionada e identificação do mesmo par no painel; ação de parear no cabeçalho. |
| Pareamento | [Passa](telas/comparacao/pareamento-claro.webp) | [Passa](telas/comparacao/pareamento-escuro.webp) | Etapa de conexão/código à esquerda e permissões iniciais à direita, **576 + 376 px**; código destacado e Concluir no pé do painel. O app mostra a etapa efetiva: a espera é substituída pelo código, em vez de afirmar simultaneamente que ainda aguarda o par. Permissões começam desligadas conforme §12.2.8. |
| Layout de telas | [Passa](telas/comparacao/layout-claro.webp) | [Passa](telas/comparacao/layout-escuro.webp) | Canvas e faixa de ações à esquerda, Ligações à direita, **664 + 288 px**; os atalhos de dispositivo adicionais ficam numa faixa abaixo. Monitores, ligação e painel continuam lado a lado. O par de imagens mostra o app sem popover aberto e o Figma com o destino da borda aberto. |
| Clipboard | [Passa](telas/comparacao/clipboard-claro.webp) | [Passa](telas/comparacao/clipboard-escuro.webp) | Faixa horizontal de estado; modo e interruptores na faixa seguinte; tabela larga abaixo, com origem/horário/tipo/tamanho/estado/ações; Enviar agora secundário. Campos adicionais de último evento/bloqueio e notas de indisponibilidade não alteram essa organização. |
| Diagnóstico | [Passa](telas/comparacao/diagnostico-claro.webp) | [Passa](telas/comparacao/diagnostico-escuro.webp) | **465 + 465 px**: Sistema → Rede → Pacote de suporte à esquerda; Adaptadores → Conflitos → Últimos bloqueios à direita. Exportar é secundário. Os detalhes extras exigem rolagem vertical; os blocos inferiores existem e ficam acessíveis por rolagem, sem empilhar as duas colunas. Ver observação abaixo. |
| Preferências | [Passa](telas/comparacao/preferencias-claro.webp) | [Passa](telas/comparacao/preferencias-escuro.webp) | **480 + 480 px**: Atalhos → Inicialização à esquerda; Notificações → Aparência → Idioma à direita; versão/licença/crédito no rodapé. O conteúdo cabe nos 800 px de altura no cenário padrão. |

Nenhuma coleta das nove telas apresentou vazamento horizontal no documento ou no `main` (`scrollWidth` igual a `clientWidth`). Os pares claro/escuro preservam a mesma disposição.

**Observação visual não bloqueante · Diagnóstico.** Há duas barras verticais, do documento e do `main`: `documentElement.scrollHeight=911`, `main.scrollHeight=1194`, ambos com `clientHeight=800`. O contêiner de conteúdo está em `crates/clipsync-app/ui/css/app.css:262`, e as colunas em `ui/css/telas/diagnostico.css:16`. No topo, Pacote de suporte começa em y=979,2 e Últimos bloqueios em y=804,3; após `maestri portal scroll "Telas 1280 · Argus" down 600 '#conteudo'`, seus títulos ficam em y=585,2 e y=410,3, respectivamente. As capturas `escuro-diagnostico-topo.png` e `escuro-diagnostico-inferior.png` comprovam o acesso. A rolagem dupla é um refinamento pendente; não foi usada para reprovar a disposição em duas colunas, dada a aceitação expressa de dados adicionais mais precisos nesta revisão.

#### Cenários vazio e erro, observados no DOM

URLs: `/?cenario=vazio&tema=claro#/<rota>` e `/?cenario=erro&tema=claro#/<rota>`. O passo 2 foi alcançado pelo botão Continuar do passo 1; não se atribuiu ao passo 2 uma captura redirecionada ao passo 1. O passo 3 também foi repetido em documento novo para não herdar a seleção provisória dos módulos. `snapshot` forneceu as refs para `fill`, `click`, `check` e `uncheck`; `evaluate` conferiu textos, estados e dimensões.

| Tela | Vazio | Erro | Evidência observada |
| --- | --- | --- | --- |
| Onboarding 1 | **Passa** | **Passa** | `vazio-onboarding1` e `erro-onboarding1`: nome e identidade disponíveis mesmo sem pares; `erro` mostra o aviso da porta ocupada. Esvaziar o nome e Continuar exibe **“Dê um nome a esta máquina.”**, mantém o passo 1 e devolve o foco ao campo. Nome válido permite seguir. |
| Onboarding 2 | **Passa** | **Passa** | `vazio-onboarding2` e `erro-onboarding2`: módulos lado a lado nos dois cenários. Desligar ambos mostra **“Ligue pelo menos um.”** e desabilita Criar perfil e continuar. O aviso do agente com falha permanece no cenário erro, mas o cadastro local continua disponível. Complemento `erro-onboarding2-gravacao-injetada`: uma falha `io` temporária, registrada somente no mock em memória por `maestri portal evaluate`, faz o botão de criar exibir **“Não foi possível gravar o perfil: permissão negada.”** sem sair do passo; não se atribui essa injeção ao comportamento nativo do cenário. |
| Onboarding 3 | **Passa** | **Passa** | `vazio-onboarding3-isolado`: antes do teste, sem resultados inventados e com aviso do clipboard. Após Rodar teste, aparecem clipboard validado e input experimental do mock. `erro-onboarding3-isolado-teste`: o clique exibe **“O agente não está rodando. Inicie o agente e tente de novo.”**, mantém a tela e libera novamente o botão. |
| Dispositivos | **Passa** | **Passa** | `vazio-dispositivos`: **“Nenhum dispositivo pareado”** e ação de parear, sem painel de par inexistente. `erro-dispositivos`: mensagem da porta ocupada, Iniciar agente e aviso de que a lista vem do arquivo de pares; os três pares aparecem **offline**. |
| Pareamento | **Passa** | **Passa** | `vazio-pareamento`: estado ocioso, Aceitar/Conectar e Aceitar pareamento; permissões e Concluir desabilitados antes de confirmar código. `erro-pareamento`: **“Não foi possível carregar”**, mensagem de agente parado e **Tentar de novo**. |
| Layout de telas | **Passa** | **Passa** | `vazio-layout`: apenas esta máquina no canvas, **“Nenhuma ligação”**, orientação para parear e **“nenhum outro dispositivo”** na faixa de atalhos. `erro-layout`: erro do agente e Tentar de novo, sem canvas com estado fictício. |
| Clipboard | **Passa** | **Passa** | `vazio-clipboard`: **“Nenhum item recebido”**, orientação e política de retenção; último evento/bloqueio = nenhum. `erro-clipboard`: erro do agente e Tentar de novo. |
| Diagnóstico | **Passa** | **Passa** | `vazio-diagnostico`: **“Nenhum par”** em Rede e **“Nenhum bloqueio recente”** em Últimos bloqueios; dados locais continuam disponíveis. `erro-diagnostico`: erro do agente e Tentar de novo. |
| Preferências | **Passa** | **Passa** | `vazio-preferencias`: valores locais e os cinco grupos permanecem, pois não dependem de pares. `erro-preferencias-gravacao`: marcar Reduzir movimento exibe **“Não foi possível salvar: Não foi possível gravar o ui.toml: permissão negada.”**; o controle volta a `checked=false` e permanece habilitado. |

O cenário `vazio` significa ausência de pares, itens e bloqueios; não significa ausência de perfil ou preferências. O cenário `erro` deixa o agente em falha, mas os comandos locais de onboarding, lista de pares e leitura de preferências continuam respondendo. Esses estados foram avaliados conforme o comportamento observado, sem exigir uma lista vazia ou falha de leitura fictícia nessas telas.

#### Veredito por item da §12.9.5

| Item | Veredito | Comando / evidência |
| --- | --- | --- |
| Nove telas × dois temas, app ao lado do Figma | **Passa** | 18 arquivos presentes e inspecionados, com resultado individual na tabela acima. |
| Estrutura em 1280×800, sem empilhar colunas/faixas do quadro | **Passa** | 41 coletas com viewport exato; grades e ordem dos blocos registradas. Dispositivos, Pareamento, Layout, Diagnóstico, Preferências e módulos do onboarding mantêm suas duas colunas. |
| Sem `innerHTML`/`style=` e sem script inline | **Passa** | `grep -rnE 'innerHTML\|style=' crates/clipsync-app/ui`: **exit 1, stdout/stderr vazios**, isto é, zero ocorrências. Parse de todos os HTML: dois scripts, ambos com `src`: `js/tema-inicial.js` (`index.html:11`) e módulo `js/app.js` (`index.html:70`). O DOM confirmou os dois com conteúdo inline vazio. |
| Sem travessão em texto de interface | **Passa** | `grep -rn '—' crates/clipsync-app/ui`: **exit 1, stdout/stderr vazios**, zero ocorrências em toda a pasta. |
| Rede na própria origem e fontes locais carregadas | **Passa** | `maestri portal logs-start`/`logs`: as 41 coletas finais não registraram mensagens. A evidência de rede vem de `performance.getEntriesByType('resource')` via `maestri portal evaluate`: **42 URLs distintas**, todas com `origin === 'http://127.0.0.1:5174'`, e lista de externas vazia em todas as coletas. `document.fonts` contém **Inter Variable**, **Geist** e **Geist Mono**, todas com `status: 'loaded'` nas 41 coletas. `logs` é saída de console; não foi descrito como captura de pacotes ou HAR. |
| Cenários vazio e erro em cada tela | **Passa** | Nove de nove telas em ambos os cenários, conforme a tabela de DOM; inclui interações de validação, autoteste e falha de gravação. |

**Resultado do critério 5: Passa em e40849c.** Não houve alteração de fonte, build, instalação, novo servidor, push ou execução de Playwright nesta revisão.

### S6 · critério 6 e 7 · v0.2.0

**Critério 6: Passa, com a execução interativa atribuída ao Maestro. Critério 7: Passa, verificado diretamente por Argus.** Revisão em **2026-10-28**, contrato **v1.24**, a partir de `docs/teste-app-real.md` nos commits `40b0446` e `2ca12be`. A confirmação documental e dos arquivos abaixo não é uma segunda execução dos fluxos de tela. Por instrução do Maestro, Argus não abriu o app, não executou os scripts AT-SPI e não leu nem alterou o clipboard.

#### Bases e procedência

- O relato separa duas execuções: pacote manual da [run 40680774165](https://github.com/devkappiushon/clipsync/actions/runs/40680774165), em **e40849c**, para a parte CLI/app com perfil `t1`; AppImage do **release v0.2.0**, para a parte de tela com perfil `ui1`. `gh run view` confirmou a base e os dois builds aprovados do pacote manual, com publicação pulada.
- A tag remota anotada `v0.2.0` resolve para **`d7a4d796067143c8393edb176d9cf526fe126a87`**, confirmado pela API GitHub e por `git rev-parse 'v0.2.0^{commit}'`. A [run de release 40691106646](https://github.com/devkappiushon/clipsync/actions/runs/40691106646) usa essa mesma base: Linux, Windows e publicação com `conclusion=success`.
- O AppImage instalado em `~/.local/bin/clipsync-app.AppImage` tem **86.010.360 bytes**, modo **0755** e SHA-256 **`9133d717ff5f719c7b2cf7557182363467b1837bc02b2a910d042afa760049d8`**. Coincide com o AppImage do cache do release, o `SHA256SUMS` e o digest do asset publicado.
- A CLI em `~/.cache/csrt/clipsync` responde **`clipsync 0.2.0`** a `--version`, mas **não é byte a byte a CLI do release final**. SHA-256 da CLI dos testes: `065a5c0f1f7e8b031501da591c9961a21a6a4efce3a06def7d2d16760bf57cde`; da CLI contida no tar.gz final: `a402f4e3b9b9627a7650dfd9582770637bbab16b8b6582bf398be22f838fc6ec`. Isso é coerente com a origem anterior declarada no relato; não se apresenta a sessão como teste de dois binários do release final. O item 6 exige outro perfil com CLI e adaptador fake, sem exigir o mesmo build.
- Evidência desta auditoria em **`~/.cache/argus-s6-v020/`**: `profiles.json`, `evidence-files.json`, `release.json`, `tag.json`, `manual-run.json`, `release-run.json`, `asset-checks.json`, `sha256sum-check.txt`, `desktop-validation.txt` e uma cópia de `SHA256SUMS` baixada novamente do release. Os scripts `~/.cache/csrt/ui.py` e `parear.py` foram lidos, sem importá-los ou executá-los; arquivos de conteúdo do clipboard não foram examinados.

#### Critério 6, item por item

**“Passa” nesta tabela aceita a execução relatada pelo Maestro em `docs/teste-app-real.md`; a coluna de limites identifica exatamente o que Argus não observou ao vivo.** Arquivos de perfil corroboram o estado persistido, mas não provam sozinhos um clique, uma leitura de clipboard, a apresentação de uma tela ou a ordem de fechar e abrir.

| Item da §12.9.6 | Veredito | Evidência conferida por Argus | O que permanece pela execução do Maestro |
| --- | --- | --- | --- |
| AppImage do CI roda nesta máquina | **Passa** | Binário instalado idêntico ao asset do release; empacotamento final aprovado. `app.log` e `app2.log` do ensaio anterior registram `agent up`, escuta em `0.0.0.0:47812` e conexão com o ID de `t2`. `ui1-app.log` e `ui1-app2.log` registram processos distintos da sessão final. Há avisos, detalhados abaixo. | A janela em execução no release final, o agente dentro dela e a navegação por AT-SPI (`teste-app-real.md:26`). O log final não tem a mesma verbosidade de `agent up` do ensaio anterior. |
| Onboarding cria o perfil | **Passa** | `ui1/identity.cert.pem` produz o ID curto **6425-6da5-90b5-1d27**, igual ao relatado antes da criação. Certificado e chave correspondem. `ui1/config.toml` tem `clipboard=true`, `input=false` e escuta `0.0.0.0:47800`. | Perfil inicialmente vazio, impressão digital mostrada antes de gravar e ação Criar perfil e continuar pela tela (`:30`). Argus confirmou o resultado persistido, não o estado anterior. |
| Pareamento pela tela com CLI/fake, mesmo código nos dois lados | **Passa** | `t2/config.toml`: `adapter="fake"`. `ui1/peers.toml` contém o ID de `t2`, e `t2/peers.toml` contém o ID de `ui1`; horários de pareamento em ambos: **13:34:16 UTC**. `parear.py:18–25` extrai o código da CLI e da tela e cancela se faltar ou divergir, antes de clicar São iguais e confirmar. | Código bem-sucedido **954 136**, cliques, permissões e conclusão nos dois lados (`:32`). Não há transcrição salva dessa execução bem-sucedida no diretório: o script prova o procedimento, não que foi executado com sucesso. `join-ui.log` é de outra tentativa, **965 504**, encerrada com `PairFailed` e `rc=3`; não foi usado como prova de sucesso. |
| `send --stdin` chega à caixa em modo manual | **Passa** | Tanto `t1/config.toml` quanto `ui1/config.toml` têm `mode="manual"`; o emissor `t2` usa clipboard fake. Os pares recíprocos e permissões de saída estão persistidos. | Envio, chegada do item e estado “na caixa de recebidos” (`:16`, `:34–35`). A caixa é volátil; não foi reconstruída a partir de arquivos nem consultada por um novo comando ao agente. |
| Aplicar/Copiar põe o texto no clipboard real | **Passa** | Perfil `ui1` usa `adapter="auto"`, não fake; `input=false` e clipboard habilitado. O relato identifica Mostrar e **Copiar** como a ação da UI que aplica o item. | Conteúdo exibido na prévia, clique e igualdade verificada por `wl-paste` (`:35`). Argus não repetiu leitura nem escrita do clipboard. |
| Cópia no sistema chega ao inbox do outro perfil | **Passa** | Configuração de `t1`/`t2` e concessões recíprocas compatíveis com a direção app → CLI; `t2` recebe em modo automático com adaptador fake. | `wl-copy`, chegada a `t2`, estado aplicado e tempo de 4 s (`:17`), no pacote anterior e40849c. Não se atribui essa execução ao AppImage final nem se repete a afirmação de restauração do clipboard como garantia da sessão inteira. |
| Diagnóstico lista os bloqueios | **Passa** | `ui1/peers.toml` conserva **`clipboard_in=false`**, `clipboard_out=true`, `input_in=false` para `t2`, coerente com a revogação de entrada feita para provocar o bloqueio. | Envio recusado por falta de permissão e linha de Últimos bloqueios com hora/módulo/motivo na tela (`:36`). O estado da permissão não prova a renderização do diagnóstico; não há dump AT-SPI dessa linha entre os arquivos fornecidos. |
| Preferências sobrevivem a fechar e abrir | **Passa** | `ui1/ui.toml`: **`appearance.theme="dark"`**, `reduce_motion=false`; `config.toml` continua manual. Há logs de dois processos do app final, com início do segundo às **10:35:40 locais**. | Sair pela bandeja, reabrir, Escuro ainda marcado, modo manual exibido e reconexão (`:37`). A existência do TOML e dos logs corrobora, mas não substitui a observação da releitura pela tela. |
| Ícone na bandeja com menu e ClipSync nas aplicações | **Passa** | `desktop-file-validate ~/.local/share/applications/clipsync.desktop`: **exit 0**, sem saída. Entrada `Type=Application`, `Name=ClipSync`, `Exec` apontando ao AppImage conferido, `Icon=clipsync`, `Categories=Utility;`, `StartupWMClass=clipsync-app`. Ícone em `~/.local/share/icons/hicolor/256x256/apps/clipsync.png` é PNG **256×256**. | Item da bandeja e menu Abrir/Pausar/Sair via D-Bus, alternância de pausa, encerramento e reconexão (`:18–22`). A entrada válida e o ícone comprovam a instalação; Argus não abriu o menu de aplicações nem observou a bandeja ao vivo. |

**Identidades verificadas.** Foi calculado SHA-256 do SubjectPublicKeyInfo em DER do certificado, conforme `clipsync-proto/src/ids.rs:14` e `clipsync-core/src/tls.rs:30`, e comparada a chave pública derivada da chave privada com a do certificado. Os três pares certificado/chave correspondem; nenhum material privado foi impresso ou copiado para a evidência. Os diretórios dos três perfis são **0700**, e certificados, chaves e TOML são **0600**.

| Perfil | ID curto calculado | Relação persistida |
| --- | --- | --- |
| `t1` | `2294-1e2a-d20f-b806` | Concede entrada/saída de clipboard a `t2`; input negado. |
| `t2` | `5c0b-51e8-036c-f85c` | Reconhece `t1` e `ui1`, com entrada/saída de clipboard e input negado. |
| `ui1` | `6425-6da5-90b5-1d27` | Reconhece `t2`; entrada de clipboard negada ao fim do teste, saída permitida e input negado. |

**Limitações e incidentes registrados, sem atribuição especulativa de causa:**

1. **Restauração do clipboard ao fim da sessão: Falha, conforme o próprio relato** (`docs/teste-app-real.md:40`): os 8 bytes anteriores não foram recuperados e o clipboard foi limpo. As afirmações anteriores de conteúdo devolvido/restaurado (`:17`, `:31`) não permitem declarar a sessão inteira sem perda. Isso não reprova os fluxos de receber/aplicar/enviar exigidos pelo item 6, mas permanece um incidente; a auditoria de arquivos não determina se a origem foi o teste, o autoteste ou outro componente.
2. **Logs não estão livres de avisos.** Há depreciação de libayatana-appindicator e ausência de `canberra-gtk-module`; `ui1-app.log` também contém duas mensagens `Gtk-CRITICAL` de `gtk_widget_get_scale_factor` às 10:32:21 e timeouts do arboard ao entregar conteúdo ao gerenciador de clipboard às 10:34:38 e 10:35:38. Não há base aqui para atribuir a perda do conteúdo a esses timeouts, nem para chamar esses logs de limpos.
3. **Mensagem duplicada após recusa:** defeito de texto relatado em `teste-app-real.md:39`, não reproduzido por Argus. Não impede os itens listados na §12.9.6. O autoteste de clipboard validado em 2 ms e a classificação de input desligado também permanecem somente pela execução relatada (`:31`); não são nova validação do input real.
4. O teste manual de captura de entrada do **critério 3** continua separado, conforme `teste-app-real.md:44`. Este veredito de 6 e 7 não o fecha nem declara a S6 inteira concluída.

#### Critério 7, item por item

[Release v0.2.0](https://github.com/devkappiushon/clipsync/releases/tag/v0.2.0), publicado em **2026-10-28 13:26:16 UTC**, `draft=false`, `prerelease=false`. Conferido nesta rodada por `gh release view v0.2.0` e pela API do release. Os sete assets estão publicados: seis pacotes e `SHA256SUMS`.

| Item / arquivo exigido pela §12.8 | Veredito | Evidência direta |
| --- | --- | --- |
| `clipsync-app-v0.2.0-linux-x86_64.AppImage` | **Passa** | Asset publicado; tamanho/digest da API coincidem com o cache; `sha256sum -c`: OK; instalado também coincide. |
| `clipsync-app-v0.2.0-linux-amd64.deb` | **Passa** | Asset publicado; tamanho/digest coincidem; `sha256sum -c`: OK. |
| `clipsync-app-v0.2.0-windows-x86_64-setup.exe` | **Passa** | Asset publicado; tamanho/digest coincidem; `sha256sum -c`: OK. |
| `clipsync-app-v0.2.0-windows-x86_64.msi` | **Passa** | Asset publicado; tamanho/digest coincidem; `sha256sum -c`: OK. |
| `clipsync-v0.2.0-linux-x86_64.tar.gz` | **Passa** | Asset publicado; tamanho/digest coincidem; `sha256sum -c`: OK. |
| `clipsync-v0.2.0-windows-x86_64.zip` | **Passa** | Asset publicado; tamanho/digest coincidem; `sha256sum -c`: OK. |
| `SHA256SUMS` publicado e correspondente ao cache | **Passa** | Baixado de novo com `gh release download v0.2.0 --pattern SHA256SUMS` para a pasta da auditoria; `cmp` com o cache original: exit 0. Digest do próprio manifesto também coincide com a API. |
| Verificação completa dos seis pacotes | **Passa** | `sha256sum -c SHA256SUMS`, executado em `~/.cache/clipsync-release-v0.2.0`: **exit 0, seis OK, nenhuma falha**. |

O SHA-256 do manifesto conferido é **`963e88ab830928e624c04a10eb48c5316f061c946611117d3e59c7e67a988a5e`**. Os hashes completos de cada asset, seus tamanhos e os digests retornados pelo GitHub estão em `asset-checks.json`. Essa prova verifica publicação e integridade dos pacotes; não representa execução dos instaladores Windows.

**Resultado:** §12.9.6 **Passa com a evidência de execução do Maestro e a corroboração documental acima**; §12.9.7 **Passa por verificação direta de Argus**. Nenhum fluxo do app ou do clipboard foi reexecutado nesta auditoria.

### S7 · critérios 1 a 5 · 8cc14c4

**Resultado: 1 Passa; 2 Passa; 3 Falha; 4 Falha; 5 Passa.** Revisão de Argus em **2026-10-28**, contrato **v1.32**, exclusivamente sobre **`8cc14c41458c9ca1c36f17b5c817b7e7d501b4b4`**. O critério 3 falha no encerramento do trabalhador Windows após saturação da fila, identificado por leitura; o 4 falha também por execução no CI Windows, no diagnóstico de atalhos. Os testes listados abaixo passaram, salvo a falha explicitada. Os critérios 6 a 8 não são encerrados por esta revisão.

#### Base, comandos e CI

Fontes extraídas com `git archive 8cc14c4`, em **`~/.cache/argus-s7-8cc14c4-5i4sqoxm/src/`**. SHA-256 do tar: **`539be51c7bfb97e7c297731417c11caa39a49bc72cb7378a49abd30fa70a4917`**. Os 310 arquivos têm manifesto em `source-sha256.json`. Foi criado apenas um índice Git descartável nessa extração, sem commit, para os scripts que enumeram fontes com `git ls-files`. Compilação com `export PATH=$HOME/.cargo/bin:$PATH`, Rust/Cargo **1.99.1**, `--locked` e cache de compilação compartilhado por `CARGO_TARGET_DIR`; não foi uma compilação sem cache. Nenhum fonte foi editado por Argus.

| Execução local, dentro do arquivo extraído | Resultado |
| --- | --- |
| `cargo test --locked -p clipsync-proto -p clipsync-core` | **Passa**: 114 testes, zero falhas/ignorados. |
| `cargo test --locked -p clipsync-input --test s7_module` | **Passa**: 21 testes, zero falhas/ignorados. |
| `cargo test --locked -p clipsync-input --lib span::` | **Passa**: 7 testes, zero falhas, 40 filtrados. |
| `cargo test --locked -p clipsync-input --test keymap --test winmath --test wincore` | **Passa**: respectivamente 10, 27 e 21 testes. |
| `cargo test --locked -p clipsync-input` | **Passa**: 157 testes; `portal_loopback` e `portal_loopback_from_layout` ignorados por exigirem sessão gráfica. Não contam como prova manual. |
| `cargo test --locked -p clipsync-ui` | **Passa no Linux**: 99 testes, incluindo 14 de `tests/layout.rs`. `layout_windows.rs` executa zero testes aqui. |
| `cargo test --locked -p clipsync-win` | **Passa nos tipos portáveis**: 2 testes de redação de `Debug`; zero testes Win32 neste host Linux. |

Os logs estão na pasta da auditoria: `proto-core.log`, `s7-module.log`, `span.log`, `windows-pure.log`, `input-full.log`, `ui.log` e `win-types.log`. As execuções dirigidas repetem parte das suítes completas; seus totais não devem ser somados como testes distintos.

**O [CI 40757283200](https://github.com/devkappiushon/clipsync/actions/runs/40757283200) terminou; não estava mais rodando ao concluir o veredito.** `headSha` coincide com a base acima. Conferência por `gh run view ... --json status,conclusion,headSha,url,jobs` e `gh run view ... --log`, salvos em `ci-status.json` e `ci.log`.

| Job / etapa | Veredito | Evidência |
| --- | --- | --- |
| Linux, `fmt + clippy + test` | **Passa** | Job concluído com `success`; fmt, clippy, os dois scripts e `cargo test --workspace --locked` verdes. |
| Windows, clippy | **Passa** | `cargo clippy --workspace --all-targets --locked -- -D warnings`, incluindo os caminhos `cfg(windows)`. |
| Windows, build | **Passa** | `cargo build --release -p clipsync-cli --locked`; é build de release da CLI, não validação de instalador. |
| Windows, testes | **Falha** | `cargo test --workspace --locked --no-fail-fast`: uma falha em `clipsync-ui/tests/diag.rs:136`. A opção `--no-fail-fast` permitiu executar também Layout, onboarding e `clipsync-win` depois dela. |
| Windows, testes específicos de S7 | **Passa nos alvos indicados** | 21 de `s7_module`, 7 de span, 10 de keymap, 21 de wincore, 27 de winmath, 2 de `layout_windows`, 8 unitários de `clipsync-win` e 6 de `clipsync-win/tests/windows.rs`. Isso não torna o job inteiro verde. |

#### Achados que impedem aprovação

1. **ALTO · Encerramento do trabalhador depende de uma mensagem descartável na mesma fila que satura.** Em `crates/clipsync-input/src/adapter/win/capture.rs:347`, o trabalhador recebe uma mensagem e trata `overflow` antes de examinar se ela é `Msg::Stop`. O ramo de overflow também drena indiscriminadamente as mensagens restantes (`:350`) e continua. `stop()` envia `Msg::Stop` por essa fila e espera `worker.join()` (`:648`, `:651`). Portanto, se o overflow já estiver marcado, a parada pode ser a própria mensagem recebida e ignorada, ou uma das removidas pela drenagem. A thread fica esperando `inbox.recv()` para sempre porque conserva seu próprio `SyncSender` (`Worker.queue`, `:325`), e `stop()` não termina. Há ainda outra espera sem cancelamento em `out.blocking_send()` (`:336`): o laço do módulo deixa de consumir seu receptor antes de aguardar `cap.stop()` (`crates/clipsync-input/src/capture.rs:254`, `:278`), permitindo bloqueio com o canal de saída cheio. Isso compromete a parada após sobrecarga, a pausa e o reload; **reprova o critério 3, R4/encerramento sob saturação**. Evidência por leitura e ordem das operações, **sem reprodução nativa Windows nesta auditoria**. O `Drop` também pode perder seu `try_send(Msg::Stop)` (`:668`), conforme S7-07 do Sentinel; o cenário aqui descrito alcança também o `stop()` normal. A correção posterior `6880b7f` apareceu na main durante a auditoria, mas está fora de 8cc14c4 e não recebe aprovação retroativa.
2. **MÉDIO · Diagnóstico Windows mistura a descrição fixa dos atalhos com seu estado efetivo.** `crates/clipsync-input/src/diag.rs:101` produz um item `input/atalhos` com `status=ok` incondicionalmente. `clipsync-ui/src/diag.rs:142` incorpora esses itens e, em `:227`, acrescenta outro item com o mesmo nome, calculado pelo estado do agente e pelas ligações locais. No CI, `diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts` encontra primeiro o item fixo, embora não exista ligação local. Em `clipsync-ui/tests/diag.rs:136`, o esperado era `status=warn`, `detail="inativos: nenhuma ligação desta máquina no layout"`; o obtido foi `status=ok`, `detail="Win+Alt+1 a 9 (ir para o dispositivo), Win+Alt+L (travar), Win+Alt+P (pausar); Win+Alt+C e Win+Alt+V: indisponível nesta versão"`. Falha registrada às **19:53:58 UTC**, detalhes em `ci.log:1142`. **Reprova o critério 4**, que exige `clipsync-ui` inteiro verde nos dois CIs. Não é uma hipótese de flutuação ou timeout: a asserção e a duplicação têm evidência direta.

#### Critério 1 · Rivet, proto e core

**Passa.** Os testes abaixo passaram localmente e constam como `ok` nos logs dos dois runners. As linhas são da extração de 8cc14c4.

| Item | Veredito | Evidência |
| --- | --- | --- |
| (a) Round-trip das três variantes | **Passa** | `clipsync-proto/tests/s7.rs:64`, `enter_span_returned_and_refused_roundtrip_through_postcard`; tags novas verificadas em `:115`. |
| (b) Validação de posição, trecho e monitor | **Passa** | `s7.rs:128`, `validate_refuses_pos_out_of_range_nan_empty_way_back_and_a_bad_monitor_name`: 1.5, NaN, trecho vazio/invertido e nomes vazio, com controle ou longo. |
| (c) Permissões e pausa | **Passa** | `s7.rs:187`, `enter_span_needs_input_in_and_no_pause_returned_and_refused_need_neither`. |
| (d) Par v2 recebe `RemoteTooOld`, nada sai | **Passa** | `s7.rs:241`, `memhub_refuses_the_v3_messages_to_a_version_2_peer_and_nothing_leaves`: três recusas, contador de envio zero e receptor vazio; `EnterAt` continua permitido. É a prova com `MemHub`, não uma sessão de rede real v2 nesta máquina. |
| (e) Hello v3, mínimo 1 | **Passa** | `clipsync-core/tests/mesh.rs:801`, `hello_v3_passes_the_0_1_0_and_0_2_x_compatibility_rules`. |
| (f) Pausa por pedido do módulo e evento da UI | **Passa** | `mesh.rs:771`, `request_pause_pauses_both_modules_like_the_control`: ambos os módulos recebem a pausa; status e `state.toml` a conservam e voltam ao retomar. `clipsync-ui/tests/agent.rs:100`, `:157`, `:218` verificam `pause_changed` para pausa, retomada e agente externo; `ui/src/events.rs:312` emite pelo mesmo delta de status. O teste do core não observa sozinho o evento da UI: a evidência é a composição desses testes e do caminho comum de estado. |

#### Critério 2 · Vector

**Passa nos testes com adaptadores falsos pedidos pelo contrato.** A suíte completa de input passou localmente e nos dois runners; Layout passou com 14 casos no Linux e 2 específicos no Windows. `tests/layout.rs` tem zero casos no Windows por `cfg`, por isso a prova Windows é explicitamente a alínea (h). Isso não cobre o trabalhador Win32 real do achado 1.

Todos os nomes desta tabela estão em `crates/clipsync-input/tests/s7_module.rs`, exceto quando indicado.

| Item | Veredito | Testes e asserções conferidos |
| --- | --- | --- |
| (a) Recusa por bloqueio e pausa do destino | **Passa** | `a_locked_destination_refuses_the_span_and_the_source_says_why` (`:263`) e `a_paused_destination_refuses_with_the_paused_text` (`:284`): motivo da recusa, ausência de emulação e encerramento na origem. |
| (b) Trecho de volta, posição e sessão correta | **Passa** | `pushing_out_through_the_span_returns_at_the_mapped_position` (`:304`), `the_destination_returns_only_through_the_way_back` (`:343`), `a_returned_of_another_session_is_ignored` (`:377`), `with_the_lock_on_a_returned_is_ignored` (`:395`). Conferem retorno mapeado, outras bordas/fora do trecho sem retorno, sessão errada e trava. Mais os **7 unitários de `src/span.rs`**, incluindo margem, parede e união com monitor vizinho. |
| (c) Botão seguro após posicionamento | **Passa** | `a_held_button_arrives_after_the_positioning_and_is_gone_after_the_return` (`:467`): posicionamento precede o pressionar; `pressed_now()==0` depois da volta. |
| (d) Atalhos, trava e pausa | **Passa** | `shortcuts_switch_lock_and_pause` (`:498`): troca, retorno ao próprio dispositivo, alternância da trava e pedidos de pausa contados no `MemHub`. |
| (e) Resolução sem mudar quantidade de monitores | **Passa** | `a_new_resolution_republishes_zones_and_moves_the_barriers` (`:534`): `Zones` novo para v2 em até 3 s, barreira atualizada e ausência de republicação sem mudança. |
| (f) Bloqueio da origem | **Passa** | `a_locked_source_closes_the_session_and_blocks_activation` (`:584`): `Leave`/`ReleaseAll`, recusa de nova ativação até desbloquear. |
| (g) Compatibilidade v2 | **Passa** | `a_v2_peer_gets_enter_at_and_the_source_decides_the_way_back` (`:629`): `EnterAt` e decisão de retorno na origem. |
| (h) Tela Layout no Windows | **Passa** | **Executado no CI Windows**: `clipsync-ui/tests/layout_windows.rs:21`, `with_the_input_module_on_the_commands_of_the_screen_work`, e `:74`, `input_off_only_when_the_module_is_off`. Ambos `ok`, às 19:54:00 UTC. |
| (i) Pausa, retomada e recusa do registro | **Passa** | `pausing_removes_the_hook_and_the_resume_hotkey_brings_it_back` (`:701`), `when_the_system_refuses_the_resume_hotkey_the_tray_stays_as_the_way_back` (`:736`), `an_adapter_without_hooks_keeps_its_session_while_paused` (`:767`). Incluem `stop/watch_resume/unwatch_resume`, preservação da sessão sem hooks e flag de recusa voltando a false. |
| (j) Última ligação, revogação e desligamento | **Passa** | `the_capture_stops_when_the_last_local_link_leaves_layout_toml` (`:663`), `the_capture_stops_when_the_peer_is_revoked` (`:831`), `the_capture_stops_when_the_module_shuts_down` (`:685`). |
| (k) `begin` só após troca bem-sucedida | **Passa** | `a_switch_begins_the_capture_once_and_a_crossing_never_does` (`:789`) e `a_failed_open_does_not_begin_the_capture` (`:850`): uma chamada na troca, nenhuma no cruzamento ou na abertura recusada. |
| (l) Plataforma sem `can_begin` | **Passa** | `a_switch_is_refused_where_the_platform_cannot_begin_a_capture` (`:813`) e `switch_supported_follows_can_begin_and_the_shortcut_is_refused_too` (`:874`): botão/comando e atalho recusados, nenhum `begin`, flag false; true no fake padrão. |

#### Critério 3 · Flux

**Falha pelo achado 1.** Os resultados verdes abaixo continuam válidos; não demonstram ausência de defeitos na ligação entre o trabalhador, a fila e o módulo.

| Item | Veredito | Evidência e limite |
| --- | --- | --- |
| Keymap ABNT2/US, teclas especiais e botões | **Passa** | 10 testes de `tests/keymap.rs`, local e nos dois CIs. Incluem ç/acentos/aspas, extras ABNT2, AltGr e descarte do LCtrl falso, Pause/NumLock/PrintScreen e ida/volta dos botões. |
| Nomes, geometria, DPI, atalhos e toque neutro | **Passa** | 27 testes de `tests/winmath.rs`, local e nos dois CIs: nomes estáveis/repetidos, origem negativa, quatro cantos, DPI 96/120/144/192, último pixel, borda entre monitores, Win+Alt+N/L/P e toque neutro. |
| Núcleo da captura | **Passa nos 21 testes puros** | `tests/wincore.rs`: ativação, posicionamento, deltas, trava, atalhos, repetição, debug redigido, resolução de barreiras, `begin` e política de reinício. |
| Clippy do Windows | **Passa** | Workspace e todos os alvos, avisos tratados como erro, run concluída sobre a HEAD pedida. |
| Isolamento de unsafe / scripts | **Passa no controle de código** | `scripts/conferir-unsafe.sh` exit 0 local e verde nos dois CIs. Busca por construções unsafe em Rust: 62 correspondências, todas em `crates/clipsync-win/src/ffi.rs`. O grep textual amplo de `allow(unsafe_code)` tem uma ressalva literal abaixo. |
| R3, estado fixo e seguros | **Passa na representação e seleção testadas** | `Held`/`HotkeyTracker`/`RestartPolicy` com tamanho fixo; teste `the_tracker_is_copy_and_holds_no_allocation`, e seleção de modificadores/botões nos testes do núcleo. Isso não resolve por si o valor antigo em `Shared.held`, já descrito pelo Sentinel como S7-05. |
| R5, ausência de códigos no debug/log | **Passa** | `scripts/conferir-privacidade.sh`: exit 0, 8 arquivos, e verde nos dois CIs. Testes `debug_never_prints_key_or_button_codes`, `debug_never_shows_a_key_a_button_or_what_is_held` e debug do tracker verdes. |
| R4, saturação e encerramento | **Falha** | Fila limitada existe (`sync_channel(8000)` e `try_send`), mas o ramo de descarte pode apagar a parada, e a saída bloqueante não observa cancelamento. Achado 1, fonte de 8cc14c4. Não existe teste dessa interleaving nos 21 testes de `wincore`. |
| C4, handles | **Passa no teste adotado em a9d0b00** | `ten_thousand_desktop_checks_do_not_leak_handles` passou no Windows: 200 chamadas de aquecimento e 10.000 medidas. A asserção real é `after <= before + 2`, não igualdade estrita; ciclos de Hooks/Watcher/CursorPark toleram +4. O log de sucesso não publica os valores antes/depois, portanto não se afirma crescimento observado de zero. |
| `INPUT_SUPPORTED` removido | **Passa** | `grep -rn INPUT_SUPPORTED crates/`: exit 1, stdout/stderr vazios. |
| HooksLost, política de três reinícios | **Passa na lógica pura, sob a decisão v1.32** | `wincore.rs:476`, `:488`, `:506`: três `Restart::Recreate`, quarta `GiveUp`, janela de 10 s, nenhuma retomada espontânea e estado fixo. Os testes de pânico dos callbacks também passaram no CI Windows. A sequência integrada fica com o limite explicitado abaixo. |

**Duas diferenças entre o texto do contrato e o alcance da prova:**

- O grep literal `grep -rn 'allow(unsafe_code)' crates/` retorna **duas linhas**, não uma: atributo em `clipsync-win/src/ffi.rs:12` e comentário explicativo em `clipsync-win/Cargo.toml:16`. A contagem literal da §13.3.5 **falha**; não há uma segunda autorização de unsafe. O script aprovado distingue código de comentário e passa. Não se apresenta o grep amplo como resultado de uma ocorrência.
- A última frase da §13.11.3 ainda descreve uma prova integrada de `HooksLost` com FFI falso. A decisão posterior e explícita da **v1.32, §13.6 e §13.13.21**, aceita que o trabalhador não tenha essa abstração. Por isso a ausência do teste integrado não é usada isoladamente como reprovação. Porém `three_losses_in_ten_seconds_are_recreated_and_the_fourth_gives_up_for_good` testa só `RestartPolicy`: não cria Hooks, não observa `Leave`/`ReleaseAll` e não executa `InputReload`. Os seis testes reais de `clipsync-win/tests/windows.rs` também não dirigem esse trabalhador. O caminho existe por leitura em `adapter/win/capture.rs:451` e `module.rs:530`; a integração completa continua sem execução observada por Argus e dependente do roteiro §13.12. A aceitação desse risco de cobertura não apaga o defeito concreto do achado 1.

#### Critério 4 · Rivet, UI

**Falha: `clipsync-ui` não está inteiro verde no Windows.** No Linux local foram 99 aprovados; no Windows, 80 aprovados e 1 falha. A falha e o impacto no diagnóstico estão no achado 2.

| Requisito específico | Veredito | Evidência |
| --- | --- | --- |
| Suíte inteira verde nos dois CIs | **Falha** | `diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts`, `tests/diag.rs:136`, run 40757283200. |
| `global_shortcuts` no Windows com input ligado | **Passa** | `tests/agent.rs:259`, `global_shortcuts_need_windows_and_the_input_module`, passou também no Windows. |
| Autoteste chama input, diferente de `not_tested` | **Passa** | `tests/onboarding.rs:175`, `self_test_calls_the_input_module_when_it_is_on`, passou no Windows e no Linux. |
| Defaults do `InputState` antigo | **Passa** | `clipsync-proto/tests/s7.rs:353`, `input_state_without_the_s7_fields_decodes_with_their_defaults`: recusa false, switch true. |
| Flag de recusa copiada, false sem agente | **Passa** | `clipsync-ui/src/status.rs:303`, `resume_hotkey_refused_comes_from_the_input_state`. |
| Capacidade de troca copiada, false sem agente | **Passa** | `status.rs:312`, `switch_device_comes_from_the_input_state`. |

#### Critério 5 · Pixel, portal e greps

**Passa no escopo de estados, texto, controles e estrutura observados pelo DOM.** Criado portal próprio **“S7 · Argus”**, usando exclusivamente o servidor já existente em `http://127.0.0.1:5174`. Foram usados `navigate`, `resize`, `evaluate`, `snapshot`, `logs-start` e `logs`; cada coleta tabulada foi ajustada e medida em **1280×800**. Os registros ficam nos JSON, snapshots e logs com os nomes abaixo, na pasta da auditoria.

| Cenário / tela | Veredito | Evidência observada |
| --- | --- | --- |
| `windows`, Layout | **Passa** | `windows-layout`: máquina `desktop-win`, **DELL U2723QE, 3840×2160 · 150%**, e **LG 24GL600F, 1920×1080**. Escala 100% omitida conforme §13.9. Canvas e painel de ligações permanecem lado a lado; monitores e nomes no SVG/DOM. Os dois botões da faixa Ir para dispositivo estão habilitados. |
| `windows`, Preferências | **Passa** | `windows-preferencias`: grupo Atalhos com **“ativos nesta máquina”**, Win+Alt+P/L/1/2/3 listados; C e V explicitamente indisponíveis conforme o contrato. |
| `windows-recusa`, Preferências | **Passa** | `windows-recusa-preferencias`: na mesma linha de **Win+Alt+P**, **“O Windows não liberou Win+Alt+P para retomar: use a bandeja ou Pausar tudo.”** |
| `padrao` Linux, Layout | **Passa** | `padrao-layout`: dois botões de troca com `disabled=true` e texto **“indisponível nesta versão neste sistema”** na faixa. |
| `padrao` Linux, Preferências | **Passa** | `padrao-preferencias`: atalhos com **“indisponível nesta versão”**, sem o selo de ativos. |
| `windows`, onboarding | **Passa para a liberação de input** | `windows-onboarding1` e `windows-onboarding2`: nome da máquina no passo 1; no passo 2, **Teclado e mouse marcado e habilitado** (`checked=true`, `disabled=false`), ao lado do módulo Clipboard. Criar perfil e continuar habilitado. Não foi executado autoteste real de input pelo mock. |
| Recusa lida de campo, não do texto de bloqueio | **Passa** | `ui/js/telas/preferencias.js:93`, `:122`, `:213` leem `features.resume_hotkey_refused`. Experimento adicional somente no mock em memória: flag true com `bloqueioRecusa=false` mantém o aviso; flag false com `bloqueioRecusa=true` retira o aviso da linha de Win+Alt+P, após `atualizarApp()` e a atualização assíncrona da tela. Evidência `refusal-flag-true-block-false.json` e `refusal-flag-false-block-true.json`. Nenhum JS das telas procura o texto do bloqueio para decidir essa recusa. |
| Troca lida de capacidade | **Passa** | `ui/js/telas/layout.js:683`, `:744`, `:920`, `:945` usam `features.switch_device`; corresponde aos estados Windows/Linux observados. |
| Greps da §12.9.5 | **Passa** | `grep -rnE 'innerHTML\|style=' crates/clipsync-app/ui` e `grep -rn '—' crates/clipsync-app/ui`: exit 1, nenhuma saída. Parse dos HTML e `document.scripts`: dois scripts, ambos com `src` e conteúdo inline vazio (`js/tema-inicial.js`, `js/app.js`). |
| Fontes e origem | **Passa nas coletas** | `document.fonts`: **Inter Variable**, **Geist**, **Geist Mono**, todas `loaded`. Recursos observados pela Performance API: **38 URLs distintas**, todas na origem `http://127.0.0.1:5174`; lista de externas vazia. As 38 respostas foram comparadas byte a byte com os arquivos de 8cc14c4: **38 iguais**, em `served-files.json`. Logs de console vazios nas coletas; não são uma captura de pacotes. |

**Limite do portal:** `screenshot` retornou timeout de renderização e `click` por referência informou que a janela poderia estar minimizada. `maestri debug` confirmou conexão normal. `evaluate` e `snapshot` continuaram funcionando; o avanço do onboarding foi feito por `button.click()` dentro de `maestri portal evaluate`. Assim, esta rodada comprova DOM, estados e medidas, **não fornece captura raster nova nem prova clique físico/foco**. Não foi usado outro navegador. A revisão visual completa dos pares Figma da S6 permanece na seção anterior, sem ser apresentada como reexecutada aqui. Nota de texto: o onboarding Windows ainda menciona InputCapture/X11 na explicação dos módulos; isso não impede a liberação de input exigida neste item, mas não descreve as APIs nativas Windows.

**Fechamento desta base: critérios 1 e 2 Passa; 3 Falha; 4 Falha; 5 Passa com o limite do portal declarado.** O CI já terminou e o job Windows está vermelho. Alterações posteriores da main e correções em andamento de outros agentes não mudam este resultado sobre 8cc14c4.

### S7 · critérios 3 e 4 · ed4b7e1

**Critério 3: Passa. Critério 4: Passa.** Reavaliação de Argus em **2026-10-28**, contrato **v1.32**, sobre **`ed4b7e13f69db954ad5bd25705020b5e5bda775d`**. Os dois achados que reprovaram 8cc14c4 foram corrigidos nesta base. A seção anterior permanece como histórico daquela revisão; este resultado não revalida automaticamente os outros critérios nem substitui o roteiro manual da §13.12.

#### Procedência e resultado das execuções

Nova extração por `git archive ed4b7e1`, em **`~/.cache/argus-s7-ed4b7e1-5c_h7q77/src/`**. SHA-256 do tar: **`c8b7120a4157471cfa04336f9baca6b015bdba1226e22cad37fb727d343ac9e2`**. Os **310 arquivos** foram conferidos contra `source-sha256.json`: nenhum alterado e nenhum arquivo extra, exceto os metadados do índice Git descartável necessário aos scripts. Confirmados como ancestrais da base: **6880b7f, b683c56, d6a2ef7, 0761414 e 205f79b** (`commits.json`).

Comandos locais executados nessa extração com `export PATH=$HOME/.cargo/bin:$PATH`, Rust/Cargo **1.99.1**, `--locked` e o cache compartilhado por `CARGO_TARGET_DIR`. Os totais de suítes dirigidas não representam execução nativa Windows nesta máquina Linux.

| Comando local | Veredito | Resultado |
| --- | --- | --- |
| `cargo test --locked -p clipsync-input --test keymap --test winmath --test wincore` | **Passa** | **59 testes**: keymap 10, winmath 27, wincore 22; zero falhas/ignorados. Inclui o novo `coming_back_presses_only_what_is_physically_down`. |
| `cargo test --locked -p clipsync-ui` | **Passa** | **100 testes**, zero falhas/ignorados. Inclui a regressão da substituição dos itens do diagnóstico. Os testes exclusivamente Windows não são contados como executados aqui. |
| `cargo test --locked -p clipsync-win` | **Passa nos tipos portáveis** | **2 testes** de redação de debug; zero testes Win32 no host Linux. |
| `scripts/conferir-unsafe.sh` | **Passa** | Exit 0, `unsafe isolado em crates/clipsync-win/src/ffi.rs: ok`. |
| `scripts/conferir-privacidade.sh` | **Passa** | Exit 0, `privacidade dos hooks: ok (8 arquivos)`. |
| `grep -rn INPUT_SUPPORTED crates/` | **Passa** | Exit 1, stdout/stderr vazios. |
| Busca por `Msg::Stop` ou `blocking_send` no trabalhador Windows | **Passa** | `rg -n 'Msg::Stop\|blocking_send' crates/clipsync-input/src/adapter/win/capture.rs`: exit 1, nenhuma ocorrência. |

**O [CI 40759799821](https://github.com/devkappiushon/clipsync/actions/runs/40759799821) foi aguardado até terminar: `completed/success`, nos dois runners, sobre o SHA exato acima.** O passo de cache Windows também terminou antes do fechamento deste veredito. A prova vem de `gh run view` com JSON e log completo, salvos em `ci-status.json` e `ci.log`; os resultados locais estão em `test-results.json` e nos logs `windows-pure.log`, `ui.log`, `win-types.log`, `unsafe.log` e `privacy.log`.

| Job / alvo | Veredito | Evidência final |
| --- | --- | --- |
| Linux, `fmt + clippy + test` | **Passa** | Fmt, clippy, os dois scripts e `cargo test --workspace --locked` verdes. |
| Windows, `windows build + test` | **Passa** | Clippy do workspace com `--all-targets --locked -- -D warnings`, build de release da CLI, scripts e `cargo test --workspace --locked --no-fail-fast` verdes. |
| `clipsync-ui` no Windows | **Passa** | **82 aprovados, zero falhas/ignorados**. O teste de diagnóstico anteriormente vermelho passou às **20:14:27 UTC**. |
| Adaptador Windows | **Passa nos quatro testes novos** | Os quatro nomes da tabela seguinte constam como `ok` no job Windows, às **20:14:07 UTC**; não foram inferidos do resultado Linux. |
| `clipsync-win` no Windows | **Passa** | **8 unitários e 6 testes Win32**, incluindo pânico dos callbacks, descarte de injetados, ciclo dos hooks e verificação dos handles. |

#### Critério 3 · encerramento, hooks e estado físico

| Item reavaliado | Veredito | Evidência em ed4b7e1 |
| --- | --- | --- |
| **6880b7f: parada independente da fila** | **Passa** | `adapter/win/capture.rs:343`: `emit` usa `try_send`; canal fechado termina, canal cheio consulta `Shared.stop` e espera em passos de **2 ms**. `Worker::run` consulta a flag e usa `recv_timeout` de **100 ms** (`:368`). `Msg::Stop` foi removida. Assim, a drenagem de overflow não pode mais consumir o sinal de parada que provocava o achado 1. |
| **6880b7f: trabalhador termina antes de `stop`/`Drop` devolverem** | **Passa** | `capture.rs:675` e `:700`: os dois caminhos ligam `Shared.stop`, retiram os hooks e aguardam `join`. O teste Windows **`the_worker_leaves_by_the_flag_even_with_everything_full`** (`:803`) inicia entrada e saída cheias, mantém o receptor de saída aberto sem consumir, confirma a thread ainda esperando, liga a flag e exige que ela termine em até 3 s. `ok` no CI. |
| **b683c56: HooksLost durante parada** | **Passa** | `capture.rs:482`: `recreate_hooks` retorna false imediatamente com `stop=true`. Teste Windows **`a_lost_hook_is_not_recreated_while_stopping`** (`:839`) confirma o retorno e ausência de hook instalado; `ok` no CI. |
| **b683c56: hook instalado na janela de corrida também sai** | **Passa por leitura da ordem de encerramento** | Depois do `join`, `stop()` faz novo `take/drop` de `shared.hooks` (`:689`), e `Drop` faz o mesmo (`:710`). Nesse ponto o trabalhador já terminou e não pode repor outro hook. Isso cobre a corrida em que ele passou pela guarda antes de a flag ser ligada. O teste anterior cobre a guarda inicial; não força essa segunda janela com hooks reais. |
| **d6a2ef7: estado antigo não atravessa instâncias** | **Passa** | `Shared::reset_input` (`:117`) zera os cinco words de `held`, `begin` e `ready`; `start_hooks` o chama antes de cada `Hooks::start` (`:213`), inclusive na recriação. Testes Windows **`a_new_hook_instance_forgets_the_keys_the_old_one_held`** (`:763`) e **`resetting_clears_the_flags_of_the_old_capture_too`** (`:791`) passaram. O primeiro simula Win/Alt, remove a closure antiga, reinicia e cruza somente com mouse: o conjunto transportado fica vazio. |
| **d6a2ef7: reaperto só do estado físico atual** | **Passa** | `end_locally` monta o reaperto com `repress_synths(&reconcile(Held::default()))` (`:324`), sem usar o `Shared.held` antigo. `reconcile` consulta apenas os oito modificadores e cinco botões da API fechada (`:249`). `tests/wincore.rs:513`, **`coming_back_presses_only_what_is_physically_down`**, passou localmente e nos dois CIs: vazio não gera pressionar; modificador e botão entram; letra é descartada. |
| Keymap, geometria, atalhos e política de HooksLost | **Passa na cobertura automática contratada** | 10 testes de keymap, 27 de winmath e agora 22 de wincore, também verdes no Windows. Permanecem os casos ABNT2/US, DPI, monitor com origem negativa, bordas, toque neutro, estado fixo, debug redigido e limite de três reinícios em 10 s. |
| Unsafe, privacidade e constante antiga | **Passa nos controles de código** | Scripts verdes localmente e nos dois CIs; 62 construções unsafe encontradas, todas em `clipsync-win/src/ffi.rs`; `INPUT_SUPPORTED` ausente. Continua a ressalva meramente textual: o grep amplo de `allow(unsafe_code)` dá duas linhas, atributo em `ffi.rs:12` e comentário em `Cargo.toml:16`, sem segunda autorização de unsafe. |
| Handles / C4 | **Passa no teste adotado pelo contrato v1.32** | `ten_thousand_desktop_checks_do_not_leak_handles` passou no Windows às **20:14:38 UTC**. Mantém 200 chamadas de aquecimento, 10.000 chamadas medidas e tolerância de +2 handles; não se declara igualdade exata de contagens, que o log de sucesso não informa. |

**O achado de parada sob saturação da revisão 8cc14c4 está fechado nesta base.** A prova reúne leitura dos caminhos de parada com o teste Windows do trabalhador bloqueado no canal cheio. A guarda contra recriação e a remoção depois do `join` fecham também o S7-08 no escopo inspecionado. O S7-05 fica coberto pelo reset e pela mudança da fonte do reaperto, com as regressões descritas acima.

Mantém-se o limite aceito expressamente na **v1.32, §13.6 e §13.13.21**: não há teste completo do trabalhador com plataforma falsa, percorrendo três quedas reais de Hooks, `Leave`/`ReleaseAll`, a quarta queda e `InputReload`. Os quatro novos testes do adaptador executam no Windows, mas usam eventos construídos e caminhos que não instalam hooks reais; os seis testes de `clipsync-win` cobrem a camada Win32 separadamente. A verificação da limpeza após `join` é por leitura, sem forçar essa corrida no sistema. Esses limites não são apresentados como execução do roteiro manual.

#### Critério 4 · diagnóstico e suíte da UI

| Item reavaliado | Veredito | Evidência em ed4b7e1 |
| --- | --- | --- |
| **0761414: UI substitui o diagnóstico fixo pelo estado do agente** | **Passa** | `clipsync-ui/src/diag.rs:202`, `with_agent_state`, remove os itens antigos que têm o mesmo par **área/nome** antes de adicionar os itens calculados. `gather` usa essa função em `:242`. O estado real decide os textos de agente parado, input desligado, ausência de ligação e atalhos ativos. |
| **0761414: no máximo um item de teclado/atalhos da área input** | **Passa** | Unitário **`the_agent_state_items_replace_the_input_crate_items_of_the_same_name`** (`src/diag.rs:750`) verifica substituição, status/texto e preservação de outra área com nome igual. O teste de integração **`diagnostics_show_the_keyboard_layout_and_on_windows_the_shortcuts`** (`tests/diag.rs:115`) agora verifica a contagem e o estado sem ligação; passou no Linux e no Windows. |
| **205f79b: diagnose da CLI não anuncia ativação sem conhecer o perfil** | **Passa por leitura, compilação e clippy Windows** | `clipsync-input/src/diag.rs:103` produz **`DiagStatus::Warn`** e o texto condicional **“ativos quando há ligação desta máquina no layout”**, em lugar do `Ok` fixo. Não foi executada uma CLI Windows interativa nesta máquina; a verificação do novo texto é estática. |
| Suíte inteira de `clipsync-ui` | **Passa** | 100 testes no Linux local, suíte verde no CI Linux e **82 testes no CI Windows**; nenhuma falha. A duplicação que provocava o achado 2 não voltou a ocorrer no teste de integração. |
| Atalhos, autoteste e flags da §13.11.4 | **Passa** | Conferidos como `ok` no log Windows: `global_shortcuts_need_windows_and_the_input_module`, `self_test_calls_the_input_module_when_it_is_on`, `resume_hotkey_refused_comes_from_the_input_state`, `switch_device_comes_from_the_input_state` e, no proto, `input_state_without_the_s7_fields_decodes_with_their_defaults`. Defaults preservados: recusa false, switch true; features false sem agente. |

**O achado de diagnóstico Windows da revisão 8cc14c4 está fechado. Resultado final em ed4b7e1: critério 3 Passa; critério 4 Passa.** Não foram modificados fontes, iniciado servidor ou repetido o teste manual do app. O CI indicado terminou completamente verde antes deste registro.

## S8 · diagnóstico e configuração da travessia · 2026-10-28

**Veredito: implementação aprovada na revisão de código, nos testes locais e no simulador; travessia nas máquinas reais pendente.** Revisão independente do warden sobre a base `48f257fb126636a73f9d6dcd83b63619b163b6c7` com as alterações não commitadas, incluindo arquivos novos, contra o contrato v1.34 §14 e o plano `docs/superpowers/plans/2026-10-28-travessia.md`. Não houve commit, push, release nem instalação nesta rodada. Aprovação de código não confirma a causa do mouse não atravessar no PC do usuário.

### Achados importantes corrigidos nesta revisão

1. **S8-W1, teste Windows preso ao texto antigo.** `clipsync-ui/tests/diag.rs` exigia igualdade com o detalhe anterior de atalhos, incompatível com `diagnose_for`. O teste agora exige exatamente um item, motivo contextual, status de aviso e ausência de afirmação de hooks ativos. Correção conferida por leitura e pela checagem cruzada de todos os alvos Windows; o ramo de execução Windows não rodou aqui.
2. **S8-W2, canto morto proposto como passagem.** Com uma ligação ocupando a borda útil de um monitor de 1080 px e cantos mortos de 8 px, o helper propunha o intervalo `0.9925925925925926..1`; o corte do backend o tornava vazio. O warden reproduziu esse resultado antes da correção. `trechosLivres` passou a excluir os cantos e lacunas que colapsam em f32, tentando outro monitor quando necessário. As regressões passaram na execução independente. A mesma rodada corrigiu o achado do Maestro no portal: notebook abaixo do LG deve escolher a borda externa do LG, mesmo com o Dell mais alto. A regra de alinhamento e desempate ficou explícita em §14.3.2.
3. **S8-W3, erro posterior à gravação descrito como ligação intacta.** `layout::save` grava o arquivo antes de recarregar o agente; portanto um erro de configuração ou timeout não prova ausência de alteração. `falhaAoLigar` agora reserva “ligação não foi alterada” às recusas conhecidas anteriores à gravação, como `stale_rev`; falhas indeterminadas preservam o detalhe do backend e informam que a atualização não pôde ser confirmada. Não há repetição automática da escrita. Teste do helper e ambos os caminhos no portal passaram. A ressalva está em §14.3.3.

Também foram alinhados os mocks: protocolo remoto é numérico; null/zero permanece `checking`; input desligado desabilita as capacidades de configuração. Nenhum achado importante permanece aberto nesta revisão. Observação visual de menor impacto: na cena final, o rótulo do terceiro computador (`pc-de-casa`) cruza a borda inferior do monitor Dell no desenho. O texto e os controles continuam legíveis; o ponto foi informado ao Maestro, sem bloquear os critérios funcionais da S8.

### Evidência executada pelo warden

Os binários Rust abaixo foram compilados pelo backend nesta rodada e executados diretamente pelo revisor, sem nova compilação e sem disputar o lock do Cargo.

| Comando | Resultado observado |
|---|---|
| `target/debug/deps/clipsync_ui-7e8bf0593e797a41 traversal::tests --nocapture` | 5 passaram, 0 falhas: pares revogados/remotos, protocolos, histórico limpo, JSON antigo e permissão local de entrada. |
| `target/debug/deps/traversal-c9cae87e62efa15c --nocapture` | 5 passaram, 0 falhas: API pública, perfil/input ausentes, layout inválido/ausente, saída sem conceder entrada na origem, pausa, trava e agente parado. A primeira tentativa no sandbox falhou por bloqueio de sockets (`Operation not permitted`); a repetição autorizada fora dessa restrição passou sem mudar código. |
| `target/debug/deps/clipsync_input-55181c31da94d9c5 context_tests --nocapture` | 3 passaram, 0 falhas: contexto ausente, pausa/input/layout e contagem só de ligações locais para pares conhecidos. |
| `target/debug/deps/module_mem-dd6fa58766c1b416 loopback_source_lock --nocapture` | 2 passaram, 0 falhas: bloqueio antes/depois da ativação, liberação dos pressionados e descarte de eventos posteriores. |
| `target/debug/deps/diagnose_context-1f91801808fd9a6d --nocapture` | 1 passou, 0 falhas: diagnóstico mantém checagens de sistema sem agente e explica contexto ausente. |
| `node crates/clipsync-app/ui-dev/testes/travessia.test.mjs` | 17 passaram, 0 falhas: geometria, monitores reais, escala/coordenadas negativas, trechos internos/ocupados/cantos, repetição sem duplicação, múltiplas ligações preservadas, resultado zero e falha parcial. |
| `node crates/clipsync-app/ui-dev/testes/mock-travessia.test.mjs` | 2 passaram, 0 falhas: capacidades com input desligado e protocolo desconhecido. |
| `node crates/clipsync-app/ui-dev/testes/oferta.test.mjs` | 3 passaram, 0 falhas: contrato do simulador, ofertas antigas e aceitação única. |
| `node --check` sobre os arquivos JS de `ui/js` e `ui-dev` | 33 arquivos, 0 falhas. |
| `scripts/conferir-unsafe.sh` | `unsafe isolado em crates/clipsync-win/src/ffi.rs: ok`. |
| `scripts/conferir-privacidade.sh` | `privacidade dos hooks: ok (8 arquivos)`. |
| `git diff --check` | Saída vazia, código 0. |

A busca em `ui/` não encontrou `innerHTML`, `style=` ou travessão; os dois scripts HTML usam `src`. O diff de `Cargo.toml`, `Cargo.lock`, `clipsync-proto`, capabilities e `tauri.conf.json` permanece vazio. Nenhuma dependência, mensagem de rede ou permissão nova foi introduzida.

### Integração executada pelo Maestro, com logs conferidos

| Comando | Resultado e log desta sessão |
|---|---|
| `cargo fmt --all --check` | Código 0; `/tmp/clipsync-s8-final-fmt.log` vazio. |
| `cargo test --locked --workspace --exclude clipsync-app` | Código 0; **441 passaram, 0 falhas, 3 ignorados** em `/tmp/clipsync-s8-final-tests.log`. Inclui `clipsync-ui`, que não pertence aos default-members. |
| `cargo clippy --workspace --exclude clipsync-app --all-targets --locked -- -D warnings` | Código 0; `/tmp/clipsync-s8-final-clippy.log`, `Finished` em 2,02 s. |
| `CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true cargo clippy --target x86_64-pc-windows-gnu -p clipsync-input -p clipsync-win -p clipsync-ui -p clipsync-cli --all-targets --locked -- -D warnings` | Código 0; `/tmp/clipsync-s8-final-windows.log`, `Finished` em 8,76 s. É compilação/clippy cruzado, sem executar Windows. |

O backend também entregou `/tmp/clipsync-s8-backend-tests.log` (292 passaram, 2 ignorados) e `/tmp/clipsync-s8-backend-clippy.log`, ambos lidos pelo warden. Não são somados aos 441 testes integrados porque repetem suítes. Os ciclos RED anteriores à implementação foram informados pelos implementadores, sem log RED persistido conferido pelo revisor: traversal retornava JSON null em vez de `no_local_link`; a CLI não tinha o item contextual; o loopback entregava 1 e 4 eventos apesar do bloqueio, com duas falhas. A execução GREEN independente está na tabela acima.

### Portal Maestri e limites da prova visual

O Maestro executou as ações pelo portal do simulador, com `evaluate`/medidas de DOM e screenshots. O warden inspecionou as imagens `/tmp/maestri-portal-52204e76-3cf1-46b7-adc8-6b8970caba64.png` e `/tmp/maestri-portal-3d5e435a-0d50-486f-befb-93371940a8e2.png`, sem dirigir o portal nem iniciar servidor.

- Layout simples em 1280×800: canvas antes do editor avançado, formulário ao lado, botões de posição e teste recolhível. Na imagem final, canvas com y=209 e altura=320. Em 960×640, o Maestro mediu `scrollWidth=960`, igual ao viewport, sem vazamento horizontal.
- `sem-ligacao` criou uma ligação apenas no clique. Trocar para abaixo atualizou o mesmo id e, após a correção, escolheu `LG 24GL600F/bottom`. Abrir tela, parear e conceder permissão não criaram ligações automaticamente. O pareamento simulado sem permissões mostrou “nenhuma” e ofereceu configurar na máquina com mouse físico.
- `receptor` e `ligacao-remota` mostraram a ausência de saída local sem exigir layout para receber. `sem-monitores` deixou Ligar desabilitado. `input-desligado` mostrou a causa e “Abrir preferências”. `protocolo-antigo` exibiu protocolo 2 e recomendação de atualização sem atribuir release do app. `teste-zero` mostrou “Teste inconclusivo” e reabilitou o botão.
- Conflito parcial injetado somente no simulador: a posição foi salva e uma gravação concorrente tornou `setLink` stale. Resultado `linksPreserved:true`, revisão 9, uma única `l1` anterior e aviso explícito de posição salva/ligação não alterada; botão reabilitado.
- Falha após persistência injetada somente no simulador: `setLink` original salvou e o wrapper retornou `config`. Resultado `linksChanged:true`, uma única `l1` em LG/bottom; a tela informou que não pôde confirmar a atualização, manteve a mensagem “Layout salvo, mas o agente não aplicou a mudança” e recarregou. Não afirmou preservação falsa nem repetiu a escrita.
- O simulador foi iniciado pelo Maestro em background e respondeu ao curl. Ao fim, o Maestro encerrou o PID 156631 (`servir-ui-dev.py`, porta 5174); o curl seguinte retornou conexão recusada, código 7/HTTP 000. O portal foi preservado.

### Veredito por critério do contrato §14.7

| Critério | Veredito e alcance |
|---|---|
| 1 · Backend | **Passa.** Estados conservadores, default `checking`, bloqueios históricos limpos, protocolo separado de release e `input_in` local sem bloquear saída; 10 testes dirigidos de traversal passaram. |
| 2 · Frontend | **Passa no código e simulador.** Posição sem monitor/borda obrigatórios, trechos válidos, atualização por id, avançado opcional, falhas parciais claras, clique exclusivo e zero eventos inconclusivo. Geometria/erros executados em Node e fluxos conferidos pelo Maestro no portal. |
| 3 · CLI/UI | **Passa na prova local e na compilação cruzada.** Helper compartilhado, contexto preservado sem agente, diagnóstico conservador e teste Windows atualizado para unicidade. Execução nativa do item de atalhos Windows continua pendente. |
| 4 · Input | **Passa na regressão com FakeCapture/FakeEmulate/MemHub.** Os dois casos encerram captura, liberam pressionados e não entregam eventos posteriores. RED atribuído ao implementador; GREEN repetido pelo warden. Não comprova bloqueio real da sessão. |
| 5 · Integração | **Passa nos alvos locais alcançáveis e no clippy cruzado.** 441/0/3, fmt, clippy e scripts. Tauri não foi compilado por falta de headers WebKitGTK; testes `cfg(windows)` não foram executados; CI desta alteração não foi disparado. |
| 6 · Revisão | **Passa.** Warden registra esta seção; segurança independente está em `docs/seguranca.md`, seção S8. Três achados importantes fechados nesta rodada. |
| 7 · Máquinas reais | **Pendente.** Faltam as saídas do PC Windows, confirmação da atualização instalada no notebook, travessia real/roteiro §13.12, instaladores e perguntas §13.14. O relato de atualização em andamento não prova atualização concluída. |

A leitura de eventos confirmou emissão somente quando o resumo muda, consumo pela janela/bandeja e recuperação por `resync`/`Lagged`. A bandeja foi revisada e seus testes estão no código, mas não foi executada numa casca Tauri nesta revisão. Saúde real de hooks/portais, permissão do destino, retorno pelo trecho e ida/volta entre as máquinas permanecem fora da prova do simulador. Nenhuma permissão do sistema foi aceita pelo revisor; nenhum segredo foi aberto.



## Preparação da v0.3.1: revisão e validação antes do commit

O warden revisou versão, lockfile, notas e workflow. Oito crates em 0.3.1, dependências externas idênticas e versão do Tauri herdada do workspace. Corrigido o único achado: o job de publicação exige push de tag; workflow_dispatch apenas empacota, mesmo apontando para tag. Revisão aprovada.

O Maestro executou `cargo test --workspace --exclude clipsync-app --locked -j 2`: 441 passaram, 0 falharam e 3 ignorados, saída 0. `node --test crates/clipsync-app/ui-dev/testes/*.test.mjs`: 22 passaram e 0 falharam. `cargo fmt --all --check`, os scripts de unsafe e privacidade e `git diff --check` passaram. `target/debug/clipsync --version` respondeu `clipsync 0.3.1`. Logs locais: `/tmp/clipsync-v031-tests.log` e `/tmp/clipsync-v031-js.log`.

CI completo, empacotamento, publicação e instalação permanecem posteriores a este registro; não há alegação de travessia física validada.


## Publicação da v0.3.1 e validação no CI

O usuário autorizou a publicação com “publique”. Tag anotada `v0.3.1` aponta para `f700caacd3ff94e058bbd2e3ac62baa8ee24831f`, o mesmo commit do CI e do empacotamento manual aprovados.

- CI `40783057611`: Linux e Windows concluídos com success. Logs: Linux 465 passaram, 0 falhas e 3 ignorados; Windows 445 passaram, 0 falhas e 1 ignorado. Inclui Tauri e testes nativos cfg(windows), além de fmt/clippy/scripts. Log local `/tmp/clipsync-v031-ci.log`.
- Empacotamento manual `40783058074`: builds Linux/Windows aprovados, publicação pulada por workflow_dispatch. Seis pacotes gerados no CI e baixados. CLI Linux respondeu `clipsync 0.3.1`; metadata do .deb confirmou 0.3.1; ZIP Windows íntegro; logs Tauri confirmaram 0.3.1 nos quatro instaladores.
- A execução duplicada por push da tag `40789025116` foi cancelada pelo Maestro durante a compilação, sem falha de build. A publicação promoveu os mesmos artefatos do empacotamento aprovado, preservando seus bytes.
- Release criado como rascunho com `--verify-tag`, notas da própria tag, seis pacotes e SHA256SUMS. Antes de publicar, todos os sete assets tinham state uploaded, tamanho esperado e digest SHA-256 do GitHub igual ao arquivo local; tag, commit e notas conferidos.
- Publicado como latest, sem draft ou prerelease, em 2026-10-28T23:54:49Z: https://github.com/devkappiushon/clipsync/releases/tag/v0.3.1. Metadata conferida pela API.
- `gh release download v0.3.1 --repo devkappiushon/clipsync --pattern SHA256SUMS --dir /tmp/clipsync-v031-publicado`, seguido de `sha256sum -c /tmp/clipsync-v031-publicado/SHA256SUMS` sobre os seis arquivos enviados: **6 de 6 OK**, saída 0. Metadata local `/tmp/clipsync-v031-release-publicado.json`.

As provas manuais da S8 e da S7 continuam pendentes: instalação nas máquinas do usuário, permissões reais do portal, comportamento físico de hooks/SendInput e travessia Windows/Linux. O CI não demonstra a causa do relato “o mouse não passa” nem encerra o roteiro §13.12. Nenhum perfil do usuário foi alterado, nenhum instalador foi executado e nenhum diálogo de permissão foi aceito.


## Alinhamento do contexto entre os andares

A pedido do usuário, o Maestro preparou docs/contexto-equipe.md e a nota comum do térreo, atualizou as entradas históricas de feature-spec/workboard e contextualizou dez notas dos quatro andares. Conteúdos anteriores preservados.

Após a troca de Claude para Codex, o CLI herdava o identificador de outro terminal. O usuário confirmou o nome Maestro · scaffold e o modo Maestro já ativo. A metadata do próprio workspace confirmou nome, diretório, função atribuída e permissão existente; os comandos desta sessão passaram a usar apenas a identidade desse mesmo terminal. Nenhuma permissão ou identidade de outro agente foi alterada. O coordenador de Segurança confirmou a mesma correção no próprio contexto.

O usuário criou as quatro ligações iniciais exigidas pelo limite de alcance do CLI. `maestri list` confirmou acesso aos quatro coordenadores e às suas notas. Cada coordenador ligou as notas à própria equipe e confirmou leitura da fonte comum. Verificação independente das conexões salvas: SEG & COMPLIANCE 15/15, Acess. 10/10, UX & UI 15/15, DOC 10/10. Total 50/50, vinte agentes. Oito colegas do térreo têm conexão com ClipSync · contexto comum.

Esta prova cobre a distribuição do contexto e a leitura pelos quatro coordenadores. Não é revisão do app nem afirmação de leitura individual dos outros dezesseis agentes. Nenhum código, servidor ou auditoria foi iniciado nessa etapa.


## S10 · revisão integrada de f3cdd01

Argus · warden: Needs fixes, INT-TEC-01/02 Important. Preparação interativa limitada ao timeout de Status; aceite/restauração omitem durabilidade não confirmada. Revisão estática do conjunto c1c9ea5..f3cdd01, sem novas execuções. Rivet possui uma onda final; re-review do delta permanece pendente. T4-SEG-01 Minor aberto e diferido, conforme triagem de Sentinel.

A observação de stale após aplicar/aceitar não é novo achado: atualização explícita é prevista no design §3/plano T4.4. A coleta não exercitou a recuperação; ela não prova travamento nem aprovação da sequência completa. Nenhum parecer libera release ou fecha prova física.


### S10 · re-review focal da correção integrada

Argus aprovou o delta final de quatro arquivos: INT-TEC-01/02 ADDRESSED, nenhum novo Critical/Important/Minor demonstrado. Prazo110s exclusivo da preparação, durabilidade visível e anunciada nos dois consumidores, sem nova escrita; regressão de atualização explícita preserva posições e renova revisão/stamps. Leitura de fonte e logs do autor, sem reexecução pelo revisor.

Integração posterior do Maestro passou: Rust workspace sem app, três gráficos ignorados, e Node264/264. Coletas complementares pelos coordenadores UX/Acess fecharam quatroV07 e duas recuperações explícitas no DOM; não fecham prova nativa/física. A falha Windows no coletor de notificações é gate separado, sem reabertura dos achados de produção encerrados.


### S10 · coletor do teste de revogação no Windows

Rivet corrigiu somente devices.rs: preserva Notification antes/depois da desconexão e delimita o Block intencional do setup. Asserts de notificação única/conteúdo e prazos mantidos; duplicatas contam e PeerBlocked/Lagged/Closed falham nas duas fases. RED determinístico confirmou o descarte do teste, sem demonstrar a ordem efetiva do CI antigo. Alvo final10/10, clippy/fmt focais aprovados; Argus aprovou o delta sem enfraquecimento. Produção e helper global intactos. Nova execução CI Windows permanece gate.


## S11 T1 · segurança · 04/11/2026 · f7cb334

Sentinel · segurança revisou `f7cb334ed6a912cca5d3bfa60626be61d574fac0`, com o contrato §17 e o plano `docs/superpowers/plans/2026-11-04-diagnostico-por-log.md` como playbook, conforme confirmação do Maestro. **Sem BLOCKER de segurança demonstrado; T1 aprovada com dois achados Minor e limites abaixo.** Este parecer não aprova a integração F1 nem o release. Só esta entrada foi acrescentada ao repositório; fontes preservadas.

### Minor · S11-T1-SEG-01 · caminhos seguem links e preservam permissões existentes

Em `crates/clipsync-core/src/logging.rs:100–130`, `DirBuilder::recursive(true)` aceita `logs/` existente e `OpenOptions::create(true).append(true)` segue o link do arquivo. Não há recusa de symlink/reparse point, checagem de arquivo regular, proprietário ou permissões existentes. Em fixtures Unix próprias, o escritor seguiu tanto o link de diretório quanto o de arquivo; diretório 0755 e arquivo 0644 permaneceram assim. Criações novas passaram com 0700/0600, sujeitos à umask. O modo de criação não corrige arquivo anterior, conforme a documentação de [OpenOptionsExt Unix](https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode).

Impacto: um caminho previamente redirecionado pode receber os registros fora do perfil e manter acesso de terceiros; também pode anexar dados a outro arquivo gravável pelo usuário. A rotação (`logging.rs:133–157`) faz remove/rename/reabertura por caminho, sem prender o diretório por handle: uma substituição local concorrente pode mudar o objeto reaberto. O mutex só exclui threads do escritor. No Unix, remover `.2` ou renomear `.1` opera sobre a entrada do link final, não sobre seu alvo; a abertura e os diretórios intermediários continuam seguindo links. A corrida foi identificada por leitura, sem execução concorrente. Classificação Minor porque exige controle local das entradas/diretório; o perfil normal é privado e nenhum vetor remoto ou ganho de privilégio foi demonstrado. Recomenda-se recusar redirecionamentos e validar objeto/permissões por handles, sem introduzir uma checagem de caminho separada da abertura.

### Minor · S11-T1-SEG-02 · 5 MiB não limita uma linha individual

Em `logging.rs:161–174`, o escritor só tenta rotacionar quando `size > 0` e não limita nem rejeita uma linha maior que `limit`. Uma linha de nove bytes mais newline foi gravada integralmente com limite de oito bytes, em fixture isolada. A serialização e o buffer também não têm teto próprio. Logo, a garantia de três arquivos de até 5 MiB depende de cada evento caber nesse tamanho. Não foi identificado emissor atual que produza uma linha acima de 5 MiB; o defeito está no limite prometido pela API. Recomenda-se definir no contrato a política para evento excessivo e aplicá-la antes da alocação/escrita, preservando o JSON válido.

### Informativos e limites verificados

- **Falha de rotação:** `logging.rs:164–174` continua escrevendo e avisa uma vez, exatamente como §17.1.4. Enquanto a falha persistir, o arquivo cresce sem teto; isso é risco previsto pelo contrato, distinto de SEG-02. O teste existente com `.2` como diretório confirmou preservação da linha corrente. Não há limite absoluto de disco nessa política.
- **Windows:** não há DACL explícita nem validação da ACL existente. Em criação normal, os objetos herdam a ACL do diretório efetivamente resolvido; arquivo existente mantém sua segurança, e links são seguidos sem `FILE_FLAG_OPEN_REPARSE_POINT`, conforme [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew). O handle padrão de Rust permite `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`, portanto o próprio handle não prova falha obrigatória de rename; outro processo pode restringir compartilhamento. Fonte: [OpenOptionsExt Windows](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode). Não houve execução Windows, prova de ACL ou corrida nativa.
- **Linha e privacidade:** `logging.rs:179–265` grava `v`, UTC com milissegundos, nível, target em `alvo`, nome local recebido em `host`, `evento` (vazio se ausente), `message` como `mensagem` e todos os campos adicionais. `serde_json` escapa quebras e aspas; nomes especiais de listas podem ser interpretados como JSON. Não inclui spans, não executa conteúdo e não há nova desserialização executável. Não existe redação, lista permitida de campos ou truncamento de IDs no escritor: strings/Debug chegam integralmente. A privacidade depende dos emissores e deve ser reavaliada com T2/F2 e filtros ampliados. A busca dos emissores do commit não demonstrou fluxo de clipboard, preview, tecla ou keycode ao arquivo. O `frame = ?other` em `transport.rs:1071` exclui as variantes `Module` e `Pair`; não é evidência de vazamento do payload. Texto remoto de desconexão pode chegar a `reason` (`transport.rs:922`), escapado como string, sem criar outra linha JSON.
- **IDs:** `PeerId::Display` e `Debug` usam `short()`, com os primeiros oito bytes em quatro grupos de quatro hexadecimais (`clipsync-proto/src/ids.rs:36–52`); os emissores existentes usam esse formato. Uma string arbitrária futura não será abreviada pelo escritor. O nome do par já registrado em `transport.rs:883` também integra o status.
- **Exclusividade:** `OnceLock`/mutex e abertura idempotente asseguram um sink por processo; o módulo não adquire lock entre processos. Em `f7cb334` ainda não existem chamadas de produção a `logging::init/open_file` (`git grep` sem ocorrências). A prova de abertura apenas após a reserva do agente pertence à T2, não foi inferida nesta T1.
- **Dependência:** delta adiciona somente a aresta `tracing-subscriber` já fixada em `=0.3.23`; nenhuma versão, script de instalação ou permissão nova no diff.

### Evidência executada por Sentinel

1. `git show f7cb334:crates/clipsync-core/src/logging.rs | sha256sum` e `sha256sum crates/clipsync-core/src/logging.rs`: ambos `0c273116e421036410ef33dd10af0a37d3dc3911adfb402b28d47239e147411b` no momento da revisão.
2. `target/debug/deps/clipsync_core-3952a415c7189b31 logging::tests --test-threads=1 --nocapture`: **6 passed, 0 failed**, executável já existente; não equivale a rebuild do workspace.
3. Cópia exata do arquivo de `f7cb334`, com quatro sondas de unidade acrescentadas somente em `/tmp/sentinel-t1-e9b5n388`, compilada com `rustc --edition=2024 --test audit.rs` e os rlibs locais de `serde_json`, `tracing`, `tracing_subscriber`, `time` e `tempfile`. O harness substitui somente `CoreError` por `std::io::Error`; não exercita o bootstrap. `/tmp/sentinel-t1-e9b5n388/audit-tests --test-threads=1 --nocapture`: **10 passed, 0 failed** (seis originais mais permissões existentes, dois links sintéticos e linha excessiva). Fontes e saída temporárias em `audit.rs`, `logging.rs`, `compile.log` e `result.log`, sem criar `target/` em `/tmp`. Todas as entradas foram sintéticas, em diretórios próprios.
4. `bash scripts/conferir-privacidade.sh`: **privacidade dos hooks: ok (8 arquivos)**. O script não é prova de todos os emissores ou dependências; estes tiveram inspeção focal separada.

Não executados: corrida de rotação, Windows nativo, CI/Tauri, integração F1 completa, perfis reais ou travessia física. Nenhum exploit foi construído e nenhum segredo real foi lido.


## S11 T1 · f7cb334 · não passa · 04/11/2026

Argus, revisão funcional solicitada pelo Maestro · scaffold. Escopo: commit `f7cb334ed6a912cca5d3bfa60626be61d574fac0`, T1 do plano `docs/superpowers/plans/2026-11-04-diagnostico-por-log.md` e contrato §17. **Não passa: um Important e dois Minor; nenhum Critical demonstrado.** Registro do parecer já entregue na sessão, sem reavaliar as alterações não commitadas posteriores em `logging.rs`.

1. **Important · S11-T1-ARG-01 · perda do arquivo após falha de reabertura e rollback.** Em `crates/clipsync-core/src/logging.rs:133–166`, se `rename(atual,.1)` funcionar, mas a abertura do novo atual e o rollback falharem, o descritor continua ligado a `.1`. As tentativas seguintes movem esse arquivo para `.2` e depois o removem. O escritor continua retornando sucesso sobre um arquivo sem nome, contrariando §17.1(4). Reproduzido em uma cópia do trecho compilada por stdin: um `chmod` injetado entre o rename e a reabertura provocou falhas reais de criação e rollback; após restaurar a permissão e escrever novamente, o diretório ficou vazio, o descritor tinha `nlink=0` e `write_line` retornava `Ok`. Preservar o caminho real do descritor antes de permitir outra rotação e acrescentar regressão desse percurso.
2. **Minor · S11-T1-ARG-02 · esquema divergente para avisos legados e filtro ampliado.** Em `logging.rs:188–198`, um aviso sem o campo estruturado sai com `evento=""`; permitindo debug no filtro, a linha leva `nivel="debug"`. Ambos foram observados em execução. O §17.2(1) exige nome de evento e restringe o nível a `info/warn/error`, enquanto §17.1(7) permite substituir a diretiva inteira. Alinhar o escritor e o contrato para explicitar como tratar esses registros.
3. **Minor · S11-T1-ARG-03 · teste de falha não alcança rename.** Em `logging.rs:363–367`, `rotacao_falha_preserva_linha_corrente` cria `.2` como diretório e falha já em `remove_file(.2)`. Não cobre falha de rename nem de reabertura/rollback. Uma sonda independente com diretório sem permissão de escrita confirmou que a falha simples de rename preserva a linha no arquivo atual; isso não cobre o caso ARG-01.

Conferido no commit: `Mutex<Option<Sink>>` serializa escrita e rotação; cada linha usa uma `write_all` e `flush`; o contador para em 200; listas serializadas entram como JSON; `log_path` é puro; não há chamadas `unwrap/expect/panic`; `Cargo.lock` acrescenta somente a aresta `tracing-subscriber` do core. Os cinco casos exigidos pela T1 estão presentes, além do teste de falha de rotação. A exclusividade entre processos depende da reserva do agente e da integração da T2, fora deste parecer.

Evidência da revisão anterior, não reexecutada neste registro:

- `rustc --edition=2024 --test`, fonte de `git show f7cb334:crates/clipsync-core/src/logging.rs` por stdin, rlibs locais e `CoreError` substituído por `std::io::Error` no harness: **9 passed, 0 failed**, sendo seis testes originais e três sondas de observação. As sondas confirmaram os comportamentos descritos; esse resultado não significa correção dos achados.
- PoC separada da rotação, também por stdin, com alteração de permissão injetada após o rename: `3 write_line=Ok; diretório vazio; descritor nlink=0`. Só fixtures temporárias próprias; nenhum arquivo do checkout foi alterado na revisão.
- `cargo test -p clipsync-core --locked logging` foi bloqueado pelo lockfile defasado das alterações concorrentes. Os logs do autor em `.superpowers/sdd/2026-11-04-diagnostico-por-log/t1-*.log` registram RED por API ausente, GREEN 6/6, suíte core 103/103 e clippy concluído; não são reexecução independente da suíte completa. Windows, bootstrap e integração F1 não foram verificados.


## S11 portal EventSink · sem commit (base f179b4f) · não passa · 04/11/2026

Argus, revisão solicitada pelo Maestro · scaffold da alteração de Vector · input em `crates/clipsync-input/src/adapter/portal.rs`, contrato §13.15. Escopo: diff **+227/-71**, blob completo `e2786b983ff74e59e0d0e572a6cb3b74a7c419f1`; a alteração não tinha commit na revisão, sobre a base `f179b4f`. O mesmo blob foi conferido ao registrar este parecer. **Não passa: três Important; nenhum Critical demonstrado.**

1. **Important · S11-PORTAL-ARG-01 · detach não cancela envio pendente.** Em `portal.rs:79–83`, `send` clona o emissor e aguarda fora do mutex. Com a fila cheia, `detach/attach` não encerram esse envio nem o receptor antigo. Reproduzido com o `EventSink` extraído: depois de detach e attach, o envio continuou pendente; liberar uma posição entregou o evento ao receptor antigo, deixando o novo vazio. Se o receptor antigo permanecer aberto e sem consumo, o laço fica preso no envio e não atende o novo receptor. Uma reprodução da estrutura `select!` também confirmou que fechar `stop_rx` não interrompe o await dentro do ramo. Isso viola §13.15(3) e pode reter o laço enquanto o receptor permanecer aberto. Os consumidores principais mitigam o caso fechando `rx` antes de `stop`, mas o adaptador não garante a propriedade sozinho. Cancelar o envio na troca de geração/detach.
2. **Important · S11-PORTAL-ARG-02 · morte do link entre as verificações de start.** Em `portal.rs:440–448`, `RecreateSession` só é tratado antes de `install_barriers().await`; depois desse await, apenas `Connect` é tratado. Se o laço morrer nesse intervalo, a segunda leitura retorna `RecreateSession`, que é ignorado. Se as demais chamadas ao portal tiverem sucesso, `start` retorna um receptor sem produtor EIS, podendo já ter descartado o erro enquanto estava detached. Evidência estática do fluxo, sem reprodução no portal real. Tratar o estado morto após o await e comunicar o término do laço.
3. **Important · S11-PORTAL-ARG-03 · stop oculta falha de Disable mantendo o link vivo.** Em `portal.rs:519–528`, o receptor é desatado e o erro de `Disable` é ignorado. Como a nova implementação mantém o EIS vivo, uma falha de desativação com captura ativa pode deixar o cursor capturado enquanto os eventos são descartados, apesar de `stop` retornar `Ok`. Evidência estática; a falha física não foi provocada. Propagar o erro e garantir desativação ou fechamento como recuperação.

Evidência própria da revisão anterior, não reexecutada neste registro:

- `cargo test -p clipsync-input --locked -q`: **319 passed, 2 ignored**. `cargo test -p clipsync-input --lib adapter::portal::tests --locked`: **7 passed**. Os testes do diff não cobrem o envio pendente do ARG-01.
- Harness Rust temporário compilado por stdin, com `EventSink` extraído e eventos sintéticos: `receptor antigo fechado = false` após detach/attach; evento pendente entregue ao receptor antigo; cancelamento do select não interrompeu o envio pendente. A sonda não abriu uma sessão EIS real.
- `scripts/conferir-privacidade.sh`: `privacidade dos hooks: ok (8 arquivos)`; `git diff --check`: saída 0. A leitura do diff não identificou novos logs de conteúdo ou keycode; o script cobre os hooks, não substitui a inspeção do adaptador Linux.

O caminho nominal preserva um único `ConnectToEIS` por sessão e reutiliza o link, conforme §13.15; os achados impedem aprovar o ciclo completo. `portal_loopback` permaneceu ignorado/manual. Nenhum diálogo de permissão foi aceito e não houve prova física Linux/Windows. Nenhum arquivo de código foi editado pelo revisor.


## S11 T2 · segurança · 04/11/2026 · 0db8a2d

Sentinel · segurança, revisão solicitada pelo Maestro · scaffold. Base: `0db8a2dd26ae134e98e30e764bb8f9d2aedc7738`, contrato §17 e plano de diagnóstico por log. **Não passa T2: um Important novo na exclusividade do escritor. Nenhum Critical/BLOCKER de segurança demonstrado.** S11-T1-SEG-02 encerrado no fluxo de produção; S11-T1-SEG-01 corrigido quanto a links preexistentes/permissões, mas mantém a corrida local já registrada. Não houve edição de fontes.

### Important · S11-T2-SEG-01 · sink sobrevive à reserva do agente

`crates/clipsync-core/src/logging.rs:16–50` mantém `Sink` em `OnceLock<Arc<FileState>>` global e só oferece abertura idempotente, sem fechamento. `agent::run_reserved` abre o sink em `agent.rs:113`, mas libera `control` e `operation` ao terminar (`agent.rs:271–273`) sem retirá-lo. O mesmo ocorre se uma falha após a abertura retorna por `?` ou se a tarefa é cancelada. A interface permite parar o agente sem sair do processo (`clipsync-ui/src/agent.rs:267–283`), e o layer permanece aceitando eventos do app/UI.

**Evidência executada:** em perfil temporário sintético, `agent::run` terminou com sucesso e removeu o socket; uma nova `Startup::reserve` do mesmo perfil teve sucesso; emitir um aviso sintético do target `clipsync_ui::auditoria` ainda acrescentou **144 bytes** ao arquivo do agente encerrado. Saída: `antes_do_run_sem_arquivo=true; run_encerrado=true; nova_reserva_possivel=true; escrita_apos_stop=true; crescimento=144 bytes`. A sonda usou o core real com lista vazia de módulos, sem adaptadores físicos, em loopback com porta efêmera; não abriu a interface nem o runtime do Maestro.

Consequência inferida do fluxo confirmado: um `clipsync run` externo pode adquirir a reserva já liberada e abrir seu próprio sink enquanto o app antigo continua escrevendo/rotacionando o mesmo conjunto de arquivos. A exclusividade de §17.1.3 deixa de valer; os tamanhos e renames de cada processo não são coordenados, podendo perder registros ou desvincular o arquivo ainda usado pelo outro escritor. Não foi executada rotação concorrente entre dois processos. **Correção necessária:** vincular o sink a um guard da execução do agente, fechar/desativar antes de liberar o listener em todas as saídas, inclusive erro/cancelamento, e cobrir parada, falha de bootstrap e tomada do perfil por outro processo. A abertura correta, sozinha, não garante o ciclo de vida exclusivo.

### Minor residual · S11-T1-SEG-01 · check/open continua separado

Os testes confirmaram que symlinks já presentes em `logs/` e no arquivo são recusados, com o destino preservado, e que objetos Unix existentes passam a 0700/0600. Contudo, `symlink_metadata` (`logging.rs:111–119,151,182–185`) precede `File::open`/`OpenOptions::open` (`163,193`) sem impedir substituição entre as operações; os renames também resolvem caminhos novamente. O comentário `109–110` reconhece a limitação. Não declarar o achado integralmente encerrado: a corrida continua Minor sob adversário local capaz de alterar entradas no perfil; sem vetor remoto ou ganho de privilégio demonstrado. Não foi construída nem executada uma corrida. Arquivo regular/proprietário/hardlinks também não são validados, como já delimitado em T1.

### Correções e fronteiras confirmadas

- **S11-T1-SEG-02:** a linha normal é limitada a 64 KiB, incluindo newline, e substituída por `log.truncated` com campos comuns e tamanho original; o conteúdo excessivo não sai no teste. O mesmo contador saturante recebe um descarte e para em 200. No fluxo de produção, `host` vem do nome validado de até 64 bytes e os targets são constantes curtas; isso mantém o registro substituto abaixo do limite. O cheque ocorre depois da serialização completa: é limite de linha persistida, não teto de memória. Falha persistente de rotação ainda permite crescimento sem teto, conforme §17.1.4.
- **Correções do Argus:** `reopen_pending` impede outra rotação enquanto a reabertura falha; os testes cobrem rename, reabertura e rollback, preservando `nlink=1`, linhas e recuperação. `evento=legado` cobre avisos sem evento estruturado; debug/trace são descartados mesmo com filtro `trace`. Isso não fecha o conflito entre dois processos descrito acima.
- **Ordem e CLI:** `logging::init()` instala os layers sem abrir arquivo. A CLI resolve `Profile` em `main.rs:169`; a UI resolve em `lib.rs:144`. `Startup::reserve/with_listener` reserva o controle e lê configuração; `run_reserved` lê identidade, pares e estado antes de `open_file`, que precede `Mesh::start`. A única chamada de produção a `open_file` é essa. Os comandos de consulta `status` e `log caminho` só chamam `log_path`, também quando o agente está parado. A exceção esperada é `clipsync run`, que executa o agente. Testes CLI/UI confirmaram ausência de `logs/` nas consultas sem agente.
- **Dados e compatibilidade:** `AgentStatus.log_path` e `AppState.log_path` têm `serde(default)`; o primeiro trafega pelo controle local, sem nova mensagem QUIC. Eventos novos do agente e dos pares preservam IDs curtos e campos anteriores; nenhum novo fluxo de conteúdo de clipboard, preview, tecla ou keycode foi identificado. O layer continua sem redação geral; T4 revisará a cauda no pacote.
- **Windows:** não há chmod/DACL explícita; a herança de ACL continua prevista. A checagem de link é compilada também nesse alvo. Na fonte do [Rust 1.99.1 para Windows](https://raw.githubusercontent.com/rust-lang/rust/1.99.1/library/std/src/sys/fs/windows.rs), `FileType::new` considera links os reparse points com bit name-surrogate, incluindo junctions; portanto não foi presumido um bypass de junction. Não houve execução Windows, prova de ACL ou garantia contra corrida local.
- **Contrato:** o registro de execução do plano menciona `legado`, `log.truncated` e piso `info`, mas o §17.2 ainda não lista esses eventos e §17.1.7 não explicita a restrição do nível. Cabe ao Maestro reconciliar o contrato; esta revisão não alterou seu arquivo.

### Evidência e limites

1. Fontes de `logging.rs` e `agent.rs` no checkout conferidas contra `git show 0db8a2d`: SHA-256 `a689e1efb874a9dc18deab1932131f12fbeb6fac9cc776057991d7ad002b802b` e `272a25d495de617e65b5ed36dfd01458238ef9739ccf87ffba66e813f5eb0e0d`, respectivamente; iguais. A revisão permaneceu no commit solicitado, apesar de a main já conter tarefas posteriores.
2. Executáveis de teste existentes: `target/debug/deps/clipsync_core-c36ca9101bd1b3af logging::tests --test-threads=1 --nocapture`: **12 passed**; `verificar_log-c223f15bb741480c --test-threads=1`: **2 passed**; `clipsync_proto-7a3761b07cade4f8 control::log_tests --test-threads=1`: **1 passed**; `clipsync_ui-cf27537477e25ea3 status::tests::caminho_do_log_com_agente_parado --test-threads=1`: **1 passed**. Não foi rebuild do workspace.
3. Harness temporário `/tmp/st2-8ri5_zmb/lifecycle.rs`, compilado com `rustc --edition=2024` e rlibs locais do core/proto/tokio/tracing/tempfile, selecionados pelo fingerprint. Comando completo em `compile-args.txt`; resultado em `result.log`. `CLIPSYNC_LOG=off /tmp/st2-8ri5_zmb/lifecycle`: **exit 0**, observações do Important acima. Não criou `target/` em `/tmp` nem acessou perfil real.
4. `bash scripts/conferir-privacidade.sh`: **privacidade dos hooks: ok (8 arquivos)**. O alcance desse script continua restrito aos hooks/tradução.

Não verificados: Tauri/Windows nativos, CI integral, tomada e rotação entre dois processos, corrida de symlink, uso físico ou tarefas posteriores à T2. O Important impede aprovar esta integração até correção e revisão do ciclo de vida do sink.


## S11 T2 · 0db8a2d · não passa · 04/11/2026

Argus, revisão funcional de `f7cb334..0db8a2dd26ae134e98e30e764bb8f9d2aedc7738`, conforme contrato §17 e Decisões do Maestro no plano. **Não passa: um Important novo; nenhum Critical demonstrado.** O achado converge com S11-T2-SEG-01, já registrado por Sentinel, e não constitui uma segunda causa.

1. **Important · S11-T2-ARG-01 · o escritor sobrevive à reserva do agente.** `crates/clipsync-core/src/agent.rs:113` abre o sink, mas o encerramento em `agent.rs:248–273` libera o controle e a reserva sem fechá-lo. `logging.rs:16–19,43–51` conserva o arquivo no estado global até o fim do processo. No app, parar o agente mantém o processo da interface vivo: outro agente pode reservar o perfil enquanto o assinante antigo ainda escreve e rotaciona o mesmo log. Isso viola o escritor único de §17.1(3); também alcança falhas de inicialização posteriores a `open_file`. Reproduzido com as APIs reais do core, perfil temporário e nenhum módulo: `agent::run` terminou, uma nova `Startup::reserve` teve sucesso e um aviso sintético do target `clipsync_ui` acrescentou **164 bytes** pelo escritor antigo. Vincular a posse do sink à reserva e fechá-lo antes de liberá-la, inclusive em erro/cancelamento.

**Revisão dos achados anteriores:** S11-T1-ARG-01/02/03 ficam atendidos no recorte revisado. `reopen_pending` impede a rotação de desvincular o descritor enquanto a reabertura falha; há teste de rename e de reabertura/rollback, incluindo recuperação. `legado` e piso `info` seguem a decisão explícita do Maestro. Conferidos também recusa de symlink existente, ajuste de permissões Unix, substituição de linha excessiva por `log.truncated` e contador único limitado a 200. O limite aceito da corrida local entre checagem e abertura não foi reaberto.

Caminho do log puro e comum aos consumidores; campos novos de status têm `serde(default)`; chamadas de produção a `open_file` ficam no agente, após a reserva; falha de abertura mantém stderr; CLI e app chamam `logging::init`. Essas propriedades não resolvem a duração excessiva do sink descrita acima.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-core --lib logging --locked`: **12 passed, 0 failed**.
- `cargo test -p clipsync-cli --locked --test verificar_log --test verificar_travessia`, com os mesmos limites de jobs/threads: **2/2** em `verificar_log`; o outro alvo pertence à T4.
- Harness Rust por stdin, ligado aos rlibs do core/proto/tokio/tracing/tempfile selecionados pelos artefatos do Cargo: **exit 0**, saída `run terminou; nova reserva obtida; escritor anterior ainda acrescentou 164 bytes ao log`. O harness usa perfil sintético curto e encerramento normal do agente; não executa Tauri nem concorrência entre dois processos.
- Fonte de `agent.rs` conferida contra `0db8a2d`; SHA-256 igual: `272a25d495de617e65b5ed36dfd01458238ef9739ccf87ffba66e813f5eb0e0d`. A suíte ampla 397/0 e o clippy constam dos logs de Rivet em `.superpowers/sdd/2026-11-04-diagnostico-por-log/`; não foram repetidos pelo revisor.

Windows/Tauri, corrida nativa de rotação e aceites físicos não verificados. Nenhum código alterado pelo revisor.


## S11 T3 · 9258ed6 · não passa · 04/11/2026

Argus, revisão funcional de `9258ed63a3b4e596c5d938aba0fbd06d2cac2ed7`, contrato §17 e decisões do plano para eventos e deduplicação. **Não passa: dois Important; nenhum Critical demonstrado.** Referências de linha abaixo são do commit solicitado; alterações posteriores de F2 em andamento não fazem parte deste parecer.

1. **Important · S11-T3-ARG-01 · a recusa do portal antes de start fica sem capture.failed.** Em `crates/clipsync-input/src/module.rs:1020–1029`, `cap.zones().await` pode falhar e retornar antes de entrar no motor de captura, onde foram acrescentados `capture.start/failed`. O adaptador portal cria a sessão em `zones()`, portanto a recusa de permissão pode ocorrer nesse ponto real. O código grava somente o aviso legado e `last_block`: o verificador não recebe o evento estruturado que distinguiria `portal_denied` de ausência de dados. Sonda com `CaptureAdapter` que retorna `PermissionDenied` em `zones`, executada pelo módulo real com MemHub: **capture.start=0, capture.failed=0**, enquanto o status informou **permissão do portal negada**. Registrar a falha também nessa preparação, sem alterar a mensagem da interface nem duplicar eventos do caminho que alcança `start`.
2. **Important · S11-T3-ARG-02 · capture.ready descreve um trecho diferente do instalado.** Em `crates/clipsync-input/src/capture.rs:333–345`, `trecho` vem de `Target.local`; contudo, `module.rs:972–988` recorta o canto morto e entrega ao adaptador `Target.spec.from/to`, mantendo `local` com o intervalo original para o mapeamento. Com monitor de altura 1080 e canto morto de 20 px, a sonda observou barreira instalada em **[0.018518519, 0.9814815]** e log **[0.0, 1.0]**. O log declara captura pronta numa parte da borda onde não há barreira, prejudicando a prova do problema de travessia. Gerar a evidência a partir da especificação efetivamente instalada e cobrir o recorte no teste. A sonda também mostrou `monitor="DESTINO"` junto à borda local; a fonte desse campo deve ficar explícita, pois hoje ele usa `peer_monitor`.

Conferido: `inventory.sent` só aparece após envio bem-sucedido; inserção aceita precede `inventory.received`; `inventory.prepared` segue o catálogo local; rotas são emitidas uma vez por resultado conforme a decisão; nomes dos motivos e ids curtos estão presentes; `capture.ready` exige targets não vazios; entradas legadas e v4 têm nome de mensagem, sessão e par. O teste de recusa existente falha em `start`, portanto não cobre ARG-01. O teste de captura verifica lista não vazia e par, mas não compara seu intervalo com o instalado, deixando ARG-02 passar.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **4 passed, 0 failed**.
- Harness temporário por stdin, baseado no harness de `tests/layout_capture.rs`, usando o módulo real, FakeCapture/MemHub e o coletor `tests/support/logs.rs`: comparação acima entre `FakeCaptureHandle::barriers()` e `capture.ready`. A configuração válida de 20 px reproduziu o achado; uma primeira preparação de sonda usou 100 px e foi corretamente recusada pela validação, sem constituir falha do produto.
- Segunda sonda com erro em `zones`: **exit 0**, observação `zones recusou; capture.start=0; capture.failed=0` e status bloqueado por permissão.
- Uma recarga de layout também foi exercitada: a barreira mudou e a contagem de `capture.ready` passou de 1 para 2. Não foi aberto achado de ausência de evento nessa recarga.

Provas sintéticas, sem sessão de portal, consentimento do sistema ou mouse físico. A suíte completa do input/clippy do autor não foi repetida. Nenhum código alterado pelo revisor.


## S11 T4 · 65ca7c6 · não passa · 04/11/2026

Argus, revisão funcional de `65ca7c6b3642413286478ad7fe2d221a84a0b9d3`, limitada ao esqueleto de cinco passos da T4. **Não passa: três Important; nenhum Critical demonstrado.** Duração relativa de `--desde`, leitura de arquivos rotacionados e os seis passos posteriores continuam atribuídos à F3; sua ausência não foi tratada como defeito desta tarefa.

1. **Important · S11-T4-ARG-01 · captura e rotas de outro par aprovam a sessão selecionada.** `crates/clipsync-cli/src/verificar.rs:242–251` filtra os eventos de inventário pelo par, mas `verificar.rs:278–311` aceita o último `routes.built` e qualquer lista não vazia de `capture.ready`, sem conferir `pares` ou `barreiras[].peer`. Fixture: `--sessao 7` aponta para `bb`; as únicas rotas/barreiras registradas levam a `aa`. Resultado executado: **rotas=ok, captura=ok, tentativa=ok, quebra=null, exit 0**. Isso atribui uma prova de captura à máquina errada. Exigir correspondência entre o par selecionado e as rotas/barreiras usadas como evidência.
2. **Important · S11-T4-ARG-02 · evidência anterior ao reinício continua válida.** Em `verificar.rs:206–240`, a seleção limita os pré-requisitos apenas pelo teto temporal da tentativa e ignora os limites entre execuções do agente; também procura `attempt` no arquivo inteiro, embora o contador reinicie em 1. Fixture com inventário/rotas/captura prontos, depois novo `agent.up` e tentativa/sessão sem nova captura: o comando devolveu **todos os cinco passos ok, quebra=null, exit 0**. O arquivo é append entre execuções, portanto uma captura antiga não prova que o agente atual a abriu. Correlacionar tentativa, sessão e pré-requisitos dentro da mesma execução, com marcador de início emitido antes dos produtores; não reutilizar silenciosamente prova antiga.
3. **Important · S11-T4-ARG-03 · decisão vigente sobre inventário não recebido não é aplicada.** Em `verificar.rs:270–274`, inventário preparado no destino sem `inventory.received` na origem vira `SemDado`. A seção Decisões do Maestro do plano exige `Falhou` nesse caso. Fixture executada: motivo **preparado no destino, não recebido na origem**, mas **quebra=null e exit 0**; o teste `preparado_no_destino_sem_recebimento_explicita_lacuna` confere só o motivo. Aplicar a decisão vigente e testar estado, primeiro ponto de quebra e código de saída. O ledger de Rivet registra uma interpretação anterior mais ampla de `SemDado`; este parecer segue a seção do plano indicada pelo Maestro nesta revisão.

Conferido no escopo: comando offline sem abrir perfil/socket; leitura de JSONL e rejeição de JSON inválido/versão futura; cinco passos com linha de prova; captura falha com motivo; saída JSON com os mesmos estados do texto; tentativa ausente é `SemDado`, motivo explícito e código 0 conforme a decisão do Maestro. As conclusões positivas continuam bloqueadas pelos achados acima.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-cli --locked --test verificar_log --test verificar_travessia`: **7 passed, 0 failed** no alvo `verificar_travessia`.
- Três fixtures JSONL temporárias, invocadas pelo binário `target/debug/clipsync verificar-travessia --origem ... --destino ... --json`; os casos de par errado e reinício usaram `--sessao 7` e `--sessao 8`, respectivamente. Saídas observadas descritas nos achados; só dados sintéticos, sem acesso ao agente.
- `git show 65ca7c6:crates/clipsync-cli/src/verificar.rs | sha256sum` e o arquivo usado pelo comando conferiram: `1e42888b6f5da1eee82206f6ec9607abd5b609e3dfd8476d96b78c163a42baf3`. As edições posteriores no teste durante a revisão não foram incluídas.
- `git diff --check` saiu 0 ao fim da inspeção. Os logs de suíte CLI e clippy do autor foram lidos; não substituem os cenários adicionais acima.

Este registro não aprova F1 nem a travessia física. Nenhum arquivo de código foi editado; os três pareceres foram acrescentados a `docs/verdicts.md`, preservando as revisões anteriores.


## S11 T5+T6 · 89aba99 · não passa · 04/11/2026

Argus, revisão de `89aba99465f01f2fa44004db7c7a7a617be5d367`, origem contínua e loopback, contrato §17 e decisões do Maestro no plano. **Não passa: seis Important e um Minor; nenhum Critical demonstrado.** As referências são do commit solicitado; o input conferiu sem diferenças contra ele durante as sondas.

1. **Important · S11-T56-ARG-01 · abertura falha fecha duas vezes no loopback.** Em `crates/clipsync-input/src/module.rs:2152`, o erro do envio emite `session.closed`; o epílogo em `module.rs:2291` emite outro para a mesma sessão. Reproduzido com host que devolve `SendError::Transport` apenas para Enter: **dois fechamentos, mesmo session**, e o primeiro sem `totais`, obrigatório no §17.2. Centralizar a emissão no encerramento, preservando o código do erro e os totais.
2. **Important · S11-T56-ARG-02 · motivo ausente ou herdado de outra operação na captura contínua.** `capture.rs:857–867` encerra por bloqueio do lifecycle sem marcar `lock`; `capture.rs:1265` também encerra Deactivated sem motivo. `close_session` retorna cedo sem limpar `close_reason` quando não há sessão (`capture.rs:905–918`). Reproduções: bloqueio pelo canal de lifecycle fechou com **stop**; Deactivated também saiu **stop**; ativar/desativar InputLock enquanto ocioso e depois abrir/encerrar outra sessão por Deactivated saiu **lock**, apesar de já destravado. Há ainda chamadas sem motivo em observação/invalidação de inventário, PrepareInventory e Home. Passar o motivo explicitamente ao encerramento, em vez de estado opcional reaproveitado.
3. **Important · S11-T56-ARG-03 · loopback perde recusa e confunde trava/parada com erro.** `module.rs:2120–2125` reduz os sinais a texto; `module.rs:2283–2291` reconstrói o motivo comparando mensagens e cai em `error`. Reproduzido: `Rejected(NotAuthorized)` fechou com **error**, sem nenhum `session.refused`; InputLock durante sessão ativa também fechou com **error**. O mesmo ramo Stop atende o encerramento do módulo. Preservar a causa tipada até o epílogo e emitir a recusa com seu código; não diagnosticar cancelamento/trava como erro de captura/transporte.
4. **Important · S11-T56-ARG-04 · primeiro envio bem-sucedido pode nunca gerar datagram.first.** Em `capture.rs:1314–1322`, `first` depende de `seq == 0`, mas a sequência avança mesmo na falha. Sonda com primeiro datagrama recusado e segundo aceito: **enviados=1, descartados=1, datagram.first=0**. A cadeia deixa de provar transmissão que aconteceu. Emitir uma vez no primeiro sucesso, conforme T6, independentemente de tentativas anteriores.
5. **Important · S11-T56-ARG-05 · loopback informa envios que falharam e não emite o primeiro datagrama.** Em `module.rs:2223–2234`, o resultado de `send_datagram` é descartado; `module.rs:2291` usa `seq` como enviados e fixa descartados em zero. Sonda com um único datagrama, que falhou: **enviados=1, descartados=0**. O ramo também não contém `datagram.first`, mesmo no sucesso. Contar os resultados reais e instrumentar o primeiro sucesso como na captura contínua.
6. **Important · S11-T56-ARG-06 · attempt é reutilizado dentro da mesma execução.** O contador nasce em zero a cada motor de captura (`capture.rs:498`), enquanto `drive` usa `new_session()` como tentativa (`module.rs:2058`), contrariando §17.2(5). Reproduzido sem reiniciar o agente: travessia contínua, loopback sem ativação, nova travessia produziram **barrier/1, test/4167026413, barrier/1**. Duas sessões distintas ficam sob a mesma tentativa e `--tentativa 1` fica ambíguo. Manter o contador na origem durante a vida do agente, incluindo os três gatilhos.
7. **Minor · S11-T56-ARG-07 · pos continua em fração, mas sem as três casas exigidas.** `capture.rs:1205`, `capture.rs:1415/1445` e `module.rs:2184` convertem diretamente f32 para f64. A ativação sintética em 0.7777778 saiu **0.7777777910232544**. Arredondar somente o campo de log para a precisão do §17.2(3), preservando a coordenada usada pelo runtime.

Evidência própria: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **11 passed, 0 failed**. Harness compilado por stdin com `rustc --edition=2024 --test`, testes de `89aba99`, coletor existente e rlibs selecionados pelos artefatos do Cargo: **10 sondas passed, 0 failed**, em `/tmp/argus-t56-jztl7x3p/check review_ --test-threads=1 --nocapture`. São asserções de observação dos comportamentos acima, não testes que aprovam o produto. O host de falha delega ao MemHub e só injeta falha de Enter/datagrama. A rodada inicial teve uma hipótese de desconexão refutada: MemHub unlink fechou corretamente com **peer_down**; ela foi corrigida no harness e não consta como falha reproduzida.

Conferido por leitura: os campos novos não serializam evento de entrada, tecla, keycode ou conteúdo; os caminhos de posição examinados usam fração, com a ressalva de precisão acima. Nos cenários executados todas as sessões abertas chegaram a um fechamento, mas não se sustentam unicidade nem motivo correto. A decisão de substituir/dispensar testes de falha com os falsos não elimina os defeitos demonstrados por um host delegador. Suíte ampla/clippy/fmt do autor não foram repetidos; portal real, Windows nativo e travessia física não foram verificados. Nenhum arquivo de código editado.


## S11 T2-bis · segurança · 04/11/2026 · ba0fcbd

Sentinel · segurança, re-revisão de `ba0fcbda940fbfd28e6e3be0b9624460d5786eae`, restrita ao ciclo de vida do escritor e ao Important S11-T2-SEG-01. **PASSA: S11-T2-SEG-01 encerrado. Nenhum novo Critical, Important ou BLOCKER de segurança identificado no delta.** O Minor residual S11-T1-SEG-01, corrida local entre checagem de caminho e abertura/rename, permanece conforme o parecer anterior; não é objeto desta correção. Este resultado não libera o release nem substitui os demais gates.

1. **Fechamento anterior à liberação do perfil.** `RunResources` (`crates/clipsync-core/src/agent.rs:57–60`) mantém os campos na ordem `_log`, `control`, `operation`. A parada normal executa `drop(resources)` em `291`; os retornos por erro e o cancelamento também descartam essa estrutura. Campos de struct são destruídos na ordem de declaração, conforme a [referência de Rust](https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation). Não há await entre abrir a guarda e colocá-la em `RunResources`. Configuração, identidade, pares e estado continuam anteriores à abertura. Se falham antes dela, nenhum sink foi criado.
2. **Nenhuma escrita após o fechamento e antes do listener cair.** `FileLease::drop` (`logging.rs:52–64`) usa o mesmo mutex de `write_event` (`116–123`), confere o proprietário e retira o sink com `*sink = None`, fechando o arquivo ainda dentro do mutex. Uma escrita já em andamento termina antes dessa retirada; uma chamada posterior encontra `None`, incrementa o contador saturante e retorna sem I/O de arquivo. O sink não é clonado nem escrito fora desse mutex no caminho de produção. A única chamada de produção a `open_file` continua em `agent::run_reserved`; não há outra guarda de produção prolongando o lease. A conclusão sobre o intervalo de destruição vem da ordem de drop e da exclusão pelo mutex, não de uma medição temporal do intervalo.
3. **Distinção das reservas.** O `ProfileOperationGuard` de inicialização continua sendo liberado na readiness (`agent.rs:248`), como antes, para permitir as operações normais do perfil. A exclusividade entre agentes durante a execução é conservada pelo listener, que permanece em `RunResources` até depois de fechar o log. Nos erros de bootstrap, a guarda de operação ainda presente também cai depois do log. Não se afirmou que o lock de operações fica preso durante toda a execução.
4. **Tomada externa executada com outro processo.** Em `tests/log_lifecycle.rs:165–193`, o processo antigo encerra o agente; um filho independente reserva o mesmo perfil e abre seu próprio sink, sinalizando `PERFIL_ADQUIRIDO`. Só então o antigo emite `ui.depois`; os bytes continuam iguais. Após o sinal do pai, o filho grava `externo.escreveu`, cuja presença é conferida. O contador de descarte do antigo aumenta. Esse teste passou: cobre dois processos e dois estados globais reais, não apenas duas reservas no mesmo processo. O filho usa `Startup::reserve` mais `open_file` diretamente, sem iniciar uma segunda malha ou adaptador físico.
5. **Outras saídas e contrato.** Passaram parada/reinício, recusa controlada no bootstrap, falha precoce de `Mesh::start` por porta ocupada e cancelamento com `task.abort()` seguido de espera pelo término. Em todos, o arquivo fica estável após o fim do agente e eventos voltam ao contador. O §17.1 agora registra a guarda, os descartes após fechar, `legado`, `log.truncated`, 64 KiB e piso `info`, encerrando também a divergência documental apontada na T2.

**Evidência própria:** `target/debug/deps/log_lifecycle-0e733930fc5a77ba --test-threads=1 --nocapture`: **6 passed, 0 failed**, cinco cenários mais o harness; `target/debug/deps/clipsync_core-c36ca9101bd1b3af logging::tests --test-threads=1 --nocapture`: **12 passed, 0 failed**. Foram reexecutados os binários existentes, sem rebuild geral. Os três fontes conferidos são byte a byte iguais a `git show ba0fcbd:<caminho>`: `logging.rs` SHA-256 `922b6cb37ab152bf66ef5a54e8d72ebce33a893badc976cf230ac9daeda9e475`; `agent.rs` `cef9960abea81f2402496ab67540eeb1f0f7f89f2c85ad15afbdf679266b71e7`; `tests/log_lifecycle.rs` `3a46a8f83cf7ac44f9cb9821ae18c102e62434e2e76563f12865b660db3122fd`.

**Limites:** não reexecutei os 110 testes da suíte completa, clippy, fmt, Windows nativo, CI/Tauri ou prova física; os resultados globais informados pelo autor não foram apresentados como execução própria. A tomada externa testada não força rotação concorrente nem injeta eventos exatamente dentro do destructor. A análise do mutex e da ordem dos campos fecha a causa do Important. Apenas esta entrada foi acrescentada, preservando integralmente os pareceres anteriores e as alterações pendentes de Argus, previamente comunicadas ao Maestro.


Nota do Argus sobre a referência do parecer S11 T5+T6: a emissão de `barrier.activated` do loopback citada em S11-T56-ARG-07 está em `module.rs:2182` no commit `89aba99` (a linha 2184 é o ramo seguinte). O achado e a evidência permanecem os mesmos.

## S11 T2-bis · ba0fcbd · passa · 04/11/2026

Argus, revisão de `ba0fcbda940fbfd28e6e3be0b9624460d5786eae`, limitada à duração do escritor, encerramento do agente e regressão de rotação. **Passa neste recorte: nenhum Critical, Important ou Minor encontrado. S11-T2-ARG-01 atendido.** `agent.rs`, `logging.rs` e `tests/log_lifecycle.rs` usados na execução conferiram sem diferenças contra o commit.

1. **Posse e término:** `crates/clipsync-core/src/agent.rs:57–60` reúne log, listener e operação em `RunResources`, com o log descartado primeiro. As leituras falíveis anteriores à construção não abrem escritor; o erro de Mesh, a recusa de bootstrap/readiness, o retorno normal e o cancelamento posterior descartam a guarda junto com os recursos. `serve_reserved` mantém o endpoint até a limpeza do agente. A única chamada de produção a `logging::open_file` continua em `run_reserved`; não há guarda clonada/transferida para tarefa ou camada global.
2. **Última escrita e fechamento:** `logging.rs:52–63` remove o sink sob o mesmo mutex usado pela escrita; o estado guarda somente Weak da posse, portanto não conserva a guarda. Chamadas repetidas no mesmo perfil compartilham a posse explicitamente, a última fecha, e eventos posteriores voltam ao contador limitado. Reabertura após o fim também foi exercitada, com host novo.
3. **Rotação:** a mudança de `Sink` acrescenta somente a referência fraca da posse; o algoritmo de rotação/reabertura foi preservado. Os testes de limite, rename recusado e reabertura/rollback falhos continuaram aprovados.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-core --locked --test log_lifecycle`: **6 passed, 0 failed**. Cobre parada/reinício, falhas de bootstrap e Mesh, cancelamento após readiness e tomada por outro processo sem escrita do processo anterior.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-core --lib logging --locked`: **12 passed, 0 failed**, incluindo os quatro cenários de rotação.
- Harness por stdin, `rustc --edition=2024`, rlibs selecionados pelos artefatos do Cargo, binário `/tmp/argus-bis-mdhcl8__/check`: **exit 0**. Readiness abandonada e cancelamento enquanto o módulo retinha a resposta de bootstrap permitiram nova reserva e acrescentaram **zero bytes** após o fim. Duas guardas no mesmo perfil mantiveram a escrita até a última; quatro threads emitindo durante/depois do último drop não alteraram o arquivo após esse drop; reabertura funcionou e o contador permaneceu em no máximo 200.

Aceite funcional em Linux com perfis sintéticos; não é execução nativa de Windows/Tauri. A suíte core 110/110 informada por Rivet não foi repetida integralmente. Nenhum código editado; registro somente por acréscimo em `docs/verdicts.md`.


## S11 T7+T8 · 239b715 · não passa · 04/11/2026

Argus, revisão de `239b7153e5dda50d137c879479b1fd8e44479a07`, destino, recusas, emulação, retorno e portal. **Não passa: quatro Important e dois Minor; nenhum Critical demonstrado.** Referências abaixo são do commit solicitado. As cinco sondas e o teste s11_log rodaram antes das edições concorrentes posteriores do input; os arquivos foram conferidos sem diferenças contra esse commit naquele ponto.

1. **Important · S11-T78-ARG-01 · return.sent afirma envio que pode falhar.** Em `crates/clipsync-input/src/module.rs:1953/1961`, o evento precede até `release_all` e a chamada a `send_v3`; esta apenas agenda uma tarefa, cujo resultado é descartado por `network.rs:32`. Host sintético que falha somente ao enviar Returned: **return.sent=1, chamadas de envio=1, retornos recebidos pela origem=0**. A linha não prova transmissão e pode atribuir ao outro lado uma falha local. Emitir `return.sent` após sucesso do envio e tornar falha/timeout/enfileiramento observável com sessão e motivo.
2. **Important · S11-T78-ARG-02 · a segunda sessão não tem emulate.start.** `module.rs:1678–1680` e `module/organization.rs:567–568` emitem início somente quando o adaptador ainda não começou. Ele é reutilizado entre sessões. Sonda com sessões 81 e 82, separadas por Leave: **emulate.start=[81], emulate.positioned=[81,82]**. A segunda sessão válida não completa a cadeia por sessão exigida no §17.3 vigente e pelo verificador, que exige início e posicionamento. Registrar o início do uso do emulador para cada sessão, sem reabrir o portal/adaptador.
3. **Important · S11-T78-ARG-03 · fila cheia registra recusa sem sequer agendar o fio.** `module.rs:1805` emite `enter.refused` antes de `network.spawn`; a recusa de capacidade em `module.rs:1822–1823` só altera `last_block`. Com 32 envios de recusa suspensos e uma 33ª entrada v4 com monitor ausente, a sonda observou **33 enter.refused, 32 chamadas de envio**: a sessão 132 tem `monitor_not_found` no log e nenhuma tentativa de Refused. O helper já tinha essa limitação antes da T7; o novo caminho também a herda, contrariando a correspondência solicitada entre log e fio. Distinguir decisão de recusa de notificação enviada e registrar falha de enfileiramento/envio sem afirmar entrega.
4. **Important · S11-T78-ARG-04 · falha durante a emulação de datagramas fica fora de emulate.error.** Em `module.rs:1921–1932`, erro de `emu.emit` após posicionar não produz o evento estruturado adicionado pela T8. No legado, erro diferente de Locked continua apenas como aviso legado. Sonda: emulador iniciou/posicionou e falhou no primeiro movimento recebido; **datagram.first=1, emulate.error=0, enter.refused=0, novos eventos aplicados=0**. A evidência estruturada não distingue esse destino quebrado de emulação bem-sucedida. Emitir `emulate.error` com sessão, adaptador e causa também nesse ramo, preservando o comportamento existente do fio.
5. **Minor · S11-T78-ARG-05 · falha de ConnectToEIS do RemoteDesktop não tem evento do portal.** Em `adapter/portal.rs:886–895`, o erro sai por `map_err(map_err)?` antes do único `portal.connect_eis`; o InputCapture ganhou ramo de erro, mas o RemoteDesktop não. Há `emulate.error` no chamador, portanto não é ausência total de diagnóstico; falta a prova específica prometida para o resultado da conexão. Cobrir ambos os resultados com o motivo aprovado pelo Maestro. Achado por leitura, sem acionar diálogo ou portal real.
6. **Minor · S11-T78-ARG-06 · total de perdidos pode ser zerado por duplicata.** O novo `log_return_sent` publica diretamente o contador existente de `Incoming`; em `module.rs:1888–1890`, qualquer sequência antiga reduz `lost`, mesmo uma duplicata que não preenche a lacuna. Sonda com **0,2,2,3**, sem receber 1: retorno registrou **recebidos=4, perdidos=0, fora_de_ordem=1**. A falha de contagem antecede este commit, mas agora aparece como total no diagnóstico. Distinguir duplicata de preenchimento de lacuna ou documentar explicitamente a estimativa; não apresentar zero como prova de ausência de perdas.

**Decisões respeitadas e T3:** a omissão de `ignorados` por sessão e dos eventos de hooks Windows, assim como os retornos silenciosos por inventário/autorização alterados durante a entrada, estão explicitamente mantidos pelo Maestro no plano; não foram transformados em bloqueadores novos. A lista de campos de `return.sent` no §17.2 ainda precisa refletir a exceção de `ignorados`. A precisão de `pos` do retorno repete S11-T56-ARG-07. No commit revisado, S11-T3-ARG-01/02 continuam abertos: `zones()` ainda retorna sem `capture.failed` antes do motor, e `capture.ready` ainda publica `Target.local`, não o trecho recortado instalado. Os novos eventos do portal/módulo não corrigem essas duas causas; a correção posterior será objeto de T3/T5/T6-bis.

**Conferido:** monitor ausente em EnterRoutes gera `enter.refused{monitor_not_found}` e `Refused{ApplyFailed}` no caminho sem falha de transporte, sem iniciar o emulador; o teste existente verificou ambos. Todas as recusas explícitas de `State` passam pelo helper instrumentado. Descartes de sessão/contexto/stream inválidos não foram confundidos com recusas enviadas. `datagram.first` do destino está depois das conferências de autorização, sessão ativa, conexão, época e limite de taxa; os eventos examinados usam a sessão do Incoming/datagrama correspondente. `recebidos` conta eventos, não pacotes, como o contador original. Os campos novos do portal não recebem clipboard, tecla, keycode, eventos de entrada ou token de restauração; os erros são de criação/conexão. Essa inspeção não substitui execução do portal real.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **15 passed, 0 failed**.
- Harness por stdin, `rustc --edition=2024 --test`, fonte dos testes de `239b715`, coletor existente e rlibs selecionados pelos artefatos Cargo: `/tmp/argus-t78-paib8t4g/check review_ --test-threads=1 --nocapture`: **5 passed, 0 failed**. São sondas que confirmam os comportamentos descritos, não aprovação do produto; todos os dados são sintéticos.
- `bash scripts/conferir-privacidade.sh`: **privacidade dos hooks: ok (8 arquivos)**; o script não cobre todos os eventos de portal, que tiveram inspeção separada.
- Uma execução posterior de `s10_routes` passou 5/5, mas já encontrou edições concorrentes em `capture.rs` e aviso de recursão em `frac3`; por isso não é evidência atribuída a `239b715` nem altera este parecer.

Suíte ampla/clippy/fmt do autor não repetidos; Windows, portal físico e travessia entre máquinas não executados. Somente acréscimo neste documento; nenhum código editado.


## S11 T9 · f5e55aa · não passa · 04/11/2026

Argus, revisão de `f5e55aac400f2bbabe8d8584136a16727a230765`, verificador completo e ordem de `agent.up`, conforme §17.3 vigente. **Não passa: dois Important; nenhum Critical demonstrado.** Código da CLI, teste do verificador, `agent.rs` e `startup_log.rs` conferidos contra o commit; as mudanças concorrentes no input não fazem parte deste parecer.

1. **Important · S11-T9-ARG-01 · eventos do destino são correlacionados só pelo número da sessão.** `crates/clipsync-cli/src/verificar.rs:365–368` filtra por `session` sobre o arquivo inteiro do destino, sem par de origem nem limite de execução/tentativa. O corte de `remote_boot` protege apenas os pré-requisitos, não os seis passos finais. Fixtures válidas: trocar o peer dos eventos da sessão por **cc**, quando o agente de origem é **aa**, manteve **onze ok, quebra=null, parada=null, exit 0**; mover todos esses eventos para antes de um novo `agent.up` no destino, deixando só inventário preparado na execução atual, deu o mesmo sucesso. Na direção inversa, acrescentar uma recusa de **cc** a uma cadeia correta de **aa** gerou **entrada_destino=falhou, exit 1**. `session` não identifica sozinho um par/uma execução, e os receptores mantêm sessões por peer. Correlacionar identidade, sessão e intervalo da execução do destino antes de usar sucesso ou recusa como prova.
2. **Important · S11-T9-ARG-02 · troca de sessão esconde o fechamento anterior.** `verificar.rs:319–325` encerra a janela no próximo `attempt.begin`. A origem real registra a nova tentativa antes de `open` fechar a sessão anterior com `switch`. Fixture desse fluxo: tentativa 1 abre sessão 7; tentativa 2 começa; sessão 7 fecha com switch; sessão 8 abre. Com `--sessao 7`, o verificador devolveu **fechamento=sem_dado**, embora a linha de fechamento esteja no arquivo, e perdeu o motivo da interrupção. Manter a separação das tentativas para abertura/pré-requisitos, mas buscar o encerramento da sessão selecionada até seu fechamento ou limite de execução, sem atribuí-lo à sessão seguinte.

**T4 reavaliada:** S11-T4-ARG-01 foi atendido para rotas/barreiras dos pré-requisitos; os testes agora conferem o par. S11-T4-ARG-02 foi atendido nos pré-requisitos e no marcador de início antes dos runners; ARG-01 acima trata a correlação ainda incompleta dos novos passos do destino. S11-T4-ARG-03 deixa de bloquear porque o §17.3 vigente determina `SemDado` para inventário preparado sem recebimento na origem, mantendo `Falhou` para `inventory.missing`; a implementação acompanha a decisão atual.

Conferido no restante do recorte: onze nomes de passos na ordem canônica; ausência de evento vira lacuna/parada, não falha inventada; motivos explícitos sustentam as quebras nos casos testados, com a ressalva de pertencimento do ARG-01. Sem tentativa há cinco pré-requisitos; sessão pedida inexistente não retorna sucesso; interrupções conhecidas preservam motivo quando o fechamento está na janela. `--desde` relativo usa o maior timestamp da origem, o corte é aplicado aos dois arquivos, e `--ultima`/sessão explícita têm testes de contador reiniciado. A CLI distingue 0/1/2/3 nos cenários de sucesso, cadeia incompleta/falha, uso inválido e arquivo/esquema inválido. A leitura só abre os caminhos informados e não procura .1/.2 implicitamente. Nenhum socket/perfil é aberto pelo verificador.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-cli --locked --test verificar_travessia`: **23 passed, 0 failed**.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-core --locked --test startup_log`: **1 passed, 0 failed**; observou `agent.up` antes do produtor de inventário em runtime multithread.
- Cinco cenários adicionais com JSONL sintético e `target/debug/clipsync verificar-travessia ... --json`, em `/tmp/argus-t9-irup3hb0`: controle completo **exit 0**; outro peer e sessão anterior ao reinício **exit 0 indevido**; recusa de outro peer **exit 1 indevido**; troca de sessão com `--sessao 7` **exit 1 com fechamento falsamente ausente**. Os resultados e as linhas usadas estão descritos nos achados.

A compilação da CLI encontrou um aviso de recursão em `frac3` nas alterações concorrentes do input, fora de `f5e55aa`; o verificador offline testado não executa essa função. A árvore depois corrigiu o aviso. Suítes amplas, clippy, Windows e aceites físicos não repetidos. Somente acréscimo em `docs/verdicts.md`.


## S11 T3/T5/T6-bis · 188b10d · não passa · 04/11/2026

Argus, re-revisão de `188b10ded5f1b53d593c537ed4d6f3713668d5dd`, substituindo a identificação inicialmente ausente nas mensagens do Maestro. **Não passa: três Important residuais; nenhum Critical demonstrado.** Os arquivos de input conferiram sem diferenças contra o commit ao executar as sondas. O aviso temporário de recursão em `frac3` observado antes do commit foi corrigido e não integra os achados abaixo.

1. **Important · S11-T356B-ARG-01 · Deactivated do loopback ainda sai como retorno bem-sucedido.** `crates/clipsync-input/src/module.rs:2327–2329` atribui `close="stop"`, mas não preenche `failure`; o epílogo em `module.rs:2375` substitui qualquer motivo por `return` quando não há falha. Sonda com sessão ativa seguida de Deactivated: **session.closed.motivo=return**, apesar do ramo escolher stop. Preservar o motivo escolhido, distinguindo prazo normal de desativação; pelo §17.3, return é encerramento de sucesso e stop é interrupção.
2. **Important · S11-T356B-ARG-02 · trava pelo comando continua sem motivo lock no loopback.** `module.rs:663–666` traduz InputLock(true) em `LoopSignal::Stop`; `module.rs:2198` agora grava stop, perdendo a causa da trava. A mesma reprodução de S11-T56-ARG-03 passou de **error** para **stop**, mas não para **lock**. A recusa do destino foi corrigida; a trava ainda precisa conservar sua identidade até o consumidor do sinal.
3. **Important · S11-T356B-ARG-03 · erro terminal de captura é registrado como trava.** `capture.rs:869–884` trata todo snapshot que não permite captura como lock, inclusive `terminal_error` com tela e desktop disponíveis. Sonda com `session_locked=false`, `desktop_available=true` e erro terminal sintético: **session.closed.motivo=lock**. O verificador passa a mostrar interrupção por trava onde houve falha do adaptador. Priorizar erro terminal e sua causa antes da classificação de bloqueio. A correção de S11-T56-ARG-02 removeu a herança do motivo, mas não tornou todos os motivos corretos; também permanecem chamadas sem motivo explícito em observação/invalidação de inventário, identificadas por leitura.

**Achados atendidos neste recorte:**

- **S11-T3-ARG-01/02:** recusa em zones gerou `capture.failed{portal_denied}` antes do motor, sem precisar de `capture.start`; o teste novo compara o trecho de `capture.ready` com `FakeCaptureHandle::barriers()`, incluindo canto morto.
- **S11-T56-ARG-01:** falha de Enter no loopback produziu **um** `session.closed`, com totais.
- **S11-T56-ARG-04/05:** na captura contínua, falha seguida de sucesso produziu **enviados=1, descartados=1, datagram.first=1**; no loopback, um único envio falho produziu **enviados=0, descartados=1**. A emissão do primeiro sucesso do loopback foi conferida no ramo Ok.
- **S11-T56-ARG-06/07:** travessia contínua, loopback e nova travessia geraram tentativas **1,2,3**, sem reiniciar o contador; posição 0.7777778 saiu **0.778**. O arredondamento é restrito ao campo de log.
- **Partes de S11-T56-ARG-02/03:** ativar/desativar a trava enquanto ocioso não contamina mais a sessão seguinte; bloqueio real pelo lifecycle fecha com lock; Deactivated contínuo fecha com stop; recusa do loopback emite **um session.refused** e fecha com **refused**. O unlink do MemHub segue fechando com peer_down. Os três resíduos acima impedem encerrar integralmente esses dois achados.

Evidência própria:

- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **18 passed, 0 failed**.
- Harness por stdin, `rustc --edition=2024 --test`, fonte dos testes de `188b10d` e rlibs selecionados pelos artefatos Cargo: `/tmp/argus-t56-l2t2arpi/check review_ --test-threads=1 --nocapture`: **13 passed, 0 failed**, cobrindo as observações acima. Algumas sondas afirmam a presença dos resíduos; o total verde do harness não aprova esses comportamentos.

O aceite parcial não encerra os pareceres T7+T8 ou T9. Suíte ampla, clippy/fmt, Windows e portal físico não repetidos pelo revisor. Nenhum código alterado; somente esta entrada acrescentada ao documento.


## S11 T7/T8-bis · b7b70aa · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, re-revisão de `b7b70aa02e1c7344ddb4814c623fb0adf1a9dac0`. **Não passa: dois Important; nenhum Critical demonstrado.** Input e contrato usados nos testes conferiram sem diferenças contra o commit. A re-revisão de `188b10d` já está registrada na entrada S11 T3/T5/T6-bis: T3-ARG-01/02 atendidos, com três resíduos de motivos de encerramento; esta alteração do destino não os corrige.

1. **Important · S11-T78B-ARG-01 · timeout remove o envio antes do evento de resultado.** `crates/clipsync-input/src/module.rs:1661` e `module.rs:1864` colocam o log de retorno/recusa dentro da tarefa, depois do await de envio. `network.rs:33–34` envolve essa tarefa em timeout de dois segundos e descarta o resultado, cancelando justamente o trecho que emitiria o diagnóstico. Reproduzido com host que mantém o envio pendente: após 2,2 s, um retorno tinha **return.sent=0 e return.failed=0**. Em outra sonda, 32 recusas pendentes mais uma rejeitada por fila cheia geraram somente **um enter.refused**, da sessão 132 com `enviada=false, erro=network_busy`; as outras 32 continuaram sem evento após o timeout. Falha imediata e fila cheia foram corrigidas, mas S11-T78-ARG-01/03 permanecem parcialmente abertos. Registrar o resultado do limite de espera/cancelamento no mesmo ponto que o controla, garantindo um desfecho por operação.
2. **Important · S11-T78B-ARG-02 · return.failed ainda não é consumido pela T9.** O novo evento vinculante do §17.2 não aparece em `crates/clipsync-cli/src/verificar.rs`; o passo retorno em `verificar.rs:461` só procura `return.received` e `return.sent`. Fixture com toda a cadeia até datagramas, seguida de `return.failed{session:7,peer:aa,erro:transport}` no destino e fechamento stop na origem: **retorno=sem_dado, linha=null, quebra=null, parada=retorno, exit 1**. O código de saída indica incompletude, mas a prova explícita de falha é apresentada como ausência de registro. Integrar o novo evento ao passo retorno, preservando linha, motivo e correlação da sessão. Trata-se de integração entre T7/T8 e T9, não de nova mudança no fio.

**Aceite parcial dos seis achados anteriores:**

- **ARG-01:** erro imediato de envio passou a produzir `return.failed{erro:transport}`, nenhum `return.sent` e nenhum retorno recebido pela origem; sucesso segue coberto no teste original. Resta o timeout descrito acima.
- **ARG-02:** sessões consecutivas 81/82 produziram `emulate.start` e `emulate.positioned` para ambas; a segunda indicou `reutilizado=true`, sem reabrir o adaptador.
- **ARG-03:** fila cheia agora produz uma única recusa com `enviada=false, erro=network_busy`; o teste de Locked verificou recusa enviada com `enviada=true`. Resta a ausência de evento quando a tarefa expira.
- **ARG-04:** erro sintético em emit após posicionar produziu **um emulate.error** para a sessão correta, mantendo o tratamento anterior do runtime.
- **ARG-05:** ConnectToEIS do RemoteDesktop ganhou o ramo de erro com evento, portal e motivo; conferido por leitura, sem diálogo real.
- **ARG-06:** sequências **0,2,2,3** passaram a registrar **perdidos=1, fora_de_ordem=1**. O limite para sequências antigas distintas está explicitamente descrito como estimativa no §17.2; não foi exigida contagem exata fora dessa decisão. A exceção de `ignorados` por módulo também foi incorporada ao contrato.

Evidência própria: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **20 passed, 0 failed**. Harness por stdin com `rustc --edition=2024 --test`, fonte dos testes de `b7b70aa` e rlibs selecionados pelo Cargo, `/tmp/argus-t78-5mtbo9ei/check review_ --test-threads=1 --nocapture`: **6 passed, 0 failed**, incluindo asserções de observação dos resíduos. Fixture adicional do verificador em `/tmp/argus-return-failed-2l3jzxp8`, executada por `target/debug/clipsync verificar-travessia --origem ... --destino ... --json`, confirmou ARG-02.

Nenhum campo novo recebe evento de entrada, tecla, keycode ou conteúdo; as falhas de transporte usam códigos e o ramo de portal usa o erro de conexão. Suíte ampla/clippy/fmt do autor, Windows e portal físico não repetidos. Somente acréscimo neste documento, sem alteração de código.


## S11 T10 · 05c8f7e · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, revisão de `05c8f7ee4529cd149457064c7129335b67248792`. **Não passa: um Important; nenhum Critical demonstrado.** Os arquivos de diagnóstico/UI revisados conferiram sem diferenças contra o commit; alterações concorrentes de input não foram editadas.

1. **Important · S11-T10-ARG-01 · a cauda amplia a entrada sem ampliar a cobertura da redação.** `crates/clipsync-ui/src/diag.rs:627–635` acrescenta o histórico completo, mas `redact` em `diag.rs:461–545` só conhece pares/status, nome e endereços atuais; nem o ID local emitido por `core/src/agent.rs:174` entra nas agulhas. Reprodução pela API pública, com perfil temporário e quatro linhas JSONL sintéticas válidas: **ID local em agent.up, ID de par removido, host anterior com aspas/barra e IPv6 histórico ficaram intactos**, tanto na prévia quanto no arquivo exportado. Os controles de nome atual escapado, par conhecido, IPv6 conhecido e IPv4 foram removidos. Passar pelo mesmo redator cumpre a ordem do §17.2, mas não basta para a promessa da tela de que identificadores foram removidos: reinício/renomeação e remoção de par deixam dados no histórico sem representação no snapshot atual. Redigir também os identificadores e endereços dos próprios registros da cauda e incluir o ID local, com regressão para histórico anterior ao estado atual.

**Conferido sem novo achado:** últimas 400 linhas em ordem, arquivo ausente/vazio sem seção vazia, cauda antes do redator comum, exportação idêntica à prévia aprovada, default aditivo de `Diagnostics.log_path` e fixtures. A tela usa `h()` em `dd.mono`, recorre ao `app_state` ou à mensagem de ausência e não acrescenta `innerHTML`, estilo em linha ou travessão. O delta não acrescenta coleta/log de tecla, keycode ou conteúdo de clipboard; essa conclusão não promete filtrar conteúdo arbitrário introduzido externamente no arquivo.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-ui --lib diag::tests --locked`: **14 passed, 0 failed**.
- `node --max-old-space-size=512 --test --test-concurrency=2 crates/clipsync-app/ui-dev/testes/diagnostico-log.test.mjs`: **5 passed, 0 failed**, incluindo payload antigo e caminho com marcação tratado como texto.
- Harness compilado por stdin com `rustc --edition=2024` e rlibs selecionados pelo Cargo, `/tmp/argus-t10-umn1sgan/check /tmp/argus-t10-umn1sgan`: **exit 0**, afirmando a presença do vazamento e os controles de redação; exercitou `support_bundle_preview` e `support_bundle_export` sem agente real.
- Inspeção das duas capturas fornecidas `t10-portal-linux-1280.png` e `t10-portal-windows-960-longo.png`: caminho visível e quebrando dentro do cartão. Os números de `scrollWidth/clientWidth` de `t10-portal.txt` são evidência do Rivet, não medição repetida pelo revisor; não há portal ClipSync conectado a esta identidade.

Suítes completas UI/Node, clippy/fmt e Windows físico não repetidos. Somente esta entrada acrescentada ao documento; nenhum código alterado.


## S11 T10 · segurança · 05c8f7e · não passa · 04/11/2026

Sentinel · segurança. Revisão da F4 no commit `05c8f7ee4529cd149457064c7129335b67248792`, conforme `docs/contrato.md` §17, a privacidade do pacote em §12.5 e T10 do plano `docs/superpowers/plans/2026-11-04-diagnostico-por-log.md`. **Não passa: três Important; nenhum Critical demonstrado.** Os três achados impedem o aceite de segurança desta fatia.

1. **Important · S11-T10-SEG-01 · a redação depende do estado atual e deixa dados do log no pacote.** `crates/clipsync-ui/src/diag.rs:458` monta as agulhas com os pares/status/bloqueios atuais, o nome atual do dispositivo e IPv6 conhecidos; `diag.rs:627` acrescenta linhas históricas sem interpretar seus campos. A identidade local de `agent.up.id` (`crates/clipsync-core/src/agent.rs:174`) também não entra nas agulhas. Fixture pela API real de prévia/exportação: o host atual foi redigido, mas permaneceram **host antigo, IDs curto e longo de par removido, nome antigo escapado em JSON, ID local, IPv6 desconhecido e caminho com nome de usuário dentro de erro**. A codificação das agulhas corrige nomes conhecidos com aspas/barras; não cobre identidades ausentes do estado atual nem texto livre de erros. A mesma importação copiou campos sintéticos `clipboard_text`, `keycode` e `tecla`: não há validação de esquema nem seleção de campos. Isso prova passagem de conteúdo já presente no arquivo, **não coleta real de clipboard/teclas pelos emissores atuais**. Sanitizar estruturalmente a cauda, incluindo identidades históricas/locais, e restringir os campos/textos exportáveis antes de aplicar a redação do restante do diagnóstico.
2. **Important · S11-T10-SEG-02 · a leitura segue links do diretório e do arquivo.** `diag.rs:610` usa `File::open` sem recusar symlink, conferir tipo regular ou vincular a abertura a componentes previamente verificados. Com um marcador inofensivo em diretório temporário, **symlink em `clipsync.jsonl` foi lido e exportado; symlink em `logs/` também foi lido**. O leitor aceita até texto que não é JSONL. Assim, conteúdo de outro arquivo legível pode entrar no pacote quando o caminho é substituído; a proteção do escritor não protege esta abertura independente, inclusive com agente parado. Exige alteração local do caminho; não foi demonstrado acesso remoto nem elevação de privilégio. Recusar links nos componentes relevantes e arquivos não regulares, sem introduzir uma janela de troca entre verificação e abertura.
3. **Important · S11-T10-SEG-03 · 400 linhas não limitam bytes nem duração da leitura.** `diag.rs:614` percorre o arquivo inteiro com `BufReader::lines`, alocando cada linha antes de descartar as antigas; `diag.rs:627` faz isso sincronamente dentro da função async. Redação, cópias/cache (`diag.rs:590`) e exportação (`diag.rs:742`) também não impõem teto de bytes. Uma linha sintética de **128 KiB** foi aceita e o pacote exportado mediu **132.335 bytes**; portanto o leitor não aplica sequer o teto de linha do escritor. Não há máximo garantido do pacote: a rotação normal de 5 MiB e o limite de 64 KiB do produtor não limitam arquivos antigos/substituídos ou o leitor. A chamada ocupa um worker do runtime durante a leitura; fonte não regular pode ainda bloquear a operação. A semântica de leitura até delimitador/EOF consta em [BufRead](https://doc.rust-lang.org/std/io/trait.BufRead.html), e o efeito de trabalho bloqueante no executor em [Tokio](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html). Não foi executado cenário de FIFO nem comprovada paralisação global do agente. Ler uma janela final com orçamento de bytes, limitar linha e resultado final, recusar fontes não regulares e isolar o trabalho bloqueante com concorrência limitada.

**Conferido sem novo achado:** cauda com 400 linhas na ordem correta; redação de IDs/nomes conhecidos, inclusive escapes JSON, e de IPv4; exportação reproduz a prévia armazenada e cria o destino com exclusividade/0600 no Unix. A exibição do caminho usa texto no DOM. A busca nos emissores do recorte e os testes de `Debug` não mostraram emissão real de conteúdo de clipboard, tecla ou keycode; o logger aceita campos genéricos e o importador não pode sustentar sozinho a promessa de que esses dados nunca entrarão.

**Evidência própria:**

- `target/debug/deps/clipsync_ui-cf27537477e25ea3 diag::tests --test-threads=1`: **14 passed, 0 failed**. Os testes existentes cobrem as identidades conhecidas, não as lacunas acima.
- Harness compilado com `rustc`, usando a API `Ui::call` e rlibs existentes do workspace: `/tmp/st10-q827_0nj/audit`, saída em `result.log`, **exit 0**. Perfis, nomes, endereços e marcadores inteiramente sintéticos; prévia e exportação reais, sem agente iniciado e sem ler segredos. As sondas confirmam os comportamentos inseguros; exit 0 não é aprovação deles. `diag.rs` conferiu idêntico ao commit (SHA-256 `755cfed9e7df78226efd1d17febfe39729fa2c29e9c91604e9307e9925479618`).
- `target/debug/deps/redact-194a53324666eb3c --test-threads=1`: **6 passed, 0 failed**; `bash scripts/conferir-privacidade.sh`: **privacidade dos hooks: ok (8 arquivos)**. O script dos hooks não verifica todos os campos de erros do log.

Limites: execução em Linux com artefatos existentes; sem nova compilação completa, clippy/fmt, Windows nativo, portal físico ou prova de responsividade do agente sob entrada bloqueante. Alterações concorrentes em input/plano foram preservadas e não integram o commit revisado. Nenhum fonte editado, commit ou push; somente acréscimo deste parecer em `docs/verdicts.md`. A nota `release-findings` não está conectada; o registro autorizado fica neste documento.


## S11 bis-2 · 4c705d6 · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, revisão de `4c705d6eaea67dcbae98ce740fbef28e85d66cb0`. **Não passa: dois Important residuais; nenhum Critical demonstrado.** Os três arquivos de input conferiram sem diferenças contra o commit nas sondas. T78B-ARG-02 continua separado, na T9-bis do Rivet.

1. **Important · S11-BIS2-ARG-01 · erro terminal ainda vira trava no loopback.** A prioridade corrigida em `capture.rs:870–880` não foi aplicada ao consumidor paralelo em `crates/clipsync-input/src/module.rs:2229–2238` (também `module.rs:2272–2276`). Sonda com loopback ativo e lifecycle contendo `session_locked=false`, `desktop_available=true` e `terminal_error="captura sintetica encerrada"`: **um session.closed com motivo=lock e erro afirmando tela bloqueada**, sem a causa do adaptador. A mesma sonda na captura contínua agora produz error. A correção de T356B-ARG-03 atende o cenário original contínuo, mas o equivalente no loopback ainda apresenta falha de adaptador como interrupção por trava; aplicar a mesma precedência e preservar o detalhe em drive.
2. **Important · S11-BIS2-ARG-02 · cancelamento antes da primeira execução continua sem desfecho.** As guardas de `module.rs:1663–1667` e `module.rs:1871–1874` são construídas dentro do async, portanto só existem depois do primeiro poll. `Network::stop` (`network.rs:38–40`) pode abortar tarefas aceitas antes desse poll. Reprodução determinística: destino travado, 32 EnterSpan enfileirados e Shutdown logo depois; ao encerrar, **enter.received=32, chamadas de envio=0, enter.refused=0**. O timeout de tarefas já iniciadas foi corrigido, mas a garantia de um evento por recusa ainda falha na parada. Fazer o acompanhamento da operação existir desde sua admissão e concluir também o cancelamento anterior ao envio, sem duplicar o evento de fila cheia.

**Correções confirmadas:** T356B-ARG-01 agora fecha Deactivated do loopback com stop; T356B-ARG-02 fecha trava de sessão ativa com lock e o teste novo cobre tentativa ainda sem sessão como `attempt.filtered{locked}`; T356B-ARG-03 fecha erro terminal contínuo com error, com atribuição do detalhe conferida no código. T78B-ARG-01: após 2,2 s, retorno pendente gerou **um return.failed{timeout}** e nenhum return.sent; 32 recusas pendentes mais uma por fila cheia geraram **33 eventos**, um por sessão, sendo 32 com `enviada=false, erro=timeout` e uma com network_busy. Falha imediata, reutilização da emulação e contagem da sequência 0,2,2,3 continuam corretas nas sondas.

Quiesce, PrepareInventory, Retarget, fim do motor e cancelamento de troca no select receberam os motivos explícitos descritos no pedido. Isso não constitui aprovação de todos os end_session: chamadas anteriores em observação/invalidação de inventário e reconciliação de conexão ainda usam o default, já ressalvadas na rodada anterior; não foram reproduzidas nesta rodada nem contadas como achado novo. O delta não acrescenta conteúdo de entrada, tecla ou keycode aos campos de log.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **22 passed, 0 failed**.
- Harness por stdin com `rustc --edition=2024 --test`, fonte de `4c705d6` e rlibs selecionados pelo Cargo: `/tmp/argus-bis2-356-vcapk1wf/check review_ --test-threads=1 --nocapture`: **14 passed**; `/tmp/argus-bis2-78-t0ixw3lt/check review_ --test-threads=1 --nocapture`: **6 passed**; `/tmp/argus-bis2-78-x_y6pxy7/check review_shutdown --test-threads=1 --nocapture`: **1 passed**. Os testes dos resíduos afirmam sua presença; verde nessas sondas não os aprova.

Suíte ampla, clippy/fmt, Windows e portal físico não repetidos. Nenhum código alterado; somente acréscimo neste documento.


## S11 T9-bis · f853383 · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, revisão de `f85338344e2fbe3a7593dc178f99a7d714af3eb3` contra o §17.3 atualizado. **Não passa: dois Important residuais; nenhum Critical demonstrado.** Verificador, testes e contrato conferiram sem diferenças contra o commit durante a revisão.

1. **Important · S11-T9B-ARG-01 · a identidade do destino ainda não protege seu inventário.** A validação nova em `remotos_da_sessao` só alcança os eventos de sessão; `crates/clipsync-cli/src/verificar.rs:602–614` e `verificar.rs:629–649` continuam usando inventory.prepared sem comparar `agent.up.id` com o peer selecionado. Fixture: origem aa atravessa para bb, mas o arquivo de destino pertence a cc. Com `monitores=1`, **inventario_destino=ok usando a linha de cc**; com `monitores=0`, **inventario_destino=falhou e quebra=inventario_destino**, embora a origem tenha recebido o inventário de bb. Os passos de sessão são corretamente SemDado, mas o primeiro diagnóstico já usa prova de outra máquina. Aplicar a correlação de identidade/execução também ao pré-requisito remoto; identidade incompatível deve produzir lacuna, não aprovação ou falha daquele par.
2. **Important · S11-T9B-ARG-02 · tentativa sem abertura própria herda a sessão que acabou de fechar.** `verificar.rs:342–350` infere a sessão pelo primeiro evento de sessão da janela, sem conferir a tentativa declarada por esse evento. Reprodução: tentativa 1 abre sessão 7; tentativa 2 começa; sessão 7 fecha com `attempt=1, motivo=switch`; a tentativa 2 termina filtrada por inventory_changed antes de session.open. **--ultima devolve tentativa=2, sessao=7 e fechamento=ok/switch**, usando o encerramento da tentativa 1. O mesmo ocorre se o log termina logo após esse fechamento, antes da abertura seguinte. Isso viola a separação explicitada no §17.3: o fechamento anterior não pertence à sessão seguinte. Excluir eventos comprovadamente pertencentes a outra tentativa ao inferir a sessão, conservando a busca estendida de fechamento quando a sessão selecionada já é conhecida.

**Achados anteriores atendidos nos cenários originais:**
- **S11-T9-ARG-01:** eventos remotos de outro peer ou anteriores à execução atual agora resultam em SemDado/parada; recusas e return.failed de outro peer não criam quebra. Sem agent.up em qualquer lado, os passos remotos de sessão informam execução não comprovada. Resta o pré-requisito descrito no item 1.
- **S11-T9-ARG-02:** --sessao 7 encontra seu fechamento switch depois de attempt.begin 2; --sessao 8, quando aberta, não herda esse fechamento. Resta a inferência sem abertura própria descrita no item 2.
- **S11-T78B-ARG-02:** return.failed correlacionado gera **retorno=falhou, quebra=retorno, motivo=transport, linha com o evento, exit 1**, tanto em JSON quanto em texto. Não houve quebra inventada nos controles de outro peer.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-cli --locked --test verificar_travessia`: **29 passed, 0 failed**.
- **14 cenários JSONL sintéticos** em `/tmp/argus-t9bis-uwwc1ejb`, executados por `target/debug/clipsync verificar-travessia --origem ... --destino ... --json`, com --ultima/--sessao conforme o caso: controle completo exit 0, rejeição das provas alheias de sessão e reproduções dos dois resíduos acima. Conferência adicional em texto de `return_failed_com_prova`: **[falhou] retorno: transport**, seguida da linha JSON original e primeiro ponto de quebra retorno, exit 1.

Suíte CLI completa 58/58, clippy/fmt e plataformas físicas não repetidos; a revisão é do verificador offline. Alterações concorrentes da UI e do plano foram preservadas. Somente esta entrada acrescentada a docs/verdicts.md, sem edição de código.


## S11 T10-bis · segurança · 99044ac · não passa · 04/11/2026

Sentinel · segurança. Re-revisão de `99044ace325fc562da53c2dce5159fb83fce0512`, somente `crates/clipsync-ui/src/diag.rs`, com §17.2 item 4 atualizado em `fa355b6`. **Não passa: um Important residual de S11-T10-SEG-01; SEG-02 e SEG-03 atendidos dentro do contrato atualizado. Nenhum Critical demonstrado.** O resíduo impede o aceite de segurança da F4.

1. **Important · S11-T10-SEG-01, residual · o varredor ainda deixa IPv6 e partes de caminhos no texto exportado.** Em `diag.rs:700`, IPv6 só é reconhecido quando o token inteiro contém dígitos hexadecimais e `:.[]`; o `%` de um endereço com escopo e prefixos textuais não são aceitos. Em `diag.rs:708`, espaços dividem caminhos mesmo dentro de aspas, e somente fragmentos com `/` ou `\` são substituídos (`diag.rs:686`). Pela API real de prévia/exportação, com dados exclusivamente sintéticos:
   - `SocketAddrV6::to_string()` produziu **`[fe80::1234%7]:47800`**, que permaneceu integralmente em `erro`.
   - **`servidor:2001:db8:7654::1 indisponível`** permaneceu em `mensagem`.
   - O erro com **`"/home/pessoa-sintetica/Documento sigiloso.txt"`** virou **`"<caminho> sigiloso.txt"`**; o caminho Windows **`"C:\Users\Nome Sobrenome"`** virou **`"<caminho> Sobrenome"`**. Nome de arquivo e fragmento de nome pessoal ainda são exportados.

   A exportação conferiu byte a byte com a prévia contendo esses resíduos. A segunda passagem por agulhas atuais não cobre endereços históricos/desconhecidos nem partes de caminhos que não estão no estado atual. Existem emissores que preservam texto de erro em `erro=%e`, como `crates/clipsync-input/src/adapter/portal.rs:123/322/851/890`; a reprodução injeta os exemplos no arquivo, sem afirmar que um portal real produziu essas mensagens. Corrigir o reconhecimento de IPv6 com escopo/prefixos e a delimitação de caminhos com espaços, removendo a expressão sensível inteira ou omitindo o texto quando não for possível delimitá-la com segurança. Acrescentar esses casos à regressão da prévia e da exportação.

**Partes atendidas:**

- **SEG-01, parcial:** `host`, `peer`, `nome`, `pares` e `id` são substituídos estruturalmente, inclusive históricos; campos desconhecidos/proibidos e objetos extras são descartados; texto não JSON vira `[linha ilegível]`. O harness confirmou IDs local/removido, nome escapado, campos sintéticos de clipboard/tecla/keycode e controles de IPv4, IPv6 sem escopo e caminhos sem espaços. Não foi encontrada emissão real de clipboard, tecla ou keycode nesta alteração. O Important remanescente é o varredor de texto, não esses controles corrigidos.
- **SEG-02, atendido:** `symlink_metadata` recusa links existentes em `logs/` e no arquivo antes de abrir; tipo não regular é recusado, e há nova conferência de tipo no descritor. Harness da API e testes existentes confirmaram recusa dos dois links, sem copiar o marcador externo. A corrida local entre check/open permanece exatamente no limite aceito pelo Maestro em §17.2; não é reaberta como achado. Não foi executado Windows nativo.
- **SEG-03, atendido no escopo atualizado:** leitura por seek da janela final de até 256 KiB, `take` limitado ao comprimento observado mesmo com acréscimo concorrente, até 400 linhas, marcador para linha acima de 16 KiB ou primeiro fragmento parcial. A seção redigida, incluindo cabeçalho, tem teto de 256 KiB. `spawn_blocking` engloba leitura/redação; o permit do semáforo é movido para a closure e só é liberado ao terminar, inclusive se o chamador for cancelado. A análise desse ciclo de vida é estática; não foi simulada falha bloqueante do filesystem. O harness confirmou que a antiga linha de 128 KiB agora é truncada. O tamanho total continua sendo o diagnóstico anterior mais até 256 KiB de seção de log; o contrato fixa o teto da seção, não um novo teto global do pacote.

**Evidência própria:**

- `CARGO_BUILD_JOBS=2 /home/kappxs/.cargo/bin/cargo test -p clipsync-ui --lib diag::tests --locked -- --test-threads=1`: **19 passed, 0 failed**, incluindo janela, tamanho após redação, 400 linhas e os dois links.
- `CARGO_BUILD_JOBS=2 /home/kappxs/.cargo/bin/cargo build -p clipsync-ui --lib --locked`: **exit 0**. Harness compilado com os rlibs desta compilação, dependências selecionadas pelos fingerprints do Cargo, em `/tmp/st10bis-_bb4133s/audit`; fonte, argumentos de compilação e resultado preservados no mesmo diretório. **Exit 0**, confirmando os controles e a presença dos quatro resíduos descritos, por `Ui::call` real. Perfis temporários sintéticos, sem agente iniciado, sem leitura de segredos, sem `target/` em `/tmp`.
- `diag.rs` e contrato sem diferenças contra `99044ac` durante a conferência; SHA-256 do fonte `7ff50b7318bfad0957fcd49e2e47b0c1a02c13c12f16533d0eaaaa9517129460`. O HEAD avançou por trabalho concorrente, sem modificar esses arquivos.

Não repetidos UI 185/185, clippy/fmt ou plataformas físicas. Somente acréscimo deste parecer, preservando o documento anterior e as alterações concorrentes; nenhum fonte editado, commit ou push. A nota `release-findings` não está conectada; o registro solicitado fica neste documento.


## S11 T10-bis · 99044ac · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, re-revisão de `99044ac`, conforme §17.2 item 4 vigente. **Não passa: um Important residual; nenhum Critical demonstrado.** `crates/clipsync-ui/src/diag.rs` conferiu sem diferenças contra o commit.

1. **Important · S11-T10B-ARG-01 · o varredor ainda exporta endereços e partes de caminhos em texto livre.** `crates/clipsync-ui/src/diag.rs:700–703` só aceita um token composto inteiramente por dígitos hexadecimais e pontuação limitada; `diag.rs:708–719` separa por espaços antes de reconhecer caminhos. Pela API pública, um erro histórico com **[fe80::abcd%3]:47800** permaneceu intacto, assim como **endereco:2001:db8::abcd**. Os caminhos sintéticos **/home/Primeiro Sobrenome Ultimo/arquivo.log** e **C:\\Users\\Primeiro Sobrenome Ultimo\\arquivo.log** viraram **<caminho> Sobrenome <caminho>**, preservando parte do nome de usuário. Esses mesmos resíduos chegaram à exportação. IPv6 simples foi redigido no controle; não depende de identidade cadastrada. Reconhecer endereço com zona/rótulo e a extensão completa de caminhos com espaços antes de fragmentar o texto, cobrindo os formatos Linux e Windows.

**Correções aceitas neste recorte:** host histórico escapado, ID local, peer removido, nome e peer dentro de barreiras foram substituídos por marcadores; campos extras de topo, objeto arbitrário, tecla em totais e keycode em barreiras foram descartados, preservando contagens e estrutura conhecida. O achado original T10-ARG-01 sobre identidades estruturadas está atendido; a redação de texto livre da mesma cauda ainda tem o resíduo acima. Os testes cobrem JSON ilegível e symlink tanto no arquivo quanto em logs/; a sonda adicional confirmou recusa do arquivo sem ler o marcador externo. A corrida local check/open permanece explicitamente aceita no contrato, sem alegação de abertura atômica.

**Prévia, limites e execução:** após preparar a prévia, substituí o log e exportei: **arquivo byte a byte igual à prévia**, incluindo a constatação do resíduo. Arquivo sintético com prefixo esparso de **64 MiB** e 500 linhas expansíveis terminou com seção de log de **261.527 bytes, 280 linhas, máximo de 933 bytes por linha**. A suíte também cobre linha acima de 16 KiB e o teto de 400 linhas. Por leitura de `diag.rs:926–940`, leitura e redação da cauda rodam em spawn_blocking, com o permit da única leitura mantido dentro da tarefa inclusive se o chamador cancelar; a seção tem teto próprio de 256 KiB, além do texto de diagnóstico preexistente. Não medi responsividade visual do app.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-ui --lib diag::tests --locked`: **19 passed, 0 failed**.
- Harness por stdin, `rustc --edition=2024` com rlibs selecionados pelo Cargo, `/tmp/argus-t10bis-3es03r6p/check /tmp/argus-t10bis-3es03r6p`: **exit 0**, afirmando remoção de campos/identidades, os quatro resíduos de texto, igualdade prévia/exportação, teto da seção e recusa de symlink. Todos os dados eram sintéticos; não foi lido perfil real.

UI completa 185/185, clippy/fmt e Windows físico não repetidos. Nenhum código editado; somente acréscimo neste documento.


## S11 T9-ter · d4cd0bb · passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, re-revisão de `d4cd0bb`. **Passa no recorte solicitado; nenhum Critical, Important ou Minor novo encontrado.** S11-T9B-ARG-01/02 atendidos. O checkout usado na execução tinha apenas textos de help posteriores em relação a esse commit no verificador; o algoritmo e os testes revisados eram os mesmos.

1. **S11-T9B-ARG-01 resolvido:** destino cc quando o par selecionado é bb, com zero ou um monitor, agora dá **inventario_destino=sem_dado, linha=null, quebra=null, parada=inventario_destino**, com execução não comprovada. Ausência de agent.up no destino também produz lacuna; inventory.missing correlacionado na origem conserva sua prova explícita.
2. **S11-T9B-ARG-02 resolvido:** tentativa 2 sem session.open, com ou sem attempt.filtered, agora mantém **sessao=null e fechamento=sem_dado/linha=null** ao aparecer fechamento da tentativa 1. Ao selecionar explicitamente a sessão 7 já conhecida, o fechamento switch após attempt.begin 2 continua presente; a sessão 8 não o herda.
3. **Regressões conferidas:** cadeia completa com onze ok e exit 0; eventos de sessão de outro peer/execução não aprovam passos; enter.refused e return.failed de outro peer não criam quebra; return.failed correlacionado conserva **retorno=falhou, linha com a prova, motivo=transport, quebra=retorno, exit 1**.

Evidência própria: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-cli --locked --test verificar_travessia`: **31 passed, 0 failed**. Reexecução dos **14 cenários JSONL** da rodada anterior, agora em `/tmp/argus-t9ter-0elwgsbt`, por `target/debug/clipsync verificar-travessia --origem ... --destino ... --json`, com --ultima/--sessao conforme o caso: todas as asserções atendidas, incluindo a correção dos quatro casos antes reprovados.

A suíte CLI completa 60/60 e clippy/fmt são evidência do autor, não repetida nesta rodada. Parecer restrito ao verificador offline; não encerra os resíduos de input ou da redação do pacote. Nenhum código editado; somente esta entrada acrescentada a docs/verdicts.md.


## S11 bis-3 · 7ae747a · não passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, revisão de `7ae747a3a5cf7d47e75433f948e85f10767d7cd2`. **Não passa: um Important residual; nenhum Critical demonstrado.** Código de input e testes usados nas sondas conferiram sem diferenças contra o commit.

1. **Important · S11-BIS3-ARG-01 · a terceira rechecagem do loopback ainda fecha erro como return.** Os dois ramos corrigidos não abrangem `crates/clipsync-input/src/module.rs:2326–2335`, executado depois de selecionar um evento. Se o lifecycle muda entre o poll do watch e essa rechecagem, o ramo só atribui `failure=SOURCE_SCREEN_LOCKED`, não examina terminal_error nem muda close, que continua return. Sonda com dados de movimento em fluxo e publicação concorrente de erro terminal, mantendo `session_locked=false` e `desktop_available=true`, reproduziu na primeira rodada: **session.closed.motivo=return**, erro dizendo tela bloqueada e totais enviados=0/descartados=10; a causa sintética do adaptador desapareceu. O ramo de autoridade logo acima (`module.rs:2320–2324`) também conserva close, conferido por leitura. Aplicar a classificação completa em todas as rechecagens após evento, com error/cause para erro terminal e motivos próprios para trava, pausa e parada.

**Correções confirmadas:** erro terminal observado pelo caminho normal do loopback agora fecha uma vez com **error** e a causa do adaptador; a captura contínua mantém esse resultado. Deactivated, trava pelo comando, recusa e desconexão conservaram seus motivos. As guardas Pendente existem antes do spawn e o teste de capacidade não tem await nem outro produtor capaz de ocupar vaga entre consulta e admissão. Com **32 EnterRoutes para monitor ausente seguidos de Shutdown**, houve **32 enter.received, zero chamadas de envio, zero emulações posicionadas e 32 enter.refused**, um por sessão, todos com `motivo=monitor_not_found, enviada=false, erro=timeout`. A fila cheia continuou gerando só um network_busy; 32 operações que efetivamente expiraram geraram um timeout cada. Retorno expirado continuou produzindo um return.failed e nenhum return.sent. **BIS2-ARG-02 atendido; BIS2-ARG-01 atendido nos dois ramos alterados, com o resíduo concorrente acima.**

**Retificação da evidência da rodada bis-2:** a sonda `review_shutdown_before_network_jobs_are_polled` registrada naquela rodada usava InputLock para tentar provocar recusa de entrada. InputLock impede captura de saída e não provoca essa recusa; portanto **32 entradas e zero enter.refused naquele cenário não demonstravam 32 recusas sem log**. Desconsiderar essa reprodução como prova do achado anterior. Nesta rodada a sonda foi corrigida para monitor ausente, verificou também zero emulações e demonstrou o comportamento da guarda antes do primeiro poll. As entradas históricas foram preservadas, conforme a regra de só acrescentar.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **23 passed, 0 failed**.
- Harnesses por stdin com `rustc --edition=2024 --test`, fonte de `7ae747a` e rlibs selecionados pelo Cargo: `/tmp/argus-bis3-356-fdmhrt94/check review_ --test-threads=1 --nocapture`: **14 passed**; `/tmp/argus-bis3-cancel-5v3lrxon/check review_ --test-threads=1 --nocapture`: **7 passed**, incluindo a reprodução corrigida de cancelamento e os controles de fila cheia.
- `/tmp/argus-bis3-race-gzn5loe5/check review_terminal_lifecycle_races --test-threads=1 --nocapture`: **1 passed**, afirmando a presença do fechamento incorreto; primeira rodada concorrente já o reproduziu. Esse verde confirma a observação do defeito, não sua aprovação.

Nenhum campo novo registra tecla, keycode ou conteúdo de entrada. Suíte ampla, clippy/fmt, Windows e portal físico não repetidos. Nenhum código editado; somente esta entrada acrescentada a docs/verdicts.md.


## S11 T10-ter · segurança · 5e27d84 · passa · 04/11/2026

Sentinel · segurança. Re-revisão de `5e27d84a868507bddaddfc90c87fbb906cd0d0ae`, somente `crates/clipsync-ui/src/diag.rs`, com a regra de texto livre de §17.2 item 4 registrada em `152d2dd`. **Passa no recorte de segurança da F4: S11-T10-SEG-01 encerrado; SEG-02 e SEG-03 continuam atendidos nos limites já aceitos. Nenhum novo achado de segurança nesta revisão.** Este parecer não aprova outras fatias nem substitui os demais gates do release.

1. **SEG-01 encerrado por execução da prévia e exportação.** Repetidos `[fe80::1234%7]:47800`, `servidor:2001:db8::1`, o caminho Linux entre aspas com `Documento sigiloso.txt` e o caminho Windows entre aspas com `Nome Sobrenome`. Os quatro valores foram colocados individualmente em **12 chaves**: `erro`, `message`, `mensagem`, `evento`, `nivel`, `motivo`, `origem`, `alvo`, `adaptador`, `portal`, `session` e `ts`. **48/48 combinações produziram `<texto omitido>`**, sem os resíduos nem os fragmentos `sigiloso.txt`/`Sobrenome`. Renomear `erro` para uma chave conhecida não liberou o conteúdo; usar `erro_renomeado`, `clipboard_text`, `keycode` ou `tecla` removeu o campo. A exportação foi idêntica à prévia, conferida byte a byte.
2. **Validação por campo confirmada.** `diag.rs:853` omite `erro`/`message`; as demais strings só sobrevivem por literal conhecido, timestamp validado ou UID de monitor permitido. O id curto de par foi ocultado nas mesmas 12 chaves, além de `uids`, `barreiras[].peer` e um contador aninhado que recebeu string. A fixture preservou os oito valores literais aprovados, números/booleanos e UID hexadecimal de 32 caracteres; rejeitou UID pessoal/não hexadecimal. Nove casos de timestamp conferiram formato e calendário: três válidos, incluindo 29/02/2000 e 29/02/2024, e seis inválidos, incluindo 29/02/2100, 31/11, hora 24 e sufixo extra. Identidades históricas/local continuam substituídas estruturalmente. A cauda não passa mais pelas agulhas do estado atual: `redact_log_line` recebe apenas a linha e o contador (`diag.rs:999`).
3. **SEG-02/03 preservados.** Não houve mudança no mecanismo de abertura/limites: recusa de links existentes em `logs/` e no arquivo, conferência de tipo regular, janela final de 256 KiB, até 400 linhas, teto de 16 KiB por linha e seção de até 256 KiB. Os testes de links, janela, cauda e expansão após redação passaram novamente. `spawn_blocking` mantém o permit na tarefa até sua conclusão. Continua aceita a corrida local entre `symlink_metadata` e `open`; não há garantia de abertura atômica sem seguir links. O teto de 256 KiB é da seção de log, somada ao diagnóstico preexistente.

**Evidência própria:**

- `CARGO_BUILD_JOBS=2 /home/kappxs/.cargo/bin/cargo test -p clipsync-ui --lib diag::tests --locked -- --test-threads=1`: **25 passed, 0 failed**.
- `CARGO_BUILD_JOBS=2 /home/kappxs/.cargo/bin/cargo build -p clipsync-ui --lib --locked`: **exit 0**.
- Harness da API real `Ui::call`, compilado contra a biblioteca recém-compilada e dependências selecionadas pelos fingerprints: `/tmp/st10ter-r21m_w14/audit`, **74 casos conferidos, exit 0**. São 48 combinações dos resíduos/chaves, quatro chaves desconhecidas, 12 posições do id curto, nove timestamps e um controle estruturado. Saída: `nenhum_dos_quatro_residuos_ou_fragmentos_presente=true; exportacao_identica=true`; pacote observado com **6.079 bytes**, seção de log com **4.824 bytes**. Fonte, argumentos e resultados em `audit.rs`, `compile-args.txt`, `compile.log` e `result.log` nesse diretório temporário.
- Fonte/contrato sem diferenças contra `5e27d84` na conferência; SHA-256 de `diag.rs`: `0b85866854fccde4a20e7c58c2b577d6d89a137f53bf3c5f087d22c3119b76ed`.

Todos os dados de teste são sintéticos, com perfil temporário e sem agente iniciado, clipboard real, portal ou segredo acessado. Windows nativo, suíte UI completa e clippy/fmt não repetidos; cancelamento da leitura foi conferido por código, sem simular filesystem bloqueante. Somente acréscimo deste parecer; alterações concorrentes preservadas, nenhum fonte editado, commit ou push. A nota `release-findings` não está conectada; o registro autorizado fica neste documento.


## S11 T10-ter · 5e27d84 · passa com ressalva · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, revisão de `5e27d84a868507bddaddfc90c87fbb906cd0d0ae`, que substitui 99044ac como versão vigente da fatia 4. **Passa com uma ressalva Minor; nenhum Critical ou Important encontrado neste recorte.** T10B-ARG-01 atendido: texto livre é omitido integralmente, eliminando os resíduos de IPv6 e caminhos. `diag.rs` conferiu sem diferenças contra o commit.

1. **Minor · S11-T10T-ARG-01 · a lista de motivo omite códigos já usados pelos emissores.** `crates/clipsync-ui/src/diag.rs:790–839` não inclui **stopped, emulator_start_failed, entry_positioning_failed, route_invalid e inventory_unconfirmed**, emitidos por `input/src/module.rs` e `module/organization.rs`. A API de prévia confirmou que todos viram `<texto omitido>`. No verificador, a mesma tentativa filtrada produziu `motivo=stopped` no JSONL original e `motivo=<texto omitido>` na cauda redigida; ambos foram lidos normalmente e mantiveram tentativa=falhou, exit 1. A proteção de privacidade e a identificação do passo permanecem, mas perde-se uma causa literal útil. Completar a lista a partir desses emissores e cobrir a correspondência sem liberar texto arbitrário.

**Conferências:** identidades históricas/locais e IDs curtos foram redigidos; campos desconhecidos no topo e em totais/barreiras foram descartados. Os cinco textos de erro usados nas rodadas anteriores, incluindo IPv6 com zona, rótulo e caminhos Linux/Windows com espaços, saíram integralmente como `<texto omitido>`. Dez campos preenchidos com texto fora da lista também foram omitidos. Valores permitidos de evento, nível, alvo, motivo (portal_denied), origem, adaptador, portal, mensagem tipada, número e booleano foram preservados; timestamp estrito e UID hexadecimal de 32 caracteres permaneceram legíveis, enquanto UID irregular e ID curto não passaram.

**Prévia/exportação e leitura:** depois da prévia, o log foi substituído; a exportação continuou **byte a byte idêntica à prévia aprovada**. O verificador consumiu a seção JSONL extraída da cauda, conservando ts e evento sem erro de esquema; a limitação dos motivos ausentes é o Minor acima. Não foi fornecido o pacote textual inteiro como se fosse JSONL. Arquivo com prefixo esparso de 64 MiB produziu seção de **19.607 bytes, 400 linhas e máximo de 48 bytes por linha** nesse cenário de omissão. O teste de expansão da suíte continuou verificando teto de 256 KiB após redação. Recusa de symlink, limites de leitura/linha e spawn_blocking com uma leitura por vez foram preservados.

Evidência própria:
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-ui --lib diag::tests --locked`: **25 passed, 0 failed**.
- Harness por stdin com `rustc --edition=2024`, rlibs selecionados pelo Cargo e API Ui::call real, `/tmp/argus-t10ter-jxtfdqei/check /tmp/argus-t10ter-jxtfdqei`: **exit 0**, cobrindo omissão, valores permitidos, igualdade prévia/exportação, limites, symlink e a observação do Minor. JSONL original e redigido no mesmo diretório foram executados por `target/debug/clipsync verificar-travessia --origem ... --json`.

Demais testes UI, clippy/fmt e plataformas físicas não repetidos. Perfis e dados sintéticos; nenhum perfil real lido. Somente acréscimo neste documento, sem edição de código.


## S11 bis-4 · e0e549d · passa · 04/11/2026

Revisor · achados, continuação dos pareceres Argus, re-revisão de `e0e549d`. **Passa no recorte solicitado; nenhum Critical, Important ou Minor novo encontrado. S11-BIS3-ARG-01 atendido.** Código de input e testes conferiram sem diferenças contra o commit.

A rechecagem após selecionar um evento agora atribui stop/pause conforme a autoridade e prioriza terminal_error como **error com a causa**, usando lock somente quando não há erro terminal. A sonda que reproduziu return indevido na primeira rodada anterior completou **160 rodadas concorrentes, sem fechamento incorreto**, publicando erro terminal com tela e desktop disponíveis enquanto havia dados de movimento em fluxo. As outras duas rechecagens mantêm a mesma classificação; não houve mudança nos envios ou nas guardas aceitas na rodada bis-3.

Evidência própria: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test -p clipsync-input --locked --test s11_log`: **23 passed, 0 failed**. Harness por stdin com `rustc --edition=2024 --test`, fonte de `e0e549d` e rlibs selecionados pelo Cargo, `/tmp/argus-bis4-race-kwbvc8fb/check review_terminal_lifecycle_races --test-threads=1 --nocapture`: **1 passed**, 160 rodadas, `fechamento incorreto=None`.

A sonda concorrente exerceu erro terminal; a distinção de pausa/parada nessa rechecagem foi conferida por leitura, sem afirmar teste concorrente de cada combinação. Suíte ampla, clippy/fmt e portais físicos não repetidos. Somente esta entrada acrescentada a docs/verdicts.md; nenhum código editado.


## S11 T11 · b968bfd · funcional · não passa · 04/11/2026

Sentinel · segurança, revisão funcional solicitada pelo Maestro em substituição ao Argus. Commit `b968bfdcb3be429ddd34ade247a452185aeb882a`, com §17.3 atualizado em `fd4b078`. **Não passa: um Important e um Minor.** Os três cenários reais de campo passam como esperado e os 35 testes existentes estão verdes; as sondas adicionais abaixo encontraram resíduos não cobertos.

1. **Important · S11-T11-FUNC-01 · reinício do destino dentro da divergência de relógio permite usar a execução anterior e esconder uma falha atual.** `crates/clipsync-cli/src/verificar.rs:429` escolhe o último `agent.up` cujo timestamp bruto é menor ou igual ao marco da origem. A tolerância de 30 s só é aplicada depois, aos eventos (`verificar.rs:449`); o recorte da execução já foi escolhido. Fixture com origem reiniciada às **14:01:00**, tentativa às **14:01:01** e fechamento às **14:01:10**; destino com provas antigas da mesma combinação peer/session às **14:00:38–14:00:43** e novo `agent.up` às **14:01:02**. Com o deslocamento conhecido de **+1,6 s**, esse reinício remoto ocorreu antes da tentativa na origem, mas o verificador escolhe a execução anterior, admite suas provas pela margem de 30 s e devolve **onze ok, quebra=null, parada=null, exit 0**. No controle com o reinício remoto marcado às 14:01:00.500, as provas antigas são corretamente excluídas.

   Segunda sonda do mesmo defeito: retirado o `return.sent` antigo e acrescentado **`return.failed{session:101,peer:bb,erro:transport}` às 14:01:08.600**, na execução nova do destino. Resultado: **retorno=ok, motivo=confirmado só na origem, onze ok, exit 0**; a prova explícita de falha foi descartada na seleção da execução. A precedência de `return.failed` dentro de `cadeia` (`verificar.rs:560`) está correta quando o evento chega até ela. O defeito é a atribuição de execução antes desse passo. Manter os limites internos do destino estritos, mas não afirmar correlação com a execução anterior quando um reinício dentro da faixa de incerteza entre relógios a torna ambígua; nesse caso, produzir `SemDado/execução não comprovada`. Acrescentar regressão com reinício adiantado e número de sessão reutilizado entre execuções. Nenhum desses dados é de máquina real.

2. **Minor · S11-T11-FUNC-02 · N/A legado é aplicado sem sessão selecionada.** `verificar.rs:652` compara dois `Option<u32>` sem exigir um número presente: `None == None` satisfaz a condição. Fixture com `inventory.missing`, tentativa e `session.open{mensagem:enter_span}` **sem campo session** produz **sessao=null, inventario_destino=ok/não se aplica, quebra=null**. Ao remover somente essa abertura incompleta, a mesma prova volta a produzir **inventario_destino=falhou, quebra=inventario_destino**. Ambos terminam em exit 1 por cadeia incompleta; não houve falso sucesso global nesta sonda. O registro é semanticamente incompleto, mas aceito pelo parser de JSON/esquema, portanto o parecer não afirma emissão desse formato pelo agente real. Exigir sessão selecionada válida/não zero e abertura correspondente antes da exceção; sem isso, conservar as regras anteriores de inventário.

**Cenários de campo, conferidos em texto e JSON:**

Diretório `<campo>`: `/tmp/claude-1000/-home-kappxs-Documentos-ANTIGOS-clipsync--maestri-roles-83bcaab5-84f6-0d02-2adf-b22d1f6ccdd4/b120769c-728e-4919-8aa6-d79bbd949b6c/scratchpad/campo-2026-11-04`.

| Comando/cenário | Resultado próprio |
| --- | --- |
| `target/debug/clipsync verificar-travessia --origem <campo>/pc.jsonl --destino <campo>/notebook.jsonl` | **11 ok, exit 0**; inventário remoto N/A por `enter_span`; datagramas e retorno confirmados nos dois lados. |
| Mesmo comando com `--sessao 69759769` | **11 ok, exit 0**, mesma sessão/provas. |
| Recorte de `pc.jsonl` com `ts < 2026-11-04T17:50:00Z`, sem `--destino` | **tentativa=falhou, motivo=peer_offline, quebra=tentativa, exit 1**. |

As linhas cruas dos logs de campo não foram copiadas para o repositório nem para este parecer. Os arquivos originais foram somente lidos e seus hashes SHA-256 conferiram iguais antes/depois. O recorte ficou exclusivamente em diretório temporário privado, com arquivo 0600.

**Demais controles:** limites inferior/superior de 30 s aceitos, 1 ms além recusado; eventos de outro peer, outra sessão ou identidade incompatível não provaram os passos remotos. Reinício com marcador inequivocamente anterior ao marco da origem excluiu os eventos velhos. `return.failed` correlacionado prevaleceu sobre `return.received` inclusive no limite de +30 s; falha de outro peer foi ignorada. Datagramas/retorno somente na origem ou somente no destino usaram a linha do lado existente; ausência nos dois produziu `SemDado`. As três variantes legadas deram N/A com `session.open` da sessão pedida; abertura de outra sessão não ativou N/A. As ressalvas são os dois achados acima.

**Evidência própria e recursos usados:**

- `CARGO_BUILD_JOBS=1 target/debug/deps/verificar_travessia-1ae0e384b7de0bed --test-threads=1`: **35 passed, 0 failed**. Executado o binário de testes já existente; nenhum Cargo/build iniciado.
- **33 cenários sintéticos** executados sequencialmente pelo `target/debug/clipsync` existente, em `/tmp/sentinel-t11-64py631k`: scripts `sondas.py`/`sondas-adicionais.py`, fixtures e saídas por cenário, resultados em `sondas-resultados.json`. Eles incluem os controles aprovados e as reproduções dos defeitos; o sucesso do script não aprova os resíduos.
- Campo: seis invocações do binário, três cenários em texto e JSON. Resumos sem linhas cruas e hashes dos originais em `campo-resumos.json`, no mesmo diretório temporário.
- Fonte/testes sem diferenças contra `b968bfd`. SHA-256: `verificar.rs` = `81f114b73e97a206824ef4fa590d5a09b8b0b37a29a01b54cf5ee9ef7a4dd7f5`; `verificar_travessia.rs` = `830432b557fa72a28421c21ba203ca4232cfc36fdbbb78821eb148bcaf5d7ee6`.

Revisão da interpretação offline; sem rebuild do workspace, clippy/fmt, novo teste físico ou agente iniciado. Nenhum fonte editado, commit ou push. Somente acréscimo deste parecer em `docs/verdicts.md`.

## R1 · 11/11/2026

**Escopo:** revisão de `5a90f0a` e `dcbdd64`, no checkout ClipSync, `main`, HEAD `c30c529`, versão `0.5.2`. Parecer por leitura dos commits, arquivos atuais e execução dos três comandos focais pedidos. Nenhum fonte alterado, commit ou push. A alteração preexistente de `CONTEXTO.md` foi preservada. As notas de outros projetos foram ignoradas.

**Vereditos:** `5a90f0a`: **passa com ressalva**, no recorte de recriação na entrada seguinte; sem prova física de GNOME. `dcbdd64`: **não passa**, pelos três achados Important abaixo. Não identifiquei Critical neste recorte. Os achados de concorrência e de seleção de candidatos são evidência estática, com sequências descritas; não foram reproduzidos pelos testes executados.

### dcbdd64 · achados por severidade

1. **Important · R1-D01 · resultado antigo sobrescreve `Mesh::reload`.** `crates/clipsync-core/src/transport.rs:883`, `:885` e `:903`. A conferência de `current == stale` ocorre antes do `await dial_once`; `readdress` depois dele aceita qualquer endereço atual diferente do candidato. Sequência permitida: a discagem por nome começa com endereço A; `reload` aplica endereço C (`transport.rs:505`); o candidato B termina o handshake; `readdress` substitui C por B e emite atualização para persistência. `install` também contém espera de publicação de `PeerUp`, ampliando a janela. Assim, uma edição mais nova é perdida e o comentário de proteção contra reload não corresponde ao que acontece durante a discagem. A confirmação precisa conferir a revisão/instância da entrada que originou a tentativa, sob o lock, antes de publicar o resultado. Nenhum teste focal intercala reload com handshake/publicação.

2. **Important · R1-D02 · evento enfileirado substitui endereço editado no disco.** `crates/clipsync-core/src/agent.rs:179` e `:182`; produtor em `transport.rs:911`. O evento leva só `(peer, addr)` e escreve incondicionalmente no campo da entrada atual. Sequência permitida, mesmo sem D01: B foi encontrado e enfileirado; UI/CLI grava endereço C; o consumidor recebe B e substitui C. `Profile::update_peers` usa lock exclusivo durante leitura, alteração e escrita (`profile.rs:752` e `:767`): preserva permissões e outros pares, e não há prova de corrupção de TOML ou de perda desses campos. Porém, serialização não torna o resultado antigo válido para o endereço atual. É necessário transportar e conferir a precondição/revisão também na persistência. Se o par já foi removido, o consumidor não o recria, o que está correto. O teste mesh só usa um escritor efetivo nessa fase e não cobre essa disputa.

3. **Important · R1-D03 · candidato válido depois dos três primeiros nunca é tentado.** `crates/clipsync-core/src/transport.rs:870` e `:873`. Cada rodada pega novamente o mesmo prefixo de até três endereços, sem rotação ou memória de progresso. Se a resolução estável retorna três endereços sem resposta e o quarto é o par válido, o reencontro falha indefinidamente, embora o resolvedor o tenha encontrado. O orçamento por rodada deve permitir progresso nas rodadas seguintes. O teste `readdress` retorna apenas dois candidatos, dos quais o primeiro responde com outra chave; não cobre esse caso.

**Laço e prazos conferidos:** não há giro ocupado; cada tentativa espera o QUIC e depois há backoff. Com três candidatos silenciosos, são até **30 s adicionais** por rodada, além dos 10 s do endereço guardado, até 3 s da resolução do sistema (consultas em paralelo) e do backoff/jitter. Os cerca de 43 s antes do backoff são cálculo do caminho de timeout, não medição desta execução. Um servidor que conclui TLS mas não Hello pode acrescentar até 5 s por tentativa. `wake` de reload não é selecionado enquanto `dial_once` está pendente: mesmo um endereço corrigido pode aguardar a tentativa antiga terminar. A proteção antes de cada próximo candidato evita continuar a lista quando o endereço já mudou; não resolve D01. A parada global fecha o endpoint no shutdown; não afirmo um atraso obrigatório de 30 s para encerrar o agente.

### 5a90f0a · achado e verificações

1. **Minor · R1-E01 · causa da morte do laço EIS continua sem registro.** `crates/clipsync-input/src/adapter/portal.rs:748` e `:780`. Erro de conversão, fim do stream e `Disconnected` terminam o laço sem registrar qual ocorreu. `alive` passa a detectar o receptor fechado, mas na próxima entrada só se registra que a plataforma encerrou o emulador (`module.rs:1741` e `module/organization.rs:568`), sem distinguir erro local de conversão e encerramento remoto. Isso prejudica a investigação de F1. O `break` **já existia no pai do commit**, portanto não é regressão introduzida por `5a90f0a` nem prova de que o commit causou o teclado morto. Registrar categoria/motivo seguro, sem evento de tecla ou keycode, pertence ao acompanhamento de F1/C1.

**`emu.stop()` morto:** não identifiquei defeito demonstrado nesse caminho. Quando `alive()` é falso por canal fechado (`portal.rs:933`), `call(ReleaseAll)` falha no envio sem aguardar resposta (`portal.rs:827`); `stop` ignora esse erro, remove o sender/watch, aguarda a thread e fecha a sessão (`portal.rs:956`). Tanto `module.rs:1740` como `module/organization.rs:567` limpam `emu_started` antes de reiniciar; `PortalEmulate::start` também limpa uma sessão morta se invocado diretamente. Isso é leitura do controle de fluxo: o teste fake não executa canal, join, D-Bus ou fechamento real. Uma sessão D-Bus sem resposta não foi ensaiada. Morte durante uma sessão ativa só será recuperada na entrada seguinte; não há recuperação automática da sessão em curso prometida por este recorte.

### Testes executados e o que provam

Todos rodados com `PATH=$HOME/.cargo/bin:$PATH` e `-j2`, sobre HEAD `c30c529`:

| Comando | Saída observada | Alcance da prova |
| --- | --- | --- |
| `cargo test -j2 -p clipsync-input --test s11_log emulador_encerrado` | `1 passed; 0 failed; 23 filtered out`, 0,02 s | `FakeEmulate::end` entre duas entradas faz a segunda iniciar e posicionar, com `reutilizado=false`. O teste força versão remota 1 (`s11_log.rs:1312`): prova o caminho legado; não exerce `enter_routes` v4, PortalEmulate, suspensão/retomada GNOME nem teclado físico. Prova parcialmente a promessa. |
| `cargo test -j2 -p clipsync-core --test readdress` | `2 passed; 0 failed`, 12,22 s | QUIC em loopback: endereço guardado silencioso, candidato de outra chave ignorado, candidato correto conecta, atualiza memória e canal. Resolver do sistema: `localhost` conserva porta e nomes inválidos resultam vazios. Não prova DNS/LLMNR/NetBIOS/mDNS de duas máquinas, três candidatos silenciosos, quarto candidato, reload ou concorrência no arquivo. Prova a recuperação simples e a autenticação no cenário exercido. |
| `cargo test -j2 -p clipsync-core --test mesh the_agent_finds` | `1 passed; 0 failed; 17 filtered out`, 17,23 s | Dois agentes reais em loopback; A reencontra B por `localhost`, guarda o endereço em `peers.toml` e reporta conectado, com B sem endereço de A. Prova integração do resolvedor/consumidor/persistência simples. Não há disputa de escrita/reload, reinício para verificar durabilidade, DHCP físico nem Windows. |

### Contrato e limites da entrega

Os commits acrescentam `EmulateAdapter::alive`, recuperação do emulador, API pública de resolução/atualização de endereço e `peer.readdressed`, sem delta correspondente nos próprios commits. O §17.4 vigente só enumera as duas mudanças anteriores e restringe outras mudanças naquela fatia. C1 já tem a atualização contratual atribuída nesta rodada: esta revisão aponta a lacuna para C1, sem editar seu arquivo nem inferir que os testes a suprem. Recuperação do transporte não comprova soltura da captura (F2) nem vigia de hooks (F3).

Não iniciei servidor/app, não interagi com perfil real nem aprovei permissões. Sem prova visual, Tauri, Windows ou suspensão física nesta R1; o portal de outro projeto não serve como evidência. Build/install/dev completos do produto não foram executados: escopo focal pedido e app local sem headers WebKitGTK conforme AGENTS.md.

**Comunicação:** `maestri note read` das duas notas retornou `Note ... not found`; as cópias `.superpowers/incidente-2026-11-11/quadro.md` e `evidencias.md` foram lidas. `maestri list` identificou este terminal como `Revisor · achados`, conectado a Designer Maestro e notas de outro projeto, sem `Maestro · scaffold`; `maestri debug` confirmou conexão ativa nesse socket. A tentativa de `note edit` da nota do incidente também falhou. A linha de veredito da nota e a entrega por ask dependem da conexão correta. Linha preparada: **R1: 5a90f0a passa com ressalva; dcbdd64 não passa (D01 reload, D02 persistência atrasada, D03 candidatos sem progresso); 4 testes focais passaram; parecer em docs/verdicts.md.** Disponível para revisar F1, F2 e F3 quando encaminhadas.

## R2 · F2 · 11/11/2026

**Veredito: não passa a promessa completa de prazo desde a abertura/última resposta; dois Important por leitura estática. Os seis testes focais passaram.**

Escopo: diff de F2 contra HEAD nos arquivos `crates/clipsync-input/src/capture.rs`, `src/module.rs` e `tests/s12_failsafe.rs` (este último ainda não rastreado, lido integralmente). Nenhum desses arquivos foi editado pelo revisor. N1 e `clipsync-core` fora desta revisão; nenhum arquivo do core alterado pelo revisor. Não houve commit. Este parecer avalia a entrega no working tree, não um commit novo.

### Achados por severidade

1. **Important · R2-F2-01 · prazo não protege a abertura enquanto ela aguarda trabalho de rede anterior.** `crates/clipsync-input/src/capture.rs:817`, `:1272` e `:749`. O relógio é criado em `opening.session`, mas `watchdog_until` e `watchdog_tick` só consultam `self.session`; a vigia fica desarmada até `opened`. Além disso, a abertura só é sondada quando `closing.is_none()`, e seu timeout de envio só começa no primeiro poll. Sequência permitida: fechamento anterior deixa limpeza de rede pendente; após reinício da geração, uma nova ativação física abre uma tentativa enquanto aquela limpeza ainda existe; a tentativa aguarda o restante do orçamento de limpeza e depois pode gastar mais 2 s no envio. Não há prazo absoluto de 2 s desde essa abertura que devolva a nova captura. Isso não significa que o primeiro fechamento espere rede: ele libera antes, corretamente. O problema é a nova abertura após a reativação. A origem por botão dos testes só chama `cap.begin` depois do envio e não reproduz a captura física já ativa por cruzamento. A correção deve observar o prazo também em `opening`, sem antecipar StatsRequest ao Enter, ou devolver imediatamente uma ativação que não possa começar por limpeza pendente. Pedir regressão com limpeza/envio bloqueados e ativação por barreira. Evidência estática; não reproduzido nesta rodada.

2. **Important · R2-F2-02 · relógio baseado no consumo da fila pode estender o prazo ou descartar resposta pontual.** `crates/clipsync-input/src/module.rs:1604`, `crates/clipsync-input/src/capture.rs:1607` e `:716`. `Ctl::Stats` não transporta horário de recebimento; `last_response` ganha `Instant::now()` quando o proprietário da captura consome o controle. Uma resposta recebida pelo módulo em t=1,0 s e consumida em t=1,9 s renova até t=3,9 s, embora a última resposta tenha chegado em t=1,0 s. Inversamente, uma resposta válida recebida antes do limite, mas ainda enfileirada quando timer e controle ficam prontos, perde para o ramo `biased` do timer e a sessão saudável fecha. Fila de controle, atraso de agendamento ou um handler anterior aguardando adaptador permitem essas sequências; `Ctl::Retarget`, por exemplo, pode aguardar `cap.set_barriers` no mesmo proprietário. A correspondência de par/conexão/sessão está correta; falta preservar a temporalidade da resposta. Levar o instante de recebimento e disponibilizar o último sinal válido ao teste de expiração sem depender da posição na fila. Pedir regressões com resposta recebida antes do limite e consumo atrasado, e com resposta antiga que não pode comprar mais 2 s. Evidência estática; os testes atuais usam filas prontamente consumidas e não reproduzem o problema.

### Comportamento conferido e riscos

- **Soltura antes da rede:** `close_session` retira a sessão, chama `cap.release`/`release_at` (`capture.rs:1018`), cancela a sondagem, limpa a fila, marca `reset_rx` e cria a limpeza como futuro separado. A volta ao laço fecha o receptor antigo, para/aguarda o produtor, descarta eventos de entrada/ativação antigos e só então inicia a geração seguinte. O envio de Leave/ReleaseAll não precede a devolução local. Isso está correto por leitura; o teste de silêncio observa o contador de soltura e outra geração, mas não bloqueia a rede para demonstrar a ordem por execução.
- **Fechamentos concorrentes:** a vigia e return/refused/peer_down/switch/pause/lock usam o mesmo proprietário e retiram `session`/`opening` por `take`; um segundo fechamento sem sessão não duplica `session.closed`. Reconciliação de autoridade e lifecycle antecede o select. Se uma notificação só existir na fila ao vencer o timer, `peer_unresponsive` pode ganhar a classificação; não identifiquei reabertura ou soltura dupla introduzida por F2 nessa disputa. Não há testes focais de simultaneidade desses caminhos. Distinguir essa observação estática de prova de ausência de corrida.
- **Perda, destino ocupado e fila cheia:** o limite é por silêncio de aplicação, mesmo com QUIC conectado. Em condições normais há cerca de três oportunidades de resposta antes do limite. Perda recuperada depois de 2 s, módulo ocupado em `emit`, ou fila de jobs do destino cheia podem devolver a captura de uma conexão que continua viva. O destino usa a fila compartilhada de rede e pode deixar de enfileirar Stats (`module.rs:1588`); esse limite de capacidade já existia. É consequência da política de segurança de 2 s, sem prova de defeito físico do destino. Não prometer continuidade sob congestionamento. Enquanto um envio de sonda está pendente, não sai outra sonda a cada 500 ms (`capture.rs:819` e `:840`); o prazo de silêncio continua vencendo. Os testes só demonstram a cadência com envio livre.
- **Teclas e botões:** no erro genérico legado, o código remove a sessão, chama `release_all` e envia ApplyFailed; Locked mantém o ramo anterior (`module.rs:2002`). O teste prova uma tecla e um botão soltos no fake quando `release_all` funciona. Na expiração, a limpeza tenta Leave e depois ReleaseAll com orçamento conjunto; se Leave falhar ou consumir o prazo, ReleaseAll não é tentado. Se o destino continuar inacessível/ocupado, não há aqui garantia de soltura remota em 2 s: teclas remotas podem permanecer pressionadas até o destino processar fechamento/PeerDown. Isso não invalida a devolução local prometida, mas impede afirmar que ambos os lados sempre ficam sem pressionados. Falha do adaptador local em `release`, thread/OS bloqueados e falha remota em `release_all` também não foram ensaiadas; não acrescento essas condições preexistentes como regressões de F2.
- **Compatibilidade:** usa variantes existentes StatsRequest/Stats desde protocolo 1 e arma por `remote_version >= 1`; não exige versão nova nem muda wire. O manipulador StatsRequest do HEAD v0.5.2 já responde sem movimento para a sessão conhecida. Para v1/v2 a falha legada usa Rejected; para v3+ usa Refused. O teste de falha força v1, não prova o envio Refused v3. Versão anterior do aplicativo não equivale a protocolo 0; pares históricos v1+ continuam incluídos. A vigia mede resposta do módulo, não comprova que o emulador de um destino antigo aplicou a entrada: ele pode continuar respondendo Stats e falhar ao emitir. Compatibilidade física com binários antigos não executada.

### Testes executados e alcance individual

Comando: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test s12_failsafe`. Saída: **`6 passed; 0 failed; 0 ignored; 0 filtered out; finished in 5.46s`**. Não foi necessário interromper processos ou matar espera de lock. Todos usam MemHub e adaptadores fake; não são execução Windows, GNOME nem transporte QUIC físico.

| Teste | O que comprovou | Limite da prova da promessa |
| --- | --- | --- |
| `silencio_de_stats_libera_captura_em_dois_segundos` (`s12_failsafe.rs:202`) | Após abertura por InputSwitch, silêncio fecha com peer_unresponsive mesmo com movimento contínuo; contador de release aumenta, produtor reinicia e há um único fechamento. | Assert aceita até 2,5 s; não mede limite exato de 2 s desde abertura nem bloqueia envio/limpeza. Não cobre R2-F2-01. |
| `sessao_saudavel_ociosa_permanece_ativa_por_cinco_segundos` (`:227`) | Respondedor controlado recebe ao menos oito pedidos; sessão ociosa permanece cinco segundos sem release/fechamento. | Prova caso saudável sem contrapressão/perda; não filas cheias, pausa ou resposta perto do limite. |
| `falha_emit_legada_recusa_e_solta_tecla_e_botao` (`:271`) | Dois módulos, protocolo v1: erro genérico gera apply_failed, solta uma tecla e um botão no fake do destino e devolve a captura da origem. | Prova a alteração legada com Rejected e release bem-sucedido. Não Locked, Refused v3, falha/bloqueio de release ou recusa sem capacidade de rede. |
| `stats_de_outro_par_conexao_ou_sessao_nao_renovam_prazo` (`:340`) | Mensagens com par, conexão ou sessão incorretos, enviadas juntas, não impedem expiração. | Prova a rejeição desse conjunto no cenário; não testa cada dimensão isoladamente nem controle válido atrasado/R2-F2-02. |
| `silencio_conta_da_ultima_resposta_valida` (`:381`) | Mantém respostas além de 2,2 s e, ao silenciar, não fecha imediatamente; espera pelo menos 1,4 s e fecha dentro de 2,5 s da desativação do respondedor. | Prova renovação em condições prontas; não conhece o instante exato da última resposta recebida nem simula backlog. |
| `destino_real_do_modulo_responde_stats_sem_movimento` (`:404`) | Dois módulos reais sobre MemHub/fakes: destino responde pelo menos oito vezes, sessão permanece ociosa cinco segundos e respostas normais não geram last_block. | Prova integração do handler atual, não binário v0.5.2 separado, rede com perda, emulação física ou fila saturada. |

`git diff --check` nos três caminhos revisados terminou com exit 0. Fontes e teste permaneceram estáveis entre leitura e fechamento: SHA-256 capture.rs `595f9a47c9f80b030291d8dad118b1b39bf32ce4f78a75033155a2347d28ea8a`; module.rs `0ad73c78431b195a3c6c8a84a0a706af2b028ff4d2f811c4a4a37265332c94f2`; s12_failsafe.rs `fc3fd46304d6ef89eb549b619948ae63ad65853f2f83a8f0e3e69a9263e28524`.

Entrega pelo arquivo, conforme orientação do Maestro nesta R2; não depende de `maestri ask`. Aguardando correção/re-review de F2 e próximas F1/F3/N1.

## R3 · N1 · 11/11/2026

**Veredito: não passa N1 enquanto R3-N01 estiver aberto; R1-D01, R1-D02 e R1-D03 fechados no recorte original. Testes: 39 passaram, zero falhas.**

Revisão do working tree contra HEAD `c30c529`, limitada ao diff de `crates/clipsync-core`, sem editar fonte, teste ou configuração e sem commit. Li também `.superpowers/incidente-2026-11-11/S1b-sentinel.md`; o parecer dele foi sobre uma revisão anterior do working tree, com hashes diferentes. Este parecer não reabre a auditoria de relay nem aprova F1/F2/F3.

### Achados por severidade

1. **Important · R3-N01 · fallback usa endereço de pareamento anterior ao reload e pode bloquear a entrada corrigida.** `crates/clipsync-core/src/transport.rs:875`, `:895` e `:903`. `paired/learned` são copiados no começo da rodada. Depois do `await` da tentativa de cache, o código disca `addr` sem reler o alvo; em KeyChanged, marca o slot atual Blocked sem conferir se a falha ainda corresponde ao endereço de pareamento atual. Sequência permitida: a rodada lê pareamento P e cache L; a tentativa L fica pendente; o usuário recarrega pareamento Q, removendo L; a tentativa L termina; o fallback ainda disca P; outra máquina responde em P e o erro põe o slot de Q em Blocked. O discador retorna; Q não é tentado por ele. Isso não é o bloqueio correto de uma chave trocada no endereço vigente, nem é evitado pelo guard de `learn()`, pois ocorre no ramo anterior à resolução de nome. A janela de KeyChanged contra reload já existia durante a tentativa direta; N1 acrescenta a espera de cache antes de iniciar essa tentativa sem revalidar o alvo. Revalidar alvo/instância após a espera do cache e conferir a validade do resultado sob o lock antes de bloquear ou publicar o bloqueio. Pedir regressão com cache silencioso, P respondendo com outra chave e reload para Q válido durante a tentativa do cache. Achado estático, não reproduzido pelos testes desta rodada.

2. **Minor · R3-N02 · eventos de cache podem sair em ordem diferente das mutações que representam.** `crates/clipsync-core/src/transport.rs:999`, `:1008`, `:562` e `:1026`; consumidor em `src/agent.rs:197`. A alteração de `Slot` usa o mutex, mas Learned/Forgotten são enviados depois de soltar esse mutex. Em runtime com múltiplas threads: `learn` instala L e solta o lock; reload remove o peer e envia Forgotten; a execução de `learn` retoma e envia Learned. O consumidor único grava novamente uma entrada do peer já removido. A ordem inversa entre Forgotten de uma revisão e Learned da seguinte também pode apagar cache novo. O único escritor do arquivo evita disputa entre escritas, mas não reordenação na origem. O endereço antigo não muda peers.toml, e prune/start_with_cache tornam uma entrada incompatível inaplicável na próxima partida: não demonstrado bypass de confiança. Contudo, a promessa de remoção imediata dos metadados e de espelhar o estado atual no cache fica incompleta. Publicar a atualização junto da mutação serializada, ou transportar revisão e descartar eventos obsoletos. O caso sequencial de S1b-PRIV-01 foi corrigido; este é o resíduo concorrente, por leitura, sem reprodução.

3. **Minor · R3-N03 · status pode anunciar endereço aprendido que falhou, mesmo conectado pelo pareamento.** `crates/clipsync-core/src/transport.rs:482`, `:890` e `:895`. Se o cache L não responde e o endereço de pareamento P conecta, o ramo Other mantém `slot.learned=L` e o sucesso de P não o limpa. `peer_statuses` continua preferindo L a P, apresentando o endereço sem resposta como o do peer conectado. Nas reconexões/partidas seguintes ainda há a espera extra no cache, reconhecida como ressalva de disponibilidade pelo Sentinel; o defeito adicional aqui é a informação de status divergente do caminho que conectou. Limpar/superar o aprendido depois do sucesso em P, ou representar separadamente endereço de conexão e candidato do cache. Achado estático, não coberto pelos testes.

### Fechamento dos achados R1 e segurança

- **D01 fechado quanto a sobrescrever o endereço recarregado.** `learn` só altera `Slot.learned` quando `slot.entry.addr == Some(paired)`, conferido e alterado sob o mesmo mutex (`transport.rs:998`). Se reload para Q vence o lock, o resultado iniciado para P não é aprendido; se learn vence, reload limpa o aprendido para P. `slot.entry.addr` não é substituído pelo resultado do nome. A conexão autenticada que terminou durante essa transição pode permanecer, conforme o desenho declarado. R3-N01 trata outro efeito do reload, sobre bloqueio e tentativa de fallback, não a antiga escrita em `entry.addr`.
- **D02 fechado.** O consumidor de AddressUpdate não chama update_peers: escreve exclusivamente known-addrs.toml (`agent.rs:198`). `LearnedAddr` carrega `peer/paired/learned`; a partida poda o arquivo por peer e endereço atual e a mesh repete essa conferência antes de criar discadores (`transport.rs:332`). Evento atrasado pode perder/reintroduzir cache, conforme R3-N02, mas não substitui endereço, permissões ou outro campo editado em peers.toml. O teste de agente conferiu preservação byte a byte desse arquivo.
- **D03 fechado para lista estável.** A lista é filtrada e deduplicada; o cursor avança inclusive ao saltar o candidato já tentado como cache (`transport.rs:962`). A rodada seguinte continua a lista. O teste do quarto candidato efetivamente atravessa os três sockets silenciosos antes dele e passou. Não há promessa de justiça com respostas cuja ordem/conteúdo muda adversarialmente.
- **S1-SEG-01 permanece fechado no fluxo novo, no recorte funcional inspecionado.** KeyChanged no cache chama forget e segue o fallback, sem Blocked; os candidatos do nome também não bloqueiam por outra chave. A importação não promove o cache a pareamento e continua exigindo o pin. O teste do estranho no cache importado e o ciclo disco/reinício do agente dão evidência complementar. Não executei relay. Como já ressalvou o Sentinel, esta mudança não identifica nem desfaz endereços que a v0.5.2 anterior já tenha gravado em peers.toml sem procedência.
- **Caminho antigo preservado no cenário normal:** `mesh_changed_key_is_blocked_with_constant_reason` passou nesta execução. Quando a tentativa usa o endereço vigente de pareamento, outra chave continua causando Blocked com o motivo constante, sem identificadores da chave na recusa TLS. Se um cache válido conecta primeiro, o endereço de pareamento não é discado naquela rodada, conforme o desenho. A regressão concorrente de R3-N01 não é exercida pelo teste antigo.

### Testes executados e alcance

Comando próprio: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-core --test readdress --test profile --test mesh`. Aguardei a trava de build; nenhum processo foi interrompido. Saídas: **mesh 18 passed, 0 failed (16,10 s); profile 16 passed, 0 failed (0,14 s); readdress 5 passed, 0 failed (32,17 s)**. Exit final 0. `git diff --check -- crates/clipsync-core` também exit 0.

| Teste novo/alterado | Prova observada e limite |
| --- | --- |
| `a_peer_that_changed_address_is_found_by_its_name` | QUIC em loopback: ignora outra chave no candidato, conecta ao peer, expõe endereço aprendido e emite Learned com paired correto. Não grava arquivo nem intercala reload com a tentativa. |
| `a_stranger_at_a_learned_address_never_blocks_the_peer` | Importa cache ocupado por estranho, descarta esse cache e conecta ao destino resolvido, observando Forgotten antes de Learned no caso sequencial. Prova o ramo tolerante do cache; não o fallback simultâneo ao reload de R3-N01 nem relay. |
| `a_candidate_past_the_first_three_is_reached_on_a_later_round` | Três candidatos silenciosos precedem o endereço válido; a conexão acontece em rodada posterior. Fecha a regressão de prefixo fixo. Não mede exatamente cada timeout nem justiça sob lista mutável. |
| `a_reload_that_changes_the_pairing_address_drops_what_was_learned_for_the_old_one` | Conecta primeiro pelo cache, depois recarrega endereço e confere descarte/Forgotten; remove peer e observa outro Forgotten; inicia nova mesh e rejeita cache para pareamento antigo. Prova as transições sequenciais. Não há handshake pendente, learn em paralelo, consumidor real do cache durante a remoção ou as ordens de R3-N01/N02. |
| `the_system_resolver_answers_with_the_port_it_was_given` | localhost mantém a porta, nomes vazios/inválidos e FQDN são recusados. Host_name agora exige um rótulo ASCII de até 63 bytes. Não mede resolução DNS/mDNS/LLMNR/NetBIOS real em duas máquinas; o nome simples ainda pode consultar DNS e sufixos configurados. |
| `learned_addresses_are_a_private_cache_that_never_touches_the_pairing_file` | Ausência de arquivo vira cache vazio; salva/substitui por peer, modo 0600 no Unix, esquece idempotentemente, poda, tolera formato novo/TOML inválido e preserva peers.toml. Não simula falha de I/O, arquivo especial/excessivo, ACL Windows nem escritores concorrentes. O único escritor de produção justifica não acrescentar lock de cache para esta arquitetura. |
| `the_agent_finds_a_readdressed_peer_by_name_and_caches_the_address` | Dois agentes em loopback: reencontro via localhost, persistência separada, peers.toml idêntico e reconexão após reiniciar em menos de 8 s, antes dos 10 s de espera do pareamento antigo. Prova que o cache entra antes dos discadores. Não intercala escrita/reload nem testa o status depois de fallback bem-sucedido de R3-N03. |

As outras regressões de mesh/profile passaram no mesmo comando, incluindo revogação, pausa, compatibilidade e a concorrência preexistente de update_peers. Isso não é execução de todo o workspace, clippy, fmt, CI Windows ou teste físico.

### Prazos, estabilidade e limites

Os 3 s limitam o handshake das tentativas de cache/nome; Hello conserva orçamento próprio de até 5 s. Com cache silencioso, pareamento silencioso, resolução do sistema até 3 s e três candidatos silenciosos: aproximadamente 3 + 10 + 3 + 9 = 25 s antes de backoff/defer, por leitura, sem medição desse cenário completo. Isso reduz o custo anterior e não garante conexão instantânea. Falha/corrupção/versão nova do cache recai no caminho sem cache; pin e permissões continuam vindos do pareamento.

SHA-256 estáveis entre início e conferência posterior à leitura: agent.rs `90f5211e9459d21a2a542e79f92e77e7e2108de209987da42d490fb293598524`; profile.rs `bf8ad75e196244041cc017997ea2d0011e7c73f6481776c52afc6782942a5f1c`; transport.rs `db645722e795369fbf92f7348665e353d28dd2e4aa363a94a418e3c3e8502253`; tests/readdress.rs `c5d900b55d40b9197f55f739b8fac071fb06bc1e765a62bddef7e7ec7647bf22`; tests/profile.rs `c9bd6cccc88ec0cd4b841ec0b70e17c0ae11a7778decf316062396affb88dfe8`; tests/mesh.rs `7c827b4699debd4bfdc35df113c3dace700dce9811790aa920c87e541da3e694`.

Sem perfil real, segredo, firewall, servidor de interface ou app iniciado pelo revisor. Entrega por este arquivo para leitura do Maestro; sem maestri ask conforme fluxo combinado. Falta fechar R3-N01 e sua regressão; R3-N02/N03 permanecem ressalvas Minor.

## R4 · F2 rodada 2 · 11/11/2026

**Veredito: passa no recorte revisado, com ressalvas de carga e cobertura; R2-F2-01 e R2-F2-02 fechados. Nove testes focais passaram; nenhum novo Critical/Important identificado.**

Re-revisão do working tree de `crates/clipsync-input/src/capture.rs`, `src/module.rs` e `tests/s12_failsafe.rs`, contra HEAD e contra os achados de R2. Nenhum fonte/teste foi editado pelo revisor, sem commit. F1 e suas falhas de `eis_devices` ficam fora deste parecer; não repeti a suíte completa de input. Os números de 354 aprovados e duas falhas são relato do autor, não execução desta R4.

### Fechamento de R2

1. **R2-F2-01 fechado.** `capture.rs:837` consulta `current()`, que inclui a sessão em opening. `watchdog_tick` também alcança `opening.session` e compara o prazo absoluto antes de criar trabalho de rede (`:850`). O tempo gasto esperando a limpeza anterior e o envio do Enter não reinicia esse relógio. O guard `!opening` impede que a vigia emita StatsRequest antes de o envio do Enter concluir. A regressão por barreira e rede bloqueada passou: já não é apenas o caminho InputSwitch em que a captura começa depois do envio.

2. **R2-F2-02 fechado no recorte da fila entre módulo e captura.** `module.rs:1608` marca `received_at` ao tratar Stats após validar a conexão corrente; `capture.rs:889` usa esse instante, confere par/conexão/sessão, rejeita horário anterior ao último confirmado, posterior ao prazo então vigente ou no futuro. Consumo atrasado não acrescenta tempo ao silêncio. Antes de expirar, a varredura considera Stats já enfileirado, evitando que a prioridade do timer descarte a resposta pontual que estava esperando o proprietário da captura. As duas regressões de recebimento/consumo atrasado passaram. O instante é o de tratamento no módulo, não um timestamp de chegada ao socket; isso corresponde ao mecanismo proposto nesta rodada.

### Lote e controles adiados

A expressão `0..ctl.len()` (`capture.rs:727`) captura a quantidade uma vez. A varredura só faz `try_recv`, sem aguardar handlers e sem aumentar o número de iterações por novas entradas; portanto um fluxo novo não prolonga indefinidamente esse mesmo lote. Stats é antecipado deliberadamente para atualizar o relógio. Os outros controles retirados entram no fim de `deferred_controls`, na mesma ordem em que saíram do canal (`:732`).

Não identifiquei inversão entre controles **não Stats**: os três consumidores do canal passam a preferir os adiados, tanto no laço normal (`:738`) como na preparação de inventário (`:682`) e após parar a geração (`:581`). Uma varredura posterior acrescenta controles ao fim dos já adiados. O futuro de consumo, quando retira um adiado, retorna pronto sem outro await; com a prioridade atual do select não há ponto de suspensão depois de retirar o controle que o deixe perdido por cancelamento do ramo. Stop/desconexão continuam encerrando o proprietário; descartar trabalho posterior ao encerramento não é perda de um controle que devesse executar depois de Stop.

A antecipação de Stats pode renovar o relógio da sessão anterior antes de processar um Switch/Paused/Stop mais antigo na fila, mas não executa esse controle fora de ordem, não muda a identidade da sessão e não transfere a resposta a uma sessão nova. O fechamento posterior segue retirando a sessão por `take`; não identifiquei reabertura ou limpeza duplicada introduzida pelo mecanismo. Isso é leitura estática dos caminhos, não teste de todas as intercalações.

**Ressalva de carga:** o lote é fixado pelo tamanho observado, **não limitado por uma constante**. O canal e a fila adiada não têm teto de quantidade. Um backlog muito grande pode consumir CPU/memória e atrasar a execução de `watchdog_tick` durante a varredura síncrona, ainda que entradas novas não estendam o lote atual. Não medi esse atraso nem demonstrei violação física dos 2 s. Não interpretar a aprovação como prova sob flood/fila extrema. Se esse cenário entrar no aceite, exigir medição de latência e um mecanismo de último sinal que não precise percorrer toda a fila antes da soltura; simplesmente truncar a busca pode voltar a perder respostas pontuais.

**Ressalva de cobertura:** os testes novos exercitam uma resposta atrasada durante `begin`, mas não uma sequência mista de controles não Stats extraídos para `deferred_controls` junto de Stats, Stop, pausa e preparação. A preservação FIFO foi conferida no código. Uma regressão dessa sequência seria útil para proteger futuras alterações; não há defeito de reordenação demonstrado nesta revisão.

### Execução própria e alcance dos testes

Comando: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test s12_failsafe`. Saída observada: **`9 passed; 0 failed; 0 ignored; 0 filtered out; finished in 5.41s`**, exit 0.

| Regressão nova | O que prova e limite |
| --- | --- |
| `abertura_por_barreira_tem_prazo_mesmo_com_limpeza_e_envio_bloqueados` (`s12_failsafe.rs:592`) | Ativa fisicamente pelo fake de barreira, bloqueia a limpeza anterior, reinicia o produtor, ativa de novo e bloqueia o segundo Enter. Observa release e peer_unresponsive em janela de até 2,4 s, confirma que o segundo Enter realmente bloqueou e que não houve begin por botão. Prova a correção da soma de esperas de R2-F2-01 com tolerância de teste; não mede limite físico exato de 2 s nem o tempo de release do Windows. |
| `stats_recebido_antes_do_limite_sobrevive_ao_consumo_depois_do_timer` (`:712`) | Módulo recebe Stats em aproximadamente t=1 s enquanto begin segura o proprietário; libera begin em t=2,1 s e confere sessão aberta em t=2,4 s, sem release. O comando Status confirma que o módulo tratou a resposta antes de liberar begin. Exercita a varredura antes da expiração e fecha a falsa recusa de R2-F2-02 nesse cenário. |
| `stats_atrasado_na_fila_nao_compra_tempo_no_instante_do_consumo` (`:726`) | Recebe em aproximadamente t=1 s, permite consumir em t=1,9 s, mantém a sessão em t=2,7 s e exige fechamento por silêncio em t=3,25 s. Diferencia prazo contado do recebimento e do consumo: a resposta não renova até t=3,9 s. |

As seis regressões anteriores também passaram: silêncio com entrada contínua e geração nova; sessão saudável ociosa com respondedor controlado; falha genérica legada soltando tecla e botão; Stats com par/conexão/sessão incorretos; prazo desde a última resposta válida; dois módulos respondendo sem movimento. O alcance e os limites individuais registrados em R2 permanecem: fakes/MemHub, protocolo v1 forçado nesses cenários, sem ensaio físico, perda de rede ou saturação de fila. Não repeti o teste de Locked isolado nem reivindico nova prova do ramo Refused v3.

### Limites preservados e estabilidade

A soltura local continua antes da limpeza de rede; fila de entrada e geração antigas são descartadas. O destino mantém a alteração de erro genérico legado para ApplyFailed e release_all, preservando o ramo Locked. A política ainda devolve a captura após silêncio de aplicação em rede congestionada ou destino ocupado; responder Stats não comprova que um emulador antigo aplicou entrada. A soltura remota sob destino inacessível continua sendo tentativa de Leave/ReleaseAll, sem garantia física de 2 s. Essas ressalvas de R2 não foram fechadas nem promovidas a regressões novas.

`git diff --check` dos três caminhos terminou com exit 0. SHA-256 do recorte conferido: capture.rs `025e2f90a06f60113b943ae79363e95d13a185102b0f2629b21de758feb9cf5c`; module.rs `0bf612bcbf9a91555b5699f79e98ad7b3bd955739190f36040eea6f7780a60a5`; s12_failsafe.rs `76a0cec59db2128a3802b5d449504b6ae5547cef354f8547ed162af7798df63f`.

Parecer entregue por este arquivo para leitura do Maestro. Sem servidor/app iniciado, perfil real, permissão de sistema, edição de core, commit ou push.

## R5 · N1 rodada 2 · 11/11/2026

**Veredito: passa no recorte revisado. R3-N01, R3-N02 e R3-N03 fechados; S1b-PRIV-01 fechado no fluxo normal de remoção/poda. Execução própria: 41 testes passaram, zero falhas. Nenhum novo achado Critical/Important/Minor identificado neste recorte.**

Re-revisão do diff de `crates/clipsync-core` contra HEAD `c30c529`, no working tree sem commit. Nenhum fonte, teste ou configuração editado pelo revisor. F1/F2/F3 fora desta revisão. A aprovação não é aceite físico nem aprovação de publicação.

### Fechamento dos achados

1. **R3-N01 fechado.** `crates/clipsync-core/src/transport.rs:896` relê o alvo após a tentativa de cache e só continua para o pareamento da rodada se ele ainda for o atual e não houver conexão ativa. Se reload aconteceu durante essa espera, a rodada recomeça. A segunda guarda, em `:914`, compara o endereço sob o mutex antes de marcar Blocked: uma chave errada recebida depois de o endereço ter sido substituído não bloqueia a entrada corrigida. Assim, a releitura evita a tentativa obsoleta conhecida e a guarda final cobre alteração ocorrida durante a tentativa seguinte. O motivo de bloqueio no endereço vigente permanece o anterior; o teste `mesh_changed_key_is_blocked_with_constant_reason` passou nesta execução.

2. **R3-N02 fechado por serialização da mutação e da publicação.** `learn` altera o aprendido e envia Learned ainda com o mutex dos slots tomado (`transport.rs:1019` e `:1027`). `forget` faz o mesmo com Forgotten (`:1040` e `:1048`). Reload envia Forgotten no mesmo trecho protegido, tanto ao remover o peer (`:530`) como ao descartar cache por mudança de pareamento (`:549`). Não sobra, entre esses produtores, a janela em que a mutação mais antiga podia publicar depois da nova. A tarefa única de `agent.rs:197` consome essa ordem e faz cada escrita antes de receber a seguinte. Isso fecha a reintrodução de metadados depois de uma remoção por evento fora de ordem descrita em R3.

3. **R3-N03 fechado.** Depois de concluir uma discagem bem-sucedida ao pareamento, `transport.rs:905` chama `forget(peer, None)`. O aprendido deixa de prevalecer no status, Forgotten é publicado, e as próximas rodadas/partidas não precisam começar pelo cache silencioso depois de a remoção ser persistida. O novo teste confirmou o endereço do pareamento no status e o evento de descarte. A atualização ocorre após a instalação da conexão; não a interpreto como promessa de atualização atômica de status e conexão no mesmo instante.

4. **S1b-PRIV-01 fechado no fluxo normal, inclusive o resíduo concorrente de R3-N02.** A remoção do slot publica Forgotten mesmo se ele já não tinha aprendido em memória. Na partida, `agent.rs:159` poda entradas de peer ausente ou com outro endereço de pareamento antes de passar o cache a `start_with_cache`. O teste existente de reload observa Forgotten ao remover o peer e o teste de profile observa poda persistida. Falha de I/O, cancelamento do consumidor antes de gravar ou encerramento abrupto ainda podem deixar dados até a próxima poda bem-sucedida; não há nova garantia de flush síncrono na revogação, nem isso é necessário para preservar pin/permissões.

### Envio sob mutex

Não identifiquei novo risco de deadlock nesse envio. O canal é `mpsc::UnboundedSender<AddressUpdate>`: `send` enfileira sincronamente, sem aguardar capacidade, resposta do receptor ou operação de disco. Os payloads contêm apenas os dados tipados de endereço/peer. Nenhum await foi colocado sob o mutex. O consumidor de `agent.rs:197` só usa Profile para gravar/esquecer, sem voltar a tomar o mutex dos slots; portanto não há dependência circular entre produtor com o mutex e consumidor do canal. O log de `learn` ocorre depois do `drop(slots)` explícito. A seção crítica ficou maior apenas pelo enfileiramento, não pela gravação em disco.

Essa conclusão é leitura do fluxo e das dependências de lock, não medição sob saturação. O canal continua sem teto de quantidade e o cache continua com um único escritor de produção, como no desenho anterior. Nenhum novo produtor de escrita direta em peers.toml foi introduzido.

### Evidência própria e alcance

Comando: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-core --test readdress --test profile --test mesh`. Exit final 0:

- **mesh: 18 passed, 0 failed**, 15,33 s;
- **profile: 16 passed, 0 failed**, 0,24 s;
- **readdress: 7 passed, 0 failed**, 32,10 s.

`a_pairing_address_corrected_during_a_round_is_not_blocked_by_the_old_one` (`tests/readdress.rs:343`) força a identidade da origem menor para evitar o defer, usa cache silencioso e outra chave no endereço antigo, recarrega o endereço válido durante a espera e exige que o peer conecte sem Blocked. Prova o cenário principal de N01. A guarda de KeyChanged durante a tentativa direta foi também conferida no código; o teste não pausa separadamente essa tentativa para isolar só essa segunda guarda. Não removi guardas nem repeti o ensaio de mutação: a falha sem as duas proteções é relato do autor.

`a_pairing_address_that_works_again_drops_the_cache` (`tests/readdress.rs:393`) usa cache silencioso e pareamento válido, espera a conexão, exige o endereço correto no status e recebe Forgotten. Prova a correção de N03 em memória/canal. Não executa um reinício específico desse cenário; o teste de agente já existente cobre persistência/importação/reinício do cache no fluxo de aprendizado.

As cinco outras regressões de readdress passaram, incluindo quarto candidato, estranho no cache importado, descarte no reload e filtro de nomes. Mesh voltou a provar o bloqueio legítimo por chave trocada, o reencontro com peers.toml preservado e o uso do cache após reinício. Profile voltou a provar modo 0600 no Unix, substituição/descarte/poda e tolerância a cache inválido. N02 foi fechado por inspeção da seção crítica e dos produtores/consumidor; não houve teste de estresse de ordenação concorrente.

`git diff --check -- crates/clipsync-core` terminou com exit 0. A suíte core completa em 19 blocos e o clippy limpo são informações do Maestro; nesta R5 executei os três alvos acima, sem reivindicar repetição dos demais.

### Estabilidade e limites

Os seis arquivos core revisados conservaram os mesmos SHA-256 entre a leitura inicial e a conferência posterior: transport.rs `670733050076ed19bdccc7ee8335103becbcf10e75c88872fe6d65bb70da4b8f`; agent.rs `90f5211e9459d21a2a542e79f92e77e7e2108de209987da42d490fb293598524`; profile.rs `bf8ad75e196244041cc017997ea2d0011e7c73f6481776c52afc6782942a5f1c`; tests/readdress.rs `432a99c6c5bb82c9658218636184322aa3412fb67235b92b5791a0685e97eb30`; tests/mesh.rs `7c827b4699debd4bfdc35df113c3dace700dce9811790aa920c87e541da3e694`; tests/profile.rs `c9bd6cccc88ec0cd4b841ec0b70e17c0ae11a7778decf316062396affb88dfe8`.

R1-D01/D02/D03 e o fechamento funcional do fluxo novo de S1-SEG-01 permanecem válidos. Continuam os limites já registrados pelo Sentinel: cache não saneia automaticamente peers.toml escrito pela implementação antiga, consulta do nome simples pode alcançar DNS configurado, e os prazos reduzidos não garantem reencontro sob supressão contínua da rede. Sem relay, Windows nativo, ACL Windows, GNOME ou teste físico nesta execução. Nenhum perfil real, segredo, firewall ou app operado. Entrega pelo arquivo para leitura do Maestro; sem commit/push.

## R6 · F1 · 11/11/2026

**Veredito: não passa a alegação de prontidão sem regressão na integração com F2: um Important, R6-F1-01. Sete testes EIS passaram; instrumentação aprovada no recorte de privacidade. A causa do teclado morto na primeira sessão continua não provada.**

Revisão do working tree de `crates/clipsync-input/src/adapter/portal.rs`, `tests/eis_devices.rs` e `examples/eis_probe.rs`, com leitura de `module.rs`, do prazo de F2 e do contrato §17.8. Nenhum código/teste editado, sem commit. A sonda não foi executada, não abriu portal e não recebeu permissão do sistema.

### Achados por severidade

1. **Important · R6-F1-01 · espera adicional pelo teclado pode fazer a primeira entrada de mouse perder o prazo de F2.** `crates/clipsync-input/src/adapter/portal.rs:1002` e `:1166`; integração em `src/module.rs:1757` e `:1789`; prazo de origem em `src/capture.rs:83` e `:837`. Depois de o ponteiro ficar pronto, `start()` pode aguardar outros 500 ms antes de retornar. O módulo espera start dentro do tratamento do Enter, só depois posiciona o mouse e instala Incoming; nesse intervalo não processa StatsRequest enfileirado. F2 conta 2 s desde a abertura na origem. Exemplo permitido: o ponteiro está pronto em aproximadamente t=1,7 s desde a abertura e o teclado não aparece; antes desta espera, o módulo poderia posicionar/responder antes de t=2 s; agora só prossegue em aproximadamente t=2,2 s, quando a origem já devolveu a captura por peer_unresponsive. Mesmo com ponteiro rápido, há atraso novo de até 500 ms no primeiro posicionamento. O teto de prontidão EIS passa a até 5 s para ponteiro + 500 ms para teclado, além das etapas de portal, sem alinhamento ao prazo de F2. Separar a espera opcional do teclado do posicionamento/atendimento inicial do ponteiro, ou resolver explicitamente a admissão inicial em conjunto com a vigia, preservando a soltura por silêncio. Pedir regressão de dois módulos com ponteiro pronto perto do limite e teclado ausente/tardio. Achado estático de integração; não foi reproduzido em GNOME nem pelos sete testes, que chamam attach diretamente e não exercitam F2.

2. **Minor · R6-F1-02 · a sonda só restaura NumLock por tentativa e conferência, não em toda saída possível.** `crates/clipsync-input/examples/eis_probe.rs:67`, `:88` e `:154`. O fluxo normal é cuidadoso: exige LEDs observáveis/coerentes antes de abrir o portal; tenta release_all e restore mesmo se a sequência retornar erro; confere os indicadores iniciais e chama stop antes de propagar esses resultados. Porém, se só um toque for aplicado e depois a sessão morrer, restore pode precisar de outro toque que já não consegue emitir; erro de snapshot, desaparecimento/divergência dos LEDs ou interrupção do processo também podem deixar NumLock diferente. Não há recuperação instalada para interrupção/cancelamento entre o primeiro toque e restore. As mensagens de erro evitam alegar restauração nesses retornos normais, mas não desfazem o estado. Portanto a resposta é **sim, a sonda pode deixar NumLock trocado**. Mantê-la como ferramenta manual com essa limitação explícita e conferência pelo usuário, sem promessa de restauração garantida. Não a executei para comprovar comportamento físico; este achado não é uma falha remota do produto.

### Emissão parcial e composição com F2

`Emulator::emit` (`portal.rs:663`) percorre o lote, salta capacidades indisponíveis, emite os pedidos dos dispositivos disponíveis, chama frame/flush e depois retorna o primeiro erro de capacidade ausente. Isso preserva pedidos válidos inclusive depois do evento sem dispositivo, mas **não preserva a sessão do módulo**.

Com a F2 atual, a resposta à pergunta do Maestro é **sim**: um lote legado contendo Motion, Key sem teclado e Motion pode mover o ponteiro e, em seguida, causar a remoção de Incoming, release_all e recusa ApplyFailed (`module.rs:2000` e `:2011`). A origem recebe a recusa e devolve a captura. Mesmo uma tecla sozinha enquanto o teclado estiver temporariamente pausado/ausente basta. Quando o teclado aparecer depois, a chamada direta ao adaptador pode funcionar sem recriá-lo, como mostra o teste; a sessão anterior do módulo já terá sido encerrada, exigindo nova entrada.

Isso é a composição da decisão de F1 de devolver erro com a política de F2 de fechar em erro genérico; não é um novo desvio dessa política de segurança, e não recomendo esconder falhas genéricas para contorná-lo. Entretanto, não anunciar o resultado como continuidade de mouse/teclado durante falta transitória de uma capacidade. Se continuidade for requisito, precisa de resultado tipado e política explícita para emissão parcial/indisponibilidade, com regressão integrada. No ramo v4, o tratamento genérico já existente usa Locked (`module.rs:2003`), portanto o mesmo problema de capacidade não é apresentado como ApplyFailed nesse caminho; essa classificação é anterior a F1 e fica registrada como limite, sem atribuir a esta entrega sua introdução.

### Instrumentação, protocolo e privacidade

R1-E01 fica fechado quanto ao break calado: fim do stream, desconexão anunciada e erro do conversor passam a ter categorias distintas em eis.loop. `loop_error` converte as variantes em literais; não serializa o erro bruto nem o evento recebido. Os campos novos de eis.device são ação/origem fixas, número local e capacidades escolhidas de uma lista fixa. As contagens são inteiros acumulados por dispositivo e pela vida do laço, não por sessão. Não identifiquei caminho novo desses emissores para tecla, keycode, nome remoto de dispositivo, keymap ou conteúdo no log.

A contagem total usa `keys_sent`, separado de `devs`, e por isso não perde os pedidos do dispositivo removido. ReleaseAll também conta os pedidos de soltura que envia. Contar pedidos não comprova aplicação no GNOME, nem flush aceito/processado se houver falha posterior de I/O. O teste e o contrato tratam da vida do laço, não de quantidade de teclas físicas ou conteúdo.

A remoção do tratamento do evento DeviceStopEmulating recebido está coerente com o recorte declarado de sender; o teste de pausa/retomada já não inventa essa notificação. Não inferi disso uma causa física do incidente. Os demais caminhos antigos de erro de emissão/portal não são revalidados como uma auditoria geral de logs por este parecer.

### Sete testes: execução e alcance individual

Comando próprio: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test eis_devices`. Saída: **`7 passed; 0 failed; 0 ignored; 0 filtered out; finished in 5.01s`**, exit 0.

| Teste | O que prova e o que não prova |
| --- | --- |
| `primeira_tecla_espera_o_teclado_e_a_recriacao_volta_a_emular` (`eis_devices.rs:319`) | O teclado do servidor fake aparece depois do ponteiro, dentro da espera curta; attach não termina antes dele. Pedidos de tecla seguem start_emulating, e a remoção/recriação recebe novo start e novas teclas sem usar o dispositivo antigo. Não reproduz o estado interno de libeis/mutter nem demonstra que esse atraso era a causa real do incidente. |
| `pausa_retoma_e_resumed_repetido_nao_duplicam_start` (`:384`) | Após pausa, emitir tecla retorna erro; retomada requer outro start; resumed repetido sem pausa não duplica start enquanto emulando. Prova pedidos/transições no fake. Não testa a reação do módulo ao erro, que pode encerrar a sessão antes da retomada. |
| `ponteiro_sem_teclado_inicia_com_espera_curta_e_preserva_o_lote_de_mouse` (`:424`) | Attach só com ponteiro retorna entre a margem inferior de 450 ms e o timeout de 1,2 s; registra missing/keyboard/timeout. O lote misto retorna erro, mas o servidor observa duas motions, e teclado adicionado depois recebe pedidos no mesmo adaptador. O harness conta pedidos de movimento, não aplicação visual de frames no sistema. Não prova continuidade da sessão nem o prazo de F2. |
| `teclado_sozinho_nao_substitui_o_ponteiro_obrigatorio` (`:478`) | Sem ponteiro, attach devolve InputError::Eis após a espera. Prova que teclado não substitui a capacidade obrigatória; não mede libeis real, permissão ou tempo de entrada do módulo. |
| `log_conta_dispositivos_removidos_e_explica_o_fim_do_laco` (`:497`) | Dois pares de pedidos de tecla em teclados distintos somam 4 no fechamento, mesmo após remover o primeiro; ponteiro e dois teclados têm fechamento, e cada teclado registra contagem 2. O motivo final é closed. Não é contagem por sessão nem prova de quatro teclas aplicadas no sistema. |
| `erro_do_conversor_tem_categoria_segura_no_log` (`:549`) | Um done duplicado provoca erro real do conversor reis, encerra o laço/alive e registra motivo converter/erro event, sem texto bruto. Prova essa categoria; os outros ramos foram conferidos no match, não injetados todos. |
| `fim_remoto_distingue_eof_de_desconexao_anunciada` (`:584`) | Fecha o socket com e sem aviso e observa disconnected versus eof. Confere a lista permitida de nomes de campos nos eventos capturados. A lista de campos isoladamente não prova valores seguros; a leitura dos emissores fixos complementa a evidência de privacidade. |

Os testes usam reis sobre UnixStream e o mesmo spawn_emulate_loop/wait_emulate_ready do adaptador por attach, sem serviço de portal. O servidor é um simulador de pedidos/transições, não libeis nem GNOME. Os testes não passam por State::on_enter/on_datagram e não fecham a integração com F2.

### Sonda, estabilidade e limites

Executei apenas compilação da sonda: `PATH=$HOME/.cargo/bin:$PATH cargo check -j2 -p clipsync-input --example eis_probe`, **exit 0**, `Finished dev profile ... in 6.32s`. Não houve `cargo run` nem execução do binário. O perfil da sonda é temporário pelo código; nenhum perfil real foi lido nesta revisão.

`git diff --check` dos três caminhos terminou com exit 0. SHA-256 estáveis entre leituras: portal.rs `8484a35bf2018387cd90f0602761947afc218fc7e1205bdde1674f22300e2059`; eis_devices.rs `9f32312e0a2cdb3318641791d2794ae87650f69f6e62b46d4c77d079be8aedfb`; eis_probe.rs `18a5a59665a2cb2cdbc8b8466e862b2d31745a9a4a5101939974e217cfa6b85c`.

Sem nova prova física, servidor/app iniciado, operação de LED, permissão, commit ou push. Falta resolver R6-F1-01 e explicitar o alcance da emissão parcial; instrumentação e testes aprovados não significam que o teclado morto tenha sido explicado ou corrigido.

## R7 · F1 rodada 2 · 11/11/2026

**Veredito: não passa a garantia de soltura na transição do teclado, R7-F1-01 Important; R6-F1-01 fechado. Dezoito testes focais passaram. Descarte sem indicação no estado da interface fica como R7-F1-02 Minor.**

Working tree revisto: `crates/clipsync-input/src/adapter/portal.rs`, `tests/eis_devices.rs`, `tests/s13_teclado_ausente.rs` e `examples/eis_probe.rs`; integração com `module.rs`, `capture.rs` e leitura do contrato §17.8. Sem edição de código/teste, execução da sonda ou commit. A decisão do Maestro de continuar a sessão do mouse quando só falta teclado é respeitada neste parecer; não solicito restaurar o fechamento genérico nesse caso.

### Achados por severidade

1. **Important · R7-F1-01 · release descartado deixa press anterior sem reconciliação na retomada.** `crates/clipsync-input/src/adapter/portal.rs:704`, `:745`, `:916` e `:938`. Sequência permitida pelo cliente: press de modificador é enviado e entra em `self.pressed`; DevicePaused marca o dispositivo indisponível; o release correspondente chega nesse período e é descartado em drop_key sem atualizar nem reconciliar aquele estado; DeviceResumed só marca resumed e encerra o episódio de descarte. Não há soltura dos pressionados anteriores antes de aceitar novas teclas. O conjunto local continua considerando o modificador pressionado, e o código não garante neutralizar o estado no destino enquanto a mesma sessão prossegue. Se o servidor mantiver o press na pausa do mesmo dispositivo, o modificador permanece ativo até outra soltura ou fechamento; outros eventos podem adquirir esse modificador. Não afirmo reprodução física de tecla presa no GNOME, nem que todo servidor preserve esse estado ao pausar: a evidência estática é a ausência de reconciliação e o estado local que conserva o press.

   A continuidade nova torna esse caminho relevante: agora a indisponibilidade retorna Ok e a sessão pode prosseguir indefinidamente. Pedir regressões com estado de pressionados no servidor para **press enviado → pausa → release descartado → retomada**, e para o inverso, distinguindo o mesmo dispositivo de remoção/recriação. Nenhum dos 18 testes executa essas sequências completas com um modificador retido no destino: o teste de pausa envia um par completo antes de pausar e o harness conta pedidos/start, não o estado de teclas pressionadas. Na implementação, reconciliar conservadoramente os pressionados anteriores antes de aceitar entrada no teclado retomado, sem reproduzir os eventos descartados nem guardar seu conteúdo. Não basta comprovar que begin precede a próxima tecla.

2. **Minor · R7-F1-02 · descarte persistente não é exposto no estado consumido pela interface.** `crates/clipsync-input/src/adapter/portal.rs:669`, `:707` e `crates/clipsync-input/src/module.rs:1351`. drop_key só conta/registra e emit retorna Ok. Não há propagação da disponibilidade/descarte para ModuleStatus, last_block, last_event ou evento de aviso da interface. O destino segue respondendo Stats, então F2 não devolve a captura apenas por teclado indisponível, mesmo se o usuário digitar por tempo ilimitado. Isso cumpre a decisão de preservar o mouse, mas pode manter o usuário sem saber que o teclado está sendo descartado. Propagar um estado persistente de teclado indisponível/retomado, sem texto ou keycode, preservando a política escolhida. Não há necessidade de um aviso por tecla. Achado de fluxo de dados; não abri UI nesta revisão.

### Fechamento de R6 e comportamento aprovado

- **R6-F1-01 fechado:** wait_emulate_ready (`portal.rs:1031`) retorna ao observar o ponteiro pronto, sem KEYBOARD_WAIT. Se o teclado ainda não apareceu, só registra missing/start/keyboard/not_ready. O teste EIS exige retorno em menos de 250 ms no cenário local; o teste integrado de S13 acrescenta 1,6 s antes do attach e observa primeiro posicionamento antes de 1,95 s, sem expiração de F2. Isso cobre precisamente a regressão dos 500 ms apontada em R6, não todo o tempo de consentimento do portal nem ausência do ponteiro.
- A ausência de teclado já não transforma um lote de mouse em ApplyFailed: o teste S13 confirma movimentos, zero teclas enviadas enquanto ausente, continuidade da sessão e apenas as duas teclas novas depois de o teclado aparecer. Não há replay do conteúdo descartado. Falta de outras capacidades e erro de frame/flush continuam retornando erro; S13 confirma especificamente perda do ponteiro com ApplyFailed e devolução local. Ausência de botão/scroll e falha de flush não foram injetadas nos testes novos; o fluxo foi conferido no código.
- **R6-F1-02 documentado, sem promessa de restauração garantida:** a sonda agora avisa antes de abrir o portal que interrupção ou morte da sessão pode deixar NumLock trocado e pede conferência/ajuste manual. A limitação permanece física; o aviso corrige o alcance anunciado. Não executei nem recompilei a sonda nesta R7.

### Modificadores: os dois sentidos são diferentes

Press descartado antes de existir teclado não é guardado nem enviado depois. O release que chegar após a disponibilidade pode ser enviado sem press anterior. Isso não cria um press por si só; porém a validade de uma soltura sem par depende do servidor. No outro sentido, press que já havia sido enviado e release descartado deixa estado prévio pendente, como descrito em R7-F1-01. Remoção/recriação e pausa/retomada do mesmo dispositivo não são evidências intercambiáveis.

Consultei a fonte primária [Mutter 50.1, meta-eis-client.c](https://raw.githubusercontent.com/GNOME/mutter/50.1/src/backends/meta-eis-client.c): handle_key ignora soltura sem press marcado e press repetido, mantendo estado por dispositivo. O cliente ClipSync não deve deduzir disso que toda pausa neutraliza o estado remoto; essa transição física não foi comprovada aqui. A fonte ajuda a distinguir os casos, sem provar qual deles causou o incidente do usuário. O acesso à fonte no GitLab falhou; a leitura foi pelo espelho GNOME no GitHub, tag 50.1.

### Logs, interface e privacidade

**No arquivo de log: sim.** Há missing quando start conclui sem teclado, paused/resumed/removed no ciclo, discarding na primeira tecla descartada de cada episódio e totais em descartados nos fechamentos. O acumulador global não perde descarte ocorrido antes de existir dispositivo; o contador por dispositivo guarda o que ocorreu enquanto ele existia. A capacidade guardada no anúncio evita que removed perca a identificação quando reis retira as interfaces. O reset de keyboard_dropping acontece ao observar teclado disponível, não por envio de tecla descartada.

**Na interface: não como estado específico.** O caminho normal do módulo vê Ok e pode continuar mostrando sessão ativa. Mesmo a contagem received das Stats é de eventos recebidos pelo módulo, não de teclas aplicadas pelo EIS. Portanto conexão/Stats saudáveis não provam teclado utilizável. Na cauda/pacote de suporte, o filtro `known_log_text` de `clipsync-ui/src/diag.rs:727` ainda não inclui eis.device/eis.loop e os textos novos; campos como acao/capacidades também não estão na lista comum. Parte da evidência fica omitida, pendência já indicada pelo contrato §17.8.3. Não alterei o arquivo da interface.

Não identifiquei conteúdo de tecla, keycode ou sequência nos novos logs. descartados e contagem são u64 com soma saturada; capacidades é montada de nomes fixos no anúncio; categorias de erro continuam literais. Uma linha de início por episódio não é um log por tecla, e os totais não armazenam quais teclas foram descartadas. São metadados de atividade permitidos pelo contrato, não anonimato da atividade.

### Execução própria e alcance

Comando: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test eis_devices --test s13_teclado_ausente --test s12_failsafe`. Exit final 0:

- **eis_devices: 7 passed, 0 failed**, 5,00 s;
- **s12_failsafe: 9 passed, 0 failed**, 5,41 s;
- **s13_teclado_ausente: 2 passed, 0 failed**, 2,28 s.

S13 usa dois InputModule reais sobre MemHub, PortalEmulate/loop reis no destino e servidor EI de teste, sem portal. O primeiro caso cobre atraso inicial perto do prazo, continuidade do mouse, descarte de dois eventos, ausência de replay, teclado tardio, contagem final 2 e um aviso de episódio. O segundo remove o ponteiro e prova ApplyFailed/fechamento/refused/release da origem. Não cobre teclado pausado indefinidamente, interface, modificadores atravessando a indisponibilidade nem estado físico do GNOME.

Os sete EIS continuam cobrindo start antes das teclas tardias/recriadas; pausa/retomada/start não duplicado com descarte contado por episódio; ponteiro sem espera de teclado e lote misto; ponteiro obrigatório; contagens após remoção; categoria de erro do conversor; eof versus desconexão anunciada. Os nove S12 mantêm as provas e ressalvas de R4. O relato de 363 aprovados, dois ignorados e clippy limpo é do autor; não repeti a suíte completa ou clippy nesta R7.

`git diff --check` dos quatro caminhos de F1 terminou com exit 0. Hashes estáveis entre leituras: portal.rs `a401464ffdb8f4fca40bf238bbfde544376c4b833c6f57795fc2b83d79fc0d67`; eis_devices.rs `ee7845a79291b0da536422c7d0c46b2b74012ce8e1b7108944b53e5af0b5dc11`; s13_teclado_ausente.rs `2e18513b54ac2be9d8ed8eb2d0bd8739a11d60a478f6c099d644a37829bdd75e`; eis_probe.rs `c53657ea009951b746b4f842308441370f9393e2d0f1ab06e7529550c9603c77`. S12 permanece em `76a0cec59db2128a3802b5d449504b6ae5547cef354f8547ed162af7798df63f`.

Sem código editado, sonda, portal/app físico, perfil real, permissão ou commit. Parecer entregue neste arquivo para leitura do Maestro. O incidente original do teclado morto continua sem causa física comprovada.

## R8 · F1 rodada 3 · 11/11/2026

**Veredito: não passa por R8-F1-01 Important: release_all perde as solturas pendentes se o teclado ainda está indisponível. O caminho normal de R7-F1-01 foi corrigido; o aviso de R7-F1-02 chega ao status, com ressalva de prioridade R8-F1-02 Minor. Execução própria: 22 testes passaram, zero falhas.**

Recorte do working tree: `crates/clipsync-input/src/adapter/portal.rs`, `src/adapter/mod.rs`, `src/module.rs`, constante nova de `src/msg.rs`, `tests/eis_devices.rs` e `tests/s13_teclado_ausente.rs`. Leitura de integração com F2 e execução dos três alvos pedidos. Nenhum código/teste editado, sonda executada ou commit feito. O total de 367 aprovados, dois ignorados e clippy limpo é relato do autor; não repeti a suíte inteira.

### Achados por severidade

1. **Important · R8-F1-01 · fechamento durante a pausa aposenta solturas que não foram enviadas.** `crates/clipsync-input/src/adapter/portal.rs:820`, `:821` e `:828`; chamadores normais em `src/module.rs:1569` (Leave) e `:519` (pausa). `release_all` tira todos os pressionados de `self.pressed` e drena `pending_releases` antes de conferir se existe teclado pronto. Quando o teclado continua pausado, `index_of` retorna None e o laço apenas continua; `frame()` pode terminar com Ok sem emitir nenhuma soltura de teclado. As pendências deixam de existir. Na retomada, `reconcile_keys` encontra o conjunto vazio e não neutraliza o press anterior.

   Sequência permitida: press enviado; teclado pausa; release chega e vira pending_release, corretamente; antes de retomar, chega Leave/ReleaseAll/pausa do módulo; o cleanup drena a pendência sem enviá-la; o teclado retoma no mesmo laço EIS, que é reutilizado entre sessões; nenhuma soltura é reconciliada. O mesmo cleanup também perde presses de teclado ainda registrados se faltou release. No modelo conservador agora usado pelo teste, o servidor continua segurando o modificador. É evidência estática do caminho; não executei reprodução física e não afirmo que todo servidor preserve press durante a pausa.

   Preservar ou transferir para pendências as solturas de teclado que não puderam ser emitidas, inclusive em release_all, e retirá-las somente depois do flush correspondente. Um erro de frame/flush após esse drain também perde a possibilidade de repetir a soltura, ao contrário do cuidado adotado em reconcile_keys. Pedir regressão **press → pausa → release descartado → release_all/Leave ainda pausado → retomada**, conferindo ausência de modificador retido antes de nova entrada, sem replay de press. Os testes novos chamam release_all depois da reconciliação bem-sucedida, não enquanto ela ainda está pendente. R7-F1-01 fica fechado no caso normal, mas a garantia de soltura ainda não está completa.

2. **Minor · R8-F1-02 · aviso de teclado sempre substitui last_block do módulo, mesmo para erro mais grave.** `crates/clipsync-input/src/module.rs:1359` e `:1363`. A expressão escolhe `emu.block_reason()` antes de `self.last_block`, sem categoria/prioridade. Quando há descarte ativo, uma falha de permissão registrada pelo módulo (por exemplo `module.rs:1034`), erro de captura ou outra recusa fica invisível no campo last_block até o teclado retomar ou o emulador parar. O erro anterior não é apagado, mas o consumidor não consegue vê-lo naquele período. Não há bypass de pausa ou autorização: o defeito é de diagnóstico/prioridade. Pausa global pode ser mostrada por outro estado da aplicação; este parecer não afirma que toda indicação de pausa desapareça da tela. Preservar os bloqueios ativos mais graves ou comunicar separadamente o aviso de teclado; evitar inferir gravidade pelo texto livre. Os testes de status exercitam o aviso sem outro bloqueio concorrente, não sua prioridade.

### Reconciliação e novo press da mesma tecla

A ordem pedida está correta no caminho normal. O loop EIS é proprietário único de Emulator. Ao processar retomada/recriação, chama reconcile_keys antes de anunciar disponibilidade (`portal.rs:985`); cada emit também começa chamando reconcile_keys (`:731`). Essa rotina faz begin se preciso, emite somente releases pendentes, fecha e faz flush do frame, e só então limpa o conjunto (`:706–717`). Não há await que permita intercalar outro Cmd::Emit no meio dessa sequência.

Assim, **um novo press aceito da mesma tecla não é seguido pela soltura pendente antiga**: a pendência já terá sido emitida e retirada antes de o novo press entrar em `pressed`. Se a reconciliação falhar, o `?` impede aceitar o novo lote; F2 continua vendo erro. Um press que chegue enquanto a retomada ainda não foi processada continua sendo descartado pela política escolhida, não considerado aceito nem reproduzido depois. O release posterior de um press que foi descartado não cria outro press.

Os três testes novos comprovam os lados da transição usando um servidor que mantém um conjunto de teclas pressionadas e uma sequência de transições. O teste com press anterior usa outra tecla nova depois da retomada, portanto **não há regressão executada com novo press do mesmo código**; a conclusão sobre esse caso vem da serialização e da chamada antes do laço de eventos. Isso não fecha o caminho de cleanup antecipado de R8-F1-01.

### Aviso, recuperação e privacidade

R7-F1-02 foi atendido no recorte de dados de status: block_reason tem implementação padrão None nos outros adaptadores; PortalEmulate usa o booleano de episódio, sem expor conteúdo. O snapshot é publicado antes de responder ao emit (`portal.rs:998`), e a retomada limpa o booleano depois de reconciliar; consultar status não requer outra tecla. A frase fixa fica em `ModuleStatus.last_block` no destino e desaparece ao retomar, como os testes confirmaram. `stop` também remove o snapshot.

Isso prova o campo consumível pelo restante da aplicação, não renderização em Tauri nem aviso no PC de origem. Não há novo envio remoto desse estado. A cauda/pacote de suporte continua dependendo da lista permitida dos eventos/campos novos, pendência já registrada em R7/§17.8.3. O aviso não deve ser confundido com detecção de teclado inacessível antes de qualquer descarte: ele é ativado pelo episódio de descarte, conforme o desenho.

pending_releases contém apenas códigos internos de presses já enviados, para neutralizar estado remoto, e não guarda presses descartados nem conteúdo para replay. A nova linha reconciled só leva dispositivo local, capacidade fixa e quantidade. Não identifiquei serialização/Debug do conjunto ou de códigos. Contadores de pedidos/descarte continuam sendo metadados numéricos permitidos; não provam aplicação física.

### Execução própria e alcance dos testes novos

Comando: `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test eis_devices --test s13_teclado_ausente --test s12_failsafe`. Exit final 0:

- **eis_devices: 10 passed, 0 failed**, 5,00 s;
- **s12_failsafe: 9 passed, 0 failed**, 5,41 s;
- **s13_teclado_ausente: 3 passed, 0 failed**, 2,45 s.

| Teste novo/alterado | Prova observada e limite |
| --- | --- |
| `release_de_modificador_descartado_e_reconciliado_antes_da_proxima_tecla` (`eis_devices.rs:761`) | O servidor conservador mantém o press durante pausa; release é descartado; na retomada recebe a soltura antes de outra tecla e o modificador deixa de estar pressionado. ReleaseAll posterior não duplica a soltura. Não cobre cleanup enquanto pausado nem novo press do mesmo código. |
| `release_pendente_tambem_e_reconciliado_no_teclado_recriado` (`:766`) | Substitui o teclado, exige start e soltura pendente no dispositivo atual antes das novas teclas e observa reconciled/contagem=1. A remoção do dispositivo no harness elimina estado do antigo; não prova neutralização física do compositor. |
| `press_descartado_e_release_aplicado_nao_criam_tecla_presa` (`:771`) | Press sem teclado não chega ao servidor nem é reproduzido; release após criar teclado não deixa tecla pressionada e release_all não inventa outro evento. Não cobre press novo real da mesma tecla após uma pendência anterior. |
| `teclado_ausente_nao_fecha_sessao_mouse_continua_e_teclado_tardio_funciona` (S13, alterado) | Além das provas de R7, consulta status antes do descarte, durante indisponibilidade e depois do teclado tardio, conferindo aviso fixo e recuperação sem derrubar o mouse. Não há bloqueio de módulo concorrente. |
| `aviso_do_teclado_pausado_persiste_e_some_ao_retomar_sem_nova_tecla` (`s13_teclado_ausente.rs:517`) | Com dois módulos reais/MemHub e adaptador EIS real no destino, aviso permanece em consultas sucessivas após descarte e desaparece na retomada sem nova tecla. Depois novas teclas funcionam e o total descartado é 2. Não é execução visual nem prioridade entre avisos. |

As sete regressões EIS anteriores, perda de ponteiro com ApplyFailed em S13 e as nove S12 também passaram no mesmo comando. Isso preserva a evidência de R6-F1-01 fechado e dos prazos/identidades de F2, sem substituir teste físico nem reproduzir todas as falhas de flush.

### Estabilidade e limites

`git diff --check` dos seis caminhos revisados terminou com exit 0. SHA-256 do recorte: portal.rs `78e939e2dbb4283a85164214b21fcc904a9ecf19581487c9a9b50cec9ed736e9`; adapter/mod.rs `667a1481be531d0d794ee5fe347d6817a321d3e987229092656e68d53b706fcc`; module.rs `639703e9138f9bd6e0a46d1d6c7ccb7f9b6325b108c70d1ee4d0458f3176add6`; msg.rs `b4c61516fcdc82e5585ccea81039848a45ee599c6a5952bfb1ea5df23fae679d`; eis_devices.rs `e71fae52b983c3959bbdf57d3ae053dd4b7bc1dc23e471fc9f67d66aee1534cb`; s13_teclado_ausente.rs `072c8fb6c54e9af6f502082db5dbfceed28f98bb55d5a54e37d16d754add367d`. S12 permaneceu em `76a0cec59db2128a3802b5d449504b6ae5547cef354f8547ed162af7798df63f`.

Sem código editado, sonda, portal/app físico, perfil real, permissão, commit ou push. Entrega por este arquivo para leitura do Maestro. Falta preservar pendências no cleanup indisponível; a causa física do incidente original continua não demonstrada.

## R9 · F1 rodada 4 · 11/11/2026

**Veredito: passa com ressalva documental Minor. R8-F1-01 e R8-F1-02 fechados; nenhum impedimento técnico novo identificado para commit de F1 junto de F2 e N1 no recorte revisado. Execução própria: 25 testes passaram, zero falhas; sonda compilada e não executada.**

Re-revisão do working tree sem commit: `crates/clipsync-input/src/adapter/portal.rs`, `src/module.rs`, `tests/eis_devices.rs` e `tests/s13_teclado_ausente.rs`, com conferência dos componentes anteriores de F1 e da integração com F2/N1. Nenhum código/teste foi editado pelo revisor, sem commit/push. A aptidão abaixo é técnica, não autorização para publicar nem aceite físico do incidente.

### Fechamento dos dois achados de R8

1. **R8-F1-01 fechado.** `portal.rs:819` retira os pressionados locais, mas agora transfere os de teclado para `pending_releases`, sem drenar pendências anteriores. Se falta teclado pronto, reconcile_keys retorna sem aposentar o conjunto. ReleaseAll repetido não elimina nem duplica a pendência, porque ela permanece no HashSet. Na retomada/recriação, as solturas são emitidas em frame e só são retiradas depois de flush bem-sucedido (`:697–717`). A falha de frame/flush não limpa esse conjunto. O evento resumed também reconcilia sem depender de outra tecla (`:982`). A política antiga de botões ficou explicitamente preservada; esta correção não é uma nova garantia para botões indisponíveis.

   As duas novas regressões executadas cobrem **press → pausa → release_all repetido → retomada**, tanto sem release anterior como com release descartado antes do cleanup. O servidor conservador mantém o modificador durante a pausa e exige uma única soltura ao retomar, sem nova tecla para dispará-la. Isso fecha o caminho de perda descrito em R8, no mesmo laço EIS que atravessa o fechamento de uma sessão de controle.

2. **R8-F1-02 fechado.** `module.rs:1359` monta primeiro o motivo do módulo, incluindo os fallbacks já existentes; `:1369` acrescenta o aviso do adaptador com ` · `. Os motivos não se substituem. Quando o teclado volta, só o aviso deixa de ser derivado, conservando last_block do módulo. O teste de S13 comprova a composição com uma recusa NotAuthorized e a recuperação sem apagar esse motivo. É prova do campo de status, sem execução visual de Tauri. O teste injeta a recusa, não realiza uma revogação física de permissão do sistema.

### Ordem das solturas e novo press da mesma tecla

Não identifiquei, no recorte de teclado revisado, caminho que aceite um novo press e depois envie a soltura antiga pendente. O proprietário único do laço EIS reconcilia ao processar disponibilidade e no início de cada emit. O frame/flush das pendências antecede o laço que processa os novos eventos; se falha, o `?` impede aceitar o lote. Não há await nessa sequência que permita intercalar outro comando.

O novo teste agora exercita também o **mesmo código**: após a retomada e a soltura única, envia outro press do modificador, observa-o pressionado no servidor e só o release_all seguinte o solta. As duas variantes de cleanup usam esse passo. Os testes anteriores mantêm a cobertura de retomada, recriação e press descartado seguido de release aplicado, sem inventar press ou replay.

Isso não equivale a garantir soltura física quando o teclado nunca volta, o compositor não aplica os pedidos ou o processo/conexão morre. A pendência é conservada para a próxima oportunidade válida no laço; flush confirma envio pelo cliente, não aplicação no GNOME. Falhas arbitrárias de I/O não foram injetadas nesta R9, embora a ordem de limpeza do conjunto tenha sido conferida no código.

### F1 inteira e composição para commit

**Sim: F1 inteira está tecnicamente em condição de commit junto com F2 e N1, no alcance destas revisões, consolidando também o pequeno ajuste documental abaixo.**

- **Prontidão:** volta ao tempo do ponteiro, sem os 500 ms adicionais; a regressão integrada com atraso inicial de 1,6 s continua passando, preservando R6-F1-01 fechado.
- **Descarte contado:** falta de teclado não derruba o mouse, conta eventos e não reproduz conteúdo; ausência das outras capacidades e erros de frame/flush continuam erro. O caminho de ApplyFailed por perda do ponteiro permanece testado.
- **Reconciliação:** cobre teclado retomado/recriado, cleanup enquanto pausado, cleanup repetido e novo press do mesmo modificador. Pendências só são aposentadas depois de flush.
- **Aviso:** chega a ModuleStatus, persiste enquanto há episódio de descarte e some ao retomar, sem outra tecla, preservando o motivo anterior do módulo.
- **Instrumentação:** eis.device/eis.loop usam categorias, capacidades fixas e totais; reconciled informa quantidade e origem, sem serializar códigos nem o conjunto de pendências. A contagem segue sendo de pedidos/descartes, não de aplicação física.
- **Sonda:** mantém o aviso de possível NumLock trocado e restauração por tentativa/conferência. A versão atual compilou nesta revisão, sem execução.
- **F2:** capture.rs e S12 mantêm os hashes de R4; os nove testes focais passaram outra vez sobre o módulo atual, incluindo prazo em opening e timestamp de Stats. Permanecem os limites de carga/agendamento já registrados, sem reabertura dos achados fechados.
- **N1:** os seis arquivos core conferidos mantêm exatamente os hashes aprovados em R5. Não repeti os testes core nesta R9; a referência própria continua sendo os 41 testes aprovados em R5. Não há nova interferência de F1 nesses arquivos.

Essa aptidão abrange essas três fatias. Não estende automaticamente o parecer aos arquivos de F3, interface ou demais mudanças concorrentes do working tree.

**Minor · R9-DOC-01:** `docs/contrato.md` §17.8 já lista a ação reconciled, mas a enumeração de `origem` ainda omite `release_all`, emitida em `portal.rs:837` através de reconcile_keys. Incluir esse literal na consolidação documental do commit, descrevendo reconciliação no cleanup além da retomada. Não editei o arquivo de C1. A ressalva não é defeito do algoritmo de soltura.

### Execução própria

`PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-input --test eis_devices --test s13_teclado_ausente --test s12_failsafe` terminou com exit 0:

- **eis_devices: 12 passed, 0 failed**, 5,01 s;
- **s12_failsafe: 9 passed, 0 failed**, 5,41 s;
- **s13_teclado_ausente: 4 passed, 0 failed**, 2,44 s.

Os testes novos `release_all_conserva_press_como_soltura_pendente_enquanto_teclado_pausado` e `release_all_conserva_soltura_ja_pendente_enquanto_teclado_pausado` (`eis_devices.rs:851` e `:856`, helper em `:802`) verificam a preservação no cleanup e o novo press do mesmo modificador. `status_preserva_bloqueio_do_modulo_antes_do_aviso_do_teclado` (`s13_teclado_ausente.rs:616`) verifica motivo isolado, motivo combinado e retirada apenas do aviso. Os outros 22 testes dos três alvos também passaram, preservando as provas anteriores.

`PATH=$HOME/.cargo/bin:$PATH cargo check -j2 -p clipsync-input --example eis_probe`: exit 0, `Finished dev profile ... in 3.29s`. Não executei cargo run ou o binário. `git diff --check` nos caminhos de F1/F2 conferidos terminou com exit 0.

O relato de 370 aprovados, dois ignorados e clippy limpo é do Rivet/Maestro; nesta R9 executei os 25 testes acima e a compilação da sonda, sem reivindicar repetição da suíte completa ou clippy.

### Limites que precisam acompanhar o commit

1. **Causa do teclado morto na primeira sessão não comprovada.** Não anunciar que o incidente físico foi reproduzido, explicado ou corrigido por esta bateria.
2. **Sem prova no GNOME/libeis real ou em travessia física Windows → notebook.** Reis/UnixStream e MemHub verificam pedidos, estado conservador do servidor de teste e lógica dos módulos; não aplicação de entrada no compositor.
3. **Continuidade escolhida é do mouse:** teclado ausente/pausado pode descartar teclas por tempo indefinido enquanto o módulo responde Stats. O aviso é local ao destino e não há replay. Stats saudáveis não comprovam aplicação do teclado. A soltura em até 2 s é da captura na origem por silêncio, não promessa de soltura física remota em todas as falhas.
4. **NumLock da sonda pode ficar trocado** se sessão/processo falhar; execução continua manual, com consentimento do usuário e conferência final. A sonda foi só compilada.
5. **Status testado, UI física não testada.** A redação da cauda/pacote de suporte para o vocabulário novo continua pendência separada de interface, já registrada no §17.8.3. Não afirmar que todos os campos novos já aparecem íntegros nesse pacote.
6. **N1 não saneia automaticamente endereços sobrescritos por versões antigas em peers.toml.** Manter o limite de procedência/migração e os prazos de recuperação já documentados em R5/S1b.

### Estabilidade do recorte

Hashes estáveis entre leituras: portal.rs `9fe42c110407c76b0cb13c85e6dbe34922983e8a61514a52647aec1a7ebc19a2`; module.rs `d57bce9cca8a55cc90477b9942c8042fe38f56b86d1dbd21fbd088a9faccf97d`; eis_devices.rs `bafc3e257bd9b6a27a437a5ef9e36669dba7de55ddb03446b1b36635c90fd7e6`; s13_teclado_ausente.rs `c4ab543e098803822f9af8fc6b14a8eab2e3c5746d59bdb44ec008444cc28f0c`; eis_probe.rs `c53657ea009951b746b4f842308441370f9393e2d0f1ab06e7529550c9603c77`. Adapter/mod.rs, msg.rs, capture.rs e S12 mantêm os hashes registrados nas revisões anteriores.

Sem código editado, sonda executada, servidor/app físico, perfil real, permissão, commit ou push. Parecer entregue por este arquivo para leitura do Maestro.

## R10 · conjunto v0.5.3 · 12/11/2026

**Veredito: passa com ressalvas para commit de F1 + F2 + N1; nenhum novo Critical/Important de composição identificado. Pode compor a v0.5.3 com os limites abaixo escritos, depois de consolidar a documentação e cumprir os gates do commit final. Não aprova publicar o working tree inteiro como está. Bateria focal própria: 66 passaram, zero falhas.**

Revisor · achados, identidade devolvida por `maestri list`. Base `main`, HEAD `c30c529`, ainda `0.5.2` em `Cargo.toml:16`. Revisão de `git diff HEAD`, inventário de `git status --short`, fontes e testes novos de F1/F2, sonda, documentação do incidente e pareceres R1 a R9. As propostas novas de lançamento/UI/acessibilidade/segurança foram identificadas como trabalho paralelo; esta rodada não aprova seu conteúdo integral nem sua implementação. Nenhum código, teste ou configuração editado; somente esta seção acrescentada. Sem commit.

### Achados e ajustes para a consolidação

1. **Minor · R10-LOG-01 · a cauda redigida perde a identificação e detalhes da instrumentação nova.** `crates/clipsync-ui/src/diag.rs:729`, `:790`, `:846`, `:859` e `:882`. O JSONL bruto recebe os eventos, mas o redator transforma textos fora da lista em `<texto omitido>` e elimina campos fora de `LOG_FIELDS` (`:926`). Não elimina necessariamente a linha inteira: sobram timestamp, alvo e alguns contadores, sem informação suficiente para reconstruir o ciclo EIS. É a pendência de integração já reconhecida em R7/R9, agora discriminada abaixo. **`apply_failed` não falta:** já é aceito como motivo em `diag.rs:836`. Não há necessidade de liberar texto arbitrário, tecla, keycode, nome de dispositivo ou keymap para resolver a lacuna.

2. **Minor · R10-DOC-01 · os documentos vigentes ainda descrevem decisões anteriores às implementações aprovadas.**
   - `docs/contrato.md:2054` agrupa F1 confirmada sob “Propostas desta rodada, sem dono confirmado”; `:2059` ainda chama `peer_unresponsive` de nome proposto. A F2 já o emite em `capture.rs:864`. Consolidar em adendo de versão o mecanismo efetivo: StatsRequest/Stats existentes no protocolo, sondagem nominal de 500 ms sem sobrepor envio pendente, prazo de 2 s desde a abertura ou última Stats válida tratada pelo módulo, com identidade de par/conexão/sessão, e soltura local antes da limpeza de rede. Registrar também o aviso de teclado no status e a preservação de solturas pendentes. `release_all` já consta em `:2055`: R9-DOC-01 está atendido.
   - `docs/contrato.md:2060` omite `eis.loop` e fala em colocar “o nome da contagem” na lista de eventos. `contagem` é campo numérico, não evento novo. `hooks.reinstalled` continua proposta de F3, sem emissor nesta entrega; não deve ser anunciado como implementado. A referência `transport.rs:910` em `:2053` também ficou antiga: o emissor atual é `transport.rs:1033`.
   - `docs/verificar-por-log.md:100` não lista `peer_unresponsive`; `:102` ainda diz que o ciclo EIS depende de confirmação e menciona contagem **por sessão**, contradizendo `portal.rs:1015` e contrato `:2056–2057`: os totais são por dispositivo/vida do laço. Descrever `session.closed{motivo=peer_unresponsive}` separadamente de `session.refused`/`enter.refused{motivo=apply_failed}`. No teste integrado, o fechamento correspondente à recusa tem `motivo=refused`, não `apply_failed` (`s13_teclado_ausente.rs:510`).
   - `CHANGELOG.md:5` (“Não publicado”) só anuncia N1 e firewall. Faltam F2 e F1 com seu alcance real: vigia por silêncio, fechamento legado por falha de aplicação, instrumentação EIS, continuidade do mouse sem teclado, descarte sem replay, reconciliação e aviso local. Não anunciar correção comprovada do teclado físico nem da F3.
   - O arquivo novo `docs/roteiro-campo-2026-11-11.md:113` ainda usa **`peer_silent`** e diz que o prazo não é exigência vigente. Atualizar para a decisão implementada `peer_unresponsive`, distinguindo a versão instalada e a medição física ainda pendente. `docs/pendencias-v0.5.0.md:13–15` conserva descrições de ausência da instrumentação/vigia como se fossem código atual, e `:37` ainda aguarda revisões de N1 já concluídas. Acrescentar o estado de 12/11, preservando os relatos históricos.

3. **Informativo de escopo · R10-ESC-01 · F3 está no diff, mas não faz parte do conjunto aprovado.** `crates/clipsync-input/src/adapter/wincore.rs:484` acrescenta `HookWatch`, `HookHealth` e testes. A busca por esses tipos em `crates/` só encontrou sua definição/implementação/testes nesse arquivo: não há consumidor de produção em `adapter/win/capture.rs` nem integração nova em `clipsync-win/src/ffi.rs`. É lógica ainda sem uso no produto, coerente com a auditoria/proposta de F3, não correção de gancho removido. **Manter esse delta fora do commit das três fatias**; não incluir a árvore toda por conveniência. As seis specs novas de lançamento/F3 e o resumo automático de `CONTEXTO.md` também não recebem aprovação de produto por este R10.

### Lista exata da lacuna no pacote de suporte

Conferência estática do produtor contra o redator atual; não executei a API de prévia/exportação nesta rodada.

| Local em `crates/clipsync-ui/src/diag.rs` | O que falta / o que já existe |
| --- | --- |
| `known_log_text`, ramo `evento`, linha 729 | Acrescentar os literais **`peer.readdressed`, `eis.device`, `eis.loop`**. `session.closed`, `session.refused` e `enter.refused` já existem. |
| Ramo `origem`, linha 846 | Acrescentar **`name`, `added`, `resumed`, `paused`, `removed`, `loop`, `start`, `emit`, `release_all`**. Hoje só aceita `barrier`, `switch`, `test`. |
| Ramo `motivo`, linha 790 | Acrescentar **`peer_unresponsive`, `not_ready`, `context`, `handshake`, `converter`, `eof`, `disconnected`, `closed`, `runtime`, `io`**. `paused` já existe. **`apply_failed` já existe na linha 836**, e `refused` na 808. |
| `LOG_FIELDS`, linha 882 | Acrescentar **`acao`, `capacidades`, `dispositivo`, `dispositivos`**. `contagem` (911) e `descartados` (914) já estão permitidos. Os dois campos de dispositivo são números locais, não nomes/IDs de hardware. Validar os novos números como inteiros não negativos. |
| Novo ramo literal `acao` em `known_log_text` | Aceitar somente **`added`, `resumed`, `paused`, `removed`, `missing`, `discarding`, `reconciled`**. Incluir a chave em `LOG_FIELDS` sem validar o texto ainda deixaria `<texto omitido>`. |
| Novo validador de `capacidades` | O produtor `portal.rs:577` monta combinações, em ordem fixa e separadas por vírgula, de **`pointer`, `pointer_absolute`, `button`, `scroll`, `keyboard`, `touch`, `text`**. Aceitar só combinações desse conjunto finito, inclusive vazio se não houver capacidade conhecida, sem nome externo, espaços livres ou sufixos. Permitir só `keyboard` deixaria os dispositivos de ponteiro omitidos. |
| `redact_log_text`, linha 859 | **`erro` é omitido incondicionalmente**, antes de `known_log_text`. Adicionar um ramo `erro` apenas à lista não basta. Para conservar a categoria de `eis.loop`, precisa de validação contextual desse evento e dos únicos literais **`event`, `parse`, `request`, `handshake`, `interface_version`, `io`**, preservando a omissão dos outros erros/textos livres. Isso deve acompanhar o contrato do redator; não abrir `erro` globalmente. |

Os alvos `clipsync_core::transport`, `clipsync_input::capture`, `clipsync_input::module` e `clipsync_input::adapter::portal` já são permitidos (`diag.rs:782–786`). `peer` deve continuar redigido. A mensagem descritiva livre não precisa ser liberada: os campos estruturados acima bastam. Não acrescentar `hooks.reinstalled` como se houvesse emissão implementada. A regressão da futura correção deve conferir valores aceitos e rejeitar texto hostil, capacidades inventadas e campos de tecla/keycode; esta revisão não alterou o filtro.

### Composição conferida

1. **F1 com F2:** `wait_emulate_ready` (`portal.rs:1087`) retorna no ponteiro; não há `KEYBOARD_WAIT` nem espera adicional por teclado. O timeout remanescente de `DEVICE_WAIT` é de prontidão do ponteiro. `emit` reconcilia solturas antes do lote (`:731`); falta de teclado descarta/conta e retorna sucesso, preservando a sessão e o atendimento de Stats. Falta de ponteiro/botão/rolagem continua erro. O ramo legado em `module.rs:2016` remove Incoming, tenta soltar e recusa ApplyFailed; no ramo v4 a classificação genérica preexistente continua Locked (`:2009`). Não afirmar cobertura nova de ApplyFailed v4. S13 passou com atraso inicial de 1,6 s, teclado ausente/tardio, perda do ponteiro e composição do aviso com bloqueio do módulo.
2. **Cleanup de F2 com estado EIS de F1:** `release_all` (`portal.rs:818`) transfere presses de teclado para `pending_releases`, conserva o conjunto enquanto indisponível e só o aposenta depois de frame/flush. Portanto Leave/ReleaseAll/pausa não reintroduzem a perda de soltura corrigida em R9. Os dois testes de cleanup pausado passaram, inclusive limpeza repetida e novo press do mesmo código após a retomada. Botões conservam a política anterior; não há promessa nova de soltura física de dispositivo indisponível.
3. **N1 com a vigia:** N1 muda a seleção/cache do endereço; não transforma IP em identidade nem altera a versão do protocolo. A geração da conexão continua no contexto do transporte (`transport.rs:1497`), `send_reliable_on` recusa geração divergente (`:1507`), o módulo descarta Stats de conexão antiga (`module.rs:1444`) e a captura também confere par/conexão/sessão (`capture.rs:889`). `reconcile_host` encerra a sessão quando perde/substitui a conexão (`:978`). Uma reconexão em outro endereço não herda o relógio nem reativa a sessão antiga. Não apareceu caminho novo em que resolução/cache bloqueie a devolução local; reencontro pode demorar bem mais que 2 s, enquanto F2 trata a captura separadamente. Esta conclusão é de leitura e testes focais separados, não teste de DHCP físico com captura simultânea.
4. **N1 e documentação de rede:** `docs/instalar.md:75–80` corresponde ao código: cache separado, pin exigido, consulta a `NOME` **e** `NOME.local`, possibilidade de DNS configurado e rejeição de nomes com ponto/espaço/acento. A frase “só NOME.local” do quadro do incidente não descreve `system_resolver` (`transport.rs:93`); o guia está correto. Os 3 s de `NAME_DIAL_TIMEOUT` limitam o handshake de cada tentativa de cache/nome; Hello ainda tem orçamento próprio de 5 s (`:1083`), e pareamento usa 10 s. “Espera curta” não significa prazo total de reencontro de 3 s. O guia restringe a regra UFW ao CIDR; nenhuma regra foi executada por esta revisão. Falta explicitar ao usuário que endereços já sobrescritos por versões antigas em `peers.toml` não são saneados automaticamente.
5. **Resíduos e privacidade:** a busca em `crates/` não encontrou `KEYBOARD_WAIT` nem `restore_learned`; os testes mencionam os 500 ms apenas para proibir a regressão. `start_with_cache` recebe o cache antes dos discadores. Os novos emissores EIS usam ações/categorias/capacidades fixas e contagens, sem registrar tecla, keycode, conteúdo ou o conjunto `pending_releases`. O estado interno de solturas não é replay de presses descartados. O comentário de tela bloqueada no ramo v4 genérico de `module.rs:2010` permanece impreciso para erros não Locked, já documentados em R6; não é regressão nova de composição.

### Execução própria

Com `PATH=$HOME/.cargo/bin:$PATH`, `-j2` e `--locked`, sem interromper processos ou disputar a verificação ampla:

| Comando | Saída observada |
| --- | --- |
| `cargo test -j2 -p clipsync-core --test readdress --test mesh --test profile --locked` | **mesh 18 passed / 0 failed (22,10 s); profile 16 / 0 (0,14 s); readdress 7 / 0 (32,27 s)**. Exit 0. |
| `cargo test -j2 -p clipsync-input --test eis_devices --test s12_failsafe --test s13_teclado_ausente --locked` | **eis_devices 12 passed / 0 failed (5,03 s); s12_failsafe 9 / 0 (5,43 s); s13_teclado_ausente 4 / 0 (2,47 s)**. Exit 0. |
| `git diff --check` | Saída vazia, exit 0. |

Core usa QUIC em loopback/perfis temporários; input usa fakes/MemHub e EI simulado sobre UnixStream. Os hashes conferidos de transport/agent/profile são os de R5; capture/S12 são os de R4; portal/module/EIS/S13 são os de R9. Não há mudança adicional nessas fontes desde os pareceres aprovados. A sonda foi lida, sem execução; sua compilação permanece evidência de R9, não repetida aqui. A verificação completa concorrente do workspace não é evidência desta revisão. Não repeti clippy, fmt, CI, Tauri, instalação, app, servidor ou validação visual/física.

### Limites escritos para commit e publicação

1. **Selecionar F1, F2 e N1 com seus testes e documentos; manter F3 fora desse commit.** A aprovação não cobre automaticamente specs paralelas nem o resumo automático. Consolidar R10-DOC-01 antes de anunciar a v0.5.3, sem reescrever pareceres históricos.
2. **F1 instrumenta e melhora a continuidade; causa física ainda desconhecida.** Teclado ausente/pausado pode descartar entrada indefinidamente enquanto o mouse e Stats continuam. O aviso está no status do destino, sem novo aviso remoto; não houve comprovação visual em Tauri. Não há replay. Reconciliação confirma pedidos enviados, não aplicação no GNOME. F3 não está corrigida.
3. **F2 é devolução local por silêncio de aplicação.** O prazo lógico é de 2 s; os testes usam tolerâncias e não provam um teto físico sob thread/OS bloqueado, fila extrema ou falha do adaptador. Não garante soltura remota em 2 s com destino inacessível nem que Stats saudáveis comprovem teclado aplicado. Consentimento/prontidão do portal ainda podem exceder o prazo inicial; o atraso adicional por teclado foi removido.
4. **N1 conserva confiança e cache separados; não corrige a procedência dos registros antigos de peers.toml.** Resolução pode consultar DNS, reconexão não é instantânea e o ensaio não prova DHCP/firewall das duas máquinas reais. Falha de I/O pode impedir a persistência/poda imediata do cache.
5. **R10-LOG-01 permanece Minor enquanto o filtro não for corrigido.** Se a publicação mantiver essa pendência, escrevê-la nas notas: a cauda/pacote de suporte não conserva integralmente a instrumentação nova e não substitui o JSONL bruto nessa investigação. Não anunciar diagnóstico EIS completo pelo pacote. O relatório acima especifica a correção sem relaxar privacidade.
6. **Publicação exige o commit final preparado e verificado.** A versão ainda é 0.5.2; este parecer não valida tag 0.5.3, CI Linux/Windows, artefatos ou hashes. Conferir os gates do release no commit final. Sem prova de travessia física Windows → GNOME, suspensão ou teclado da primeira sessão nesta R10; nenhum aceite físico foi fechado. A sonda manual pode deixar NumLock trocado se interrompida e continua exigindo consentimento pessoal.

**Linha de veredito para o workboard:** R10: passa com ressalvas para commit de F1+F2+N1; 66/66 focais; consolidar documentação e declarar cauda incompleta; F3 fora; publicação v0.5.3 condicionada aos limites e gates do commit final.

**Fecho e alterações concorrentes:** a conferência final encontrou alterações surgidas depois do inventário inicial em `crates/clipsync-win/src/lib.rs`, `crates/clipsync-input/tests/s14_hook_watch.rs` e `docs/releases/v0.5.3.md`. Não são cobertas pela bateria nem pela aprovação acima. O rascunho de release já começa a documentar F1/F2/N1 e limites, mas sua consolidação final continua fora deste snapshot. Os nove hashes de fontes/testes principais conferidos acima permaneceram idênticos no fecho. `git diff --check -- docs/verdicts.md` terminou com exit 0.

**Comunicação:** tentei `maestri ask "Maestro · scaffold" "R10: ..."`; retornou exit 1, `Terminal 'Maestro · scaffold' not found`. `maestri debug` confirmou conexão ativa (HTTP 200), mas `maestri list` mostra agentes/notas de outro projeto, sem o Maestro ou workboard deste incidente. Nenhuma nota desse outro projeto foi alterada. A entrega fica neste arquivo, conforme a alternativa autorizada no pedido.


## R11 · F3 · 12/11/2026

**Veredito: não passa para integrar uma versão. R11-STATE-01 (Important) reproduz perda do estado necessário à soltura local; R11-SYNC-02 (Minor) deixa a exclusão da renovação dependente de uma amostragem concorrente. Pode seguir para ramo próprio de investigação/CI do Windows, com estes achados abertos, sem merge nem publicação aprovados.**

Revisão independente da entrega `/tmp/clipsync-f3-entrega.md`, auditoria `docs/superpowers/specs/2026-11-11-windows-gancho-vigia.md`, contrato §17.8 e implementação efetiva, sobre main/c30c529, sem commit. Foram lidos os diffs e arquivos novos da F3, incluindo a intervenção originalmente entregue em F2. Somente este parecer foi acrescentado ao repositório pelo revisor; nenhum fonte foi alterado, nenhum app/servidor iniciado.

**Atenção ao snapshot:** durante a revisão, outro trabalho retirou do checkout as intervenções de log em `capture.rs`, `adapter/mod.rs`, a implementação do contador no trait de `adapter/win/capture.rs` e `tests/s14_hook_log.rs`. A entrega anterior está preservada em `/tmp/clipsync-f3-log-adiado/`, com os caminhos relativos originais. Os 37 testes focais abaixo foram executados e passaram **antes dessa retirada**. O teste de log não existe no checkout ao fechar esta revisão; não dizer que o comando com cinco alvos é reproduzível agora sem recompor essa fatia. Não restaurei arquivos alheios. As linhas do achado de estado correspondem ao checkout sem o log; as linhas da intervenção F2 correspondem à cópia preservada. Manifesto do estado de sete fontes/testes no fecho: `/tmp/clipsync-r11-ajskp2p5/snapshot.sha256`.

### Achados, por severidade

1. **R11-STATE-01 — Important — renovação periódica apaga tecla ainda fisicamente pressionada, antes da soltura local da próxima travessia.** Em `crates/clipsync-input/src/adapter/win/capture.rs:314`, toda renovação bem-sucedida parte de `reconcile(Held::default())`; esse reconciliador (`:352`) só consulta modificadores e botões. O resultado substitui o estado compartilhado (`:315`) e, no próximo evento, `core.renew_idle(physical)` (`:229`) executa `self.held = physical` em `crates/clipsync-input/src/adapter/wincore.rs:330`. Logo, uma tecla comum que o gancho saudável já viu baixar é esquecida, mesmo que ainda esteja segurada. A manutenção periódica ocorre também com ganchos saudáveis; não exige perda prévia nem ausência de teclas pressionadas.

   Sequência concreta: tecla comum baixa em Idle; vence a manutenção; renovação; movimento cruza a borda antes de outro repeat da tecla; soltura física já em Arming/Active. `Out::Activate` leva o conjunto agora vazio (`wincore.rs:418`); `park_and_carry` reconcilia e chama `release_locally` (`adapter/win/capture.rs:403`), mas a tecla apagada não participa das solturas (`:370`). O key-up físico é bloqueado por `wincore.rs:479`. O desktop local recebeu o down e não recebe o up correspondente por esses caminhos. Isso abre regressão de tecla presa/repetição local numa renovação de rotina; **o efeito no aplicativo/Windows físico não foi executado nesta máquina**.

   **Reprodução própria:** harness externo em `/tmp/clipsync-r11-ajskp2p5/review.rs`, compilado contra os rlibs reais indicados por `cargo test -j2 -p clipsync-input --test s14_hook_watch --no-run --locked --message-format=json`, sem copiar a lógica do núcleo e sem adicionar teste ao checkout. Exercita `CaptureCore::on_event`, `renew_idle`, uma barreira real e a soltura. Saída em `result.log`: `controle: solturas locais candidatas=1; apos renovacao: 0; release fisico bloqueado em ambos; perda reproduzida no nucleo, sem Win32`. Exit 0 significa que as asserções **confirmaram o defeito**, não que ele foi corrigido. Nenhuma tecla/keycode aparece na saída.

   **Necessário antes de integrar:** conservar informação suficiente do estado já conhecido para neutralizar localmente as teclas, ou adiar a troca enquanto esse estado não puder ser reconciliado com segurança. Não ampliar consulta de letras nem adicionar conteúdo ao log; não fazer replay de presses. Acrescentar regressão que mantenha tecla comum pressionada durante a renovação, cruze antes do repeat e confirme a soltura local necessária. Os testes atuais cobrem modificador antigo, atalho e núcleo já armado; não cobrem essa sequência.

2. **R11-SYNC-02 — Minor — a decisão de renovar não reserva o estado ocioso contra `begin`/parada concorrentes.** `crates/clipsync-input/src/adapter/win/capture.rs:301` lê `capture_allowed`, `capturing`, `end`, `begin` e `ready`; `crates/clipsync-win/src/ffi.rs:1114` recebe essa decisão e só depois chama `HookPair::renew` (`:1120`). Em outra thread, `WinCapture::begin` publica `begin=true` dentro de `spawn_blocking` (`adapter/win/capture.rs:865`), e `stop` publica a parada (`:916`). É possível a manutenção ler Idle, o outro fluxo publicar begin/stop, e só então instalar os novos hooks. As cargas Acquire não constituem exclusão mútua nem tornam essa sequência atômica. Portanto a afirmação absoluta “nada de renovação armando ou parando” excede a guarda implementada.

   **Limite do achado:** travessia produzida pelo callback e timer normais pertencem à mesma thread; não demonstrei corrida de dois callbacks físicos, evento duplicado ou dano causado especificamente por essa janela. O cleanup da parada ainda recolhe os hooks. O teste de begin preservado após renovação não exercita a intercalação entre Poll e install. Para sustentar a garantia declarada, coordenar a admissão com as transições concorrentes e testar essa intercalação de forma determinística; repetir apenas as cargas não elimina a janela. Alternativamente, explicitar no contrato a manutenção já admitida que pode terminar durante a transição e provar que seu efeito é seguro. O bloqueio de versão permanece sustentado independentemente por R11-STATE-01.

### Troca dos hooks, erros, callbacks e privacidade

1. **Sobreposição dos pares:** `ffi.rs:1066` instala o par completo antes de substituir o antigo e executar seu Drop. O caminho não chama bomba de mensagens, não muda o callback/CaptureCore e fica na thread proprietária. Isso é coerente com os hooks low-level serem chamados na thread instaladora por entrega de mensagem ([Microsoft: teclado](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc), [mouse](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc)). Não encontrei duplicação explícita de `on_input` no caminho da renovação. **Não é prova nativa de ausência de duplicatas na janela:** `ffi.rs:1525` verifica troca dos handles e chama as funções de teste de teclado/mouse depois de `renew`; não faz entrada do SO atravessar a sobreposição. Movimento em dobro, repetição e teclas/botões segurados precisam de ensaio nativo dirigido nessa janela. O defeito de estado acima independe de duplicatas.
2. **Falha ao instalar o novo:** `HookPair::install` (`ffi.rs:1046`) possui cada handle assim que instalado; se o teclado falha, Drop tenta remover o mouse parcial. `renew` só substitui `self` depois do sucesso completo, conservando o par antigo naquele instante. Porém `maintain_hooks` chama `mark_lost` no erro (`:1126`): os callbacks passam a deixar os eventos locais passarem e a mailbox aciona a recuperação. Não é fallback funcional contínuo no par antigo. `recreate_hooks` solta localmente, remove o par antigo (`adapter/win/capture.rs:693`) e tenta recriar sob a política existente; falha dessa recriação publica erro terminal (`:715`) e pode deixar o adaptador sem hooks até reinício do módulo. O relatório do autor deve ser lido com essa ressalva.
3. **Posse e vazamento:** RAII cobre instalação parcial, substituição e saída no caminho nominal; não apareceu handle intencionalmente abandonado. `UnhookWindowsHookEx` tem retorno ignorado (`ffi.rs:1084`), inclusive para remoção silenciosa anterior. A API pode falhar ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex)); esta leitura não prova ausência de hook órfão se a remoção falhar por outra razão. Não há teste novo que injete falha da segunda instalação ou da remoção. Não classifiquei vazamento hipotético como defeito reproduzido.
4. **Callbacks e unsafe:** não encontrei novo `unwrap()`/`expect()`/`panic!` no caminho de produção. Os procedimentos de input e o procedimento do timer contêm `catch_unwind`; a manutenção usa `try_borrow`/`try_borrow_mut`, e conflito resulta em não renovar. Os novos blocos unsafe permanecem em `ffi.rs`, com comentários SAFETY sobre handles, ponteiros, tamanho das estruturas e duração dos callbacks. Não apareceu exportação nova de `unsafe fn`. Check/clippy cruzados e script de isolamento passaram; isso não substitui execução Win32.
5. **Privacidade:** o script passou, e a busca por `tracing::`, `log::`, `println!`, `eprintln!` e `evento =` não encontrou emissores nos fontes de `clipsync-win` nem em `adapter/win`. O timestamp novo é atualizado depois do filtro de eventos injetados. Não há campo com tecla/keycode nem contador por tecla. Na entrega original, o log sai exclusivamente do motor, com `evento=hooks.reinstalled`, `contagem` acumulada e `motivo=idle_maintenance`. No checkout final, esse emissor foi adiado; o contador interno continua sendo atualizado, sem exposição pelo trait. §17.8 (`docs/contrato.md:2058`) descreve a entrega completa pretendida, não um evento que o estado atual já emita.
6. **Sinal e energia:** Option resolve a sentinela que confundia ausência de injeção com tick zero; os nove testes próprios de HookWatch passaram. 30 s, amostras de pelo menos 500 ms e intervalo mínimo de 10 s são política lógica, não garantia de tempo de parede. `GetLastInputInfo` é agregado da sessão e seus ticks podem não crescer monotonicamente ([Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlastinputinfo)); duas suspeitas não diagnosticam remoção. O doc comment herdado em `wincore.rs:486` a `:497` ainda chama a discrepância de “evidence” e sugere ausência de falso positivo; `:527` também fala de injetado como sonda, enquanto a cola atual o filtra. Alinhar esses comentários à cautela já correta em `HookHealth::Lost`. ABOVE_NORMAL e a solicitação de desativar EXECUTION_SPEED são mitigação; falhas são toleradas, o pedido de energia tem alcance de processo e não é revertido na pausa. Não houve medida de latência/energia nem comprovação da causa em campo.

### Intervenção em F2

A comparação com `/tmp/clipsync-f3-before/` e a cópia preservada em `/tmp/clipsync-f3-log-adiado/` confirma: `adapter/mod.rs:158` acrescentava método padrão readonly retornando zero; `capture.rs:546` guardava baseline, `:548` consultava no início do laço, `:802` consultava depois do fechamento e `:806` emitia somente quando o total crescia. A implementação Windows fazia leitura atômica do contador. Não alterava o relógio de Stats, prazo de 2 s, decisão de soltura, sessão, motivo de fechamento nem ordem interna do cleanup. A emissão é síncrona como outros logs, sem novo await no motor; nenhuma promessa nova de teto físico decorre disso. Os 9 S12, 4 S13 e 12 EIS passaram junto da entrega. O teste de log passou, conferindo ausência de duplicata e campos, mas não executa uma renovação Win32 real. A retirada concorrente dessas linhas deixa F2 sem esse acréscimo; a autorização desta revisão não manda restaurá-lo.

### Execução própria e alcance Windows

Todos os comandos Cargo usaram `PATH=$HOME/.cargo/bin:$PATH`, `-j2` e `--locked`; travas foram aguardadas, sem matar compilação concorrente.

| Comando | Resultado observado |
| --- | --- |
| `cargo test -j2 -p clipsync-input --test s12_failsafe --test s13_teclado_ausente --test eis_devices --test s14_hook_watch --test s14_hook_log --locked` | Exit 0; **37 passed / 0 failed**: S12 9, S13 4, EIS 12, S14 watch 11, S14 log 1. Executado antes da retirada do log. |
| `cargo test -j2 -p clipsync-input --lib hook_watch --locked` | Exit 0; **9 passed / 0 failed**, incluindo regressão da meia volta do relógio. Executado depois da retirada; aguardou trava, terminou em 40,87 s incluindo compilação. |
| `scripts/conferir-privacidade.sh` | Exit 0; `privacidade dos hooks: ok (8 arquivos)`. |
| `scripts/conferir-unsafe.sh` | Exit 0; `unsafe isolado em crates/clipsync-win/src/ffi.rs: ok`. |
| `cargo check -j2 -p clipsync-win --target x86_64-pc-windows-gnu --locked` | Exit 0, 0,43 s. |
| `CC_x86_64_pc_windows_gnu=true AR_x86_64_pc_windows_gnu=true cargo clippy -j2 -p clipsync-input -p clipsync-win --all-targets --target x86_64-pc-windows-gnu --locked -- -D warnings` | Exit 0, 0,17 s. |
| Harness externo contra o núcleo compilado | Exit 0, **defeito R11-STATE-01 confirmado**; controle conserva uma soltura, renovação conserva zero. |

Check e clippy verificaram os fontes e testes `cfg(windows)` da entrega. Não linkaram nem executaram binário Windows; usar `CC/AR=true` nessa checagem não é build nativo. A rodada ampla de 387 testes e o fmt relatados por Rivet não foram repetidos por esta revisão e não são apresentados como execução própria.

**CI do Windows:** ainda precisa compilar/linkar e executar os testes nativos de callbacks, timestamps, prioridade/configuração, handles, confirmação na thread e cola do adaptador. O teste `timer_real_reinstala_e_confirma_na_thread_dos_ganchos` (`ffi.rs:1580`) é `#[ignore]`: CI padrão não o executa. Mesmo o teste nativo de troca não simula entrada física durante a sobreposição nem falhas de instalação/remoção.

**PC real:** falta executar timer em desktop interativo desbloqueado, entrada durante trocas repetidas, teclas e botões segurados, travessia/retorno, pausa/bloqueio/parada concorrentes, remoção real de hook e o roteiro prolongado que originou o incidente. Só isso pode sustentar recuperação percebida pelo usuário e efeito da mitigação de escalonamento. CI verde não fecha esses aceites, e nem o relógio agregado nem a instalação bem-sucedida demonstram a causa original.

**Encaminhamento:** pode preparar ramo próprio da F3 para obter evidência nativa, sem misturar a F3 na v0.5.3. Corrigir R11-STATE-01, resolver/documentar R11-SYNC-02, recompor conscientemente a fatia do evento com seu teste e contrato quando autorizada, e revisar novamente o commit candidato após CI. Nenhum commit, ramo, push ou execução de workflow foi feito por este revisor.

**Linha de veredito para o workboard:** R11: não passa para versão; perda de soltura local reproduzida (Important), guarda concorrente incompleta (Minor); 37 focais + 9 HookWatch passaram, check/clippy Windows cruzados passaram; somente ramo isolado de CI, sem aceite nativo/físico e com log adiado no checkout.

**Fecho:** os sete hashes do manifesto permaneceram iguais e `git diff --check -- docs/verdicts.md` passou. Tentei `maestri ask "Maestro · scaffold" "R11: ..."`; exit 1, `Terminal 'Maestro · scaffold' not found`. Executei `maestri debug`: conexão HTTP 200, mas `maestri list` continua mostrando outro projeto, sem o coordenador ou workboard de ClipSync. Não alterei notas desse outro projeto. Entrega pelo arquivo.


## R12 · L1 · 12/11/2026

**Veredito: passa a L1. Sem achado impeditivo nos dois arquivos revisados; fecha R10-LOG-01 para F1/F2/N1 no código deste snapshot. Os 11 testes de diagnóstico passaram, incluindo os 8 novos. A aprovação do crate completo e da integração continua pendente da suíte e do clippy após a entrega da A1; não é aprovação de publicação. `hooks.reinstalled` permanece deliberadamente fora.**

Revisão de `git diff HEAD -- crates/clipsync-ui/src/diag.rs crates/clipsync-ui/tests/diag.rs`, dos arquivos completos nas áreas afetadas e dos emissores reais em portal/capture/transport, com conferência do caminho de serialização JSONL. Base main/c30c529, working tree compartilhado, sem commit. A implementação concorrente da A1 em `events.rs`/`traversal.rs` não recebe aprovação por este parecer. Só acrescentei esta seção; nenhum código foi editado.

### Correspondência entre lista e emissores

Todos os **novos literais** aceitos pela L1 têm origem concreta no código; nenhum deles precisa de nome externo ou texto de erro para ser construído. Linhas abaixo correspondem ao snapshot revisado.

| Lista em `crates/clipsync-ui/src/diag.rs` | Emissor conferido |
| --- | --- |
| `:760`: `peer.readdressed`; `:864`: origem `name` | `crates/clipsync-core/src/transport.rs:1033`. O nome é uma categoria literal; o campo `peer` continua redigido como identidade. |
| `:761`: `eis.device`; `:876`: ações `added`, `resumed`, `paused`, `removed` | `crates/clipsync-input/src/adapter/portal.rs:596`, `:608`; chamadas em `:951`, `:957`, `:965`, `:973`, `:1010`. Os argumentos de ação/origem nessas chamadas são literais. |
| `:865`–`:872`: origens `start`, `emit`, `resumed`, `release_all`, `added`, `paused`, `removed`, `loop` | `portal.rs:1100`, `:688`, `:731`, `:837`, `:951`, `:957`, `:965`, `:973`, `:983`, `:1010`. `reconcile_keys` recebe origem estática e só os três valores emit/resumed/release_all nessas chamadas. |
| `:876`: ações `missing`, `discarding`, `reconciled`; `:848`: motivo `not_ready` | `portal.rs:1098`, `:686`, `:719`. Ausência/descarte usam `capacidades=keyboard`; reconciliação registra tamanho do envio, sem conteúdo das solturas. |
| `:762`: `eis.loop`; `:849`–`:856`: motivos `context`, `handshake`, `converter`, `eof`, `disconnected`, `closed`, `runtime`, `io` | `portal.rs:885`, `:896`, `:917`, `:925`, `:977`, `:983`, `:1002`, `:1015`, `:1024`, `:1061`, `:1073`. |
| `:991`–`:996`: erros `event`, `parse`, `request`, `handshake`, `interface_version`, `io` | `portal.rs:1073` mapeia variantes de `reis::Error` para pares de strings estáticas; `:983` usa `io` literal. `:1017` formata essa categoria, **não** o erro original. |
| `:847`: motivo `peer_unresponsive` | `crates/clipsync-input/src/capture.rs:867`, consumido por `session.closed` em `:1086`. |
| `:839`: motivo `apply_failed` | Já permitido antes da L1, sem alteração neste ramo. `capture.rs:367` traduz `RejectReason::ApplyFailed`; `session.refused` usa `reject_code` em `:1635`/`:1717`; `module.rs:1908` encaminha a mesma tradução ao emissor `enter.refused` (`:1929`/`:1950`). O teste novo protege a permanência desse comportamento. |
| `:892`: capacidades | A ordem `pointer,pointer_absolute,button,scroll,keyboard,touch,text` é exatamente a de `portal.rs:577`. O produtor verifica flags, acrescenta só esses nomes e junta com vírgula; não usa nome do dispositivo. Vazio é uma possibilidade do mesmo algoritmo. |
| `LOG_FIELDS :966`–`:969`: `acao`, `dispositivo`, `dispositivos`, `capacidades` | `portal.rs:598`, `:610`, `:719`, `:1015`/`:1024`. `dispositivo` recebe ordinal local de anúncio (`:940`/`:942`), e `dispositivos` recebe o total anunciado. Não são nomes nem identificadores externos. |

### Privacidade e valores fora da lista

1. **Texto novo continua limitado a vocabulário fechado.** Eventos, motivos, origens e ações usam correspondência literal. O validador de capacidades (`diag.rs:892`) consome um iterador da lista em ordem: aceita somente subsequências estritamente ordenadas, no máximo sete nomes, sem duplicata, espaços, sufixos, separador vazio ou quebra de linha. Os testes em `tests/diag.rs:521` e `:544` exercitam casos válidos e inválidos, inclusive tentativas de texto privado. `text` é o nome fixo de uma capacidade; não libera seu conteúdo.
2. **Erro contextual não abre passagem para o protocolo bruto.** `diag.rs:978` exige string exatamente igual a `eis.loop` no campo `evento` do mesmo objeto. `:985` aceita `erro` apenas como string exatamente igual a uma das seis categorias. Qualquer outro erro, tipo JSON, evento ausente, sufixado ou diferente produz `<texto omitido>`; não há regra por prefixo nem encaminhamento de objetos. O filtro de nomes de campo roda primeiro (`:982`), de modo que um objeto `totais` com `evento`/`erro` não ganha a exceção. O teste `tests/diag.rs:610` comprova esses contextos; `:649` verifica sufixos, caminho privado e valores estruturados. O redator genérico (`diag.rs:909`) continua omitindo erro e message livres.
3. **Identidades e conteúdo não foram liberados.** `host`, `peer`/`id`, `nome` e `pares` continuam virando marcadores em `diag.rs:1021`; campos extras como tecla, keycode, clipboard e `pending_releases` continuam eliminados antes da redação (`:982`). `acao`/`capacidades` não textuais são descartados (`:1018`); strings desconhecidas viram `<texto omitido>` (ou marcador de ID no caso preexistente de ID curto). Isso não autoriza registrar tecla/keycode nos emissores. O ensaio usa conteúdo sintético e não captura entrada do usuário.
4. **Precisão sobre números:** a guarda `as_u64()` de `diag.rs:1015` vale para **`dispositivo` e `dispositivos`**, não para o campo chamado `contagem`. Esses dois campos recusam strings, negativos, frações, booleanos, nulos, arrays e objetos. `contagem` já estava em `LOG_FIELDS` (`:962`), assim como `descartados`; os emissores reais usam contadores numéricos. O tratamento genérico dos campos antigos ainda admite escalares não textuais (`:1081`). Portanto “valor fora da lista continua omitido” descreve o vocabulário textual; a L1 não é validação completa de esquema para todos os campos antigos. Não apareceu regressão nem passagem nova de texto livre por essa diferença.
5. **F3 segue fora:** `hooks.reinstalled` não está na lista de eventos, e `idle_maintenance` não está na de motivos. A fixture em `tests/diag.rs:652` confirma que o evento desconhecido permanece omitido. Reintegrar F3 exige mudança e revisão próprias; a L1 não a torna observável por antecipação.

A conferência de `crates/clipsync-core/src/logging.rs:380`/`:389`/`:392` confirma que números e strings dos emissores chegam à cauda nos tipos esperados. A mensagem livre gerada por tracing é armazenada como `mensagem` (`:351`) e continua sujeita à lista curta preexistente em `diag.rs:858`; as descrições livres de EIS não são copiadas por esta mudança.

### Verificação própria

Executado `PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-ui --test diag --locked`. Aguardei a trava sem interromper outro processo. Resultado: **exit 0, 11 passed / 0 failed / 0 ignored**, incluindo os **8 casos L1**; Cargo terminou a etapa de preparação em 2 min 21 s, e o alvo executou em 15,44 s. Os testes verificam a API real de prévia/exportação, leem o arquivo gerado e comparam seu conteúdo/bytes com a prévia; as linhas JSONL são sintéticas e os adaptadores são fakes. Não houve execução do app nem sessão física EIS/Windows.

`git diff --check -- crates/clipsync-ui/src/diag.rs crates/clipsync-ui/tests/diag.rs` passou. O RED relatado pelo andar TESTES e o total de 110 integrações não foram repetidos nesta revisão: são evidência do autor, distinta dos 11 testes executados aqui. Não rodei suíte completa nem clippy enquanto a A1 está em trabalho; as falhas informadas no pedido não foram reinterpretadas como falhas da L1 nem certificadas como ainda presentes.

Hashes da fatia revisada:

- `crates/clipsync-ui/src/diag.rs`: `58caff5e079c1197c6592676602bdb112c4e6164ecc4ea6253751be6f8352b21`.
- `crates/clipsync-ui/tests/diag.rs`: `a08c6979a65e78fc4badb31d63d16abbc9ebb953eb181494700803c71fedc877`.

### Gate após a entrega da A1

1. Sobre o estado final integrado, repetir **`PATH=$HOME/.cargo/bin:$PATH cargo test -j2 -p clipsync-ui --locked`**, sem filtros que escondam os testes de events/traversal. Esse comando deve incluir novamente os 11 de diag, os demais testes de integração, unitários e doctests aplicáveis. O verde focal desta R12 não substitui esse resultado.
2. Repetir **`PATH=$HOME/.cargo/bin:$PATH cargo clippy -j2 -p clipsync-ui --all-targets --locked -- -D warnings`**. Resolver os problemas de código em andamento e registrar o resultado final; não declarar o crate inteiro aprovado com base apenas nesta leitura.
3. Conferir que os hashes/diff da L1 permanecem iguais ou revisar qualquer mudança posterior. Os gates de workspace/release e validação própria da A1 continuam com seus donos. Após integração, o estado do filtro nos documentos pode apontar esta R12; não reescrever o achado histórico R10 nem afirmar que o binário já instalado recebeu a mudança.

**Linha de veredito para o workboard:** R12: passa L1, fecha R10-LOG-01 para F1/F2/N1 neste snapshot; 11/11 diag, vocabulário conferido nos emissores, erro bruto continua omitido; F3 fora; suíte completa e clippy de clipsync-ui pendentes após A1. Sem código editado ou commit.

**Fecho:** os dois hashes permaneceram iguais e `git diff --check` dos dois arquivos e deste parecer passou. A tentativa de `maestri ask "Maestro · scaffold" "R12: ..."` retornou exit 1, `Terminal 'Maestro · scaffold' not found`; executei `maestri debug`. A identidade/conexão listada continua sendo a de outro projeto, sem workboard de ClipSync; não alterei suas notas. Entrega por este arquivo.

## R13 · A1 · 13/11/2026

**Veredito: não passa o conjunto F1 + F2 + N1 + L1 + A1 para commit candidato à v0.5.3 nem para publicação neste estado. R13-A1-01 (Important) permite uma nova devolução sem nova notificação, mesmo com a preferência ligada. Corrigir e rever esse caminho antes de fechar A1/S2-PRIV-01; os pareceres anteriores de F1/F2/N1/L1 permanecem válidos nos seus recortes. Não houve commit ou edição de código.**

Revisor: **Argus · warden**, identidade confirmada por `maestri list`. Base `main`, HEAD `c30c529`; árvore compartilhada sem commit. Li o quadro do incidente, S2 item 4, R4/R5/R9/R10/R12, o delta `a1-delta.patch`, `git status`, diffs e fontes atuais. A aplicação **em memória** dos nove trechos do patch às bases indicadas reproduziu exatamente os nove arquivos atuais; não apliquei patch no checkout. L1 conserva os dois hashes de R12. F3, seus arquivos Windows e `s14_hook_watch.rs` estão fora desta aprovação. Por orientação expressa do Maestro nesta rodada, não há gate de portal/runtime nem atualização de notas pelo revisor: o parecer fica somente aqui.

### Achados por severidade

1. **Important · R13-A1-01 · deduplicação pela frase perde uma nova devolução após recuperação entre consultas.** `crates/clipsync-ui/src/events.rs:466`–`:475`, `:587`–`:606`; produtor em `crates/clipsync-input/src/module.rs:564`–`:570`. A UI guarda apenas `input_block` e compara as duas frases reconhecidas, sem identidade/contador do episódio. Cenário: a consulta anterior observou `CONTROL_RETURNED_APPLY_FAILED`; uma nova sessão abre para o mesmo par, limpando o aviso; uma falha de aplicação devolve novamente a captura antes da consulta seguinte. As duas amostras contêm a mesma frase, portanto `previous_return != current_return` é falso e sai **zero** notificação nova. A falha rápida por ApplyFailed não precisa esperar os 2 s da vigia. O histórico não socorre: `BlockNotifications::observe` exclui esses motivos (`events.rs:143`) e `diff` não usa suas ocorrências para este kind. A preferência ligada e o SO permitindo avisos não resolvem a perda. O usuário voltou a controlar o destino e pode continuar digitando no programa local depois dessa nova devolução, precisamente o risco que A1 pretende mitigar.

   Reproduzi a decisão executando uma extração literal do início de `Snapshot::diff` e de `msg::control_returned_notice`, com tipos mínimos para o harness, compilada por `~/.cargo/bin/rustc --edition=2024 -o ~/.cache/cs-r13/amostragem -`. Resultados: recuperação **observada** seguida de falha = **1 aviso**; falha → recuperação → mesma falha entre duas consultas = **0 avisos novos**. É execução do comparador de produção, não ensaio completo de rede/Tauri nem reprodução física. A leitura do produtor e do laço estabelece a intercalação possível. O teste `a1_current_return_is_immediate_and_repeats_after_recovery` (`events.rs:662`) só compara `failed → good → failed` com `good` observado, por isso não cobre o defeito. Conservar uma identidade de ocorrência independente do texto entre consultas e acrescentar regressão determinística dessa intercalação; preservar o descarte do alerta histórico e a soltura antes da rede. Não basta reduzir o intervalo do poll.

2. **Minor · R13-A1-02 · não há contenção temporal própria para devoluções sucessivas.** `crates/clipsync-ui/src/events.rs:474`–`:482` e `crates/clipsync-app/src/events.rs:144`–`:148`. Estado estável não repete aviso, mas cada nova transição observada para uma das frases passa imediatamente; alternar as causas também passa sem precisar observar estado limpo. O mesmo harness produziu **60 avisos em 60 snapshots alternando as causas**. Esse número prova ausência de orçamento no comparador, não 60 notificações reais por segundo. O limite prático depende do poll, das consultas locais e de novas sessões/devoluções válidas; o timeout por silêncio ainda leva 2 s. `Refused` exige par/sessão corrente, e `Rejected` exige o par corrente: não é um emissor remoto irrestrito quando não há captura. Um par escolhido que provoque falhas durante tentativas repetidas pode, contudo, causar incômodo e habituação ao aviso. A casca não acrescenta limite e seu teste admite dois avisos no mesmo instante. Recomendo uma política explícita de agrupamento/substituição para a sequência, preservando o primeiro aviso imediato e o estado persistente; não restaurar cegamente o atraso genérico de 30 s. Não demonstrei fila/inundação no SO nem engenharia social com texto controlado pelo par.

3. **Minor · R13-DOC-03 · contrato e release ainda não descrevem a A1 entregue.** `docs/contrato.md:2075` e `:2078` registram corretamente um snapshot histórico que substituía a composição por retorno; o fonte atual já compõe os três motivos. O adendo não contrata ainda `control_returned`, sua deduplicação, a exceção ao agregado de 30 s e seus limites. `docs/releases/v0.5.3.md:9` anuncia a devolução, mas não explica o aviso, a preferência, a espera por consulta ou a possibilidade de supressão pelo sistema. Acrescentar entrada de versão após a correção, sem reescrever o snapshot/histórico. Atualizar também o estado de L1 para R12 e os limites abaixo. A API/evento novo exige esse registro pelo AGENTS.md.

### Respostas aos sete critérios

1. **Defeitos:** os três achados acima. R13-A1-01 impede o aceite da A1; R13-A1-02 é ressalva de robustez, e R13-DOC-03 é consolidação necessária antes do release. Não identifiquei regressão nova na mecânica da F2 nem abertura de texto livre no corpo da notificação.
2. **Todos os caminhos da F2:** **sim no produtor de estado; não como garantia de notificação entregue ao usuário.** `capture.rs:852`–`:869` usa sessão ativa ou `opening.session` no mesmo ramo de expiração e emite a frase de silêncio após `end_session`. `:1638`–`:1640` cobre `Refused/ApplyFailed` correlacionado; `:1720`–`:1721` cobre `Rejected/ApplyFailed` legado. Em todos, `control_returned` envia `Note::ControlReturned` e `Note::Block` (`:1031`). Os testes S12 cobrem abertura bloqueada, silêncio ativo e falha de emit v1/v3. A1 não acrescenta aviso específico para todo fechamento preexistente: por exemplo, `PeerDown` conserva `PEER_OFFLINE`; o ramo de emissão v4 continua classificando falha como `Locked` conforme R10/contrato. Não ampliar a afirmação para esses caminhos ou para toda falha remota.
3. **Composição e R9:** **preservada no campo `ModuleStatus.last_block`.** `module.rs:1385` começa com o motivo/fallback, `:1395` acrescenta o aviso derivado do teclado e `:1401` prefixa o retorno. `on_note` ignora apenas o `Block` literal que duplicaria o retorno já guardado, preservando o motivo independente. Abrir para outro par não o limpa; abrir para o mesmo par limpa o retorno e apenas os motivos reconhecidos da mesma falha (`:567`–`:580`). Isso não torna `last_block` um histórico de todos os erros: uma nova recusa legada ainda escreve seu motivo em `:1428`, comportamento anterior a A1. A garantia de R9 é a composição do motivo vigente com o aviso do teclado, não retenção eterna de todos os motivos anteriores.
4. **Histórico e repetição:** a frase antiga não volta pelo histórico em `traversal.rs:224`–`:237`, e os retornos históricos também ficam fora do resumo genérico. `retain_unobserved` conserva o baseline quando o agente não foi observado (`events.rs:425`), evitando replay por simples falha de leitura. A mesma causa volta a notificar **se a UI observou a recuperação**; sem essa amostra intermediária, falha conforme R13-A1-01. Abertura bem-sucedida aqui é a transição local `session.active` após envio/begin (`capture.rs:1183`–`:1188`), não ACK de aplicação no destino. O estado é persistente durante a vida do módulo, não armazenamento durável contra reinício.
5. **Corpo da notificação:** **não carrega texto livre pelo caminho revisado.** `msg.rs:16` retorna uma das duas referências `&'static str`, aceitando apenas igualdade ou prefixo seguido do separador ` · `. `events.rs:481` copia essa referência, não a string completa do status. Nome do par, motivo adicional, erro e aviso do teclado não entram nesse corpo. O harness confirmou que um complemento sintético é excluído; os testes da UI cobrem complementos e sufixo inválido. Título fixo `ClipSync`. A casca recebe os eventos da UI; este recorte não é auditoria geral de todos os outros kinds.
6. **Inundação:** existe a possibilidade limitada a transições observadas de devoluções válidas, conforme R13-A1-02. Não há repetição por tecla, por Stats ou por poll sem mudança de frase. O teto de 30 s do kind `block` não se aplica; `NotifyPrefs::allows` em `notify_prefs.rs:34` mantém `control_returned` sob a preferência `block`, conferida novamente antes de `show`.
7. **F2 preservada:** o delta de `capture.rs` só acrescenta a nota e seleciona a frase de ApplyFailed. Mantém constantes 500 ms/2 s (`:81`–`:83`), identidade e timestamp de Stats, prioridade da vigia e cleanup. `close_session` libera o dispositivo (`:1071`–`:1077`), depois cancela trabalho e monta a future de rede (`:1098`); a nota é emitida antes de voltar ao laço que a executará. Não surgiu await de rede antes da soltura/aviso. `release()` ainda depende do adaptador, e os limites de agendamento/carga de R4 permanecem: não é teto físico absoluto.

### Verificação própria

Todos os comandos Cargo usaram `PATH=$HOME/.cargo/bin:$PATH` e `-j2`. A trava da verificação concorrente foi aguardada, sem encerrar processos. A primeira etapa informou `Finished test profile ... in 5m 10s`, incluindo espera/preparação. Os comandos seguintes usaram `TMPDIR=~/.cache/cs-r13`, sem guardar relatórios em `/tmp`.

| Comando | Resultado observado |
| --- | --- |
| `cargo test -j2 -p clipsync-input --test s12_failsafe --test s13_teclado_ausente --test eis_devices` | Exit 0; **26 passed / 0 failed / 0 ignored**: EIS 12 (5,00 s), S12 10 (5,41 s), S13 4 (2,45 s). |
| `cargo test -j2 -p clipsync-ui --locked` | Exit 0; **205 passed / 0 failed / 0 ignored**: 95 unitários e 110 de integração; doctests com zero casos. Inclui os cinco unitários A1 de events/traversal e os 11 de diagnóstico L1. |
| `cargo test -j2 -p clipsync-app --locked` | Exit 0; **28 passed / 0 failed / 0 ignored**, 0,05 s; compilação/preparação 19,50 s. Inclui o caso A1 que libera o kind fora do throttle e o recusa com preferência desligada. |
| `cargo test -j2 -p clipsync-input --lib a1_return_preserves_other_blocks_and_keyboard_until_same_peer_recovers --locked` | Exit 0; **1 passed / 0 failed**, 128 filtrados. Exercita os três motivos juntos, mudança de complemento, outro par e recuperação do mesmo par. |
| `cargo clippy -j2 -p clipsync-ui --all-targets --locked -- -D warnings` | Exit 0; `Finished dev profile ... in 10.71s`, sem diagnóstico de warning/erro. Fecha com a suíte o gate local de integração solicitado em R12, sem sanar R13-A1-01. |
| Extração executável da decisão de A1 | Exit 0; recuperação observada: 1 aviso; mesma causa após recuperação não observada: 0; 60 snapshots alternando causas: 60; complemento sintético excluído do corpo. |
| `git diff --check` nos nove caminhos A1 e em `docs/verdicts.md` | Exit 0, saída vazia. |

Total desta bateria: **260 testes aprovados**, zero falhas; a extração do comparador é evidência adicional, não incluída nesse total. Logs próprios de UI/app/composição/clippy ficam em `~/.cache/cs-r13/`. Os resultados da verificação ampla concorrente do Maestro não foram somados. O app **compilou e testou nesta máquina nesta rodada**, apesar do impedimento de headers descrito nas instruções históricas; não executei sua janela ou notificação nativa. Não rodei instalação nem `dev`, dispensados expressamente nesta revisão Rust. Não criei teste/fonte no repositório; o harness foi passado ao rustc por stdin.

O comando focal da F2 continua protegendo silêncio com entrada, sessão saudável, resposta de par/conexão/sessão incorretos, timestamp de Stats e abertura atrás de rede bloqueada. Os resultados não são teste do comparador da UI com recuperação não observada, nem medição física no Windows. Os testes S12 confirmam estado local após release, sem sustentar aviso visível simultâneo.

### Condições para novo aceite e limites escritos

1. Corrigir R13-A1-01, executar a regressão entre consultas e rever o delta final. Depois consolidar a política de R13-A1-02 e o contrato/release de R13-DOC-03. Este parecer não aprova commitar o working tree inteiro: selecionar F1/F2/N1/L1/A1 e excluir F3. Não autoriza commit, push ou tag.
2. O aviso depende da consulta nominal de **1 s mais as consultas locais** (`UiOptions::default`, `lib.rs:67`; `Snapshot::take`, `events.rs:333`). É imediato em relação à observação, não simultâneo à soltura, e não tem teto físico garantido. A digitação local já pode ocorrer antes de o usuário percebê-lo. Preferência desligada, ausência/falha do notificante, app encerrado ou modo de concentração podem impedir sua apresentação. Não bloqueia teclas nem toma foco; não promete impedir exposição de texto.
3. Não houve prova de notificação no Windows real, app minimizado, duração visível, GNOME real ou travessia entre as duas máquinas. Os testes do app exercitam admissão/lógica sem comprovar apresentação pelo SO. Nenhum aceite físico foi fechado, e a causa original do incidente continua não demonstrada.
4. Permanecem os limites de R9/R10/S2: F1 pode descartar teclas indefinidamente enquanto o mouse e Stats continuam; não há replay, e o aviso do teclado é local ao destino. Flush/reconciliação não provam soltura física no compositor. F2 garante a decisão lógica de devolução local por silêncio, não soltura remota garantida. N1 não saneia pareamentos sobrescritos por versões antigas, pode usar DNS e não garante reencontro instantâneo. L1 conserva vocabulário permitido, sem liberar conteúdo/erro bruto; contagens e timestamps ainda são metadados de atividade.
5. Após novo aceite técnico, a publicação exige versão/tag coerentes, CI Linux/Windows e pacotes/hashes do commit final. A bateria local e os pareceres das fatias não substituem esses gates; o checkout desta revisão ainda declara v0.5.2.

### Identificação do recorte

SHA-256 do `a1-delta.patch`: `81722e919901c1057281242ccc8592c29ba4caf7eb2ce2fb8da5c7469bca8ba6`. Fontes de produção da A1:

| Arquivo | SHA-256 |
| --- | --- |
| `crates/clipsync-input/src/capture.rs` | `7c3a2fdbfad95bf7c6b447942b9c9e3feb8837605255b3cc28b934f098180e60` |
| `crates/clipsync-input/src/module.rs` | `9dc896bd1cdad28fb439b1628ddd054540f058408797152d117bcf854e2b214c` |
| `crates/clipsync-input/src/msg.rs` | `c3ebb97638e1c83f255e00d8462988e2881e0dffd0d1870305348ce01520e2de` |
| `crates/clipsync-ui/src/events.rs` | `edbee2d61e269115fb1aae9150f4f489c6c053bd017dd4f28e8309f82c7e25de` |
| `crates/clipsync-ui/src/traversal.rs` | `53deb98aee9be60b327c1679ddb9ad8d58d74555844a746af990c850c86c8124` |
| `crates/clipsync-app/src/events.rs` | `f16a9de885cd72e41f0d2bdaa6ec24a36903aa5a10ed3aaca8e55815c3193591` |
| `crates/clipsync-app/src/notify_prefs.rs` | `9ca4ac2a2547a6b6f75c0fc9ace29507c8613094685ee1eba25076bf69847a2c` |

**Linha de veredito entregue ao Maestro:** R13 não passa A1 para candidato/publicação v0.5.3: Important por novo retorno perdido entre consultas; Minor por série sem contenção e contrato/release defasados; 260 testes e clippy UI aprovados; F2 e composição R9 preservadas, corpo literal, F3 fora, sem prova física ou commit.

**Fecho:** os nove hashes A1 e os dois L1 permaneceram iguais após os testes; `Cargo.lock` não mudou. `git diff --check` do recorte e deste parecer passou. Só acrescentei R13 a este documento, preservando as seções anteriores. Comunicação final por `maestri ask "Maestro · scaffold"`, conforme a delegação; quadro/notas ficam com o Maestro.

## R14 · A1 rodada 2 · 13/11/2026

**Veredito: PASSA COM LIMITES. O conjunto F1 + F2 + N1 + L1 + A1 pode ser commitado e publicado como v0.5.3, com os limites escritos abaixo e no §17.9, após conferir a árvore selecionada sem F3 e cumprir CI/empacotamento/hashes do commit final. R13-A1-01 e R13-DOC-03 fechados neste recorte; R13-A1-02 permanece limite aceito da versão por decisão do Maestro, sem correção de contenção. Não identifiquei novo impedimento técnico. A bateria própria passou: 831 testes principais, cinco execuções auxiliares, zero falhas e dois gráficos ignorados; fmt, ambos os clippys e privacidade aprovados. Não houve commit, edição de código ou aceite físico.**

Revisor: **Argus · warden**, identidade confirmada por `maestri list`. Base `main`, HEAD `c30c529`; working tree sem commit, ainda com versão 0.5.2. Li a entrega do Rivet como alegação, o delta integral contra `snapshot-a1-r13/`, os fontes atuais e os documentos consolidados pelo Maestro. A aplicação dos hunks **somente em memória** reproduziu os 15 arquivos alterados. Os cinco restantes dos 20 arquivos do snapshot ficaram byte a byte iguais: `input/src/capture.rs`, `app/src/events.rs`, `app/src/notify_prefs.rs`, `ui/src/lib.rs` e `ui/src/status.rs`. Os 20 hashes permaneceram iguais entre o início e o fim da bateria. F3 não recebe aprovação por este parecer.

### 1. R13-A1-01 fechado dentro da mesma instância do módulo

`crates/clipsync-input/src/module.rs:367` inicia o contador em zero; `:567`–`:569` avança `wrapping_add(1).max(1)` a cada `Note::ControlReturned`, independentemente de frase/par. `Note::Active` limpa o aviso, mas não reinicia a contagem. `status()` publica contador e texto do mesmo estado (`:1420`), sem await intercalado. A UI lê os dois campos da mesma resposta em `events.rs:398`–`:408`; `:482` compara identidade **ou** frase e ainda exige frase atual reconhecida. Portanto duas devoluções iguais separadas por recuperação não observada deixam IDs diferentes, sem depender de capturar o estado limpo intermediário.

**A regressão nova exercita o comparador de produção.** `a1_same_return_after_unobserved_recovery_notifies_again` (`crates/clipsync-ui/src/events.rs:704`) inicia a UI, interrompe seu poll automático para controlar a intercalação e oferece respostas pelo IPC local real. `Snapshot::take` lê primeiro frase/ID 1; o fixture passa por recuperação sem consulta e disponibiliza a mesma frase com ID 2; a segunda coleta vai para `Snapshot::diff`, pelo helper `return_notices`. Exige um aviso e o corpo literal. Também exige zero para estado estável, preferência desligada e ID novo já sem aviso atual. O produtor é coberto separadamente por `a1_return_identity_changes_for_every_note_and_survives_recovery` (`module.rs:2659`), com chamadas reais a `State::on_note`, repetição de causa/par, troca de par/causa e recuperação. Esses testes passaram nesta revisão. O fixture não dirige uma captura física nem substitui o teste separado do produtor.

Repeti a extração executável do R13: compilei por stdin o início literal de `Snapshot::diff` e o extrator literal de cada snapshot, com tipos mínimos de harness, usando `~/.cargo/bin/rustc --edition=2024`. Para a mesma frase de silêncio e IDs 1 → 2, **R13 produziu 0 aviso; R14 produziu 1**. A extração atual também confirmou estado estável = 0, ID novo já recuperado = 0, preferência desligada = 0 e corpo sem complemento. A expressão de avanço conserva zero reservado ao passar MAX → 1. São provas da decisão extraída, adicionais aos testes reais de UI/IPC e State, sem alegar execução completa de captura no harness. O RED histórico do autor não foi somado como execução própria nem exigiu reverter o checkout compartilhado.

### 2. Reinício: limite aceito, sem promessa de unicidade entre instâncias

**Aceito o contador por instância como limite escrito da v0.5.3; não exijo época por instância antes deste release.** A colisão remanescente exige recriar o módulo enquanto a UI continua viva e voltar a uma combinação id/frase igual sem uma amostra limpa intermediária. Ela não ocorre pela recuperação/recaptura normal da mesma instância, que motivou o bloqueio de R13. A1 é mitigação parcial por aviso, não barreira de confidencialidade nem entrega durável de toda ocorrência. Não há evidência nesta revisão de que o par remoto consiga reiniciar o módulo local. Isso não elimina o risco de S2 nem promete aviso em todo retorno.

Conferência do caminho real:

- `InputModule::run` cria um novo `State` com zero (`module.rs:340`–`:367`). Reiniciar somente a captura em `start_continuous` não recria esse State nem reinicia o contador. Uma reconfiguração de módulos pode parar/reconstruir o agente mantendo a UI (`crates/clipsync-ui/src/modules.rs:308`, `:332`), assim como stop/start ou substituição do agente externo.
- `events.rs:432`–`:435` conserva **frase e ID** quando `live` é falso. A ausência pode durar várias consultas: ela **não** equivale a uma amostra limpa. `agent::start/stop` não reinstala o poll nem lhe entrega um baseline vazio. A volta do agente não obriga o poll a observar zero antes da primeira devolução.
- O harness executou `(ID 1, frase) → ausência observada → nova instância com (ID 1, mesma frase)`: **0 avisos**. Com uma amostra efetivamente limpa antes do retorno: **1 aviso**. Com ID diferente depois da ausência: **1 aviso**. A ausência conserva o baseline para evitar replay, mas não distingue instâncias.
- Reiniciar a própria UI tem semântica diferente: `Ui::start` toma a primeira amostra e a entrega como baseline (`lib.rs:170`–`:172`), sem replay por desenho. O contador não é persistido. Um ciclo completo de u64 também não oferece unicidade ilimitada; a passagem adjacente MAX → 1 apenas evita zero.

O §17.9 agora descreve explicitamente reinício, recriação/reconfiguração com UI viva e ausência de garantia de amostra limpa. As notas da versão avisam que pode faltar o aviso após reinício com janela aberta. Uma futura garantia entre instâncias precisaria de identidade de época; não declarar que essa garantia existe nesta versão.

### 3. Posse, API local e compatibilidade

Fora da posse original, o delta contém **exatamente três linhas no proto** (comentário, `#[serde(default)]`, campo final `pub control_returned_id: u64`) e **12 inicializações neutras**. A comparação linha a linha removeu somente as linhas `control_returned_id: 0,` e recuperou integralmente cada arquivo de base:

| Arquivo | Inicializações com zero |
| --- | ---: |
| `crates/clipsync-core/src/agent.rs:458` e `:473` | 2 |
| `crates/clipsync-clipboard/src/module.rs:658` | 1 |
| `crates/clipsync-input/src/diag.rs:310` | 1 |
| `crates/clipsync-ui/src/agent.rs:491` | 1 |
| `crates/clipsync-core/tests/s10_prepare_inventory.rs:41` | 1 |
| `crates/clipsync-core/tests/modules_runtime.rs:267` e `:355` | 2 |
| `crates/clipsync-core/tests/mesh.rs:121`, `:196` e `:1289` | 3 |
| `crates/clipsync-ui/tests/diag.rs:98` | 1 |

Não há mudança adicional de lógica, ordem ou texto nesses oito arquivos. `traversal.rs` ganhou apenas zero no fixture de teste, dentro da posse da UI; seu comportamento de produção não mudou.

`ModuleStatus` continua no controle local: é campo de `AgentStatus` e variante de `ModuleReply` (`clipsync-proto/src/control.rs:62`, `:246`), transportado em `ControlResponse`; o controle usa `serde_json` no socket/pipe (`clipsync-core/src/control.rs:51`, `:59`). O canal interno de comandos usa `oneshot::Sender<ModuleReply>`. Os frames postcard de rede são `Frame`/`Datagram`, com `ModuleMsg::{Clip, Input}`, sem `ModuleStatus` (`clipsync-proto/src/wire.rs:13`, `:109`, `:139`). `wire.rs`, `input.rs`, limites e dependências não receberam delta da rodada; protocolo **4**, mínimo **1**, preservados. O teste `a1_legacy_json_status_defaults_return_identity_to_zero` passou: JSON antigo sem campo decodifica com **0**. Isso não transforma agente antigo em produtor de IDs; nesse caso permanece a detecção pela frase.

### 4. Privacidade, frase e invariantes preservadas

1. **Contador fora de log e pacote.** Os usos novos são armazenamento, IPC e comparação, sem emissor de log. `core/src/agent.rs:463` continua passando apenas `last_block` a `note_block`. O pacote monta os campos do módulo explicitamente (`ui/src/diag.rs:324`–`:332`), sem contador; `LOG_FIELDS` (`:933`) não o admite, e o filtro rejeita chave ausente da lista (`:983`). O exportador L1 não mudou; seu fixture ganhou somente zero. Os 11 testes de diagnóstico passaram na suíte UI. Não houve pacote capturado de um agente físico nem teste novo específico com contador não zero dentro de pacote: a exclusão desse campo foi conferida no caminho de produção.
2. **Corpo continua literal.** `msg::control_returned_notice` retorna uma de duas referências `&'static str`, e `events.rs:489` usa essa constante como corpo. ID não é serializado no evento de notificação; nome do par, erro e complemento do status também não entram. A extração executada confirmou exclusão de complemento sintético. A casca e as preferências estão idênticas ao R13.
3. **Frase ajustada.** Busca em `crates/` encontrou **zero** ocorrências de `não conseguiu aplicar a entrada`. A nova frase completa é idêntica em `input/src/msg.rs:4`, `input/tests/s12_failsafe.rs:21` e `ui/src/events.rs:652`: “Controle devolvido a este computador: o outro não aceitou o mouse e o teclado. As próximas teclas serão digitadas aqui.” O ajuste solicitado por S3-UX-01 está aplicado; não é ensaio de compreensão com usuários.
4. **Estado/baseline/histórico.** Mesma identidade/frase não repete; primeira coleta permanece baseline; ausência conserva ambos os campos (teste com ID 42); ID novo sem frase atual não emite aviso atrasado. Histórico continua excluído do retorno na travessia e do agregado genérico. Os seis testes A1 da UI passaram.
5. **F2 e R9.** `capture.rs` é byte a byte igual ao snapshot R13: sonda nominal de 500 ms, prazo lógico de 2 s, soltura antes da rede e os mesmos três pontos de aviso. A composição continua `retorno · motivo vigente · aviso de teclado`; o delta no módulo acrescenta o contador, não muda essa composição ou sua limpeza. S12 passou 10/10; S13, 4/4; EIS, 12/12; o teste unitário da composição também passou dentro da suíte completa.

### 5. Documentação e seleção sem F3

**R13-DOC-03 fechado.** Conferência final de `docs/contrato.md:2111` (§17.9), `docs/releases/v0.5.3.md:10`, `:20`–`:21`, `CHANGELOG.md:10`, `docs/uso.md:64` e da linha A1 em `docs/pendencias-v0.5.0.md:69`: registram composição, campo local/default, kind, preferência, baseline, amostragem, persistência em memória e os limites de apresentação. O adendo novo prevalece sem reescrever §17.8. As notas incluem reinício e avisos em série; a linha R10-LOG-01 e `docs/verificar-por-log.md` apontam R12 como aprovação do filtro. A suíte completa e o clippy UI/workspace desta rodada fecham o gate local pedido por R12. R13-A1-02 não é marcado corrigido: permanece limite expressamente aceito pelo Maestro, também descrito no contrato e no release.

**Não identifiquei arquivo do conjunto aprovado dependente das mudanças da F3.** A busca dos símbolos novos `HookWatch`, `HookHealth`, `HookSample`, `HookMaintenance`, `HookMaintenanceRunner`, `start_with_maintenance` e do contador de reinstalações ficou restrita aos cinco arquivos excluídos. A única referência externa a `hooks.reinstalled` é uma string sintética no teste de diagnóstico (`ui/tests/diag.rs:653`), que verifica a omissão do evento desconhecido, sem dependência do código F3. `input/src/adapter/mod.rs` acrescenta apenas `EmulateAdapter::block_reason` com default `None`, parte de F1; não exige método novo do adaptador Windows. `capture.rs` não tem cola da F3.

A bateria abaixo rodou no working tree que ainda contém F3, inclusive seus testes locais. Isso **não** aprova F3 nem equivale a compilar a árvore final sem ela. A seleção deve restaurar os quatro arquivos rastreados da F3 ao HEAD e excluir `tests/s14_hook_watch.rs`, além de não incluir material paralelo de P2/P3. A verificação da cópia exportada anunciada pelo Maestro e o CI do commit final continuam necessários; não fiz essa exportação nem alterei o index/working tree para simular a seleção.

### 6. Bateria própria e alcance

Execução sequencial, com `PATH=$HOME/.cargo/bin:$PATH`, `TMPDIR=$HOME/.cache/cs-r14` e `-j2` nos testes/clippy. Nenhum relatório ou target foi guardado em `/tmp`; nenhuma compilação concorrente foi encerrada. Todos os comandos abaixo terminaram com **exit 0**:

| Comando | Resultado observado |
| --- | --- |
| `cargo fmt --all -- --check` | Saída vazia. |
| `cargo test -j2 -p clipsync-input` | **392 aprovados, 0 falhas, 2 ignorados**. Ignorados: `portal_loopback` e `portal_loopback_from_layout`, ambos requerem sessão gráfica. |
| `cargo test -j2 -p clipsync-ui --locked` | **206 aprovados, 0 falhas, 0 ignorados**: 96 unitários e 110 de integração. Inclui a regressão por IPC e os 11 de diagnóstico. |
| `cargo test -j2 -p clipsync-app --locked` | **28 aprovados, 0 falhas**. |
| `cargo test -j2 -p clipsync-core --locked` | **120 principais aprovados, 0 falhas**, mais **5 execuções auxiliares aprovadas** em subprocessos, separadas do total principal. Inclui preparação de inventário; a suíte terminou em 242,05 s. |
| `cargo test -j2 -p clipsync-proto --locked` | **85 aprovados, 0 falhas**. |
| `cargo clippy -j2 --workspace --all-targets --locked -- -D warnings` | Sem warnings/erros; `Finished dev profile ... in 11.43s`. |
| `cargo clippy -j2 -p clipsync-app --all-targets --locked -- -D warnings` | Sem warnings/erros; `Finished dev profile ... in 19.96s`. |
| `scripts/conferir-privacidade.sh` | `privacidade dos hooks: ok (8 arquivos)`. |
| Comparadores extraídos de R13/R14 e retenção atual | Exit 0; R13 = 0 e R14 = 1 para nova ocorrência com frase igual; colisão entre instâncias permanece demonstrada, conforme limite. |
| `git diff --check` | Saída vazia, exit 0. |

**Total: 831 testes principais aprovados + 5 execuções auxiliares aprovadas; zero falhas e dois ignorados.** Não somei novamente ensaios focais nem os logs do autor. A falha de socket com TMPDIR longo relatada por Rivet não ocorreu nesta bateria de caminho curto. Logs, `resultados.json`, manifesto e saídas do harness ficam em `~/.cache/cs-r14/`. O app compilou/testou localmente; não abri janela, notificante, portal, servidor dev, sonda ou sessão física. Não executei Windows nativo, CI remoto ou instaladores.

### 7. Limites que acompanham o commit e a publicação

1. **A1 é aviso posterior à devolução.** Consulta nominal de 1 s mais trabalho local, sem teto físico garantido; não bloqueia teclas nem toma foco. Preferência, SO/modo de concentração, falha do notificante e app encerrado podem impedir apresentação. Não anunciar proteção contra digitação na janela errada, aviso simultâneo ou entrega de toda ocorrência. S2-PRIV-01 fica parcialmente mitigado, sem aceite físico.
2. **Amostragem e ciclo de vida.** Episódios já recuperados antes da consulta não geram alerta atrasado; múltiplos episódios podem resultar em um aviso. Primeiro snapshot é baseline. Contador por instância pode colidir após reinício/recriação com UI viva sem amostra limpa, como demonstrado acima. Esses limites não são ocultados pela aprovação de R13-A1-01 na mesma instância.
3. **Avisos em série continuam possíveis.** Não há contenção temporal própria; preservar essa ressalva em contrato/notas conforme a decisão do Maestro. O texto permanece fechado, sem ação/link/conteúdo livre do par.
4. **F1/F2/N1/L1 conservam os limites anteriores.** Teclas podem ser descartadas indefinidamente sem replay enquanto mouse/Stats continuam; aviso de teclado é local ao destino. Soltura remota e aplicação no compositor não são garantidas pelos testes. O prazo de 2 s é lógico, não teto físico sob qualquer falha/carga. N1 não saneia pareamentos antigos sobrescritos e não torna o reencontro instantâneo; resolução pode usar DNS. L1 não libera texto/erro bruto, e os contadores/timestamps continuam metadados de atividade.
5. **Publicar somente a seleção aprovada.** F3 fora; versão/tag coerentes; cópia exportada sem F3 verificada e CI Linux/Windows, pacotes e hashes do mesmo commit final aprovados. A autorização de release informada pelo Maestro não transforma os testes locais em prova desses gates. Não foi demonstrada causa/correção física do incidente, apresentação no Windows minimizado, suspensão, rede real ou travessia entre as máquinas.

**Identificação:** SHA-256 do delta R2 `11afa284b52fa30a851929b4ee7f5d720c87f9b2476c36fb5cc859403d1b0ea7`; os 20 fontes conferidos permaneceram estáveis durante a revisão. `Cargo.toml` e `Cargo.lock` não mudaram nesta rodada. Só esta seção foi acrescentada pelo revisor; nenhum fonte/teste, configuração, nota compartilhada, commit ou push foi alterado.
