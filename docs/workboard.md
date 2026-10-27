# workboard · ClipSync MVP de hoje

Contrato: `docs/contrato.md` (v1.6). Fonte de verdade são os arquivos em `docs/`, não as notas. ADRs em `docs/decisoes/`, matriz em `docs/matriz-compatibilidade.md`. Critério de pronto de cada fatia = §9 do contrato, comando por comando. Ninguém roda git. Ninguém sobe processo longo fora de `cargo test`. `source ~/.cargo/env` sempre. Dúvida de contrato: `maestri ask "Keel · esqueleto" "..."`.

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
Estado: entregue (commit 79be293) · veredito do Argus pendente

## S1 · Rivet · base
Posse (v1.6): `crates/clipsync-proto/**`; no core: `Cargo.toml`, `src/{lib,tls,pairing,transport,agent,msg}.rs`, `tests/{smoke,tls,pairing,mesh}.rs`.
PRIMEIRO (até 15 min, destrava o Flux): lib.rs com todos os `pub mod` e o `CoreError` da §5; `tls::pin_of`; deps serde_json, async-trait e quinn-proto no Cargo.toml do core.
Ordem interna: (1) proto + MemHub + authorize_* + CloseCode + PairMsg; avisar "proto pronto" aqui. (2) perfil, config, identidade, peers. (3) PinVerifier, retomada TLS desligada, pareamento com compromisso nB/nA. (4) malha QUIC, Hello nos dois sentidos, agente, socket de controle, CLI. (5) diag de sistema.
Pronto: §9 S1 itens 1 a 7 (renumerados na v1.6).
Estado: em andamento (core) · proto: ENTREGUE (40 testes; extra `CloseCode::ALL`; `DeliveryState::Unreachable` no commit 1c6ba06)

## S2 · Flux · clipboard
Posse: `crates/clipsync-clipboard/**`.
Faz: trait ClipboardAdapter (§7), FakeClipboard (eco, owner_takeover, clear), X11Clipboard (x11rb XFixes + arboard), módulo (livro de escritas, conteúdo inalterado, Lamport, caixa, acks, pausa), probe --watch/--write, diagnose (DISPLAY).
Pronto: §9 S2 itens 1 a 5.
Estado: 9 testes de módulo verdes (relato do Flux) · Unreachable em uso (commit 18a461c) · conferir no veredito: expiração da caixa (§7 regra 6)

## S3 · Vector · input
Posse: `crates/clipsync-input/**`.
Faz: traits CaptureAdapter/EmulateAdapter (§8), FakeCapture, FakeEmulate, PortalCapture (ashpd InputCapture + reis, barreira por segmento), PortalEmulate (ashpd RemoteDesktop + reis), módulo (loopback, stats p50/p95, limite de taxa, release_all), probe --capture/--emulate, print_loopback, diagnose.
Pronto: §9 S3 itens 1 a 7.
Estado: em andamento · spike input 2 incorporado (v1.4)

## S4 · Flux · perfil, controle, diag e cli (nova em v1.6)
Posse: no core, `src/{profile,control,diag}.rs` e `tests/{profile,control}.rs`; o crate `crates/clipsync-cli/**` inteiro.
Depende do Rivet: lib.rs com `pub mod` + `CoreError` + `tls::pin_of` (primeira entrega dele). Os testes pair_/mesh_ do cli dependem de pairing e agent.
Pronto: §9 S4 itens 1 a 8.
Estado: a fazer

## I · Flux · integração (desde v1.6)
Pronto: §9 I itens 1 a 4.
Estado: bloqueado por S1, S3 e S4

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
- PENDENTE · usuário · **Licença** a decidir antes do primeiro commit público. Sugestão do Keel: MIT (ou MIT/Apache-2.0 dual, padrão do ecossistema Rust). GPL-3 só se copiarmos código do Lan Mouse, o que o contrato evita.
- PROVISÓRIO · Keel · Chave privada em `identity.key.pem` com modo 0600 no perfil, até existir o cofre da plataforma (Secret Service, CNG, Keychain).
- PENDENTE · usuário · "Conteúdo inalterado não é cópia nova" (contrato §7 regra 3): copiar o mesmo texto duas vezes seguidas não reenvia. Protege contra a tomada de posse do Mutter, que reenviaria conteúdo velho por cima de cópia nova do par. Reenvio forçado: `clipsync send`. Aguarda confirmação do usuário.

## Medições (preencher)
- P2 datagrama 64 B p95: · fluxo 1 KiB p95: · tshark/tcpdump:
- P4c loopback p50/p95:
