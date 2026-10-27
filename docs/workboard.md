# workboard · ClipSync MVP de hoje

**Estado: MVP de 2026-10-27 FECHADO.** S0 a S4 e I passam no warden (Argus). Segurança liberada, com 0 bloqueantes (`docs/seguranca.md`). **S5 (Windows e release): código entregue, e a segurança LIBEROU** (Sentinel 10777b4: 0 BLOCKER, 3 MÉDIO, 8 BAIXO). **Release v0.1.0 publicado** em 2026-10-27, a partir de c04de08. **CI verde nos dois runners** em ec8eee3 (run 40576377462). Os três critérios da S5 passam: a S5 está FECHADA. O próximo release sai de ec8eee3 ou depois. Pendências consolidadas e próxima fatia no fim deste arquivo.

**S6 (interface visual): ABERTA em 2026-10-27.** Contrato §12 (v1.14). Atribuições na seção S6 abaixo.

Contrato: `docs/contrato.md` (v1.14). Fonte de verdade são os arquivos em `docs/`, não as notas. ADRs em `docs/decisoes/`, matriz em `docs/matriz-compatibilidade.md`. Critério de pronto de cada fatia = §9 do contrato, comando por comando. Git: até a S4 só o Maestro commitava; na S5 cada builder commita os próprios arquivos na main local, sem push (contrato §11 regra 12). Ninguém sobe processo longo fora de `cargo test`. `source ~/.cargo/env` sempre. Dúvida de contrato: `maestri ask "Keel · esqueleto" "..."`.

## Ordem
1. S0 Bolt fecha primeiro (esqueleto compila). Só então S1, S2, S3 começam a editar seus crates.
2. Dentro de S1, Rivet entrega `clipsync-proto` (§4, verbatim) ANTES de tudo e marca aqui "proto pronto". Até lá Flux e Vector fazem adaptador + fake (não dependem de proto além de Edge/InputEvent, que podem esperar).
3. S2 e S3 testam módulo sobre `MemHub` (feature `testing` do proto), sem esperar o QUIC do Rivet.
4. I (integração) é do Rivet, depois de S2 e S3.

## S0 · Bolt · setup
Posse: `Cargo.toml` raiz, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `.github/workflows/ci.yml`, esqueleto inicial de `crates/*` (a posse de cada crate passa ao dono quando S0 fecha).
Faz: workspace edition 2024, rust-version 1.99, toolchain 1.99.1, lints do §2, deps do §1 com `=`, 5 crates com smoke test, CI (fmt, clippy -D warnings, test).
Pronto: §9 S0 itens 1 a 7.
Depois de S0: plantão para adicionar dep aprovada pelo Keel.
Estado: PASSA no warden (commit 79be293)

## S1 · Rivet · base
Posse (v1.6): `crates/clipsync-proto/**`; no core: `Cargo.toml`, `src/{lib,tls,pairing,transport,agent,msg}.rs`, `tests/{smoke,tls,pairing,mesh}.rs`.
PRIMEIRO (até 15 min, destrava o Flux): lib.rs com todos os `pub mod` e o `CoreError` da §5; `tls::pin_of`; deps serde_json e async-trait no Cargo.toml do core (quinn-proto não entra hoje, v1.7).
Ordem interna: (1) proto + MemHub + authorize_* + CloseCode + PairMsg; avisar "proto pronto" aqui. (2) perfil, config, identidade, peers. (3) PinVerifier, retomada TLS desligada, pareamento com compromisso nB/nA. (4) malha QUIC, Hello nos dois sentidos, agente, socket de controle, CLI. (5) diag de sistema.
Pronto: §9 S1 itens 1 a 7 (renumerados na v1.6).
Estado: PASSA no warden · segurança liberada (S1-01, 02, 08 e 09 corrigidos) · proto com 40 testes, extra `CloseCode::ALL`, `DeliveryState::Unreachable` no commit 1c6ba06

## S2 · Flux · clipboard
Posse: `crates/clipsync-clipboard/**`.
Faz: trait ClipboardAdapter (§7), FakeClipboard (eco, owner_takeover, clear), X11Clipboard (x11rb XFixes + arboard), módulo (livro de escritas, conteúdo inalterado, Lamport, caixa, acks, pausa), probe --watch/--write, diagnose (DISPLAY).
Pronto: §9 S2 itens 1 a 5.
Estado: PASSA no warden · Unreachable em uso (commit 18a461c)

## S3 · Vector · input
Posse: `crates/clipsync-input/**`.
Faz: traits CaptureAdapter/EmulateAdapter (§8), FakeCapture, FakeEmulate, PortalCapture (ashpd InputCapture + reis, barreira por segmento), PortalEmulate (ashpd RemoteDesktop + reis), módulo (loopback, stats p50/p95, limite de taxa, release_all), probe --capture/--emulate, print_loopback, diagnose.
Pronto: §9 S3 itens 1 a 7.
Estado: PASSA no warden · itens manuais do roteiro §14 item 5 pendentes do usuário (ver pendências)

