# workboard · ClipSync MVP de hoje

Contrato: `docs/contrato.md` (v1.10). Fonte de verdade são os arquivos em `docs/`, não as notas. ADRs em `docs/decisoes/`, matriz em `docs/matriz-compatibilidade.md`. Critério de pronto de cada fatia = §9 do contrato, comando por comando. Ninguém roda git. Ninguém sobe processo longo fora de `cargo test`. `source ~/.cargo/env` sempre. Dúvida de contrato: `maestri ask "Keel · esqueleto" "..."`.

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
Estado: PASSA (commit 79be293)

## S1 · Rivet · base
Posse (v1.6): `crates/clipsync-proto/**`; no core: `Cargo.toml`, `src/{lib,tls,pairing,transport,agent,msg}.rs`, `tests/{smoke,tls,pairing,mesh}.rs`.
PRIMEIRO (até 15 min, destrava o Flux): lib.rs com todos os `pub mod` e o `CoreError` da §5; `tls::pin_of`; deps serde_json e async-trait no Cargo.toml do core (quinn-proto não entra hoje, v1.7).
Ordem interna: (1) proto + MemHub + authorize_* + CloseCode + PairMsg; avisar "proto pronto" aqui. (2) perfil, config, identidade, peers. (3) PinVerifier, retomada TLS desligada, pareamento com compromisso nB/nA. (4) malha QUIC, Hello nos dois sentidos, agente, socket de controle, CLI. (5) diag de sistema.
Pronto: §9 S1 itens 1 a 7 (renumerados na v1.6).
Estado: PASSA · proto com 40 testes, extra `CloseCode::ALL`, `DeliveryState::Unreachable` no commit 1c6ba06

## S2 · Flux · clipboard
Posse: `crates/clipsync-clipboard/**`.
Faz: trait ClipboardAdapter (§7), FakeClipboard (eco, owner_takeover, clear), X11Clipboard (x11rb XFixes + arboard), módulo (livro de escritas, conteúdo inalterado, Lamport, caixa, acks, pausa), probe --watch/--write, diagnose (DISPLAY).
Pronto: §9 S2 itens 1 a 5.
Estado: PASSA · Unreachable em uso (commit 18a461c)

## S3 · Vector · input
Posse: `crates/clipsync-input/**`.
Faz: traits CaptureAdapter/EmulateAdapter (§8), FakeCapture, FakeEmulate, PortalCapture (ashpd InputCapture + reis, barreira por segmento), PortalEmulate (ashpd RemoteDesktop + reis), módulo (loopback, stats p50/p95, limite de taxa, release_all), probe --capture/--emulate, print_loopback, diagnose.
Pronto: §9 S3 itens 1 a 7.
Estado: PASSA

## S4 · Flux · perfil, controle, diag e cli (nova em v1.6)
Posse: no core, `src/{profile,control,diag}.rs` e `tests/{profile,control}.rs`; o crate `crates/clipsync-cli/**` inteiro.
Depende do Rivet: lib.rs com `pub mod` + `CoreError` + `tls::pin_of` (primeira entrega dele). Os testes pair_/mesh_ do cli dependem de pairing e agent.
Pronto: §9 S4 itens 1 a 8.
Estado: em reemissão (veredito refeito)

## I · Flux · integração (desde v1.6)
Pronto: §9 I itens 1 a 4.
Estado: bloqueado por S4

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
- PENDENTE · pós-MVP · Embrulhar `quinn::crypto::{ServerConfig, ClientConfig, Session}` via quinn-proto =0.11.19 para o reason de falha de handshake virar `CloseCode::reason()` (ADR 0002).
- 2026-10-27 · Keel · Contrato v1.9: §9 S4 item 2 passa a usar `cargo test -p clipsync-core --test control`. O filtro `control_` não pegava nada: o teste de `tests/control.rs` se chama `serve_request_roundtrip_stale_socket_and_cleanup`.
- PENDENTE · pós-MVP · Ajuda e mensagens de erro nativas do clap saem em inglês; o contrato (§2 regra 1, §6) pede português. Localizar com template de ajuda do clap e mapeamento de `clap::Error` para textos do `msg.rs`.
- PENDENTE · pós-MVP · Padronizar os nomes dos testes com o prefixo da área (`control_`, `profile_`...) para o filtro por nome funcionar igual em todo o §9. Hoje vale `--test <arquivo>` (v1.9).
- 2026-10-27 · Keel · Contrato v1.10, auditoria `docs/seguranca.md` (veredito BLOQUEADO por S1-01). O que conta como tentativa de pareamento (§5 regra 3), proposta do Maestro aceita com três acréscimos:
  - a tentativa começa no primeiro Commit recebido, após handshake TLS em ALPN_PAIR e Hello válido;
  - antes disso nada conta (sonda, handshake falho, Hello inválido), e o host volta ao accept() até o prazo. Uma conexão pré-Commit por vez, com 5 s até o Commit (acréscimo do Keel: evita prender o slot);
  - no máximo 3 tentativas por janela; a 3ª falha fecha;
  - Reveal que não confere com o Commit fecha a janela na hora (acréscimo do Keel: prova de ataque);
  - conexões extras durante uma tentativa são recusadas (acréscimo do Keel);
  - teto do MITM: 3/10^6 por janela. ADR 0003 atualizada.

  Implementação pós-MVP (S1-04, baixa), salvo se o Rivet couber em 15 min.
- EM CORREÇÃO HOJE · Rivet/Flux · S1-01 (BLOCKER): `validate_name` única no proto recusa controle e formatação Unicode, e o cli filtra todo texto do par (§4, §2 regra 8). S1-02: pairing nunca grava nome inválido; `load_peers` põe a entrada ruim em quarentena. S1-08: pausa aplica na memória primeiro e fica pausada se não gravar (§5 regra 8). S1-09: socket criado em nome temporário + chmod + rename; apaga o socket velho só em ConnectionRefused/NotFound; `run` recusa DIR com modo diferente de 0700.
- PENDENTE · pós-MVP · S1-03 (nome repetido apaga par existente; busca por nome ambígua) · S1-04 (tentativa de pareamento, regra v1.10 acima) · S1-05 (`validate()` aceita valores fora de faixa útil) · S1-06 (bloqueio por chave mudada não se desfaz sem reiniciar) · S1-07 (listener da malha sem teto de handshakes simultâneos). Detalhes em `docs/seguranca.md`.
- PENDENTE · usuário · **Licença** a decidir antes do primeiro commit público. Sugestão do Keel: MIT (ou MIT/Apache-2.0 dual, padrão do ecossistema Rust). GPL-3 só se copiarmos código do Lan Mouse, o que o contrato evita.
- PROVISÓRIO · Keel · Chave privada em `identity.key.pem` com modo 0600 no perfil, até existir o cofre da plataforma (Secret Service, CNG, Keychain).
- PENDENTE · usuário · "Conteúdo inalterado não é cópia nova" (contrato §7 regra 3): copiar o mesmo texto duas vezes seguidas não reenvia. Protege contra a tomada de posse do Mutter, que reenviaria conteúdo velho por cima de cópia nova do par. Reenvio forçado: `clipsync send`. Aguarda confirmação do usuário.

## Medições
- P2 (Argus/Rivet, loopback, build debug, 300 RTTs): fluxo uni 1024 B p95 432 µs · datagrama 64 B p95 313 µs. Passa no critério do §9 S1 item 4 (p95 < 1 ms). Spike em release deu < 100 µs; a diferença vem do build debug. tshark/tcpdump: não informado.
- P4c loopback p50/p95: a preencher
