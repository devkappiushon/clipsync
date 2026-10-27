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
| S1.6 · pairing/agent pela CLI | Será completado no veredito S4, próximo na fila. Não afirmar aprovação desses testes antes da execução. |

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

## Findings

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