## S4 · Flux · perfil, controle, diag e cli (nova em v1.6)
Posse: no core, `src/{profile,control,diag}.rs` e `tests/{profile,control}.rs`; o crate `crates/clipsync-cli/**` inteiro.
Depende do Rivet: lib.rs com `pub mod` + `CoreError` + `tls::pin_of` (primeira entrega dele). Os testes pair_/mesh_ do cli dependem de pairing e agent.
Pronto: §9 S4 itens 1 a 8.
Estado: PASSA no warden (após reemissão)

## I · Flux · integração (desde v1.6)
Pronto: §9 I itens 1 a 4.
Estado: PASSA no warden

## S5 · Windows e release (v1.11 a v1.13; FECHADA em 2026-10-27: release em c04de08, CI verde em ec8eee3)
Regras: contrato §11. Pronto: §9 S5 itens 1 a 10. Os três critérios do Maestro são:
- (A) checagem de compilação Windows verde no workspace (itens 1 a 7);
- (B) CI verde nos dois runners (item 8);
- (C) release v0.1.0 com dois artefatos e SHA256SUMS (itens 9 e 10).

Verificação local: `export PATH=$HOME/.cargo/bin:$PATH`, depois `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` (alvo Windows, sem linkar) e `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` (Linux). Nada de `target/` em /tmp.

- **Rivet · core e proto no Windows.** Posse: `crates/clipsync-proto/**` e o `crates/clipsync-core/**` inteiro. Na S5, profile, control e diag voltam ao Rivet. Faz:
  - named pipe só local (`first_pipe_instance`, `reject_remote_clients`), com nome sorteado em `DIR\agent.pipe`;
  - perfil em `%LOCALAPPDATA%\clipsync`, protegido pela ACL, sem chmod;
  - Hello anuncia só os módulos que rodam;
  - testes de control e mesh verdes no Windows.

  Pronto: itens 1 a 4. Estado: entregue. Commits:
  - 661bea9: primeira versão;
  - b66df03: nome sorteado a cada agente, publicado em `DIR\agent.pipe`, e perfil padrão em `%LOCALAPPDATA%`.
- **Flux · clipboard e cli no Windows.** Posse: `crates/clipsync-clipboard/**` e `crates/clipsync-cli/**`. Faz:
  - adaptador `win-poll` (arboard, 250 ms, só texto), experimental;
  - init e run fora do Linux sem input;
  - teste do watcher e da escolha de módulos.

  Pronto: itens 1, 2, 5 e 7. Estado: entregue. Commits:
  - f4daff4: clipboard e cli compilam no Windows;
  - 36568a7: do Vector, compartilhado;
  - b81b08b (S5-02): texto recebido escrito com `exclude_from_history` e `exclude_from_cloud`;
  - 707c780: §11 regra 8. O `init` sem módulo utilizável sai 3 sem gravar, e o `run` sem módulo sai 1 com `RUN_NO_MODULE`. Testes puros por plataforma.
- **Vector · input indisponível no Windows.** Posse: `crates/clipsync-input/**`. Pronto: itens 1, 3 e 6. Estado: entregue em b4f472a e 36568a7. Fora do Linux, o `probe input` sai 3 (v1.13).
- **Bolt · release, CI e README.** Posse: `Cargo.toml` raiz, `Cargo.lock`, `.github/workflows/{ci,release}.yml` e `README.md`. Faz:
  - `[profile.release]` com LTO e strip (já feito);
  - `release.yml` por tag `v*`, sem cache, só `actions/*`, CRT estática no Windows, `ubuntu-24.04` e conferência tag = versão;
  - job Windows do CI com clippy e `test --workspace`;
  - quadro do README com clipboard Windows "experimental".

  Pronto: itens 1, 8, 9 e 10. Estado: entregue. Commits:
  - 5010629: Bolt;
  - 176ac4e e 9da97d9: Maestro, porque o Bolt estourou a cota do modelo. O 9da97d9 traz os avisos de Windows no README e `permissions` e `persist-credentials: false` nos workflows.

  O item 9 passa na conferência estática do Keel em 2026-10-27:
  - `rust-cache` 0;
  - só `actions/*`;
  - crt-static no Windows;
  - `ubuntu-24.04`;
  - passo tag = versão;
  - YAML válido.

  O job Windows do CI roda clippy `-D warnings` e `test --workspace --locked`. Resultado dos itens 8 e 10: ver "Resultado do push" abaixo.
