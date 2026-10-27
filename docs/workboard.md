# workboard · ClipSync MVP de hoje

Contrato: `docs/contrato.md` (v1.1). Fonte de verdade são os arquivos em `docs/`, não as notas. ADRs em `docs/decisoes/`, matriz em `docs/matriz-compatibilidade.md`. Critério de pronto de cada fatia = §9 do contrato, comando por comando. Ninguém roda git. Ninguém sobe processo longo fora de `cargo test`. `source ~/.cargo/env` sempre. Dúvida de contrato: `maestri ask "Keel · esqueleto" "..."`.

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
Estado: a fazer

## S1 · Rivet · base
Posse: `crates/clipsync-proto/**`, `crates/clipsync-core/**`, `crates/clipsync-cli/**`.
Ordem interna: (1) proto + MemHub + authorize_* + CloseCode + PairMsg; avisar "proto pronto" aqui. (2) perfil, config, identidade, peers. (3) PinVerifier, retomada TLS desligada, pareamento com compromisso nB/nA. (4) malha QUIC, Hello nos dois sentidos, agente, socket de controle, CLI. (5) diag de sistema.
Pronto: §9 S1 itens 1 a 10.
Estado: a fazer · proto: pendente

## S2 · Flux · clipboard
Posse: `crates/clipsync-clipboard/**`.
Faz: trait ClipboardAdapter (§7), FakeClipboard (eco, owner_takeover, clear), X11Clipboard (x11rb XFixes + arboard), módulo (livro de escritas, conteúdo inalterado, Lamport, caixa, acks, pausa), probe --watch/--write, diagnose (DISPLAY).
Pronto: §9 S2 itens 1 a 5.
Estado: a fazer

## S3 · Vector · input
Posse: `crates/clipsync-input/**`.
Faz: traits CaptureAdapter/EmulateAdapter (§8), FakeCapture, FakeEmulate, PortalCapture (ashpd InputCapture + reis, barreira por segmento), PortalEmulate (ashpd RemoteDesktop + reis), módulo (loopback, stats p50/p95, limite de taxa, release_all), probe --capture/--emulate, print_loopback, diagnose.
Pronto: §9 S3 itens 1 a 5.
Estado: a fazer

## I · Rivet · integração
Pronto: §9 I itens 1 a 4.
Estado: bloqueado por S2 e S3

## Decisões
- 2026-10-27 · Keel · Contrato v1 publicado.
- 2026-10-27 · Keel · Spike Bolt: versões fixadas com `=`; ring em todo lugar (aws-lc exige cmake/nasm); ashpd com screencast junto de remote_desktop; target/ compartilhado no mesmo checkout.
- 2026-10-27 · Keel · Spike Rivet: PinVerifier único cliente+servidor; retomada TLS 1.3 desligada; código de pareamento com compromisso H(nB)/nA/nB, uma tentativa por janela; Connected só após Hello nos dois sentidos; CONNECTION_CLOSE só com CloseCode curto.
- 2026-10-27 · Keel · Spike Flux: observação por XFixes via XWayland (validada); livro de escritas próprias obrigatório; conteúdo inalterado não é cópia nova (cobre tomada do Mutter); vazio não sincroniza; x11rb =0.13.2 (a do arboard); adaptador renomeado para `x11`.
- 2026-10-27 · Maestro · Contrato aprovado. Fonte de verdade passa a ser `docs/contrato.md` e `docs/workboard.md`.
- 2026-10-27 · Keel · Contrato v1.1: identidade ECDSA P-256 em vez de Ed25519. Motivos: P-256 foi o provado ponta a ponta no spike do Rivet, e é o que TPM, CNG e Secure Enclave guardam em hardware (cofre futuro). Afeta só S1 (Rivet). ADR 0003.
- 2026-10-27 · Keel · ADRs 0001 (stack), 0002 (transporte QUIC) e 0003 (pareamento) e a matriz de compatibilidade publicadas em `docs/`.
- PENDENTE · usuário · **Licença** a decidir antes do primeiro commit público. Sugestão do Keel: MIT (ou MIT/Apache-2.0 dual, padrão do ecossistema Rust). GPL-3 só se copiarmos código do Lan Mouse, o que o contrato evita.
- PROVISÓRIO · Keel · Chave privada em `identity.key.pem` com modo 0600 no perfil, até existir o cofre da plataforma (Secret Service, CNG, Keychain).
- PENDENTE · usuário · "Conteúdo inalterado não é cópia nova" (contrato §7 regra 3): copiar o mesmo texto duas vezes seguidas não reenvia. Protege contra a tomada de posse do Mutter, que reenviaria conteúdo velho por cima de cópia nova do par. Reenvio forçado: `clipsync send`. Aguarda confirmação do usuário.

## Medições (preencher)
- P2 datagrama 64 B p95: · fluxo 1 KiB p95: · tshark/tcpdump:
- P4c loopback p50/p95:
