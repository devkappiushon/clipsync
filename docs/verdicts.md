# Vereditos e achados (dono: Argus · warden)

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
cd /home/kappxs/.cache/clipsync-verdict/src
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

Diretório: `/home/kappxs/Documentos/ANTIGOS/clipsync`. Cada comando abaixo foi executado em zsh após `source ~/.cargo/env`. Hashes dos arquivos antes/depois dos 13 comandos foram idênticos; não houve mudança detectada durante essa janela. Os cinco crates têm `[lints] workspace = true`; as 25 dependências de workspace têm versões iniciadas por `=`. Manifestos conferidos por leitura e parse de TOML. Código-fonte não foi editado.

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
1.99.1-x86_64-unknown-linux-gnu (overridden by '/home/kappxs/Documentos/ANTIGOS/clipsync/rust-toolchain.toml')
```

Comando: `cargo build --workspace`. Início: `2026-10-27T12:04:33.621930-03:00`. Exit code: **0**.

stderr:

```text
   Compiling clipsync-proto v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard)
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
   Compiling clipsync-cli v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.21s
```

Comando: `cargo test --workspace`. Início: `2026-10-27T12:04:34.877864-03:00`. Exit code: **101**.

stderr:

```text
   Compiling clipsync-proto v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-proto)
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
    Checking clipsync-proto v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-proto)
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
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:1:
 use super::{ChangeEvent, ClipError, ClipSnapshot, ClipboardAdapter, WatchMode};
-use clipsync_proto::{mono_us, MIME_TEXT_UTF8};
+use clipsync_proto::{MIME_TEXT_UTF8, mono_us};
 use std::collections::VecDeque;
```

Locais reportados por rustfmt (extraídos de stdout, sem inventar diferenças):

```text
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:1:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:25:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:43:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:51:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:95:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:102:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/fake.rs:106:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/mod.rs:30:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:1:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:8:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:11:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:42:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:49:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:61:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:69:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:74:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:133:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:140:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/adapter/x11.rs:151:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/config.rs:1:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/config.rs:33:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/config.rs:40:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/config.rs:48:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:9:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:16:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:22:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:30:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:43:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:56:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:63:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:67:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:79:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/diag.rs:98:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/ledger.rs:33:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/ledger.rs:47:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/ledger.rs:54:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/lib.rs:6:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/msg.rs:14:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/probe.rs:1:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/probe.rs:41:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/probe.rs:54:
Diff in /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-clipboard/src/probe.rs:78:
```

stderr integral:

```text
Error writing files: failed to resolve mod `testing`: /home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-proto/src/testing.rs does not exist
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
│       └── clipsync-core v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-core)
│           └── clipsync-cli v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-cli)
├── rcgen v0.14.10
│   └── clipsync-core v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-core) (*)
├── rustls v0.23.46
│   ├── clipsync-core v0.1.0 (/home/kappxs/Documentos/ANTIGOS/clipsync/crates/clipsync-core) (*)
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

Base local: export limpo **e611a88**, informado pelo Maestro, em `/home/kappxs/.cache/clipsync-verdict/src`; `CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target`, `source ~/.cargo/env` antes de cada comando. Execução 12:15:05–12:16:07 -03:00. Todos os arquivos inventariados permaneceram idênticos antes/depois e na conferência posterior. Nenhum git foi executado; o hash de commit é a identificação fornecida pelo Maestro, os SHA-256 abaixo são a evidência local.

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
1.99.1-x86_64-unknown-linux-gnu (overridden by '/home/kappxs/.cache/clipsync-verdict/src/rust-toolchain.toml')
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
     Running unittests src/main.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync-be9f8c6ccda39f75)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-ed14c5d2789ba84f)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-92d4afbd93db2a46)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-6d4b6b899601d3c6)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_input-e5c8f10e63c27e5a)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-6f0702f4c653dfde)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
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
│       └── clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
│           └── clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
├── rcgen v0.14.10
│   └── clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core) (*)
├── rustls v0.23.46
│   ├── clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core) (*)
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

Logs integrais e inventários: `/home/kappxs/.cache/clipsync-verdict/argus-e611a88-3609dhci/{meta,results,before,after}.json`.

### S2 · v1.3 · 2026-10-27T12:20:54-03:00 · Não passa