- **Sentinel · segurança da S5:** LIBERADO para a tag v0.1.0 em 10777b4 (`docs/seguranca.md`, seção S5). 0 BLOCKER, 3 MÉDIO e 8 BAIXO. O S5-02 já foi corrigido em b81b08b; os demais estão em "Pendências pós-MVP → S5".
- **Resultado do push (2026-10-27, commit c04de08, relato do Maestro):**
  - **Critério C, release: PASSA.**
    - Workflow `release` run 40573132036 verde. O release v0.1.0 traz os dois artefatos e o `SHA256SUMS`.
    - `clipsync-v0.1.0-linux-x86_64.tar.gz`: 3.988.409 bytes. O binário tem 8,3 MB (strip + LTO) e roda neste notebook: `clipsync --version` dá `clipsync 0.1.0`.
    - `clipsync-v0.1.0-windows-x86_64.zip`: 2.672.545 bytes. O `clipsync.exe` tem 6.089.216 bytes, PE32+ x86-64.
    - `sha256sum -c SHA256SUMS` deu OK nos dois: tar.gz `6d641b52…` e zip `03acc283…`.
  - **Critério B, CI nos dois runners: PASSA** (2026-10-27, relato do Maestro).
    - CI run 40576377462 no HEAD ec8eee3. O job Linux ficou verde. O job Windows também: clippy, build e 127 testes passando, 0 falhas.
    - É a primeira execução completa de core, proto, clipboard e cli no Windows, incluindo mesh com QUIC e o named pipe.
    - Caminho até lá:
      - run 40573127883: 1 falha de corrida no teste do pipe (`ERROR_PIPE_BUSY`), corrigida em e4d8ab1;
      - run 40574717785: 1 falha porque o relógio por processo devolvia 0 na primeira chamada, corrigida em ec8eee3;
      - 38814c9 pôs `--no-fail-fast` no job Windows.
  - **O release v0.1.0 continua o de c04de08. O próximo release leva ec8eee3.**
    - Depois de c04de08, o binário só mudou no `mono_us()` do Windows (proto `time.rs`), que nunca mais devolve 0.
    - No binário 0.1.0, o 0 só podia sair na primeira chamada do processo, e nenhum código de produção decide por esse valor. O Keel conferiu por grep em 2026-10-27: no clipboard, `mono_us()` só alimenta `ChangeEvent.t_mono_us` (`poll.rs`, `x11.rs`, `fake.rs`) e o texto sintético do probe. O `module.rs` não lê `t_mono_us`. O input, único leitor de latência, não roda no Windows.
    - Por isso o comportamento do 0.1.0 no Windows não muda. Os outros commits (e4d8ab1, 38814c9) só tocam teste e CI.
  - O tag publicou com o job Windows do CI vermelho. É o caso que o S5-08 descreve: o ruleset de tag `v*` deve exigir CI verde antes de publicar.
- **Licença (v1.12, decisão do Maestro):** a 0.1.0 sai num repositório privado, sem LICENSE; o README segue com "a definir". A licença passa a ser pré-condição do primeiro release público (pendência 1). Push e tag são só do usuário.

## S6 · interface visual (v1.14; ABERTA em 2026-10-27)
Regras, comandos, eventos e critérios: contrato §12. Critérios do Maestro: (a) `cargo test -p clipsync-ui` com os 7 fluxos; (b) 9 telas no portal com o mock, conferidas contra o Figma; (c) AppImage do CI com o agente real; (d) bandeja e entrada na barra de aplicações; (e) release v0.2.0; (f) Sentinel liberado.

Verificação local: `export PATH=$HOME/.cargo/bin:$PATH`, depois `cargo clippy --all-targets -- -D warnings && cargo test` SEM `--workspace` (o `clipsync-app` fica fora do `default-members` e só compila no CI).

Ordem: o Rivet entrega o proto da §12.6 primeiro e marca aqui "proto S6 pronto". O resto anda em paralelo; o Pixel começa já, com o mock.

- **Rivet · clipsync-ui, proto e core.** Posse: `crates/clipsync-ui/src/{lib,api,agent,status,onboarding,devices,pairing,clipboard,diag,prefs,events,msg}.rs`, `tests/fixtures/`, `crates/clipsync-proto/**`, `crates/clipsync-core/**`. Faz:
  - proto da §12.6 (variantes no fim, `PROTOCOL_VERSION = 2`, mínimo 1, `RemoteTooOld`, `recent_blocks`);
  - core: `init_with` (L1), `recent_blocks` (L6), versão do par no `PeerInfo`, gravação do `[clipboard] mode`;
  - `Ui` com `start`, `call`, `subscribe` e `shutdown`, os comandos das telas 1, 2, 3, 5, 6 e 7, os eventos e o `plan` + `build` duplicados (dívida).

  Pronto: §12.9 itens 1 e 2. Estado: em progresso (clipsync-ui, etapa 1) · proto S6: pronto (06815b7)
- **Vector · layout e input.** Posse: `crates/clipsync-ui/src/{layout,input}.rs` e `crates/clipsync-input/**`. Faz: `layout.toml`, comandos da tela 4, `InputState`, `InputReload`, `InputLock`, `InputSwitch`, `InputOffer`, `Zones`, `EnterAt`, captura contínua guiada pelas ligações, `self_test` do input (L7). Pronto: §12.9 item 3. Estado: a fazer
- **Flux · casca Tauri e clipboard.** Posse: `crates/clipsync-app/**` menos `ui/`, `crates/clipsync-ui/src/autostart.rs`, `crates/clipsync-clipboard/**` e `crates/clipsync-cli/**`. Faz: `tauri.conf.json`, capabilities, comando `clipsync`, laço de eventos, bandeja, fechar = esconder, Sair com Bye, ícones, autostart; no clipboard, `ClipApply`, `ClipDiscard`, `ClipSendCurrent` e `self_test` (L5, L7). Pronto: §12.9 item 4. Estado: a fazer
- **Pixel · frontend.** Posse: `crates/clipsync-app/ui/**`. Faz: as 9 telas (onboarding em 3 passos mais 6), temas claro e escuro, fontes locais OFL, `mock.js` com cenários, estados de carregamento, vazio, erro e "indisponível nesta versão". Pronto: §12.9 item 5. Estado: a fazer
- **Maestro · workspace, CI, release e instalação.** Posse: `Cargo.toml` raiz, `Cargo.lock`, `.github/workflows/**`, `README.md`. Faz: `default-members`, as 4 deps aprovadas (§12.8 item 2), apt no job Linux, `version` 0.2.0, bundles no `release.yml`, instalação local com `.desktop`. Pronto: §12.9 itens 6 e 7. Estado: a fazer
- **Argus:** veredito por tela no portal (critério b) e pelo artefato (critérios c e d).
- **Sentinel:** webview e superfície `clipsync` (critério f).

## Telas (Figma)
Prontas em 2026-10-27:
- as nove telas, nos temas claro e escuro;
- os ajustes de atalho com a tecla Windows (Win+Alt);
- a malha de 16 px.

Commits 0e76039, f54c995, 8daf2f5 e 3c2bad0. A implementação (Tauri) segue depois do `apt install` (próxima fatia, item 4).

## Decisões
- 2026-10-27 · Keel · Contrato v1 publicado.
- 2026-10-27 · Keel · Spike Bolt: versões fixadas com `=`; ring em todo lugar (aws-lc exige cmake/nasm); ashpd com screencast junto de remote_desktop; target/ compartilhado no mesmo checkout.
- 2026-10-27 · Keel · Spike Rivet: PinVerifier único cliente+servidor; retomada TLS 1.3 desligada; código de pareamento com compromisso H(nB)/nA/nB, uma tentativa por janela; Connected só após Hello nos dois sentidos; CONNECTION_CLOSE só com CloseCode curto.
- 2026-10-27 · Keel · Spike Flux: observação por XFixes via XWayland (validada); livro de escritas próprias obrigatório; conteúdo inalterado não é cópia nova (cobre tomada do Mutter); vazio não sincroniza; x11rb =0.13.2 (a do arboard); adaptador renomeado para `x11`.
- 2026-10-27 · Maestro · Contrato aprovado. Fonte de verdade passa a ser `docs/contrato.md` e `docs/workboard.md`.
- 2026-10-27 · Keel · Contrato v1.1: identidade ECDSA P-256 em vez de Ed25519. Motivos: P-256 foi o provado ponta a ponta no spike do Rivet, e é o que TPM, CNG e Secure Enclave guardam em hardware (cofre futuro). Afeta só S1 (Rivet). ADR 0003.
- 2026-10-27 · Keel · ADRs 0001 (stack), 0002 (transporte QUIC) e 0003 (pareamento) e a matriz de compatibilidade publicadas em `docs/`.
- 2026-10-27 · Keel · Contrato v1.2, seis decisões do Rivet no proto ACEITAS sem veto: (1) sha2 no proto para PeerId::from_spki_der; (2) `IdError { Hex, Length }`; (3) SendError com #[error] curto em inglês, texto pt no msg.rs de quem exibe; (4) ordem de autorização: saída LocalModuleDisabled > RemoteModuleDisabled > NotAuthorized > Paused, entrada ModuleDisabled > NotAuthorized > Paused; Ack/Stats/StatsRequest/Leave/ReleaseAll ignoram pausa e permissão mas exigem módulo local ligado; (5) MemHub::link entrega PeerUp aos dois lados, recusa vira ModuleInput::Rejected no remetente, sent_count só conta entregues; (6) validate: device_name 1..=64 bytes, Hello com min_version <= version.
- 2026-10-27 · Keel · Contrato v1.3, decisões do Flux (S2): (1) VETADA: par alvo não pode sumir do ClipSent; motivo: falha precisa ser identificável e não pode parecer sucesso (anexo §8), e o roteiro imprime estado por par. Troca: `DeliveryState::Unreachable` (proto, Rivet), ClipSent lista todo alvo (Flux), `send` sai 3 se algum alvo não ficou Applied/InInbox (cli, Rivet). (2) ACEITA com precisão: ttl conta do recebimento local em relógio monotônico; ttl_ms == 0 expira ao chegar; item da caixa com idade > ttl expira e perde o texto. (3) ACEITA: ClipSend avança Lamport sem contar como cópia local.
- 2026-10-27 · Keel · Consequência da decisão 4 para S3 (Vector): Leave, ReleaseAll, StatsRequest e Stats só têm efeito se a session pertence ao mesmo remetente; senão são ignorados e contados (contrato §8 regra 2).
- 2026-10-27 · Keel · Contrato v1.4: `CloseCode::ALL` registrado no §4 (extra do Rivet, aceito). Spike input 2 do Vector no §8 regras 7 a 11: barreira na borda direita em x = zone.x + width (x=1919 recusado pelo portal, `cap_1919_539.log`); Enter por movimento relativo a partir de canto saturado (EIS sem absoluto); RemoteDesktop com persist_mode ExplicitlyRevoked + restore_token em `DIR/portal-tokens.toml` 0600 (dono: clipsync-input), DoNot proibido; InputCapture v1 = 1 diálogo por sessão do agente; proibido `supported_capabilities()`; P4c automatizado com até 3 tentativas. Matriz: captura passa a validada para ponteiro.
- 2026-10-27 · Keel · Contrato v1.5: dep `futures-util =0.3.34` aceita (pedido do Vector, StreamExt para streams do ashpd/reis), default-features=false, mesma versão já presente no Cargo.lock via ashpd/zbus. Bolt adiciona.
- 2026-10-27 · Keel · Corrigido: licença e regra da recópia seguem PENDENTES. O "MIT, aceito a recópia" que vi na tela do Maestro era a pergunta dele ao usuário, não resposta.
- 2026-10-27 · Maestro + Keel · Contrato v1.6, mudança de posse para fechar o MVP hoje (motivo do Maestro: 40 min depois do S0, core e cli tinham 25 linhas, e o Rivet está no caminho crítico da cripto). Flux recebe core/{profile,control,diag}.rs e o crate cli inteiro (nova fatia S4, mais a integração I). Rivet fica com core/{lib,tls,pairing,transport,agent,msg}.rs. Tipos: Identity, Config, InitOptions, Peers, PeerEntry, State e Profile em profile.rs (Flux); PairOutcome, PairPrompt e PairDecision em pairing.rs (Rivet); CoreError único em lib.rs (Rivet). Dep quinn-proto =0.11.19 aprovada (Rivet troca o reason do CONNECTION_CLOSE). `pairing::host` ganha `on_listen`.
- 2026-10-27 · Keel · Furos da divisão, fechados na §5: (a) `tls::pin_of` é a fonte única do pino, e `Identity::id()` chama essa função; (b) `CoreError::AgentAlreadyRunning`, que faltava para o control::serve; (c) `ControlHandler` + `control::serve` em control.rs (Flux), e o agente só implementa o handler; (d) core não imprime nada, confirm roda em spawn_blocking; (e) o cert usa a chamada do spike (SAN "clipsync") e não leva o nome do dispositivo; (f) S1 ganha teste de malha no core (mesh_) para não depender do cli.
- 2026-10-27 · Maestro · Contrato v1.7: hoje o Rivet NÃO embrulha quinn::crypto. O PinVerifier devolve só erros fixos do rustls (ApplicationVerificationFailure para par desconhecido ou chave trocada, UnknownIssuer para X.509 inválido), então o reason do CONNECTION_CLOSE em falha de handshake é uma string constante do rustls, sem Debug de cert nem erro interno. A regra 9 continua integral para Bye e closes de aplicação. Motivo: caminho crítico (tls, transport, pairing e agent ainda eram placeholders às 13h). Contrato §5 item 6, §9 S1 itens 3c e 5d ajustados; quinn-proto fora do Cargo.toml do core hoje.
- 2026-10-27 · Maestro · Contrato v1.8: `clipsync run` sem `--listen`. A porta vem só de `network.listen` no config.toml (o agent::run lê a config sozinho; o Flux já tinha omitido o flag). Perfil de teste b usa porta própria via `init --port`.
- 2026-10-27 · Keel · Contrato v1.9: §9 S4 item 2 passa a usar `cargo test -p clipsync-core --test control`. O filtro `control_` não pegava nada: o teste de `tests/control.rs` se chama `serve_request_roundtrip_stale_socket_and_cleanup`.
- 2026-10-27 · Keel · Contrato v1.10, auditoria `docs/seguranca.md` (veredito BLOQUEADO por S1-01). O que conta como tentativa de pareamento (§5 regra 3), proposta do Maestro aceita com três acréscimos:
  - a tentativa começa no primeiro Commit recebido, após handshake TLS em ALPN_PAIR e Hello válido;
  - antes disso nada conta (sonda, handshake falho, Hello inválido), e o host volta ao accept() até o prazo. Uma conexão pré-Commit por vez, com 5 s até o Commit (acréscimo do Keel: evita prender o slot);
  - no máximo 3 tentativas por janela; a 3ª falha fecha;
  - Reveal que não confere com o Commit fecha a janela na hora (acréscimo do Keel: prova de ataque);
  - conexões extras durante uma tentativa são recusadas (acréscimo do Keel);
  - teto do MITM: 3/10^6 por janela. ADR 0003 atualizada.

  Implementação pós-MVP (S1-04, baixa), salvo se o Rivet couber em 15 min.
