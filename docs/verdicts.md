# Vereditos e achados (dono: Argus · warden)

## Verificações planejadas

Responsável: Argus · warden. Estado em 2026-10-27: **planejado; nenhum comando de validação do scaffold executado nesta publicação**. Fonte de verdade: [contrato.md](contrato.md), v1 de Keel, em especial §§1–9; posse e ordem em [workboard.md](workboard.md). Adaptação do roteiro anterior `verificacoes-planejadas.md` aos cinco crates e à CLI efetiva do contrato. Estes comandos são planos, não saídas observadas nem aprovação.

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

Todos os blocos são executados na raiz, após carregar o ambiente Cargo:

```sh
cd /home/kappxs/Documentos/ANTIGOS/clipsync
source ~/.cargo/env
date --iso-8601=seconds
sha256sum docs/contrato.md docs/workboard.md
rustc --version
cargo --version
rustup show active-toolchain
cargo metadata --locked --no-deps --format-version 1
cargo test --workspace -- --list
```

Conferir `clipsync-proto`, `clipsync-core`, `clipsync-cli`, `clipsync-clipboard` e `clipsync-input`, binário `clipsync`, Rust 1.99.1 e os testes existentes. Não executar cedo: esta preparação acompanha a revisão enviada pelo Maestro. Usar `target/` compartilhado nos gates normais; não mudar configuração versionada de target-dir. Os comandos da §9 abaixo são mantidos sem acrescentar `--locked`; o inventário e a prova fria adicional verificam também o lockfile.

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
  export CARGO_TARGET_DIR="$(mktemp -d /tmp/clipsync-argus-target.XXXXXX)"
  CLIPSYNC_INSTALL_DIR="$(mktemp -d /tmp/clipsync-argus-install.XXXXXX)"
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
### S0 · 2026-10-27 12:04:33–12:04:36 -03:00 · Não passa no checkout atual

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


## Findings
### S0 · achados do checkout revisado em 2026-10-27 12:04 -03:00

1. **P1 · `crates/clipsync-proto/src/lib.rs:16`** declara `pub mod testing` com feature testing, mas o arquivo `testing.rs`/`testing/mod.rs` estava ausente. A dev-dependency da própria crate em `crates/clipsync-proto/Cargo.toml:26` habilita a feature. `cargo test --workspace` e `cargo clippy --workspace --all-targets -- -D warnings` retornaram 101 com E0583; nenhum smoke foi executado. Bloqueia critérios S0.3/S0.4 e a revisão dos módulos dependentes. Posse atual: Rivet/S1; sem atribuição ao commit S0 informado, que não foi inspecionado.
2. **P2 · `crates/clipsync-clipboard/src/adapter/fake.rs:2` e demais locais na saída de fmt** divergem de rustfmt (primeiro exemplo: ordenação do import de `mono_us`/`MIME_TEXT_UTF8`). `cargo fmt --all --check` retornou 1 e também informou o módulo testing ausente. Bloqueia S0.4 no checkout compartilhado; diferenças de clipboard pertencem à fatia S2 ainda em trabalho. Não rodei fmt que escreve arquivos.

Achados são observações do estado/hash revisado; não constituem pedido para interromper builders ou corrigir código de outro dono.