Base: export **e611a88** em `/home/kappxs/.cache/clipsync-verdict/src`, cache em `/home/kappxs/.cache/clipsync-verdict/target`. O módulo e os testes de clipboard têm os mesmos hashes da entrega final 18a461c; contrato v1.3 desta base, incluindo regra 6 de TTL e regra 9 de Unreachable. A leitura parcial anterior foi substituída por esta revisão da versão final. Nenhuma fonte do projeto/export foi editada.

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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-d5b415483d8ba192)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-a2a6cec29a2a4f52)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-034f1adebe3f47bd)
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
rustc --edition=2024 --test --crate-name argus_s2_boundary - -o /home/kappxs/.cache/clipsync-verdict/argus-e611a88-3609dhci/boundary-tests -L dependency=/home/kappxs/.cache/clipsync-verdict/target/debug/deps --extern clipsync_clipboard=/home/kappxs/.cache/clipsync-verdict/target/debug/deps/libclipsync_clipboard-ad5d540442636c69.rlib --extern clipsync_proto=/home/kappxs/.cache/clipsync-verdict/target/debug/deps/libclipsync_proto-200667442ae0d934.rlib --extern tokio=/home/kappxs/.cache/clipsync-verdict/target/debug/deps/libtokio-f1ce68d902523d30.rlib --extern sha2=/home/kappxs/.cache/clipsync-verdict/target/debug/deps/libsha2-dd923891c50e1b65.rlib < /home/kappxs/.cache/clipsync-verdict/argus-e611a88-3609dhci/probe-stdin.txt
```

Comando executado:

```sh
/home/kappxs/.cache/clipsync-verdict/argus-e611a88-3609dhci/boundary-tests --nocapture --test-threads=1
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

Inventários antes/depois iguais; conferência posterior também sem alterações. Evidência persistente: `/home/kappxs/.cache/clipsync-verdict/argus-e611a88-3609dhci` (results.json, before.json, after.json, boundary-results.json, probe-stdin.txt e boundary-tests).

### S2 · v1.4 · 2026-10-27T12:28:01-03:00 · Passa no escopo automático

Base informada pelo Maestro: export **7923e20**, em `/home/kappxs/.cache/clipsync-verdict/src`; cache `/home/kappxs/.cache/clipsync-verdict/target`. Gates executados de 12:25:23 a 12:25:30 -03:00, após `source ~/.cargo/env`. **Substitui a reprovação de S2 v1.3 no escopo automático e encerra S2-1/S2-2.** Itens manuais S2.2–S2.4 permanecem pendentes; não há aprovação de clipboard gráfico. S4 parcial (profile/control/diag/init) não foi revisada nem aprovada neste bloco.

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.40s
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-d5b415483d8ba192)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-a2a6cec29a2a4f52)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-034f1adebe3f47bd)
   Doc-tests clipsync_clipboard
```

Comando: `cargo clippy -p clipsync-clipboard --all-targets -- -D warnings`; início `2026-10-27T12:25:29.582489-03:00`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.71s
```

Comando: `cargo fmt -p clipsync-clipboard --check`; início `2026-10-27T12:25:30.342140-03:00`; exit **0**.

stdout/stderr vazios.

#### Reexecução dos casos originais

Compilação por stdin (exit 0), texto original preservado em `/home/kappxs/.cache/clipsync-verdict/argus-7923e20-3po5__ky/probe-stdin.txt`; comando completo e bibliotecas em `boundary-results.json`. O harness inclui também dois testes unitários do novo ledger; cada execução abaixo seleciona exatamente o caso original, por isso três testes ficam filtrados.

```sh
/home/kappxs/.cache/clipsync-verdict/argus-7923e20-3po5__ky/boundary-tests --exact duplicate_at_capacity_must_still_be_duplicate --nocapture
```

Exit **0**.

```text

running 1 test
capacity=1024 duplicate_accepted_as_new=false
test duplicate_at_capacity_must_still_be_duplicate ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

```