- 2026-10-27 · CORRIGIDO e liberado pela segurança · Rivet/Flux · S1-01 (BLOCKER): `validate_name` única no proto recusa controle e formatação Unicode, e o cli filtra todo texto do par (§4, §2 regra 8). S1-02: pairing nunca grava nome inválido; `load_peers` põe a entrada ruim em quarentena. S1-08: pausa aplica na memória primeiro e fica pausada se não gravar (§5 regra 8). S1-09: socket criado em nome temporário + chmod + rename; apaga o socket velho só em ConnectionRefused/NotFound; `run` recusa DIR com modo diferente de 0700 (ressalva da segurança: esta última parte NÃO entrou; ver pendências).
- 2026-10-27 · Maestro · Fechamento do MVP: S0 a S4 e I passam no warden; segurança liberada com 0 bloqueantes. As pendências que estavam espalhadas aqui foram consolidadas na seção "Pendências pós-MVP".
- 2026-10-27 · Maestro · Telas do Figma prontas: as nove telas claras e escuras, os ajustes de atalho Win+Alt e a malha de 16 px.
- 2026-10-27 · Maestro · Fatia S5 · Windows e release (o usuário pediu release no GitHub com binário Linux e Windows):
  - no Windows, a 0.1.0 entrega malha QUIC, pareamento e clipboard; teclado e mouse ficam "indisponível nesta versão";
  - nada de unsafe novo;
  - named pipe só local, com `reject_remote_clients` e `first_pipe_instance`;
  - no Windows, a proteção do perfil vem da ACL do usuário, sem chmod;
  - o adaptador de clipboard do Windows é "experimental" até o teste em máquina real;
  - cada builder commita os próprios arquivos na main local, sem push.