```sh
/home/kappxs/.cache/clipsync-verdict/argus-7923e20-3po5__ky/boundary-tests --exact pause_during_prewrite_read_must_prevent_write --nocapture
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

Evidência persistente: `/home/kappxs/.cache/clipsync-verdict/argus-7923e20-3po5__ky` (meta/results/before/after/boundary-results.json, probe-stdin.txt e boundary-tests).

### S3 · Vector · 2026-10-27T12:35:27-03:00 · Não passa

Base informada pelo Maestro: export **3eaa730**, em `/home/kappxs/.cache/clipsync-verdict/src`; `CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target`. §9 S3 itens 1 e 5 executados de **12:28:30 a 12:28:49 -03:00**, após `source ~/.cargo/env`. Os critérios S3 do contrato no export são idênticos aos do contrato atual no repositório (workboard v1.8). Nenhum arquivo do export mudou entre o inventário inicial, os gates e as reproduções, inclusive Cargo.lock.

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling tempfile v3.27.0
   Compiling ashpd v0.13.13
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 10.97s
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_input-17510809f8dddc56)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-6791af65086b93b6)
     Running tests/portal_loopback.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/portal_loopback-637131c0ec0c5ce4)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-3665d3e3b17d8b56)
     Running tests/units.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/units-1b23caa983c13b1f)
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
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking tempfile v3.27.0
    Checking ashpd v0.13.13
    Checking clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.13s
```

Comando: `cargo fmt -p clipsync-input --check`; início `2026-10-27T12:28:49.148611-03:00`; exit **0**.

stdout/stderr vazios.

#### Reproduções independentes, fora dos fontes

Harness compilado por stdin contra as bibliotecas produzidas pelo gate; texto preservado como `s3-probe-stdin.txt` e comando completo em `s3-boundary-results.json`. SHA-256 do stdin: `4e68bae1e65b6a61f23df78b63c0457e707ef9d3f88dc246cd76af4aafd3836b`. Não foi acrescentado nem editado arquivo .rs no repositório/export. O primeiro comando de compilação do harness escolheu um artefato Tokio com features diferentes e falhou; após selecionar as dependências exatas do fingerprint de clipsync-input, compilou com exit 0. O histórico completo está no JSON. Os quatro warnings de helpers não usados são só do harness; o Clippy do crate permanece verde.

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr/s3-boundary-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **101**.

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

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr/s3-boundary-tests --exact hash_zero_monitor_must_be_zone_index_zero --nocapture`; exit **101**.

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

Evidência persistente: `/home/kappxs/.cache/clipsync-verdict/argus-3eaa730-7l7l0kyr` (meta/results/before/after.json, s3-probe-stdin.txt, s3-boundary-results.json e s3-boundary-tests). Argus não aplicou correções e não executou git.

### S3 · v1.1 · 2026-10-27T12:42:05-03:00 · Passa no escopo automático

Base informada: **30dcd43**, export `/home/kappxs/.cache/clipsync-verdict/src`, cache `/home/kappxs/.cache/clipsync-verdict/target`. **Encerra S3-1 e S3-2**; substitui a reprovação de 3eaa730 no escopo automático. Gates oficiais: **18 aprovados, 0 falhos, 1 ignorado** (portal_loopback); clippy/fmt saem 0. Inventário antes/depois idêntico. Itens gráficos §9 S3.2/3/4/6/7 continuam pendentes do Maestro.

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.80s
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_input-17510809f8dddc56)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-6791af65086b93b6)
     Running tests/portal_loopback.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/portal_loopback-637131c0ec0c5ce4)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-3665d3e3b17d8b56)
     Running tests/units.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/units-1b23caa983c13b1f)
   Doc-tests clipsync_input
```

Comando: `cargo clippy -p clipsync-input --all-targets -- -D warnings`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.27s
```

Comando: `cargo fmt -p clipsync-input --check`; exit **0**.

stdout/stderr vazios.

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-original-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **101**.

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

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-original-tests --exact hash_zero_monitor_must_be_zone_index_zero --nocapture`; exit **0**.

stdout:

```text

running 1 test
configured_monitor=#0 adapter_zone=Index(0)
test hash_zero_monitor_must_be_zone_index_zero ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

```

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s3-bucket-tests --exact rate_budget_must_survive_new_session_from_same_peer --nocapture`; exit **0**.

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

Evidências completas e hashes do export: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s3-results.json, s3-boundary-results.json, before.json/after-s3.json).

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.02s
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-4b49005cfb6ac050)
     Running tests/authorize.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/authorize-951137c2be6334f2)
     Running tests/ids.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/ids-13024bae7b84b7f7)
     Running tests/memhub.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/memhub-7439a3a7f369aeb3)
     Running tests/redact.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/redact-567bd45310785f8c)
     Running tests/wire.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/wire-e2699de60d8c4ef6)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling tempfile v3.27.0
   Compiling quinn-proto v0.11.19
   Compiling quinn v0.11.12
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.37s
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
RTT fluxo 1 frame: p50 519.715µs p95 629.432µs · datagrama 4 eventos: p50 177.177µs p95 405.839µs (300 de 300)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.93s
```

Comando: `cargo fmt --all --check`; exit **0**.

stdout/stderr vazios.

Comando: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/p2-exact-tests --nocapture`; exit **0**.

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

Evidência integral: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s1-results.json, p2-exact-results.json, p2-exact-stdin.txt e inventários).

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

**Códigos observados:** 0 em help/diagnose/init; 1 em módulos vazios, stdin vazio e probe sem DISPLAY; 2 em --listen removido e send sem --stdin; 3 na recusa de pareamento, send com destino pausado/desligado (asserções do roteiro e log manual); 4 em status/pause sem agente. Init com resposta vazia de clipboard grava desligado; `stat` observou diretório 700 e chave 600. Perfis de revisão novos ficaram em `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles`, sem usar os perfis do Maestro.

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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
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
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-8a5c11d448b87ec2)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-e544e2ce5ee9f983)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-d94961f944997205)
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
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc5f117a17034f5d)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.17s
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
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
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
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
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-b1ac902e50280f98)
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

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync run --help`; exit **0**.

stdout:

```text
Sobe o agente em primeiro plano (Ctrl-C encerra)

Usage: clipsync run [OPTIONS]

Options:
      --profile <DIR>  Pasta do perfil (padrão: $CLIPSYNC_PROFILE, depois ~/.config/clipsync)
  -h, --help           Print help
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync diagnose`; exit **0**.

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

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync init --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a` · stdin nome + resposta vazia de clipboard; exit **0**.

stdout:

```text
Módulos do ClipSync (nada é transmitido agora, e cada permissão é concedida depois, no pareamento):
  • input: este computador pode enviar teclado e mouse a um par, e receber os de um par que você autorizar.
  • clipboard: textos copiados aqui podem ser enviados a pares autorizados, e os deles chegam aqui.
Nome deste dispositivo [kappxs-Aspire-A515-57]: Ligar o módulo de clipboard? [s/N] Perfil criado em /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a
  dispositivo: argus-fixture
  id: 03fd-b3c5-bb4c-a2d5
  escuta: 0.0.0.0:47800
  input: ligado · clipboard: desligado
Nada foi transmitido.
Próximo passo: `clipsync pair host` neste computador e `clipsync pair join ENDEREÇO` no outro.
```

Comando: `stat -c '%a %n' /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a/identity.key.pem`; exit **0**.

stdout:

```text
700 /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a
600 /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a/identity.key.pem
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync status --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **4**.

stdout:

```text
argus-fixture · escuta 0.0.0.0:47800 · input ligado · clipboard desligado · agente parado (suba com `clipsync run`)
Pares:
  nenhum par pareado
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a pause`; exit **4**.

stderr:

```text
erro: agente parado (suba com `clipsync run`)
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync init --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/off --name off --modules ''`; exit **1**.

stderr:

```text
erro: pelo menos um módulo precisa estar ligado
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync run --listen 127.0.0.1:0`; exit **2**.

stderr:

```text
error: unexpected argument '--listen' found

Usage: clipsync run [OPTIONS]

For more information, try '--help'.
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync send --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **2**.

stderr:

```text
error: the following required arguments were not provided:
  --stdin

Usage: clipsync send --stdin --profile <DIR>

For more information, try '--help'.
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync send --stdin --profile /home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/cli-profiles/a`; exit **1**.

stderr:

```text
erro: nada para enviar: a entrada está vazia
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync probe clipboard --write` · env -u DISPLAY; exit **1**.

stderr:

```text
erro: a ponte de clipboard exige XWayland ativo (DISPLAY)
```

Comando: `/home/kappxs/.cache/clipsync-verdict/target/debug/clipsync diagnose` · env -u DISPLAY; exit **0**.

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

Evidência integral: `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p` (s4-results.json, s4-mesh-isolated.json, s4-extra-results.json, cli-binary.sha256 e after-all.json). Nenhum fonte foi editado, nenhum git executado.

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

Evidências diretas de S2.4 e diagnose em `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/s4-extra-results.json`; evidência manual integral no arquivo do Maestro e em `/home/kappxs/.cache/clipsync-verdict/argus-30dcd43-mo2pmw7p/manual-evidence-copy.md`.

### S4 · reemissão 796e46c · 2026-10-27T12:56:09-03:00 · Passa — S4-1 e S1.6 encerrados

Base informada pelo Maestro: **796e46c**, export `/home/kappxs/.cache/clipsync-verdict/src`, `CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target`. **Substitui a reprovação de S4 em 30dcd43 e encerra S4-1 e o critério de integração S1.6.** Nenhum arquivo do export mudou durante a revisão.

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.24s
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 11.54s
     Running unittests src/main.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync-0967276870f4a82a)
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-cd5a0289063c2691)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-c72612d8116dfe30)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-832e8e82e2bb6e1a)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-312579bf9b7d9ca5)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-cc539867a6e3d6bf)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-a9645200717e2296)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-d468434fb273c2cd)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-5d9c5b98048de6d8)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_input-b172c508683df1d7)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-d213bec33e27bc54)
     Running tests/portal_loopback.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/portal_loopback-b8a062dca1ade761)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-8ffafc657e108765)
     Running tests/units.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/units-8bfc9ad83c6aad0a)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
   Doc-tests clipsync_clipboard
   Doc-tests clipsync_core
   Doc-tests clipsync_input
   Doc-tests clipsync_proto
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; duração 1.363 s; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.52s
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-669d3bb2d26a7f6c)
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

Evidência completa: `/home/kappxs/.cache/clipsync-verdict/argus-796e46c-bf8p000u` (meta/results/before/after.json). Nenhum fonte foi editado pelo Argus e nenhum comando git foi executado.

### Gates finais do MVP · c1ed65d · Passa

Executado por **Argus** de **2026-10-27T13:08:12.829380-03:00** a **2026-10-27T13:11:25.501762-03:00**, no export informado pelo Maestro como **c1ed65d**: `/home/kappxs/.cache/clipsync-verdict/src`. Ambiente: `source ~/.cargo/env`; `CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target`; toolchain **1.99.1-x86_64-unknown-linux-gnu**, rustc 1.99.1 (58cf8f9c3), cargo 1.99.1 (5f94df478).

**Todos os cinco gates solicitados saíram 0.** Os hashes de todos os arquivos do export são idênticos antes/depois, incluindo Cargo.lock. Não houve falha de espaço/cota.

| Gate | Resultado | Duração do comando |
|---|---|---|
| `cargo test --workspace` | **123 aprovados, 0 falhos, 1 ignorado** (`portal_loopback`, requer sessão gráfica). Inclui o roteiro 8/8. | 72.532 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; sem warnings. | 2.196 s |
| `cargo fmt --all --check` | Exit 0; saída vazia. | 0.185 s |
| `cargo build --release -p clipsync-cli` | Exit 0; binário otimizado gerado. | 68.793 s |
| `cargo test -p clipsync-cli --test roteiro` | **8/8 aprovados** em execução separada, 0 falhos/ignorados; testes em 43,60 s. | 48.942 s |

Os novos testes de nomes hostis/controles/bidi, socket privado e pausa quando a persistência falha também aparecem aprovados na saída do workspace. Este bloco registra os gates finais; não substitui o relatório da auditoria de segurança.

**Artefato release:** `/home/kappxs/.cache/clipsync-verdict/target/release/clipsync` · **16.396.096 bytes**.

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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 12.64s
     Running unittests src/main.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync-0967276870f4a82a)
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-cd5a0289063c2691)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-c72612d8116dfe30)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_clipboard-baed76060361b842)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-c494c8daf81d2787)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-85906dbb9d313bfa)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_core-e47edfcf9db606a7)
     Running tests/control.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/control-be77a47c4b2e6213)
     Running tests/diag.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/diag-5be713f23f06ba4b)
     Running tests/mesh.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/mesh-74da5007e3e212f2)
     Running tests/profile.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/profile-49bad18a8c67f2a8)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-fddeda6a0139483a)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_input-b172c508683df1d7)
     Running tests/module_mem.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/module_mem-d213bec33e27bc54)
     Running tests/portal_loopback.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/portal_loopback-b8a062dca1ade761)
     Running tests/smoke.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/smoke-8ffafc657e108765)
     Running tests/units.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/units-8bfc9ad83c6aad0a)
     Running unittests src/lib.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/clipsync_proto-0c66604f71ca982e)
     Running tests/authorize.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/authorize-e391bd3d15518a60)
     Running tests/ids.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/ids-dc4409c0c4cc9425)
     Running tests/memhub.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/memhub-08e08ec1e88b0447)
     Running tests/redact.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/redact-b33032db01aafc8d)
     Running tests/wire.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/wire-bc8f4451e647d888)
   Doc-tests clipsync_clipboard
   Doc-tests clipsync_core
   Doc-tests clipsync_input
   Doc-tests clipsync_proto
```

Comando: `cargo clippy --workspace --all-targets -- -D warnings`; início `2026-10-27T13:09:25.365969-03:00`; exit **0**.

stderr:

```text
    Checking clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
    Checking clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
    Checking clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
    Checking clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
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
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling quinn-proto v0.11.19
   Compiling ashpd v0.13.13
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
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
   Compiling clipsync-proto v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-proto)
   Compiling clipsync-core v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-core)
   Compiling clipsync-input v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-input)
   Compiling clipsync-clipboard v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-clipboard)
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.26s
     Running tests/roteiro.rs (/home/kappxs/.cache/clipsync-verdict/target/debug/deps/roteiro-8bb0db80bf9a908a)
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

Evidência completa: `/home/kappxs/.cache/clipsync-verdict/argus-c1ed65d-final-68hp_drk` — meta.json, results.json (stdout/stderr/código/duração de cada comando), before.json, after.json, release-binary.json e toolchain.txt.

### S5 · Windows e release · b66df03 · Falha

Revisão de Argus em 2026-10-27, iniciada às 16:39:50 -03:00. Contrato v1.12, §9 S5 e §11. Base: `b66df034996e20e4d1ce2a369a8dcb629d098c41`, HEAD no início da revisão, exportado com `git archive` para `/home/kappxs/.cache/clipsync-verdict/src`. O checkout estava limpo na exportação; avançou durante a revisão. Os **103 arquivos do export permaneceram idênticos** entre início e fim. Nenhuma fonte foi editada.

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
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
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
$ CC_x86_64_pc_windows_gnu=/tmp/claude-1000/-home-kappxs-Documentos-ANTIGOS-clipsync--maestri-roles-83bcaab5-84f6-0d02-2adf-b22d1f6ccdd4/a5cf313a-cd0b-4663-9fe4-e4e809ac819a/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo check --workspace --locked --target x86_64-pc-windows-gnu
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.56s
[exit 0; 10.608 s]
```

```text
$ CC_x86_64_pc_windows_gnu=/tmp/claude-1000/-home-kappxs-Documentos-ANTIGOS-clipsync--maestri-roles-83bcaab5-84f6-0d02-2adf-b22d1f6ccdd4/a5cf313a-cd0b-4663-9fe4-e4e809ac819a/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-gnu -- -D warnings
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.46s
[exit 0; 2.516 s]
```

```text
$ cargo build --release -p clipsync-cli --locked
   Compiling clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `release` profile [optimized] target(s) in 1m 43s
[exit 0; 103.499 s]
```

O stub de CC fornecido apenas cria arquivos de objeto vazios. Check/clippy Windows verificam Rust e cfg, sem linkar, executar testes Windows ou validar a criptografia C do ring. Não produzem um `.exe` utilizável.

Binário Linux em `/home/kappxs/.cache/clipsync-verdict/target/release/clipsync`: **8.754.744 bytes (8,35 MiB)**. `file` identificou ELF x86-64 PIE, dinamicamente ligado, **stripped**; `--version` imprimiu `clipsync 0.1.0`. SHA-256: `24711e20543df5b37e1d0bcce2911ad33a6ac60186705f1d24b41499b9c562b2`. Perfil lido do Cargo.toml: `lto = true`, `codegen-units = 1`, `strip = true`, `opt-level = 3`. Este build local em Ubuntu 26.04 não prova a glibc mínima do artefato que será gerado em ubuntu-24.04.

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