- 2026-10-27 · Keel · Contrato v1.11, acréscimos à S5 (§11):
  - (a) a posse do core inteiro volta ao Rivet durante a S5, porque o pipe mexe em control, profile e agent juntos;
  - (b) o nome do pipe é sorteado a cada `serve` e publicado em `DIR\agent.pipe`. O namespace `\\.\pipe\` é global e listável: com nome fixo, outro usuário local criaria o pipe antes e receberia o texto do `send --stdin`;
  - (c) o perfil padrão fica em `%LOCALAPPDATA%`, não no Roaming, para a chave não viajar ao servidor de domínio;
  - (d) o Hello anuncia só os módulos que rodam;
  - (e) o release não usa cache (risco de envenenar o binário publicado), só usa `actions/*`, tem CRT estática no Windows (sem depender do VC++ Redistributable), roda em `ubuntu-24.04` fixo e confere tag = versão;
  - (f) o job Windows do CI roda clippy, único lint do código `cfg(windows)`, e `test --workspace`;
  - (g) sem dep nova; Win32 direto fica fora da 0.1.0;
  - (h) licença decidida é pré-condição do tag (substituída na v1.12: vale para o primeiro release público).
- 2026-10-27 · Maestro + Keel · Contrato v1.13, fechamento da S5:
  - (a) decisão do Maestro: fora do Linux, o `probe input` sai 3 (recusado ou indisponível), igual ao `probe clipboard`; o 1 fica para erro de execução;
  - (b) a §11 regra 8 foi cumprida em 707c780;
  - (c) a §11 regra 5 foi corrigida conforme o S5-04: o nome do pipe é público, e quem protege é a DACL. A regra registra a lacuna S5-01;
  - (d) o Keel aceitou o desenho do Sentinel para o S5-01: segredo de 16 bytes na 2ª linha de `agent.pipe`, nonce do cliente e `HMAC-SHA256` do servidor via `ring::hmac` (dep direta aprovada). Implementação depois da 0.1.0;
  - (e) a §11 regra 6 registra o S5-02 (b81b08b) e deixa o S5-03 como pendência.
- 2026-10-27 · Maestro · Acréscimos 1 a 6 da v1.11 aceitos. Licença: a 0.1.0 sai em repositório privado sem LICENSE; a regra passa a ser "licença antes do primeiro release público".
- 2026-10-27 · Keel · Contrato v1.12: o Windows entra no próprio lugar, pelos pontos que o Rivet apontou:
  - §3: perfil padrão `%LOCALAPPDATA%\clipsync` e `agent.pipe` na árvore do perfil;
  - §4: `mono_us` por processo no Windows. O Unix segue no CLOCK_MONOTONIC, com rustix só em cfg(unix);
  - §5: `resolve`, `socket_path` = `DIR\agent.pipe` e o named pipe no `serve` e no `request`.

  A licença foi ajustada em três lugares: §9 S5 item 10, §10 item 1 e §11 regra 9.

- 2026-10-27 · Maestro + Keel · Contrato v1.14, fatia S6 · interface visual (§12):
  - (a) Maestro: Tauri 2 num processo com o agente dentro; lógica em `clipsync-ui` (sem Tauri); um comando Tauri `clipsync` com `UiRequest` tipado; eventos por consulta de 1 s; UI sempre pelo socket de controle; `ui.toml` e `layout.toml` no perfil;
  - (b) Maestro: permissão inicial do pareamento tudo desligado (`docs/telas.md` corrigida); agente sobe sozinho com perfil existente; o que não tem backend aparece desabilitado com "indisponível nesta versão";
  - (c) Maestro: `layout.toml` é a fonte única do layout; `InputReload`; `set_layout_options`, `lock_to_screen` e `switch_to_device`; `plan` + `build` duplicados (dívida);
  - (d) **Keel, ajuste à decisão "variante no fim, sem bump":** o 0.1.0 fecha a conexão com `CloseCode::Protocol` quando um frame de módulo não decodifica (`transport.rs`). Variantes vão no fim, mas `PROTOCOL_VERSION` sobe para 2 com mínimo 1 (o 0.1.0 aceita), e o core só manda variante nova a par com versão 2 (`RemoteTooOld`). O monitor de entrada vai em `EnterAt`, não num campo novo do `Enter`;
  - (e) Keel: `Zones` só sai para par a quem concedi `input_in`; `EnterAt` e `LayoutOffer` só entram com `input_in`; com ligação no `layout.toml`, o agente captura sozinho;
  - (f) Keel: deps aprovadas `tauri =2.12.5` (tray-icon), `tauri-build =2.7.4`, `tauri-plugin-notification =2.7.0`, `if-addrs =0.15.0`. Recusados: plugins opener, dialog, global-shortcut e autostart, `toml_edit` e a feature `devtools`;
  - (g) Keel, nomes: `layout_set_options`, `layout_set_device` (junta mover e atalho), `InputState` (no lugar de `InputZones`); `lock_internal_edges` não é comando.

## Medições
- P2 (Argus/Rivet, loopback, build debug, 300 RTTs): fluxo uni 1024 B p95 432 µs · datagrama 64 B p95 313 µs. Passa no critério do §9 S1 item 4 (p95 < 1 ms). Spike em release deu < 100 µs; a diferença vem do build debug.
- Texto em claro no fio: NÃO verificado. tshark e tcpdump exigem CAP_NET_RAW, e não há sudo nesta máquina.
- P4c loopback p50/p95 pelo `clipsync` real: a preencher quando o usuário rodar o roteiro §14 item 5. Referência do spike input 2: frame→app p50 132 µs, p95 214 µs (relato do Vector).

## Pendências pós-MVP (consolidado em 2026-10-27)

### Decisões do usuário
1. **Licença**, antes do primeiro release público (v1.12). A 0.1.0 sai em repositório privado, sem LICENSE. Sugestão: MIT (ou MIT/Apache-2.0 dual, o padrão do ecossistema Rust). GPL-3 só se copiarmos código do Lan Mouse, o que o contrato evita.
2. **Regra da recópia** (contrato §7 regra 3): copiar o mesmo texto duas vezes seguidas não reenvia. Ela protege contra a tomada de posse do Mutter. O reenvio forçado é `clipsync send`. Aguarda confirmação.

### Segurança (`docs/seguranca.md`)
1. S1-04: tentativa de pareamento, conforme a regra v1.10 (Commit conta, máx 3 por janela, Reveal divergente fecha). S1-07: listener da malha sem teto de handshakes simultâneos. **Os dois antes de usar em LAN real.**
2. S1-06: o bloqueio por chave mudada não se desfaz sem reiniciar. S1-03: nome repetido apaga o par existente, e a busca por nome é ambígua.
3. S1-05: `validate()` aceita valores fora de qualquer faixa útil. Ressalva da liberação: caracteres invisíveis confundíveis (fora da lista da v1.10) ainda passam no nome do dispositivo.
4. Ressalva da liberação: o `run` não recusa perfil com modo diferente de 0700 (parte da regra S1-09 que não entrou). A chave fica em arquivo 0600 até existir o cofre da plataforma (Secret Service, CNG, Keychain).
5. Sem `cargo audit` nem `cargo deny` instalados: a auditoria de dependências não foi feita. Texto em claro no fio não verificado (falta CAP_NET_RAW). Embrulho do `quinn::crypto` via quinn-proto =0.11.19 para o reason de falha de handshake virar `CloseCode::reason()` (ADR 0002, contrato v1.7).
6. Linux: o marcador `x-kde-passwordManagerHint` dos gerenciadores de senha não é lido; uma senha copiada vai para os pares com `clipboard_out`. Para o Windows, ver S5-03 abaixo.

### S5 · Windows e release (`docs/seguranca.md`, Sentinel 10777b4)
1. **S5-01 (MÉDIO), squatting do pipe depois de saída suja do agente.** Só afeta máquina com mais de um usuário. O desenho do segredo + HMAC foi aceito (contrato §11 regra 5). Implementar antes de usar em máquina compartilhada, RDP ou servidor de terminal.
2. **S5-03 (MÉDIO), o marcador de gerenciador de senha não é lido no Windows** (`ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` = 0). A API do `clipboard-win` é segura e já está no `Cargo.lock` via arboard, mas virar dep direta passa pelo Keel. Até lá, o README manda pausar ao copiar senha.
3. **BAIXO do Sentinel:**
   - S5-04: o nome do pipe é público, e conexões só de leitura prendem tarefas do controle. Limitar as tarefas simultâneas e o timeout.
   - S5-05: ninguém confere a ACL do perfil (exige Win32).
   - S5-06: `first_pipe_instance` não impede um segundo agente do mesmo perfil.
   - S5-07 (ci.yml sem `permissions`) e S5-09 (token do checkout no `.git/config`): corrigidos em 9da97d9. O Keel conferiu por grep: `permissions: contents: read` e `persist-credentials: false` nos dois workflows. Falta a confirmação do Sentinel.
   - S5-08: qualquer tag `v*` publica sem CI verde. Correção: um **ruleset de tag `v*`** no GitHub.
   - S5-10: actions fixadas por tag móvel, e `dtolnay/rust-toolchain@master` no CI. Correção: fixar por SHA.
   - S5-11: o `SHA256SUMS` não prova a origem. Correção: **attestations** (`actions/attest-build-provenance`) sobre os artefatos e o `SHA256SUMS`.
4. **Prova em Windows real:**
   - pelo CI: FEITO. O run 40576377462 em ec8eee3 rodou 127 testes no runner Windows, com 0 falhas, incluindo mesh (QUIC) e pipe. Isso prova o código no Windows, não o uso real: o clipboard do Windows segue experimental;
   - numa máquina do usuário: o roteiro de 10 minutos com duas contas do Sentinel (`docs/seguranca.md`, "Como confirmar em Windows"), que confirma ou derruba o S5-01 e o S5-04, mais o item 3 das provas pendentes abaixo.

### Qualidade
1. Ajuda e erros nativos do clap saem em inglês. O contrato (§2 regra 1, §6) pede português: localizar com template de ajuda e mapeamento de `clap::Error` para o `msg.rs`.
2. Nomes de teste sem o prefixo da área: o filtro por nome do §9 não pega todos os arquivos. Hoje vale `--test <arquivo>` (v1.9).

### Provas pendentes do usuário (sessão gráfica)
1. Roteiro §14 item 5: `probe input --capture` (barreira só na metade superior direita), `--emulate` (quadrado + "ção") e `--loopback` (p50/p95). Registrar as saídas na matriz.
2. Itens da matriz sem prova: teclado durante a captura, posição do cursor depois do Release, segundo `--emulate` sem diálogo (token), latência do clipboard medida de novo (P1).
3. Windows real (S5): `clipsync.exe` do release v0.1.0 num PC Windows. Pareamento com o notebook, `send` nos dois sentidos e cópia num app nativo chegando ao Linux. Só depois disso a linha Windows da matriz sai de "não testada", e o adaptador `win-poll` deixa de ser "experimental".

## Próxima fatia recomendada (Keel)

1. **Antes de tudo:** o usuário preenche o §3 do prompt mestre (SO e sessão gráfica dos outros dois PCs, mesma LAN ou VPN). Sem isso, nenhum adaptador novo tem alvo.
2. **2A · clipboard entre máquinas reais.** Começa pelos itens de segurança da rede real (S1-04, S1-06, S1-07, perfil 0700). Depois:
   - binário release no segundo PC;
   - pareamento pela LAN e malha das 3 máquinas;
   - adaptador de clipboard do SO de lá (Windows: backend do arboard atrás de `cfg(windows)`, marcado não testado até rodar lá);
   - uma linha nova na matriz.
3. **2B · input com layout por borda de monitor.** O agente passa a capturar sozinho, guiado por `[[input.barrier]]`, e não só no loopback. Também entram:
   - cada máquina publica seus monitores;
   - Enter posicionado por movimento relativo;
   - keymap ABNT2 testado;
   - adaptadores do outro SO (Windows: hooks + `SendInput`);
   - checagem de conflito com Input Leap e Barrier antes de capturar.
   2A e 2B andam em paralelo com builders diferentes, como no prompt mestre §7.
   A S5 (2026-10-27) já adianta parte da 2A: binário release, e o clipboard Windows por polling, experimental. Na 2A sobra a prova em máquina real e a segurança de LAN.
4. **Tauri** (janela, bandeja, editor de layout): virou a S6 (v1.14). Compila só no CI; aqui roda pelo AppImage.