Evidência integral local: `/home/kappxs/.cache/clipsync-verdict/argus-s5-3hmvsylg` (`gates.json`, `trees.json`, `static.json`, `binary.json`, `roteiro-release.json`, reproduções e hashes before/after). SHA-256 do archive: `093886f0e0d2e4a1f75d9686eb072b8ccb91e8b22f23290e56c482c4c703bde2`.

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
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.87s
[exit 0; 2.936 s]
```

```text
$ CC_x86_64_pc_windows_gnu=/tmp/claude-1000/-home-kappxs-Documentos-ANTIGOS-clipsync--maestri-roles-83bcaab5-84f6-0d02-2adf-b22d1f6ccdd4/a5cf313a-cd0b-4663-9fe4-e4e809ac819a/scratchpad/wincc/cc AR_x86_64_pc_windows_gnu=/usr/bin/ar cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-gnu -- -D warnings
    Checking clipsync-cli v0.1.0 (/home/kappxs/.cache/clipsync-verdict/src/crates/clipsync-cli)
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

Evidência integral: `/home/kappxs/.cache/clipsync-verdict/argus-s5-adendo-p2qa07qp` (`meta.json`, `gates.json`, `reads.json`, hashes before/after). SHA-256 do archive: `8cd4518ad46272f92250ac4b070cbe0880e378ac75c8b073e058c19fee644119`.

### S6 · parcial · 5a55688

**Passa no escopo solicitado:** §12.9 item 1 (a–g), item 4 (clipboard, autostart e clippy do app no CI informado) e somente os greps do item 5. Verificado por Argus em **2026-10-28**, com contrato v1.15. **Não encerra S6:** telas, temas, fontes, rede no portal, cenários visuais e app real (item 5 completo e item 6) não foram avaliados nesta rodada; itens 2, 3 e 7 também estão fora deste veredito.

Base local: `5a55688d19c9c55f93a87daf1c00caee819327f4`. O checkout compartilhado tinha alterações de outros agentes; os testes e greps abaixo rodaram no conteúdo de `git archive 5a55688`, exportado para `/home/kappxs/.cache/argus-s6-5a55688-tufou2ei/src`. Os **205 arquivos** do export, inclusive `Cargo.lock`, mantiveram os mesmos SHA-256 antes e depois. Archive SHA-256: `af2e1fcad93afa409c5534fba54bcd56f782807ca5df3057bc03880283dd6206`. Ambiente: Linux x86_64, rustc/cargo **1.99.1**; cache Cargo reaproveitado, sem alegação de build frio.

Preparação aplicada antes dos quatro comandos, na raiz do export:

```sh
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target
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

Evidência integral local: `/home/kappxs/.cache/argus-s6-5a55688-tufou2ei/`, com `logs/{proto,core,clipboard,autostart,ci}.log`, `logs/ci.json`, `results.json`, `greps.json`, `meta.json` e inventários SHA-256 `before.json`/`after.json`. **121 testes aprovados, zero falhos, zero ignorados; nenhum achado bloqueante no recorte solicitado.**

### S6 · critérios 2 a 4 · 415caf2

**Critério 2: Passa. Critério 3: Passa nos testes automáticos, mas não passa integralmente enquanto faltar a prova M com `layout.toml`. Critério 4: Passa nos testes locais e no último CI verde disponível, identificado abaixo; o CI de 415caf2 ainda estava em execução na consulta.** Critérios 5 (telas) e 6 (app real) ficam fora desta revisão. O critério 1 permanece na revisão anterior, commit c74f1c0.

Argus, **2026-10-28**, contrato **v1.17**, §12.9. Base exata: `415caf2d8fa235fffc669ce7b9552f24a7ab413a`, via `git archive 415caf2`, em `/home/kappxs/.cache/argus-s6-415caf2-wzp4if22/src`. Os **269 arquivos** do export, inclusive `Cargo.lock`, mantiveram os SHA-256 antes/depois; alterações concorrentes da main não entraram na revisão. SHA-256 do archive: `16b1b15b5896e99be5090cdcf65a69e666fa9080303c37ff742fdd922da4eca1`. Ambiente: Linux x86_64, rustc/cargo **1.99.1**, cache Cargo reaproveitado. Preparação aplicada na raiz do export:

```sh
export PATH=$HOME/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-verdict/target
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

Evidência integral: `/home/kappxs/.cache/argus-s6-415caf2-wzp4if22/`, com `logs/ui-{1,2,3}.log`, `logs/layout-{1,2,3}.log`, `logs/autostart-{1,2,3}.log`, logs de input/clipboard/core-revoke/CI, `results.json`, `summary.json`, `core-revoke.json`, `meta.json` e inventários `before.json`/`after.json`.

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

