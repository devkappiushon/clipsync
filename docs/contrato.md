# Contrato · ClipSync (v1.37, S9, 2026-10-29)

v1.1: identidade passa de Ed25519 para ECDSA P-256 (§3 e §5 regra 1; motivo na ADR 0003).
v1.2: seis ajustes do Rivet no proto aceitos (§4: sha2, IdError, SendError com #[error], ordem de autorização, MemHub, validate); consequência no §8 regra 2 (módulo input confere dono da sessão).
v1.3: `DeliveryState::Unreachable` novo (§4, Rivet); ClipSent lista TODO par alvo (§7 regra 9, Flux); código de saída do `send` (§6, Rivet); ttl conta do recebimento local e vale para a caixa (§7 regra 6); ClipSend avança Lamport sem contar como cópia local (§7 regra 12).
v1.4: `CloseCode::ALL` (§4, já entregue pelo Rivet); spike input 2 do Vector no §8 (regras 7 a 11): barreira em x = largura da zona, Enter por movimento relativo, restore_token do RemoteDesktop em `DIR/portal-tokens.toml`, proibido `supported_capabilities()`, P4c automatizado com retry; §3 ganha `portal-tokens.toml`; §9 S3 ganha itens 6 e 7.
v1.5: dep `futures-util =0.3.34` (§1, pedido do Vector).
v1.6: posse dividida (Maestro): core/{profile,control,diag}.rs e o crate cli inteiro passam ao Flux; o Rivet fica com core/{lib,tls,pairing,transport,agent,msg}.rs. Tipos: Identity/Config/InitOptions/Peers/PeerEntry/State/Profile em profile.rs (Flux); PairOutcome/PairPrompt/PairDecision em pairing.rs (Rivet); CoreError único em lib.rs (Rivet). Keel acrescentou: `tls::pin_of` como fonte única do pino, `CoreError::AgentAlreadyRunning`, `ControlHandler`/`control::serve`. `pairing::host` ganha `on_listen`. Dep `quinn-proto =0.11.19` (Rivet troca o reason do CONNECTION_CLOSE). §9: S1 dividido; nova S4 (Flux); I passa ao Flux.
v1.7 (decisão do Maestro): hoje o Rivet NÃO embrulha `quinn::crypto`. Em falha de handshake, o reason do CONNECTION_CLOSE é a string CONSTANTE do rustls para os erros fixos que o PinVerifier devolve (`ApplicationVerificationFailure` para par desconhecido ou chave trocada, `UnknownIssuer` para X.509 inválido), sem Debug de cert nem erro interno. A regra 9 continua integral para Bye e closes de aplicação. O embrulho via quinn-proto fica pendente para depois do MVP. §5 itens 3 e 6, §9 S1 itens 3c e 5d ajustados.
v1.8 (decisão do Maestro): `run` perde o flag `--listen` (§6). A porta vem só de `network.listen`, porque o `agent::run` lê a config sozinho.
v1.9: §9 S4 item 2 passa a usar `--test control`, porque o filtro por nome não selecionava o arquivo. Pendências pós-MVP registradas no workboard: ajuda e erros do clap em inglês; embrulho quinn-proto.
v1.10 (auditoria `docs/seguranca.md`): §5 regra 3 define o que conta como tentativa de pareamento (Commit recebido; máx 3 por janela; Reveal divergente fecha na hora; implementação pós-MVP, S1-04). Regras dos achados em correção hoje: nome de dispositivo (S1-01, §4 validate e §2 regra 8), nome inválido nunca gravado (S1-02, regra 3), pausa falha fechada (S1-08, regra 8), socket criado de forma atômica (S1-09, control.rs).
v1.11 (fatia S5 · Windows e release, decisões do Maestro + Keel): nova §11 e §9 S5. No Windows, a 0.1.0 tem malha, pareamento e clipboard; teclado e mouse ficam indisponíveis. Controle por named pipe só local (`reject_remote_clients`, `first_pipe_instance`, nome sorteado publicado em `DIR\agent.pipe`). Perfil protegido pela ACL do usuário, sem chmod, em `%LOCALAPPDATA%\clipsync`. Clipboard `win-poll` (arboard, 250 ms) experimental. Release por tag `v*` sem cache e com CRT estática. Na S5 cada builder commita os próprios arquivos (§11 regra 12). Posse do core inteiro volta ao Rivet durante a S5.
v1.12 (Maestro aceitou os acréscimos da v1.11; pontos que o Rivet apontou): o Windows entra no próprio lugar, não só na §11. A §3 ganha o perfil padrão e o `agent.pipe`; a §4 ganha `mono_us` por processo no Windows; a §5 ganha `resolve`, `socket_path` e o named pipe. Licença (decisão do Maestro): a 0.1.0 sai num repositório privado, sem LICENSE. A regra passa a ser "licença antes do primeiro release público", e o README segue com "a definir". §9 S5 item 10, §10 item 1 e §11 regra 9 ajustados.
v1.13 (fechamento da S5):
- Decisão do Maestro: `probe input` fora do Linux sai 3 (recusado ou indisponível, `Fail::refused`), igual ao `probe clipboard`; o 1 fica para erro de execução (§11 regra 7, §9 S5 item 6).
- A regra 8 da §11 foi cumprida em 707c780: `init` sem módulo utilizável recusa com 3 sem gravar nada, e `run` sem módulo sai 1 com `RUN_NO_MODULE`.
- Revisão do Sentinel (`docs/seguranca.md`, S5): a regra 5 corrige o texto sobre o nome do pipe (S5-04: o nome é público, e quem protege é a DACL) e registra a lacuna de saída suja (S5-01). O desenho de segredo + HMAC foi aceito pelo Keel para depois da 0.1.0.
- A regra 6 registra a escrita sem histórico e sem nuvem (S5-02, b81b08b).

v1.14 (fatia S6 · interface visual, decisões do Maestro + Keel): nova §12. Crates novos `clipsync-ui` (lógica das telas, sem Tauri) e `clipsync-app` (casca Tauri 2, só compila no CI). Proto: `PROTOCOL_VERSION = 2` com mínimo 1, variantes novas no fim (`Zones`, `LayoutOffer`, `EnterAt`, comandos `Clip*` e `Input*`). Com ligação no `layout.toml`, o agente de input captura sozinho (§8 regra 1). Permissão inicial do pareamento: tudo desligado.
v1.15 (registro do Flux, casca em 367def0): §12.7 item 3. O Tauri 2.12.5 exige permissão explícita para comando declarado no `app_manifest`, então a capability do app leva `core:default`, `notification:default` e `allow-clipsync`.
v1.16 (decisões do Maestro, 2026-10-28): §12.6 e §12.9 item 2 (2). Revogação avisada ao par: `CloseCode::Revoked = 8` no fim do enum; no `device_revoke`, A manda `Bye{Revoked}` ao par v2 antes de fechar, e B tira A do próprio `peers.toml` (dono: Rivet). A §12.5 ganha a notificação `device_removed`. Assinatura comum do `self_test` (L7) no clipboard e no input: `self_test() -> SelfTestOutcome`.
v1.17 (decisões do Maestro, 2026-10-28, rodada de correções da S6): §12.7 capabilities mínimas (`allow-clipsync`, `core:event:allow-listen`, `core:event:allow-unlisten` e só o que o front usar), a partir do informativo I-S6-1 do Sentinel (7a722ca); o mock e a galeria saem do pacote para `crates/clipsync-app/ui-dev/`, o `api.js` falha fechado, e o portal usa `scripts/servir-ui-dev.py`. §12.1: `prefs.rs` e `diag.rs` do `clipsync-ui` passam ao Flux. §12.5: códigos do Vector (`input_off`, `no_layout`, `no_offer`, `link_not_found` depois do `stale_rev`, `denied` e `offline` do `switch_to_device`) e `window_secs` do `pair_host_start` até `PAIR_WINDOW_SECS` (120). Sair da tela 3 cancela a tentativa, menos em `waiting_peer`.
v1.18 (Maestro, a partir do 7c770ef do Rivet): §12.6, item 6 da revogação. Com B offline, quem é recusado registra: um par conhecido que rejeita o certificado deste dispositivo deixa uma entrada em `recent_blocks` com o motivo "recusou a conexão: este dispositivo não está mais pareado lá"; quem recusa não registra. §12.7 item 3 ganha a lista final das capabilities (c68d980).
v1.19 (decisões do Maestro, 2026-10-28, achados S6-18 e S6-19 do Sentinel em 0f0b312): §12.5, o `layout_offer_answer` recebe `from_peer_id` e `rev` da oferta exibida, e oferta trocada dá `stale_offer` (490335f do Vector). §12.6, teto de 1.000.000 em `x` e `y` no `Layout::validate` (e4e6308).
v1.20 (decisão do Maestro, 2026-10-28): até o usuário escolher a licença, o texto exibido é "Todos os direitos reservados" no `Prefs.license` (§12.5) e no README, no lugar de "a definir" (§11 regra 9).
v1.21 (Keel, a partir do d43a13b do Rivet, S6-21 do Sentinel): §12.6, item 6 da revogação. O registro "recusou a conexão" só vale quando o par provou a identidade; um texto de fechamento sem essa prova não registra nada.
v1.22 (decisão do Maestro, 2026-10-28, rodada de fidelidade ao Figma): §12.1, as telas Clipboard, Diagnóstico e Preferências do front passam ao Flux, e a tela Layout ao Vector; o Pixel fica com o resto de `ui/` e de `ui-dev/`, inclusive os arquivos compartilhados. §12.9 item 5 ganha o tamanho da comparação (1280×800) e a regra das colunas.
v1.23 (decisão do Maestro, 2026-10-28, a partir do 13255e3 do Rivet e da falha do CI 40680756334): §5 regra 3h e §12.5 tela 3. No pareamento, a primeira recusa de qualquer lado encerra os dois na hora; quem cancela depois do código manda `Confirm{false}` e espera o ack (até 2 s) antes de fechar; o outro lado vê `declined_peer`.
v1.24 (decisão do Maestro, 2026-10-28, a partir do d7a4d79 do Rivet): §5 regra 3i e §12.5 tela 3. Cancelar em `listening` ou `connecting` solta o endpoint e a porta de pareamento e fecha a conexão com PairFailed nos dois lados, por guardas no drop.
v1.25 (pedido do Maestro, 2026-10-28): §13 nova, S7 · layout de telas de verdade. Input no Windows (hooks e SendInput) num crate próprio, `clipsync-win`, com `unsafe` só no `ffi.rs`, pendente da revisão do Sentinel; teclado pela posição física; proto v3 (`EnterSpan`, `Returned`, `Refused`, `ModuleHost::request_pause`); `Zones` em tempo real; release v0.3.0.
v1.26 (Sentinel 7cd67a3 e decisão do Maestro, 2026-10-28): §13.3 item 7 com as condições C1 a C4 (C1 feito pelo Maestro em 2bd35a9); §13.2 regra 2 com R2 a R5 (o hook cai a cada mudança, estado de teclado de tamanho fixo, canal limitado a 8 000, redação provada por teste); R1: pausar tira os hooks e retomar com Win+Alt+P usa o `RegisterHotKey` (§13.2 regra 2g, `set_paused` no trait, `ResumeHotkey` no `clipsync-win`); §13.1: `diag.rs` e `prefs.rs` do `clipsync-ui` voltam ao Rivet, e o `autostart.rs` segue com o Flux.
v1.27 (decisões do Flux em 40070d3, registradas a pedido do Maestro): §13.7 item 2 (`VK_SNAPSHOT` pelo VK, 0x54 e 0x55 descartados, bloco principal 0x01..=0x53 e 0x56..=0x58 mais 0x73 e 0x7E); §13.6 adaptador Windows (`normalize_abs` com teto, `barrier_crossing` no último pixel, `HotkeyTracker` com a tecla neutra `VK 0xE8`); §13.12 passos 2 e 3 com pontos de atenção; §13.13 decisão 10. Também (decisões do Maestro): §13.5 com `Notice::HooksLost`, `ResumeHotkey::register` e o laço de 500 ms do `DesktopSwitched`, e a dep `windows` com as features só na raiz (decisão 11); §13.9 com `keyboard_layout` no `InputState` e os atalhos "ativos" só com hook (decisão 12).
v1.28 (Maestro, 2026-10-28): C1 fechado, com o script conferindo também os lints do `clipsync-win` (§13.3 item 7, 3027c09); decisões do Vector: `holds_hooks`, `watch_resume` e `unwatch_resume` no `CaptureAdapter`, no lugar do `set_paused` (§13.6), e `MemHub::unlink` para o teste do R2 (§13.4 item 6); §13.11 item 2 ganha os testes (i) e (j); §13.13 decisões 13 e 14.
v1.29 (decisão do Maestro, I-S7-2, a pedido do Pixel em 020451d): a recusa do Win+Alt+P vira o campo `resume_hotkey_refused` no fim do `InputState` (§13.4 item 7) e em `features` (§13.9 decisão 3), com o Rivet primeiro; critérios 2 (i), 4 e 5 da §13.11 e §13.13 decisão 15.
v1.30 (Maestro, 2026-10-28): `CaptureAdapter::begin` para a troca por atalho ou botão (§13.6, pedido do Flux; critério 2 (k); decisão 16); revisão do `ffi.rs` pelo Sentinel (c8cf755) e as correções: `Modifier`, `modifier_down` e `button_down` no lugar do `async_key_down` (S7-01, §13.5), envio parcial como falha, ponteiro da consulta de monitores e teste do pânico (aac844e), e o `scripts/conferir-privacidade.sh` (S7-04, 96c89fa, §13.2 regra 2e); decisão 17. §13.1: o `adapter/wincore.rs` (com `tests/wincore.rs`), criado em a559bdb e 57dcde0, fica com o Flux.
v1.31 (Maestro, 2026-10-28): posse do `wincore` confirmada com o Flux; S7-02 no adaptador: `HooksLost` termina a sessão e recria o `Hooks`, e mais de 3 quedas em 10 s param a captura com bloqueio até reiniciar ou `InputReload` (§13.6, critério 3, decisão 18); `CaptureAdapter::can_begin` (Vector, 87da922), false no portal do Linux, e a troca vira dado para a interface: `InputState::switch_supported` (§13.4 item 8) e `features.switch_device` (§13.9 decisão 4), com o Rivet primeiro; critérios 2 (l), 4 e 5; decisão 19.
v1.32 (Maestro, 2026-10-28): §13.6 registra o adaptador Windows entregue pelo Flux em c2e73bc e as escolhas dele a afinar no roteiro (tecla repetida uma vez só, volta no segundo pixel a partir da borda, só modificadores e botões na ativação, roda a 120 por 15 px, Win+Alt+P sintético), e a decisão do Maestro de não dar trait de plataforma ao trabalhador do adaptador nesta versão, com o risco registrado; decisões 20 e 21.
v1.33 (Maestro, 2026-10-28): §13.12 passo 5 ganha a prova do reaperto do Shift na volta (I-S7-16, ressalva do Flux na reverificação do Sentinel, 520e031).

v1.34 (S8, Maestro/Codex, 2026-10-28): §14 reúne as pendências de diagnóstico e usabilidade do handoff; estado de saída conservador, configuração explícita da ligação, aviso de protocolo, teste guiado e diagnóstico da CLI com contexto. Sem nova dependência nem alteração do protocolo de rede.

v1.35 (DOC, 2026-10-28): entrada documental, sem mudança de comportamento; o cabeçalho mantém v1.34, a última versão que mudou o contrato. Reconcilia texto e código sem alterar regra: a §1 lista sete crates e omite o `clipsync-win`, que existe desde a §13.5 e fecha os oito do workspace; as regras de trabalho vigentes são as do `AGENTS.md`, e o "sem Node" do §0 vale para o build, não para a checagem local opcional do simulador (`node --test`, que o CI não roda); o rótulo do botão é "Ligar nessa posição". O §12.8 item 1 manda incluir o `clipsync-ui` nos `default-members`, mas o `Cargo.toml` o deixa fora junto com o `clipsync-app`; os comandos locais usam `--workspace --exclude clipsync-app` até o Térreo decidir. O §12.8 item 3 cita `ubuntu-24.04` e o `ci.yml` usa `ubuntu-latest`: divergência registrada para decisão do Térreo. A §13.12 continua com a v0.3.0 e a ligação manual por borda; a adaptação à v0.3.1 fica para o andar TESTES. Nenhuma seção anterior é reescrita.

v1.36 (S9, Maestro, 2026-10-28): João autorizou fechar DOC, corrigir os achados confirmados e testar. A §15 registra as correções por fatia; implementação e evidência são separadas. A versão publicada continua v0.3.1; sem autorização nova de commit, push ou release.

v1.37 (S9, Maestro, 2026-10-29): §§15.10–15.13 e spec de execução fixam B01/B03/B05/B07 e seus handoffs. Observação de Status na UI tem prazo de 3 s e erro externo permanece desconhecido. Nenhuma variante wire, versão publicada ou autorização de commit/release nova. B07 depende do FIX1 H01/B02 revisado.

Fonte: docs/prompt-mestre.md v4 (§1, §2, §5, §6, §13, §14). Este contrato vence qualquer suposição de builder. Dúvida: `maestri ask "Keel · esqueleto" "..."`. Mudança de contrato: Keel edita AQUI primeiro, depois avisa.

## 0. Regras do time
1. Ninguém roda git (o Maestro commita); exceção na S5: cada builder commita os próprios arquivos na main local, sem push (§11 regra 12). Ninguém sobe processo de longa duração fora de teste automatizado (`cargo test` pode subir `clipsync run` como filho e matar no fim).
2. Um dono por crate/arquivo (§2). Editar arquivo de outra fatia = pedir ao dono.
3. Sempre `source ~/.cargo/env` antes de cargo. Toolchain fixado: 1.99.1. Sem sudo, sem Node. Tauri só em `clipsync-app` (v1.14, §12).
4. Dep nova no `[workspace.dependencies]`: pedir ao Keel; Bolt adiciona. Crates só usam `dep.workspace = true`.

## 1. Árvore do workspace

```
clipsync/
  Cargo.toml              workspace, edition 2024, rust-version 1.99, lints, deps   [Bolt]
  Cargo.lock              gerado; sem dono; ninguém edita à mão
  rust-toolchain.toml     channel "1.99.1", components rustfmt+clippy               [Bolt]
  rustfmt.toml  clippy.toml                                                          [Bolt]
  .github/workflows/ci.yml                                                           [Bolt]
  crates/
    clipsync-proto/      tipos compartilhados, limites, codec, traits de módulo, MemHub  [Rivet]
    clipsync-core/       DIVIDIDO por arquivo (v1.6, §5):
                           tls, pairing, transport, agent, lib, msg, Cargo.toml   [Rivet]
                           profile, control, diag                                 [Flux]
    clipsync-cli/        binário `clipsync` (clap), monta agente, testes e2e do roteiro  [Flux desde v1.6]
    clipsync-clipboard/  adaptador clipboard (trait, fake, arboard), módulo, probe, diag  [Flux]
    clipsync-input/      adaptadores captura/emulação (trait, fake, portal), módulo, probe, diag  [Vector]
    clipsync-ui/         lógica das telas, sem Tauri (v1.14, §12.1)                       [Rivet, Vector, Flux por arquivo]
    clipsync-app/        casca Tauri 2 + frontend estático em ui/ (v1.14, §12.1)          [Flux; ui/ do Pixel]
```
Grafo de dependência (sem ciclo): proto <- core; proto <- clipboard; proto <- input; cli -> core + clipboard + input. **clipboard e input NUNCA dependem de core.**
S0 cria o esqueleto de todos os crates (Cargo.toml + lib.rs/main.rs + teste smoke). Quando S0 fecha, a posse de cada `crates/X/**` passa ao dono acima.

### Dependências do workspace (versões resolvidas no spike do Bolt, fixadas com `=` no `[workspace.dependencies]`)
tokio =1.54.1 (rt-multi-thread, macros, sync, time, net, io-util, signal) · quinn =0.11.12 (default-features=false, features runtime-tokio + rustls-ring) · rustls =0.23.46 (default-features=false, features ring, std, logging) · rcgen =0.14.10 · arboard =3.7.0 (SEM wayland-data-control; cai para X11 sob XWayland sozinho) · x11rb =0.13.2 (feature xfixes; a MESMA que o arboard puxa, uma versão só no workspace) · ashpd =0.13.13 (tokio + input_capture + remote_desktop + **screencast**: remote_desktop não compila sem screencast) · reis =0.7.1 (feature tokio) · clap =4.6.7 (derive) · serde =1.0.229 (derive) · toml =1.1.8 · tracing =0.1.44.
Sem versão fixada pelo spike (Bolt resolve a última estável e fixa com `=`): postcard 1 (alloc) · serde_json 1 · async-trait 0.1 · thiserror 2 · anyhow 1 (só cli e probe) · tracing-subscriber 0.3 (env-filter) · sha2 · hex 0.4 · rand · rustix 1 (time) · humantime 2 · time 0.3 (formatting) · tempfile 3 (dev).
Adicionada em v1.6 (pedido do Rivet, aprovada pelo Maestro): quinn-proto =0.11.19 (default-features=false; mesma versão que o quinn 0.11.12 puxa; só no core, para embrulhar quinn::crypto e controlar o reason do CONNECTION_CLOSE).
Adicionada em v1.5 (pedido do Vector): futures-util =0.3.34 (default-features=false; a MESMA versão que ashpd/zbus já puxam no Cargo.lock; só `StreamExt` em clipsync-input; se precisar de método que exige `alloc`/`std`, ligar só essa feature).
Provedor TLS: **ring em todo lugar**. aws-lc-sys exige cmake e nasm, ausentes aqui: rustls e quinn com default-features=false. Nenhuma lib de sistema é necessária (nem pkg-config).
Build: frio ~6 min, incremental ~30 s. Os quatro builders trabalham no MESMO checkout e compartilham o `target/` da raiz (padrão do cargo; não setar target-dir em arquivo versionado). Se o Maestro separar builders em andares, todos exportam `CARGO_TARGET_DIR=/home/kappxs/.cache/clipsync-target`. Compilar com `-p <crate>` para reduzir espera de lock.

## 2. Convenções (vinculantes)
1. Código, identificadores e comentários em inglês. Texto para o usuário em português, só em `src/msg.rs` de cada crate (consts ou fns). `RejectReason::pt()` fica em `clipsync-proto/src/msg.rs`.
2. Logs: `tracing` para stderr, filtro `CLIPSYNC_LOG` (padrão `warn`). **Nunca** conteúdo de clipboard, preview, keycode ou tecla em log. Logs têm contagem, tamanho, mime, latência, estado, id curto.
3. Tipos que carregam conteúdo têm `Debug` **manual e redigido**: `ClipOffer`, `ClipSnapshot`, `InboxItem`, `InputEvent`, `InputDatagram`, `ControlRequest`, `ModuleCommand`. Ex.: `ClipOffer { id: 1a2b.., text: <42 bytes> }`, `Key(<redacted>)`. Cada um tem teste: `!format!("{:?}", x).contains(conteúdo)`.
4. Lints do workspace: `[workspace.lints.rust] unsafe_code = "forbid"`; `[workspace.lints.clippy] unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro = "deny"`. `clippy.toml`: allow-unwrap-in-tests, allow-expect-in-tests, allow-panic-in-tests = true. Todo crate tem `[lints] workspace = true`.
5. Erros: `thiserror` nas libs, `anyhow` só em `clipsync-cli` e funções `probe::run`. Sem `unwrap/expect` fora de teste.
6. Teste que exige sessão gráfica, portal ou diálogo: `#[ignore = "requer sessão gráfica"]`. CI roda só os não ignorados.
7. Saída de CLI e probe: nunca imprime tecla; clipboard só no `inbox` (preview) e no `probe clipboard --write` (comparação ok/falha, sem ecoar).
8. (v1.10, S1-01) Todo texto que vem do par (nome, motivo de bloqueio, preview do inbox) passa por filtro de caracteres de controle e formatação antes de ir ao terminal. No pareamento, a linha `Código:` é a última antes da pergunta, depois do nome.

## 3. Perfil (`--profile DIR`)
`--profile` é global (qualquer posição). Padrão no Unix: `$XDG_CONFIG_HOME/clipsync` (ou `~/.config/clipsync`). Padrão no Windows: `%LOCALAPPDATA%\clipsync`, nunca o Roaming; sem `LOCALAPPDATA` dá `CoreError::Profile` pedindo `--profile`. Env `CLIPSYNC_PROFILE` nas duas plataformas. No Unix, DIR é criado com 0700. No Windows não há chmod: vale a ACL herdada do usuário (§11 regra 4).

```
DIR/
  config.toml         configuração versionada (usuário edita)
  identity.key.pem    chave privada ECDSA P-256 PKCS#8, modo 0600 (ADR 0003)
  identity.cert.pem   certificado autoassinado (rcgen, chamada do spike: SAN "clipsync"; nome do dispositivo NÃO vai no cert)
  peers.toml          pares confiáveis + permissões (escrito por pair; usuário pode editar)
  state.toml          estado persistido pelo agente: paused
  agent.sock          Unix: socket de controle (0600) enquanto `run` está de pé
  agent.pipe          Windows: arquivo com o nome do named pipe (`\\.\pipe\clipsync-<32 hex>`, sorteado a cada `run`), escrito por tmp + rename e apagado ao sair (§11 regra 5)
  portal-tokens.toml  restore_token do portal RemoteDesktop, modo 0600; escrito e lido SÓ por clipsync-input (Vector)
```
`portal-tokens.toml`: `version = 1` · `remote_desktop = "<token>"`. Token inválido ou recusado: apaga a chave e pede o diálogo de novo. InputCapture v1 não tem token: 1 diálogo por sessão do agente (documentado na matriz).
Segredo em arquivo 0600 é provisório (cofre da plataforma fica para depois; registrado em Aberto).
No Windows, os modos 0600 e 0700 desta seção não se aplicam: a ACL herdada do perfil protege `identity.key.pem` e `agent.pipe` (§11 regra 4; a conferência da ACL é pendência).

### config.toml
```toml
version = 1
device_name = "notebook-a"          # 1..=64 bytes UTF-8

[modules]                            # pelo menos um true, senão erro
input = true
clipboard = false                    # init pergunta; padrão NÃO

[network]
listen = "0.0.0.0:47800"             # porta do agente (run). Perfil b de teste usa 47801

[clipboard]                          # lido só se modules.clipboard = true (Flux parseia)
adapter = "auto"                     # "auto" | "x11" | "fake"   (x11 = ponte XWayland, exige DISPLAY)
mode = "auto"                        # "auto": aplica e registra na caixa | "manual": só caixa
max_text_bytes = 1048576             # <= MAX_CLIP_TEXT_BYTES

[input]                              # lido só se modules.input = true (Vector parseia)
capture = "auto"                     # "auto" | "portal" | "fake"
emulate = "auto"                     # "auto" | "portal" | "fake"

[[input.barrier]]                    # segmento de borda de UM monitor
monitor = "eDP-1"                    # nome do conector, ou "#0" para índice da zona
edge = "right"                       # left | right | top | bottom
from = 0.0                           # 0..1 ao longo da borda (de cima p/ baixo, da esquerda p/ direita)
to = 0.5
peer = "notebook-b"                  # nome ou id curto do par destino
peer_edge = "left"
```
Chave desconhecida = erro (`deny_unknown_fields`). `version != 1` = erro com motivo. Core parseia `version`, `device_name`, `modules`, `network` e entrega `[clipboard]`/`[input]` como `Option<toml::Table>` crua ao módulo.
`init --fake-adapters` grava `adapter = "fake"`, `capture = "fake"`, `emulate = "fake"`.

### peers.toml (no dispositivo que CONCEDE; padrão tudo false, §2.1)
```toml
version = 1
[[peer]]
id = "<64 hex: SHA-256 do SubjectPublicKeyInfo DER>"
name = "notebook-b"
addr = "127.0.0.1:47801"             # IP observado no pareamento + listen_port anunciado pelo par
paired_at = "2026-10-27T14:00:00Z"
clipboard_in = false                 # o par pode me ENVIAR clipboard
clipboard_out = false                # eu posso enviar meu clipboard AO par
input_in = false                     # o par pode CONTROLAR meu teclado e mouse (confiança total)
```

### state.toml
`version = 1` · `paused = false`. Pausa sobrevive a reinício.

## 4. clipsync-proto (Rivet entrega PRIMEIRO, verbatim, antes de qualquer outra coisa)
Deps: serde, postcard, serde_json, thiserror, async-trait, tokio(sync, io-util), rand, hex, rustix, sha2 (todas `workspace = true`). Feature `testing` liga `MemHub`.

```rust
// ids.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]); // SHA-256(SPKI DER)
impl PeerId {
    pub fn from_spki_der(spki: &[u8]) -> Self;
    pub fn to_hex(&self) -> String;
    pub fn from_hex(s: &str) -> Result<Self, IdError>;
    pub fn short(&self) -> String; // "ab12-cd34-ef56-7890" (8 primeiros bytes)
}
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError { #[error("invalid hex")] Hex, #[error("invalid length")] Length }
impl fmt::Display for PeerId {} // = short()
impl fmt::Debug for PeerId {}   // = short()

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub [u8; 16]);
impl EventId { pub fn random() -> Self; pub fn short(&self) -> String; }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind { Clipboard, Input }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSet { pub clipboard: bool, pub input: bool }
impl ModuleSet { pub fn contains(&self, k: ModuleKind) -> bool; pub fn is_empty(&self) -> bool; }

// limits.rs
pub const PROTOCOL_VERSION: u16 = 1;
pub const MIN_PROTOCOL_VERSION: u16 = 1;
pub const ALPN_MESH: &[u8] = b"clipsync/1";
pub const ALPN_PAIR: &[u8] = b"clipsync-pair/1";
pub const PAIR_EXPORTER_LABEL: &[u8] = b"EXPORTER-clipsync-pair-v1";
pub const PAIR_COMMIT_DOMAIN: &[u8] = b"clipsync-pair-commit-v1"; // H(nB) = SHA-256(DOMAIN || nB)
pub const MAX_FRAME_BYTES: usize = 1_114_112;        // 1 MiB + 64 KiB, fluxo de módulo
pub const MAX_CONTROL_FRAME_BYTES: usize = 65_536;   // fluxo de controle e socket local
pub const MAX_CLIP_TEXT_BYTES: usize = 1_048_576;
pub const MAX_NAME_BYTES: usize = 64;
pub const MAX_MIME_BYTES: usize = 128;
pub const MAX_REASON_BYTES: usize = 256;
pub const MAX_DATAGRAM_BYTES: usize = 1_200;
pub const MAX_EVENTS_PER_DATAGRAM: usize = 32;
pub const MAX_INBOX_ITEMS: usize = 50;
pub const CLIP_TTL_MS: u32 = 600_000;
pub const SEEN_EVENTS_CAP: usize = 1_024;
pub const PAIR_WINDOW_SECS: u64 = 120;
pub const MAX_INPUT_DATAGRAMS_PER_SEC: u32 = 2_000;

// wire.rs: enquadramento = u32 big-endian de tamanho + corpo postcard.
// Leitor checa tamanho <= max ANTES de alocar; depois decode; depois validate().
#[derive(Serialize, Deserialize, Debug)]
pub enum Frame {
    Hello(Hello),
    Bye { code: CloseCode },
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Pair(PairMsg),       // só em conexão ALPN_PAIR; em ALPN_MESH = CloseCode::Protocol
    Module(ModuleMsg),
    Rejected { kind: ModuleKind, ref_id: Option<EventId>, reason: RejectReason },
}
/// Compromisso do pareamento (B = quem faz join, A = host), no fluxo de controle, nesta ordem:
/// B->A Commit{H(nB)}; A->B Nonce{nA}; B->A Reveal{nB}; A confere H(nB).
/// Código = u64 BE de export_keying_material(8 bytes, PAIR_EXPORTER_LABEL, contexto nB||nA) mod 1_000_000.
/// Depois cada lado envia Confirm{accept} com a decisão do usuário.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PairMsg { Commit { h_nb: [u8; 32] }, Nonce { na: [u8; 32] }, Reveal { nb: [u8; 32] }, Confirm { accept: bool } }
/// Código de fechamento QUIC (application close). O reason enviado ao par é SÓ `reason()`:
/// token ASCII curto e fixo. Nunca texto Debug/Display do rustls, quinn ou io.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CloseCode { Normal = 0, Version = 1, UnknownPeer = 2, Duplicate = 3, Protocol = 4,
                     Shutdown = 5, PairFailed = 6, KeyChanged = 7 }
impl CloseCode {
    pub const ALL: [CloseCode; 8]; // os 8 códigos em ordem; usado nos testes de TLS (§9 S1 item 3c)
    pub fn code(self) -> u32;
    pub fn reason(self) -> &'static [u8]; // b"normal", b"version", b"unknown-peer", b"duplicate", b"protocol", b"shutdown", b"pair-failed", b"key-changed"
    pub fn from_code(c: u64) -> Option<Self>;
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello { pub version: u16, pub min_version: u16, pub device_name: String,
                   pub modules: ModuleSet, pub listen_port: u16 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModuleMsg { Clip(ClipMsg), Input(InputMsg) }
impl ModuleMsg { pub fn kind(&self) -> ModuleKind; }
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason { ModuleDisabled, NotAuthorized, Paused, TooLarge, Invalid,
                        Duplicate, Stale, Expired, ApplyFailed, Locked, VersionMismatch }
#[derive(Serialize, Deserialize, Debug)]
pub enum Datagram { Input(InputDatagram) }
pub trait Validate { fn validate(&self) -> Result<(), WireError>; } // impl p/ Frame, Datagram e filhos
pub fn encode_frame(f: &Frame, max: usize) -> Result<Vec<u8>, WireError>; // já com prefixo
pub fn decode_frame_body(body: &[u8]) -> Result<Frame, WireError>;        // decode + validate
pub async fn read_frame<R: tokio::io::AsyncRead + Unpin>(r: &mut R, max: usize) -> Result<Frame, WireError>;
pub async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(w: &mut W, f: &Frame, max: usize) -> Result<(), WireError>;
pub fn encode_datagram(d: &Datagram) -> Result<Vec<u8>, WireError>;   // erro se > MAX_DATAGRAM_BYTES
pub fn decode_datagram(b: &[u8]) -> Result<Datagram, WireError>;      // decode + validate
#[derive(thiserror::Error, Debug)]
pub enum WireError { #[error("frame too large: {len} > {max}")] TooLarge { len: usize, max: usize },
                     #[error("decode")] Decode, #[error("invalid: {0}")] Invalid(&'static str),
                     #[error("io: {0}")] Io(#[from] std::io::Error) }
// validate(): strings dentro dos MAX_*, floats finitos, pos/from/to em 0..=1,
// events.len() <= MAX_EVENTS_PER_DATAGRAM, text.len() <= MAX_CLIP_TEXT_BYTES, mime == MIME_TEXT_UTF8 no MVP,
// device_name com 1..=MAX_NAME_BYTES bytes (vazio recusado), Hello com min_version <= version.
// device_name (v1.10, S1-01): UMA função `validate_name` no proto recusa `char::is_control` (C0, DEL, C1 e U+009B)
// e os caracteres de formatação Unicode U+200B..U+200F, U+2028, U+2029, U+202A..U+202E, U+2066..U+2069 e U+FEFF.
// O `profile::valid_name` chama essa mesma função; as duas regras não podem divergir.
// Compatibilidade de versão entre os dois Hello NÃO é do validate(): é do core (CloseCode::Version).

// clip.rs
pub const MIME_TEXT_UTF8: &str = "text/plain;charset=utf-8";
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClipMsg { Offer(ClipOffer), Ack(ClipAck) }
#[derive(Serialize, Deserialize, Clone)] // Debug manual redigido
pub struct ClipOffer { pub id: EventId, pub lamport: u64, pub mime: String, pub text: String,
                       pub created_unix_ms: u64 /* só exibição */, pub ttl_ms: u32 }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipAck { pub id: EventId, pub state: DeliveryState }
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState { Received, Applied, InInbox, Rejected(RejectReason), Expired,
                         /// Só local, nunca vai no fio: par alvo não conectado, erro de transporte, ou sem Ack em 5 s.
                         Unreachable }

// input.rs
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge { Left, Right, Top, Bottom }
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)] // Debug manual: Key/Button sem código
pub enum InputEvent {
    Motion { dx: f32, dy: f32 },          // px lógicos, relativo
    Button { code: u32, pressed: bool },  // evdev BTN_*
    Scroll { dx: f32, dy: f32 },          // suave, px lógicos
    ScrollDiscrete { dx: i32, dy: i32 },  // unidades v120
    Key { code: u32, pressed: bool },     // evdev KEY_*; NUNCA em log
}
#[derive(Serialize, Deserialize, Clone)] // Debug manual: só session, seq, len
pub struct InputDatagram { pub session: u32, pub seq: u32, pub t_mono_us: u64, pub events: Vec<InputEvent> }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InputMsg {
    Enter { session: u32, edge: Edge, pos: f32 },
    Leave { session: u32 },
    ReleaseAll { session: u32 },
    StatsRequest { session: u32 },
    Stats(LoopbackStats),
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoopbackStats { pub session: u32, pub received: u64, pub lost: u64, pub out_of_order: u64,
                           pub p50_us: u64, pub p95_us: u64, pub max_us: u64 }

// time.rs
/// Unix: CLOCK_MONOTONIC em µs (rustix, dep só em cfg(unix)). Comparável entre processos SÓ na mesma máquina (loopback).
/// Windows (v1.12): µs desde um `Instant` fixado na primeira chamada do processo (std, `OnceLock`). Vale SÓ dentro do
/// processo; o único leitor entre processos é o loopback de input, que não existe no Windows (§11 regra 3).
pub fn mono_us() -> u64;

// module.rs: contrato core <-> módulos
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerPerms { pub clipboard_in: bool, pub clipboard_out: bool, pub input_in: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo { pub id: PeerId, pub name: String, pub perms: PeerPerms,
                      pub remote_modules: ModuleSet, pub connected: bool }
/// #[error] em inglês, curto. Texto em português fica no msg.rs de quem exibe (cli, módulos).
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum SendError { #[error("peer not connected")] NotConnected, #[error("not authorized")] NotAuthorized,
                     #[error("module disabled on peer")] RemoteModuleDisabled,
                     #[error("module disabled locally")] LocalModuleDisabled, #[error("paused")] Paused,
                     #[error("message too large")] TooLarge, #[error("transport: {0}")] Transport(String) }
#[derive(thiserror::Error, Debug)]
pub enum ModuleError { #[error("config: {0}")] Config(String), #[error("adapter: {0}")] Adapter(String),
                       #[error("{0}")] Other(String) }

#[async_trait::async_trait]
pub trait ModuleHost: Send + Sync + 'static {
    fn local_id(&self) -> PeerId;
    fn local_name(&self) -> String;
    fn peers(&self) -> Vec<PeerInfo>;   // todos os pares confiáveis, conectados ou não
    fn paused(&self) -> bool;
    /// Um frame `Frame::Module` num fluxo QUIC uni novo. Checa authorize_outbound antes.
    async fn send_reliable(&self, to: PeerId, msg: ModuleMsg) -> Result<(), SendError>;
    /// Datagrama QUIC. Checa authorize_outbound_datagram e tamanho.
    fn send_datagram(&self, to: PeerId, dg: InputDatagram) -> Result<(), SendError>;
}

#[derive(Debug)]
pub enum ModuleInput {
    Reliable { from: PeerId, msg: ModuleMsg },        // já autorizado pelo core
    Datagram { from: PeerId, dg: InputDatagram },     // já autorizado pelo core
    Rejected { from: PeerId, ref_id: Option<EventId>, reason: RejectReason },
    PeerUp(PeerInfo),
    PeerDown(PeerId),
    Paused(bool),
    Command { cmd: ModuleCommand, reply: tokio::sync::oneshot::Sender<ModuleReply> },
    Shutdown,
}

#[async_trait::async_trait]
pub trait Module: Send + 'static {
    fn kind(&self) -> ModuleKind;
    fn adapter_name(&self) -> String;
    /// Roda até Shutdown ou erro fatal. Não pode bloquear o runtime (adaptador síncrono vai p/ spawn_blocking).
    async fn run(self: Box<Self>, host: std::sync::Arc<dyn ModuleHost>,
                 rx: tokio::sync::mpsc::Receiver<ModuleInput>) -> Result<(), ModuleError>;
}

/// Fonte ÚNICA das regras de autorização (core e MemHub chamam estas funções).
/// Outbound, checado NESTA ORDEM, primeira falha vence:
///   LocalModuleDisabled -> RemoteModuleDisabled -> NotAuthorized -> Paused.
/// NotAuthorized na saída só existe para Clip::Offer (perms.clipboard_out); Input::Enter e datagramas
/// não têm permissão local de saída (a permissão de controle é checada no DESTINO).
/// Ack, Stats, StatsRequest, Leave e ReleaseAll ignoram pausa e permissão, mas exigem o módulo local ligado.
pub fn authorize_outbound(local: ModuleSet, perms: &PeerPerms, remote: ModuleSet, paused: bool, msg: &ModuleMsg) -> Result<(), SendError>;
pub fn authorize_outbound_datagram(local: ModuleSet, remote: ModuleSet, paused: bool) -> Result<(), SendError>;
/// Inbound, checado NESTA ORDEM: ModuleDisabled -> NotAuthorized -> Paused.
/// Clip::Offer exige perms.clipboard_in; Input::Enter e datagrama exigem perms.input_in; pausa bloqueia
/// Offer, Enter e datagrama. Ack, Stats, StatsRequest, Leave e ReleaseAll ignoram pausa e permissão,
/// mas exigem o módulo local ligado. Por isso o MÓDULO só age sobre esses se a session/id pertence ao remetente (§8 regra 2).
pub fn authorize_inbound(local: ModuleSet, perms: &PeerPerms, paused: bool, msg: &ModuleMsg) -> Result<(), RejectReason>;
pub fn authorize_inbound_datagram(local: ModuleSet, perms: &PeerPerms, paused: bool) -> Result<(), RejectReason>;
// Inbound rejeitado: core responde Frame::Rejected{kind, ref_id: id da Offer se houver, reason}
// e NÃO entrega ao módulo. Datagrama rejeitado: descartado e contado (sem resposta).

// control.rs: IPC CLI <-> agente. Unix: socket DIR/agent.sock. Windows: named pipe local cujo nome está em DIR\agent.pipe (§11 regra 5).
// Quadro = u32 BE + JSON (serde_json),
// máx MAX_FRAME_BYTES. Uma requisição, uma resposta, fecha.
#[derive(Serialize, Deserialize)] // Debug manual (ClipSend redigido)
pub enum ControlRequest { Status, Pause, Resume, ReloadPeers,
                          Module { kind: ModuleKind, cmd: ModuleCommand } }
#[derive(Serialize, Deserialize, Debug)]
pub enum ControlResponse { Status(AgentStatus), Ok, Module(ModuleReply), Error { message: String } }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentStatus { pub device_name: String, pub id: PeerId, pub listen: std::net::SocketAddr,
                         pub paused: bool, pub peers: Vec<PeerStatus>, pub modules: Vec<ModuleStatus> }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerStatus { pub name: String, pub id: PeerId, pub addr: Option<std::net::SocketAddr>,
                        pub state: PeerState, pub perms: PeerPerms, pub remote_modules: ModuleSet }
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PeerState { Connected, Connecting, Disconnected, Blocked { reason: String } }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModuleStatus { pub kind: ModuleKind, pub enabled: bool, pub adapter: Option<String>,
                          pub authorized_peers: Vec<String>, pub last_event: Option<String>,
                          pub last_block: Option<String> } // textos já em português, sem conteúdo
#[derive(Serialize, Deserialize, Clone)] // Debug manual
pub enum ModuleCommand { Status, ClipSend { text: String, to: Option<PeerId> }, ClipInbox,
                         InputLoopback { to: PeerId, seconds: u32 } }
#[derive(Serialize, Deserialize, Debug)]
pub enum ModuleReply { Status(ModuleStatus), ClipSent(Vec<SendOutcome>), ClipInbox(Vec<InboxItem>),
                       InputLoopback(LoopbackStats), Error(String) }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SendOutcome { pub peer: String, pub state: DeliveryState }
#[derive(Serialize, Deserialize, Clone)] // Debug manual: sem preview
pub struct InboxItem { pub id: EventId, pub from: String, pub received_unix_ms: u64, pub mime: String,
                       pub bytes: usize, pub preview: String /* até 200 chars; só p/ terminal */,
                       pub state: DeliveryState }
// Agente com módulo desligado responde ControlResponse::Module(ModuleReply::Error(msg::MODULE_DISABLED)).

// diag.rs
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagStatus { Ok, Warn, Missing, Error }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiagItem { pub area: String, pub name: String, pub status: DiagStatus, pub detail: String }

// testing.rs (feature "testing"): rede em memória com as MESMAS regras authorize_*.
pub struct MemHub { /* ... */ }
pub struct MemNode { pub id: PeerId, pub host: std::sync::Arc<dyn ModuleHost>,
                     pub tx: tokio::sync::mpsc::Sender<ModuleInput>,
                     pub rx: tokio::sync::mpsc::Receiver<ModuleInput> }
impl MemHub {
    pub fn new() -> Self;
    pub fn add_node(&self, name: &str, modules: ModuleSet) -> MemNode;
    /// a_grants = permissões que A concede a B (gravadas em A); idem b_grants.
    /// Entrega ModuleInput::PeerUp aos DOIS lados.
    pub fn link(&self, a: PeerId, b: PeerId, a_grants: PeerPerms, b_grants: PeerPerms);
    pub fn set_paused(&self, node: PeerId, paused: bool); // também entrega ModuleInput::Paused
    /// Conta só frames reliable ENTREGUES ao destino. Entrada recusada não conta e vira
    /// ModuleInput::Rejected{from: destino, ref_id, reason} no remetente (mesmo efeito do Frame::Rejected no core).
    pub fn sent_count(&self, from: PeerId, to: PeerId) -> usize;
}
```

## 5. clipsync-core (dividido em v1.6: Rivet = cripto e rede; Flux = perfil, controle, diag)
Posse por arquivo (decisão do Maestro):
- **Rivet:** `Cargo.toml` do core, `src/lib.rs` (inclui `CoreError`), `src/tls.rs`, `src/pairing.rs`, `src/transport.rs`, `src/agent.rs`, `src/msg.rs` (textos pt que o agente põe em status), `tests/{smoke,tls,pairing,mesh}.rs`.
- **Flux:** `src/profile.rs` (Identity, Config, InitOptions, Peers, PeerEntry, State, Profile), `src/control.rs`, `src/diag.rs`, `tests/{profile,control}.rs`.
- Teste unitário dentro de um arquivo é do dono do arquivo.

**Primeira entrega do Rivet, antes de seguir na cripto (até 15 min):**
1. `lib.rs` declara `pub mod {tls, pairing, transport, agent, msg, profile, control, diag};` e o `CoreError` completo abaixo.
2. `tls::pin_of`.
3. `Cargo.toml` do core ganha `serde_json` e `async-trait` (`workspace = true`). quinn-proto NÃO entra hoje (v1.7).

Sem isso o Flux não compila os arquivos dele.

Regras de fronteira:
1. **Um erro só: `CoreError` em lib.rs (Rivet)**, com variantes para as duas metades. O Flux usa as variantes existentes. Variante nova = pedir ao Rivet.
2. **Pino com uma fonte só:** `tls::pin_of` (Rivet) é a ÚNICA função que transforma certificado em PeerId. `Identity::id()` (Flux) e o PinVerifier (Rivet) chamam essa função. Se os dois lados extraíssem o SPKI de jeitos diferentes, o próprio id não bateria com o pino que o par calcula.
3. **Identidade em profile.rs (Flux):** gera com `rcgen::generate_simple_self_signed(vec!["clipsync".into()])`, a chamada exata do spike (ECDSA P-256 com ring; o verificador ignora o nome). Grava a chave com 0600 e expõe DER para o tls.rs montar as configs do rustls.
4. **Socket de controle:** cliente e servidor ficam em `control.rs` (Flux). O agente (Rivet) só implementa `ControlHandler` e chama `control::serve`.
5. **Core não imprime nada.** O pareamento avisa a porta por `on_listen`, e o texto em pt é do cli (Flux). O `confirm` lê stdin, então o pairing o roda em `spawn_blocking`.

```rust
// ===== lib.rs (Rivet) =====
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("config: {0}")] Config(String),      // config.toml inválido, dois módulos false, version != 1
    #[error("identity: {0}")] Identity(String),  // chave ausente, PEM ruim, modo != 0600
    #[error("profile: {0}")] Profile(String),    // perfil não inicializado, já inicializado, peers/state ruins
    #[error("pairing: {0}")] Pairing(String),
    #[error("transport: {0}")] Transport(String),
    #[error("agent not running")] AgentNotRunning,
    #[error("agent already running")] AgentAlreadyRunning, // furo apontado pelo Keel: control::serve com outro agente vivo
    #[error("closed: {0:?}")] Closed(CloseCode),
}

// ===== tls.rs (Rivet) =====
/// SHA-256 do SPKI via rustls::server::ParsedCertificate::subject_public_key_info() (como no spike).
pub fn pin_of(cert: &CertificateDer<'_>) -> Result<PeerId, CoreError>;

// ===== profile.rs (Flux) =====
pub struct Identity { /* cert DER + chave PKCS#8 */ }
impl Identity {
    pub fn generate() -> Result<Self, CoreError>;                          // chamada exata do spike (regra 3)
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self, CoreError>;
    pub fn cert_pem(&self) -> String;
    pub fn key_pem(&self) -> String;                                       // quem grava aplica 0600
    pub fn cert_der(&self) -> CertificateDer<'static>;                     // usado pelo tls.rs
    pub fn key_der(&self) -> PrivateKeyDer<'static>;                       // usado pelo tls.rs
    pub fn id(&self) -> PeerId;                                            // = tls::pin_of(&cert_der)
}
impl fmt::Debug for Identity {}                                            // só o id curto; nunca a chave
pub struct Profile { pub dir: PathBuf }
impl Profile {
    pub fn resolve(arg: Option<PathBuf>) -> Result<Self, CoreError>;      // --profile > CLIPSYNC_PROFILE > XDG (Unix) | %LOCALAPPDATA%\clipsync (Windows)
    pub fn init(&self, opts: InitOptions) -> Result<Identity, CoreError>; // DIR 0700, key 0600, config, peers vazio, state
    pub fn load_config(&self) -> Result<Config, CoreError>;
    pub fn load_identity(&self) -> Result<Identity, CoreError>;           // recusa key com modo != 0600
    pub fn load_peers(&self) -> Result<Peers, CoreError>;
    pub fn save_peers(&self, p: &Peers) -> Result<(), CoreError>;         // escrita atômica (tmp + rename)
    pub fn load_state(&self) -> Result<State, CoreError>;                 // ausente = State::default()
    pub fn save_state(&self, s: &State) -> Result<(), CoreError>;
    pub fn socket_path(&self) -> PathBuf;                                 // Unix: DIR/agent.sock | Windows: DIR\agent.pipe (arquivo com o nome do pipe)
}
pub struct InitOptions { pub device_name: String, pub modules: ModuleSet, pub listen: SocketAddr, pub fake_adapters: bool }
pub struct Config { pub version: u32, pub device_name: String, pub modules: ModuleSet, pub listen: SocketAddr,
                    pub clipboard: Option<toml::Table>, pub input: Option<toml::Table> }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Peers { pub version: u32, #[serde(default)] pub peer: Vec<PeerEntry> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerEntry {
    #[serde(with = "peer_id_hex")] pub id: PeerId,   // 64 hex no TOML (o serde do proto é binário)
    pub name: String,
    pub addr: Option<SocketAddr>,
    pub paired_at: String,                           // RFC 3339, só exibição
    #[serde(flatten)] pub perms: PeerPerms,          // clipboard_in, clipboard_out, input_in
}
impl Peers {
    pub fn get(&self, id: &PeerId) -> Option<&PeerEntry>;
    pub fn upsert(&mut self, e: PeerEntry);                   // mesmo id substitui
    pub fn find(&self, name_or_short_id: &str) -> Option<&PeerEntry>;
    pub fn pins(&self) -> Vec<PeerId>;
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State { pub version: u32, pub paused: bool }

// ===== control.rs (Flux) =====
#[async_trait::async_trait]
pub trait ControlHandler: Send + Sync + 'static { async fn handle(&self, req: ControlRequest) -> ControlResponse; }
/// Cria o socket com 0600 de forma atômica (v1.10, S1-09): bind em nome temporário no DIR, chmod 0600, rename para agent.sock.
/// Socket velho: apaga SÓ se o connect der ConnectionRefused ou NotFound; se connect funciona, AgentAlreadyRunning;
/// qualquer outro erro sobe sem apagar nada. DIR com modo diferente de 0700 é recusado pelo `run`.
/// Uma conexão = uma requisição + uma resposta (quadro u32 BE + JSON, máx MAX_FRAME_BYTES). Apaga o socket ao sair.
/// Windows (v1.12, §11 regra 5): `socket` é DIR\agent.pipe. Named pipe local com `first_pipe_instance(true)` e
/// `reject_remote_clients(true)`; nome `\\.\pipe\clipsync-` + 32 hex sorteados a cada serve, gravado em agent.pipe por tmp + rename.
/// Vivo (connect ok) = AgentAlreadyRunning; NotFound = arquivo velho, regrava; outro erro sobe. Sem chmod; apaga agent.pipe ao sair.
/// `request` conecta só ao nome lido de agent.pipe. Mesmas assinaturas nas duas plataformas.
pub async fn serve(socket: &Path, handler: Arc<dyn ControlHandler>, shutdown: impl Future<Output = ()> + Send) -> Result<(), CoreError>;
pub async fn request(p: &Profile, req: ControlRequest) -> Result<ControlResponse, CoreError>; // AgentNotRunning sem socket vivo

// ===== diag.rs (Flux) =====
pub async fn system() -> Vec<DiagItem>; // SO, XDG_SESSION_TYPE, DISPLAY/XWayland, Input Leap e Barrier (/proc, inclusive flatpak); só detecta e orienta

// ===== pairing.rs (Rivet) =====
pub struct PairPrompt { pub code: String /* "123 456" */, pub peer_name: String, pub peer_id: PeerId, pub peer_modules: ModuleSet }
pub struct PairDecision { pub accept: bool, pub grants: PeerPerms }
pub enum PairOutcome { Paired(PeerEntry), Declined { by_peer: bool } }
/// on_listen recebe o endereço real (porta efêmera resolvida): o cli imprime "Aguardando em <addr>" e o teste pair_ lê a porta.
/// confirm roda em spawn_blocking (pode ler stdin). Grava peers.toml via Profile::save_peers só se os dois aceitarem.
pub async fn host(p: &Profile, port: u16, window: Duration,
                  on_listen: impl FnOnce(SocketAddr) + Send,
                  confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static) -> Result<PairOutcome, CoreError>;
pub async fn join(p: &Profile, addr: SocketAddr,
                  confirm: impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static) -> Result<PairOutcome, CoreError>;

// ===== agent.rs (Rivet) =====
/// Lê config, identidade, peers e state via Profile; serve o controle via control::serve com um ControlHandler próprio.
pub async fn run(p: &Profile, modules: Vec<Box<dyn Module>>, shutdown: impl Future<Output = ()> + Send) -> Result<(), CoreError>;
```
6. **Reason em falha de handshake (v1.7, substitui o embrulho da v1.6):** o quinn manda o `e.to_string()` do rustls no CONNECTION_CLOSE quando o handshake falha. Hoje o PinVerifier só devolve erros FIXOS do rustls: `ApplicationVerificationFailure` (par desconhecido, chave trocada) e `UnknownIssuer` (X.509 inválido). Nunca `CertificateError::Other` com texto próprio, Debug, pino ou bytes de cert. O reason no fio fica uma string constante do rustls. O motivo detalhado vai só para o log local de quem recusou. **Pendente pós-MVP:** embrulhar `quinn::crypto::{ServerConfig, ClientConfig, Session}` via quinn-proto para o reason virar `CloseCode::reason()`.

Regras de transporte (implementação livre dentro delas):
1. Identidade: ECDSA P-256 (rcgen; ADR 0003), PeerId = SHA-256(SPKI DER). **Um verificador único** (`core::tls::PinVerifier`) implementa `ClientCertVerifier` e `ServerCertVerifier`: valida o X.509 com o próprio cert como âncora (autoassinado), verifica a assinatura TLS 1.3 real com o provider ring, e o pino é SHA-256 do SPKI. Modo malha: pino precisa estar em peers.toml. Modo pareamento (ALPN_PAIR): aceita qualquer pino, mas exige posse da chave. Cert de cliente obrigatório nos dois modos (TLS mútuo).
2. **Retomada TLS 1.3 desligada (obrigatório)**, senão o verificador não roda na reconexão. Servidor: `session_storage = NoServerSessionStorage`, `send_tls13_tickets = 0`. Cliente: `resumption = Resumption::disabled()`. Vale para malha e pareamento.
3. Pareamento: host escuta em porta efêmera (`--port 0` padrão), imprime `Aguardando em <addr>:<porta> (expira em 2 min)`. No fluxo de controle aberto pelo join (B): Hello B->A, Hello A->B, depois o compromisso de `PairMsg` (§4): B envia Commit{H(nB)}, A envia Nonce{nA}, B revela nB, A confere H(nB). Código = u64 BE de `export_keying_material(8 bytes, PAIR_EXPORTER_LABEL, contexto nB||nA)` mod 1_000_000, "NNN NNN". nA e nB: 32 bytes aleatórios novos por tentativa. **O que conta como tentativa (v1.10):**
   - **a)** Uma tentativa começa quando o host recebe o primeiro `PairMsg::Commit` numa conexão que completou o handshake TLS em ALPN_PAIR e mandou um Hello válido.
   - **b)** Antes do Commit nada conta: handshake falho, ALPN errado, sonda sem Hello, Hello inválido (nome recusado) ou timeout. O host descarta a conexão e volta ao `accept()` até o prazo da janela. Só uma conexão pré-Commit por vez, com 5 s do fim do handshake até o Commit.
   - **c)** No máximo **3 tentativas por janela**. Recusa, código divergente, timeout depois do Commit e frame fora de ordem contam como tentativa. A 3ª falha fecha a janela com CloseCode::PairFailed e encerra `pair host` (sai 3). O cli mostra quantas tentativas sobram.
   - **d)** **Reveal que não confere com o Commit fecha a janela NA HORA**, sem olhar o contador: é prova de ataque ativo. O cli avisa "possível ataque: refaça o pareamento numa rede confiável".
   - **e)** Durante uma tentativa (Commit recebido até o fim), conexões extras são recusadas.
   - **f)** Segurança: nA é sorteado depois do Commit, então cada tentativa dá ao MITM chance 1/10^6, independente das outras. Com 3, o teto por janela é 3/10^6 (ADR 0003).
   - **g)** Implementação é pós-MVP (S1-04, baixa), salvo se o Rivet couber em 15 min. Até lá vale o comportamento atual: a primeira conexão é a tentativa, e qualquer falha fecha. É mais restritivo; o único custo é negação de serviço. Cada lado mostra código + nome + id curto e pergunta permissões só dos módulos ligados (input_in com aviso de confiança total). Cada lado envia Confirm{accept}; grava peers.toml **só se os dois aceitarem** e só com nome que passa em `validate_name`; nome inválido = PairFailed antes de gravar (v1.10, S1-02). `load_peers` põe em quarentena a entrada inválida (aviso + ignora só ela) em vez de derrubar o perfil. Se o agente estiver rodando, manda `ReloadPeers`.
   - **h)** Recusa (v1.23, Rivet 13255e3): a primeira recusa de qualquer lado encerra a tentativa nos dois na hora, sem esperar a decisão do outro usuário. Quem recusa, ou cancela depois do código, manda `Confirm{false}` no fluxo de controle (o QUIC reenvia esse frame) e espera o ack dele por até 2 s (`CONFIRM_FLUSH`) antes de fechar com CloseCode::PairFailed. O outro lado termina em `PairOutcome::Declined { by_peer: true }`. Nenhum dos dois grava o `peers.toml`. Motivo: o FIN e o CONNECTION_CLOSE saem uma vez só; perdidos, o outro lado esperava o idle timeout (5 s + 15 s) com o usuário parado na tela (CI 40680756334).
   - **i)** Tentativa largada antes do código (v1.24, Rivet d7a4d79): quem larga a tentativa (drop da future do `host` ou do `join`) não deixa nada vivo. Duas guardas no drop fazem isso: `AbortOnDrop` derruba a tarefa que recusa conexões extras, o que solta o endpoint e a porta de pareamento; `CloseOnDrop` fecha a conexão com CloseCode::PairFailed, no host e no join, no lugar do fechamento implícito com código 0 (Normal). Se a tentativa já fechou antes, vale o primeiro fechamento. A porta volta depois do draining do QUIC: menos de 0,15 s com a conexão estabelecida e cerca de 3,1 s com o handshake travado (3 PTO sem amostra de RTT).
4. Malha: `run` escuta em `network.listen`, disca cada par (backoff 1 s até 30 s, jitter ±20%). Fluxo bidi de controle aberto por quem disca. **Conectado só depois de Hello de aplicação autenticado nos DOIS sentidos** (o `connect()` do quinn volta antes de o servidor validar o cert do cliente): antes disso nada de PeerUp, nada de módulo, status = Connecting. Versão incompatível fecha com CloseCode::Version. Mensagens de módulo: um fluxo uni por frame. Datagramas: `encode_datagram`. keep-alive 5 s, idle 15 s, datagramas ligados.
5. Duplicada: se há duas conexões com o mesmo par, fica a discada pelo menor PeerId; a outra fecha com CloseCode::Duplicate.
6. Par desconhecido: handshake recusado; servidor loga `par desconhecido recusado: <id curto>`; cliente desconhecido vê a mensagem de `msg.rs` "recusado pelo par: certificado não confiável".
7. Chave trocada: ao discar `addr` de um par conhecido e receber outro SPKI: aborta (CloseCode::KeyChanged), PeerState::Blocked{"chave do par mudou; repareie"}, alerta em warn e em `status`. Não reconecta até novo pareamento.
8. Pausa: core guarda (state.toml), aplica authorize_* e propaga `ModuleInput::Paused`. **Falha fechada (v1.10, S1-08):** aplica na memória primeiro e depois persiste. Se gravar falhar, continua pausado e o erro avisa que a pausa não sobrevive a reinício.
9. **CONNECTION_CLOSE e Bye levam só `CloseCode::code()` + `reason()`** (token curto fixo). Nunca texto Debug/Display de rustls, quinn ou io vai ao par. Erro detalhado só em log local.
10. Frame acima do limite do fluxo: o leitor para antes de alocar e reseta o fluxo (`stop`/RESET com CloseCode::Protocol). Fluxo de controle: MAX_CONTROL_FRAME_BYTES (64 KiB); fluxo de módulo: MAX_FRAME_BYTES.

## 6. clipsync-cli (Flux desde v1.6): comandos
`clipsync [--profile DIR] <cmd>` (`--profile` global). Saída em português de `src/msg.rs`. Códigos de saída: 0 ok, 1 erro, 2 uso (clap), 3 recusado/não confirmado, 4 agente não está rodando.
```
init [--name N] [--modules input,clipboard] [--port P] [--fake-adapters]   pergunta o que faltar; pergunta de clipboard com padrão NÃO
status                    agente de pé: AgentStatus; parado: config + "agente parado" e sai 4
pair host [--port P] [--timeout 120s] [--allow LISTA]
pair join ADDR [--allow LISTA]       LISTA: clipboard-in,clipboard-out,input-in (pré-responde permissões; o CÓDIGO sempre é confirmado por stdin "s/N")
run                       foreground até Ctrl-C; a porta vem de `network.listen` da config (sem flag; para mudar, edite config.toml)
pause | resume            via socket; resume não reenvia nada copiado durante a pausa
send --stdin [--to PAR]   lê stdin (<= max), ModuleCommand::ClipSend; imprime uma linha por par alvo (inclusive "não alcançado"); sai 0 só se TODOS ficaram Applied ou InInbox, senão sai 3; ModuleReply::Error (nenhum par autorizado, --to desconhecido) sai 1
inbox                     lista InboxItem (id curto, de, hora, bytes, estado, preview)
diagnose                  core::diag::system() + clipboard::diagnose() + input::diagnose()
probe clipboard ...       -> clipsync_clipboard::probe::run(args)
probe input --loopback --to PAR [--seconds N]   -> ControlRequest::Module{Input, InputLoopback} ao agente do perfil; imprime com clipsync_input::probe::print_loopback
probe input --capture|--emulate                 -> clipsync_input::probe::run(args)
```
Montagem do agente (`src/agent.rs`): para cada módulo ligado chama `clipsync_<m>::build(cfg, &profile.dir)`; módulo desligado não é construído (não carrega adaptador, não pede permissão, não é anunciado).

## 7. clipsync-clipboard (Flux)
```rust
pub mod adapter {
    pub enum WatchMode { Events, Poll { interval_ms: u32 } }
    /// Evento de mudança. NÃO diz quem mudou: a escrita do próprio adaptador também gera evento.
    pub struct ChangeEvent { pub seq: u64, pub t_mono_us: u64 }
    pub struct ClipSnapshot { pub mime: String, pub text: String } // Debug redigido; text nunca vazio
    #[derive(thiserror::Error, Debug)] pub enum ClipError { NoDisplay, Unavailable(String), Busy, Unsupported(&'static str), Other(String) }
    pub trait ClipboardAdapter: Send + 'static {
        fn name(&self) -> &'static str;                       // "fake" | "x11-xfixes"
        fn watch_mode(&self) -> WatchMode;                     // x11-xfixes = Events
        /// Bloqueia até `timeout`. Some(evento) a cada SetSelectionOwner do CLIPBOARD,
        /// inclusive o eco da nossa própria escrita (~2 ms) e a tomada de posse pelo Mutter.
        fn wait_change(&mut self, timeout: Duration) -> Result<Option<ChangeEvent>, ClipError>;
        /// Ok(None) = vazio: TARGETS vazio, só application/x-zerosize, ou sem UTF8_STRING. Vazio não sincroniza.
        fn read_text(&mut self) -> Result<Option<ClipSnapshot>, ClipError>;
        /// Escreve texto no CLIPBOARD. Vai gerar um ChangeEvent; o módulo descarta pelo livro de escritas.
        fn write_text(&mut self, text: &str) -> Result<(), ClipError>;
    }
    pub struct FakeClipboard; pub struct FakeClipboardHandle; // Clone
    impl FakeClipboard { pub fn new() -> (Self, FakeClipboardHandle); } // write_text TAMBÉM emite ChangeEvent, como o X11
    impl FakeClipboardHandle { pub fn user_copy(&self, text: &str); pub fn owner_takeover(&self); /* reemite evento c/ mesmo conteúdo (Mutter) */
                               pub fn clear(&self); pub fn contents(&self) -> Option<String>; pub fn writes(&self) -> usize; }
    /// Observa com x11rb XFixes (SelectionNotify do CLIPBOARD); lê TARGETS + UTF8_STRING; escreve via arboard.
    /// Exige DISPLAY (XWayland). Só CLIPBOARD; PRIMARY ignorada.
    pub struct X11Clipboard; impl X11Clipboard { pub fn new() -> Result<Self, ClipError>; } // NoDisplay sem DISPLAY
}
pub struct ClipboardConfig { pub adapter: AdapterChoice, pub mode: ClipMode, pub max_text_bytes: usize }
pub enum AdapterChoice { Auto, X11, Fake }   pub enum ClipMode { Auto, Manual }
impl ClipboardConfig { pub fn from_table(t: Option<&toml::Table>) -> Result<Self, ModuleError>; }
pub fn build(cfg: ClipboardConfig, profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError>;
pub fn build_with_adapter(cfg: ClipboardConfig, adapter: Box<dyn ClipboardAdapter>) -> Box<dyn Module>;
pub async fn diagnose() -> Vec<DiagItem>;  // ponte XWayland: DISPLAY definido? (sem DISPLAY = Missing "a ponte exige XWayland ativo"); XFixes disponível; data-control presente ou não
pub mod probe {
    #[derive(clap::Args, Debug)] pub struct ProbeArgs { #[arg(long, value_parser = humantime::parse_duration)] pub watch: Option<Duration>, #[arg(long)] pub write: bool }
    pub async fn run(args: ProbeArgs) -> anyhow::Result<()>;
}
```
Regras do módulo:
1. **Nunca encaminha**: só oferece conteúdo vindo de cópia LOCAL do usuário. Conteúdo recebido e aplicado nunca é reoferecido. Malha completa, sem relay.
2. **Livro de escritas próprias** (obrigatório): antes de `write_text`, registra SHA-256(texto) com validade de 2 s. ChangeEvent cujo conteúdo lido bate com uma entrada válida = eco, descartado (vale para re-escrita idêntica).
3. **Conteúdo inalterado não é cópia nova**: o módulo guarda o hash do conteúdo atual conhecido (última leitura ou última escrita nossa). ChangeEvent cujo hash é igual ao atual é descartado, a qualquer tempo. Cobre a tomada de posse do Mutter quando o app de origem morre (segundo evento, mesmo conteúdo) e o eco. Consequência aceita: copiar X duas vezes seguidas = 1 oferta; X, Y, X = 3 ofertas. Reenvio forçado é `clipsync send`.
4. Vazio (read_text = None) nunca gera oferta e não altera o hash atual.
5. Dedupe por EventId no recebimento (vistos, SEEN_EVENTS_CAP, TTL CLIP_TTL_MS): repetido -> Ack Rejected(Duplicate). Hashes ficam só em memória, nunca em log.
6. Ordem: relógio de Lamport. Cópia local: lamport = max_visto + 1. Oferta só é aplicada se (lamport, origem PeerId) > último aplicado/copiado; senão vai para a caixa com Rejected(Stale). **ttl conta do recebimento local, em relógio monotônico do receptor** (created_unix_ms é só exibição, relógio de parede não decide nada): ttl_ms == 0 -> Expired ao chegar; item na caixa com idade > ttl_ms passa a Expired e o texto/preview é descartado da memória (nada de fila esquecida).
7. Antes de aplicar: reavalia pausa e hash atual; se o usuário copiou depois da oferta, não sobrescreve (caixa).
8. mode auto: aplica e registra na caixa como Applied; manual: só caixa (InInbox). Caixa em memória, MAX_INBOX_ITEMS.
9. Ack ao remetente com o estado final. `ClipSend` espera Acks por até 5 s. `ClipSent` traz **todo par alvo** (`to: Some` = esse par; `None` = todos com clipboard_out = true): não conectado, erro de transporte ou sem Ack em 5 s = `Unreachable`; recusa local de authorize = `Rejected(motivo)`. Par alvo nunca some da lista. `ModuleReply::Error` só quando não há alvo (nenhum par autorizado, ou --to desconhecido).
10. Pausa: não oferece, não aplica; resume não reenvia o que foi copiado durante a pausa (atualiza o hash atual em silêncio).
12. `ClipSend` (envio explícito) avança o Lamport (lamport = max_visto + 1, para o destino aplicar como mais novo) mas não conta como cópia local: não escreve no clipboard local e não muda o hash atual.
11. Adaptador síncrono roda em thread própria (`spawn_blocking`), ponte por canal. Escopo validado: só texto UTF-8 no CLIPBOARD. Imagem, arquivos e PRIMARY: fora de hoje.

## 8. clipsync-input (Vector)
```rust
pub mod adapter {
    #[derive(Debug, Clone)] pub struct Zone { pub index: u32, pub name: Option<String>, pub x: i32, pub y: i32, pub width: u32, pub height: u32, pub scale: f64 }
    #[derive(Debug, Clone)] pub enum ZoneRef { Name(String), Index(u32) }
    #[derive(Debug, Clone)] pub struct BarrierSpec { pub zone: ZoneRef, pub edge: Edge, pub from: f32, pub to: f32 }
    pub enum CaptureEvent { Activated { barrier: usize, pos: f32, t_mono_us: u64 }, Input { t_mono_us: u64, ev: InputEvent }, Deactivated, Error(String) } // Debug redigido
    #[derive(thiserror::Error, Debug)] pub enum InputError { PortalUnavailable(String), PermissionDenied, ZoneNotFound(String), Eis(String), Other(String) }
    #[async_trait::async_trait]
    pub trait CaptureAdapter: Send + 'static {
        fn name(&self) -> &'static str;                // "fake" | "portal-inputcapture"
        async fn zones(&mut self) -> Result<Vec<Zone>, InputError>;
        /// Instala barreiras e começa a entregar eventos. Diálogo do portal pode aparecer aqui.
        async fn start(&mut self, barriers: &[BarrierSpec]) -> Result<tokio::sync::mpsc::Receiver<CaptureEvent>, InputError>;
        async fn release(&mut self) -> Result<(), InputError>;   // devolve o cursor ao desktop local
        async fn stop(&mut self) -> Result<(), InputError>;
    }
    #[async_trait::async_trait]
    pub trait EmulateAdapter: Send + 'static {
        fn name(&self) -> &'static str;                // "fake" | "portal-remotedesktop"
        async fn start(&mut self) -> Result<(), InputError>;     // diálogo do portal aqui
        async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError>;
        async fn release_all(&mut self) -> Result<(), InputError>; // solta teclas e botões pressionados
        async fn stop(&mut self) -> Result<(), InputError>;
    }
    pub struct FakeCapture; pub struct FakeCaptureHandle; // Clone
    impl FakeCapture { pub fn new(zones: Vec<Zone>) -> (Self, FakeCaptureHandle); }
    impl FakeCaptureHandle { pub fn activate(&self, barrier: usize, pos: f32); pub fn push(&self, ev: InputEvent); pub fn deactivate(&self); }
    pub struct FakeEmulate; pub struct FakeEmulateHandle; // Clone
    impl FakeEmulate { pub fn new() -> (Self, FakeEmulateHandle); }
    impl FakeEmulateHandle { pub fn emitted(&self) -> usize; pub fn pressed_now(&self) -> usize; }
    pub struct PortalCapture; pub struct PortalEmulate; // ashpd + reis; cfg(target_os = "linux")
}
pub struct InputConfig { pub capture: AdapterChoice, pub emulate: AdapterChoice, pub barriers: Vec<BarrierConfig> }
pub enum AdapterChoice { Auto, Portal, Fake }
pub struct BarrierConfig { pub monitor: String, pub edge: Edge, pub from: f32, pub to: f32, pub peer: String, pub peer_edge: Edge }
impl InputConfig { pub fn from_table(t: Option<&toml::Table>) -> Result<Self, ModuleError>; }
pub fn build(cfg: InputConfig, profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError>;
pub fn build_with_adapters(cfg: InputConfig, cap: Box<dyn CaptureAdapter>, emu: Box<dyn EmulateAdapter>) -> Box<dyn Module>;
pub async fn diagnose() -> Vec<DiagItem>;  // versões InputCapture e RemoteDesktop, zonas, aviso Input Leap/Barrier capturando
pub mod probe {
    #[derive(clap::Args, Debug)] pub struct ProbeArgs {
        #[arg(long)] pub capture: bool, #[arg(long)] pub emulate: bool, #[arg(long)] pub loopback: bool,
        #[arg(long)] pub to: Option<String>, #[arg(long, default_value_t = 20)] pub seconds: u32, #[arg(long)] pub yes: bool }
    pub async fn run(args: ProbeArgs) -> anyhow::Result<()>;       // capture | emulate (loopback é do cli)
    pub fn print_loopback(stats: &LoopbackStats) -> String;        // texto em português
}
```
Regras do módulo:
1. MVP de hoje: o agente **não captura sozinho** (v1.14: com ligação no `layout.toml`, captura sozinho, §12.6). Captura só sob `ModuleCommand::InputLoopback{to, seconds}`: usa `[[input.barrier]]` (ou, sem barreira, metade superior da borda direita da zona 0), ao `Activated` manda `InputMsg::Enter`, cada evento vira datagrama (lote até MAX_EVENTS_PER_DATAGRAM, sem esperar), ao fim `Leave` + `StatsRequest`, responde `ModuleReply::InputLoopback(stats)`.
2. Destino: ao receber `Enter` autorizado inicia o emulador (preguiçoso); com `emulate = "fake"` só conta. Mede latência = mono_us() na chegada menos t_mono_us; p50/p95 por ordenação; responde `Stats`. `Leave`/`ReleaseAll`/pausa/PeerDown -> `release_all()`. Como o core deixa passar `Leave`, `ReleaseAll` e `StatsRequest` sem checar input_in (§4), o módulo só age se `session` é uma sessão aberta por aquele MESMO remetente; senão ignora e conta. Idem `Stats` recebido: só aceita para sessão que este nó pediu àquele par.
3. Limite de taxa MAX_INPUT_DATAGRAMS_PER_SEC por par; excedente descartado e contado. Datagrama de sessão desconhecida descartado.
4. Probe `--capture`: lista zonas (nome via Mutter DisplayConfig ou wl_output quando o portal não der nome), barreira só na metade superior da borda direita, imprime contagem de eventos por tipo e latência, nunca tecla; `Esc` (KEY_ESC lido, não impresso) ou `--seconds` libera.
5. Probe `--emulate`: pede confirmação "s/N" (pula com `--yes`), espera 5 s, move o cursor num quadrado e digita "ção" (dead keys/compose conforme keymap) na janela focada.
6. Nunca loga código de tecla. Debug de InputEvent::Key e Button é `<redacted>`.
7. **Coordenada da barreira (provado no spike input 2):** borda direita usa `x = zone.x + zone.width` (zona 1920 de largura: x = 1920; x = 1919 é RECUSADO pelo portal). Por analogia, borda inferior usa `y = zone.y + zone.height`; esquerda `x = zone.x`; superior `y = zone.y`. Só a direita foi provada: as outras três têm teste manual antes de "pronto". Segmento parcial: `from..to` vira `offset + round(from*len) ..= offset + round(to*len) - 1`.
8. **Sem ponteiro absoluto:** o EIS do RemoteDesktop só oferece movimento relativo. No `Enter`, o destino leva o cursor a uma posição conhecida (rajada relativa até saturar no canto da borda de entrada) e depois anda por movimento relativo até `pos` ao longo da borda.
9. **Diálogos do portal:** RemoteDesktop com `persist_mode = ExplicitlyRevoked` e `restore_token` guardado em `DIR/portal-tokens.toml` (0600); com token válido, nenhum diálogo. `persist_mode = DoNot` é proibido (2 diálogos por rodada). InputCapture v1 não tem token: 1 diálogo por sessão do agente.
10. **Proibido chamar `ashpd ... supported_capabilities()`**: o portal do GNOME 50 devolve 15, com bit desconhecido, e o parse quebra. Pedir as capacidades direto (Keyboard | Pointer) em `create_session`/`select_devices`.
11. **P4c automatizado:** emulação via RemoteDesktop dispara a barreira da captura, então o loopback pode rodar sem mão humana. Uma em três tentativas não ativou no spike: o teste tenta até 3 vezes e só falha se as 3 falharem, registrando quantas precisou.

## 9. Critérios de pronto (passa/não passa)
Todos os comandos rodam de `/home/kappxs/Documentos/ANTIGOS/clipsync` com `source ~/.cargo/env`. "M" = manual em sessão gráfica (usuário/Maestro executa; builder entrega o comando e a saída esperada).

### S0 · Bolt
1. `cat rust-toolchain.toml` mostra channel "1.99.1" e components rustfmt, clippy; `rustup show active-toolchain` começa com `1.99.1`.
2. `cargo build --workspace` sai 0; existem os 5 crates da §1 com `[lints] workspace = true`.
3. `cargo test --workspace` sai 0 com 1 teste smoke por crate (5 passed).
4. `cargo clippy --workspace --all-targets -- -D warnings` sai 0. `cargo fmt --all --check` sai 0.
5. `grep -n 'unwrap_used\|unsafe_code' Cargo.toml` mostra os lints; `cat clippy.toml` mostra allow-*-in-tests.
6. `.github/workflows/ci.yml` roda em ubuntu-latest: fmt --check, clippy -D warnings, test --workspace; `python3 -c "import yaml,sys;yaml.safe_load(open('.github/workflows/ci.yml'))"` sai 0.
7. `[workspace.dependencies]` com todas as deps da §1 fixadas com `=`; `cargo tree -i ring` mostra ring; `cargo tree -i aws-lc-sys` e `cargo tree -i aws-lc-rs` falham (não existem); `cargo tree -d -i x11rb` não mostra duas versões.

### S1 · Rivet (base: cripto e rede; dividido em v1.6)
1. Proto: `cargo test -p clipsync-proto` verde com: frame acima do máx recusado ANTES de alocar; datagrama > 1200 bytes recusado; validate recusa float NaN, pos 1.5, 33 eventos, nome de 65 bytes; Debug redigido de ClipOffer, InputEvent::Key, ControlRequest; tabela de authorize_* (módulo desligado -> ModuleDisabled, sem perm -> NotAuthorized, pausado -> Paused, Ack passa pausado); MemHub entrega e recusa com as mesmas regras.
2. Pareamento seguro: `cargo test -p clipsync-core pairing_` verde com: (a) Reveal com nB que não bate com o Commit -> PairFailed, nada gravado, host encerra; (b) segunda conexão durante a tentativa recusada; (c) após uma falha, a mesma janela não aceita nova tentativa; (d) o código depende de nA e nB (mesma sessão TLS, nonces diferentes = código diferente).
3. TLS: `cargo test -p clipsync-core tls_` verde com: (a) **contador do PinVerifier sobe nas DUAS conexões** ao conectar, desconectar e reconectar (prova de retomada desligada); (b) cliente com cert desconhecido: servidor nunca marca Connected nem entrega PeerUp, mesmo com `connect()` do cliente tendo retornado Ok; (c) reason de todo Bye e close de APLICAÇÃO pertence a `CloseCode::reason()`. Em falha de HANDSHAKE (par desconhecido, chave trocada, X.509 inválido), o reason recebido é uma das strings constantes do rustls para ApplicationVerificationFailure ou UnknownIssuer: não contém o hex do pino, nem `Other(`, nem texto do verificador (v1.7).
4. P2 canal: teste `stream_and_datagram` verde: no mesmo Connection, um frame em fluxo uni e um datagrama chegam ao outro lado; frame de controle de 64 KiB + 1 recusado com RESET antes de alocar. Medição registrada no workboard (spike: datagrama 64 B p95 < 100 µs; fluxo 1 KiB p95 < 80 µs); passa se p95 < 1 ms nos dois. Se `tshark` ou `tcpdump` existir sem sudo, captura em lo sem o texto sintético em claro; senão registrar "não verificado".
5. Malha no core, sem cli: `cargo test -p clipsync-core mesh_` verde com dois `agent::run` no mesmo processo, perfis em tempdir, sem MemHub (QUIC real em 127.0.0.1). Os casos: (a) Connected e PeerUp só depois do Hello nos dois sentidos; (b) uma terceira identidade desconhecida é recusada e nunca chega a Connected; (c) derrubar e subir B reconecta, com o PinVerifier rodando de novo; (d) B com identidade nova no mesmo addr: A decide localmente pelo próprio verificador e marca B como PeerState::Blocked{"chave do par mudou; repareie"}; o reason que chega a B é a string constante do rustls (v1.7), sem pino.
6. Comportamento que os testes do cli (S4 itens 3 e 4) exercitam: se falharem na camada de pairing ou agent, o conserto é do Rivet.
7. `cargo clippy --workspace --all-targets -- -D warnings` e `cargo fmt --all --check` verdes.

### S4 · Flux (perfil, controle, diag e cli; nova em v1.6)
1. Perfil: `cargo test -p clipsync-core profile_` verde com: (a) `init` cria DIR 0700, key 0600 e `peers.toml` vazio; (b) `load_identity` recusa a key com modo 0644; (c) `Identity::id() == tls::pin_of(&cert_der())`; (d) round-trip do peers.toml com id em 64 hex; (e) config com os dois módulos false é recusada com `CoreError::Config`.
2. Controle: `cargo test -p clipsync-core --test control` verde (v1.9: o filtro `control_` não pega o teste de `tests/control.rs`) com: (a) o socket nasce com 0600; (b) socket velho (sem dono) é apagado e o serve sobe; (c) um segundo serve com o primeiro vivo dá `AgentAlreadyRunning`; (d) request sem agente dá `AgentNotRunning`; (e) quadro acima de MAX_FRAME_BYTES recusado antes de alocar.
3. Roteiro 2: `cargo test -p clipsync-cli --test roteiro pair_` verde: host e join em 127.0.0.1, stdout dos dois contém o mesmo `Código: NNN NNN`; aceitar dos dois lados grava peers.toml nos dois com as permissões de `--allow`; recusar em um lado não grava em nenhum. (pairing do Rivet; `on_listen` dá a porta ao teste)
4. Roteiro 3: `cargo test -p clipsync-cli --test roteiro mesh_` verde: dois `clipsync run` filhos, `status` dos dois mostra o par Connected, módulos e permissões; terceira identidade desconhecida recusada (status do servidor ou log com "par desconhecido"); matar e subir B de novo reconecta; trocar identity de B (novo init no mesmo dir/addr) gera PeerState::Blocked com "chave do par mudou".
5. Roteiro 1: `printf 'notebook-a\nn\n' | cargo run -q -p clipsync-cli -- init --profile /tmp/cs-a` (e variante com flags) cria config.toml, identity.key.pem modo 600 (`stat -c %a`), peers.toml vazio; `init` sem resposta de clipboard grava `clipboard = false`; config com os dois módulos false é recusada com motivo.
6. Roteiro 6: `pause` grava paused=true, `status` mostra pausado, `resume` volta; teste `pause_` verde.
7. `clipsync diagnose` (M ou teste) lista sessão, DISPLAY e Input Leap/Barrier detectados (o Input Leap 3.0.3 Flatpak está rodando neste notebook: deve aparecer).
8. clippy e fmt verdes em clipsync-cli e nos arquivos do Flux no core.

### S2 · Flux (clipboard)
1. `cargo test -p clipsync-clipboard` verde com testes de módulo sobre `MemHub` + `FakeClipboard`: (a) A->B->C->A malha completa, 1 cópia em A: B e C recebem 1 write cada, total 2 ofertas, nenhuma reoferta após 1 s; (b) entrega duplicada do mesmo EventId: 1 aplicação, Ack Duplicate; (c) oferta com lamport antigo chega depois de uma nova: não sobrescreve (Stale, vai p/ caixa); (d) cópias locais X, Y, X geram 3 ofertas; X, X seguidas geram 1; (e) eco do próprio write_text (inclusive re-escrita idêntica) não gera oferta; (e2) `owner_takeover()` do fake (tomada do Mutter) não gera oferta; (e3) `clear()` e texto vazio não geram oferta; (f) B com clipboard desligado: A recebe Rejected(ModuleDisabled); (g) pausado: não envia nem aplica, resume não reenvia; (h) mode manual: InInbox, clipboard intacto; (i) Debug de ClipSnapshot e InboxItem sem conteúdo.
2. Roteiro 4 (M): `cargo run -q -p clipsync-cli -- probe clipboard --watch 15s` com 3 cópias diferentes feitas num app Wayland (gedit/navegador) imprime `observação: eventos (XFixes via XWayland)` e 3 linhas `mime=text/plain... bytes=N latência_leitura=NN ms`, nenhuma com conteúdo; fechar o app de origem não imprime cópia nova.
3. Roteiro 4 (M): `cargo run -q -p clipsync-cli -- probe clipboard --write` imprime `escrita: ok · leitura idêntica: ok · evento da própria escrita: descartado (eco) · nova cópia disparada: não`.
4. Sem DISPLAY: `env -u DISPLAY cargo run -q -p clipsync-cli -- probe clipboard --write` sai 1 com "a ponte de clipboard exige XWayland ativo (DISPLAY)"; `diagnose` mostra o mesmo item como Missing.
5. clippy e fmt verdes no crate.

### S3 · Vector (input)
1. `cargo test -p clipsync-input` verde com `FakeCapture` + `FakeEmulate` + `MemHub`: (a) loopback: 200 eventos, Stats.received = 200, p50 <= p95; (b) destino sem input_in: Enter recusado NotAuthorized, emulador nunca inicia; (c) Leave e PeerDown chamam release_all (pressed_now = 0); (d) limite de taxa descarta excedente e conta; (e) pausado no destino: datagrama descartado; (f) Debug de InputEvent::Key/Button e CaptureEvent sem código; (g) barreira "metade superior da borda direita" de uma Zone 1920x1080 em (0,0) vira o segmento x=1920, y 0..539 (nunca 1919); (h) arquivo de token gravado com modo 0600 (`stat -c %a` = 600) e token inválido é descartado.
2. Roteiro 5a (M): `cargo run -q -p clipsync-cli -- probe input --capture` lista zona com `eDP-1`, cria só a barreira da metade superior direita, encostar o mouse ali imprime `Activated`, contagens por tipo e latência; encostar na metade inferior não ativa; Esc devolve o cursor.
3. Roteiro 5b (M): `cargo run -q -p clipsync-cli -- probe input --emulate` pede confirmação, move em quadrado e digita "ção" no gedit aberto pelo usuário.
4. Roteiro 5c (M, depende de S1 integrado): perfis a (capture=portal) e b (emulate=fake, input_in concedido a a) rodando; `clipsync probe input --loopback --profile /tmp/cs-a --to notebook-b --seconds 20` imprime contagem, perdidos, p50 e p95 em ms.
5. clippy e fmt verdes no crate.
6. P4c sem mão humana (sessão gráfica): `cargo test -p clipsync-input -- --ignored portal_loopback` verde, com até 3 tentativas (regra 11), imprimindo eventos recebidos > 0, p50, p95 e o número de tentativas.
7. Token (M): rodar `probe input --emulate --yes` duas vezes seguidas; a segunda não abre diálogo do RemoteDesktop. Teclado na captura e posição do cursor após Release: registrar resultado na matriz (não verificados no spike).

### I · Integração (Flux desde v1.6, depois de S1, S2, S3 e S4)
1. Roteiro 4 automático: `cargo test -p clipsync-cli --test roteiro clip_` verde: perfis com `--fake-adapters`, A e B com clipboard, B concede clipboard_in; `printf 'olá ção' | clipsync send --stdin --profile A` sai 0 com `notebook-b: aplicado`; `clipsync inbox --profile B` lista o item com `bytes=10` e preview `olá ção`; com `clipboard = false` em B, mesmo send sai 3 com "recusado: módulo clipboard desligado em notebook-b".
2. Roteiro 6 automático: B pausado: send recusado com "pausado"; resume; nada reenviado (inbox de B inalterado).
3. Roteiro 7: `cargo test --workspace` verde; `clipsync diagnose` lista portais (InputCapture v1, RemoteDesktop v2), data-control ausente, Input Leap ativo.
4. `cargo build --release -p clipsync-cli` gera `target/release/clipsync`.

### S5 · Windows e release (v1.11; regras na §11)
Todos os comandos rodam da raiz do repo com `export PATH=$HOME/.cargo/bin:$PATH`. Os critérios do Maestro são três: (A) checagem de compilação Windows verde no workspace, itens 1 a 7; (B) CI verde nos dois runners, item 8; (C) release v0.1.0 com dois artefatos e SHA256SUMS, itens 9 e 10.
1. **Compilação Windows** (cada builder no seu crate; o Bolt roda a final no workspace): `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` sai 0. Isso cobre o `cargo check --workspace --target x86_64-pc-windows-gnu` do Maestro e ainda passa o lint no código `cfg(windows)`, sem linkar. O `ring` já compila para esse alvo nesta máquina.
2. **Linux sem regressão:** `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` sai 0.
3. **Grafo por plataforma:**
   - cada comando abaixo imprime `0`:
     - `cargo tree -q -p clipsync-input --target x86_64-pc-windows-gnu -e normal | grep -cE 'ashpd|reis'`
     - `cargo tree -q -p clipsync-clipboard --target x86_64-pc-windows-gnu -e normal | grep -c x11rb`
     - `cargo tree -q -p clipsync-core --target x86_64-pc-windows-gnu -e normal | grep -c rustix`
   - `grep -rn unsafe_code crates` não imprime nada: nenhum crate afrouxa o `forbid`.
   - Conferido pelo Keel em 2026-10-27, com o trabalho em andamento: os três dão 0 no alvo Windows, e o primeiro dá 2 no alvo Linux, prova de que o grep pega.
4. **Controle no Windows (Rivet):** `cargo test -p clipsync-core --test control` fica verde no Linux e no job Windows do CI. Casos `#[cfg(windows)]`:
   - (a) um segundo `serve` com o primeiro vivo dá `AgentAlreadyRunning`;
   - (b) `request` sem agente dá `AgentNotRunning`;
   - (c) com um `agent.pipe` velho (nome sem servidor), o `serve` sobe e grava um nome novo;
   - (d) o nome em `agent.pipe` casa com `^\\\\\.\\pipe\\clipsync-[0-9a-f]{32}$` e muda a cada `serve`;
   - (e) `agent.pipe` some depois do shutdown.

   `cargo test -p clipsync-core --test mesh` também fica verde no job Windows: QUIC e named pipe juntos, com o agente real.
5. **Clipboard (Flux):** `cargo test -p clipsync-clipboard` fica verde com testes do watcher de polling (`poll.rs`, que compila em `cfg(any(windows, test))`):
   - (a) troca de conteúdo gera 1 evento;
   - (b) o mesmo conteúdo por 3 tiques gera 0 eventos;
   - (c) fonte `Busy` não gera evento, e o tique seguinte roda normalmente;
   - (d) `stop` encerra a thread em até 1 tique.

   `grep -n experimental crates/clipsync-clipboard/src/msg.rs` mostra o texto que o `diagnose` usa no Windows.
6. **Teclado e mouse (Vector):** `grep -n UNAVAILABLE_ON_PLATFORM crates/clipsync-input/src/*.rs` mostra a mesma const em `lib.rs` (build), `probe.rs` e `diag.rs`. `probe input` fora do Linux sai 3, não 1 (v1.13). Entregue em b4f472a e 36568a7.
7. **cli (Flux):** um teste unitário cobre os três casos da §11 regra 8. Ele passa a plataforma como parâmetro de uma função pura, sem `cfg!` dentro dela, para rodar no Linux:
   - `init` fora do Linux grava `input = false`; sem módulo utilizável, o `init` sai 3 sem gravar nada (v1.13);
   - `input = true` fora do Linux avisa e segue sem o módulo;
   - `run` sem módulo sobrando sai 1 com `RUN_NO_MODULE`.

   Cumprido em 707c780.
8. **CI (Bolt):**
   - `python3 -c "import yaml;[yaml.safe_load(open(f)) for f in ('.github/workflows/ci.yml','.github/workflows/release.yml')]"` sai 0.
   - O job Windows de `ci.yml` roda `cargo clippy --workspace --all-targets --locked -- -D warnings` e `cargo test --workspace --locked`.
   - Depois do push, que é do usuário: `gh run list --workflow ci.yml --branch main --limit 1 --json conclusion,databaseId` dá `success`.
   - `gh run view <id> --json jobs --jq '.jobs[]|[.name,.conclusion]'` mostra os dois jobs (Linux e Windows) com `success`.
9. **Release, conferência estática (Bolt).** Em `.github/workflows/release.yml`:
   - `grep -c rust-cache` dá 0;
   - todo `uses:` começa com `actions/`;
   - `grep -n crt-static` mostra o RUSTFLAGS do build Windows;
   - `grep -n ubuntu-24.04` aparece no build Linux;
   - existe um passo que falha quando a tag sem o `v` difere do `version` do `[workspace.package]`.
10. **Release v0.1.0** (repositório privado, sem LICENSE, decisão do Maestro na v1.12; §11 regra 9). Depois que o usuário empurra a tag `v0.1.0`:
    - `gh release view v0.1.0 --json assets --jq '.assets[].name' | sort` lista exatamente `SHA256SUMS`, `clipsync-v0.1.0-linux-x86_64.tar.gz` e `clipsync-v0.1.0-windows-x86_64.zip`.
    - Num diretório vazio, `gh release download v0.1.0 && sha256sum -c SHA256SUMS` dá OK nos dois pacotes.
    - `tar -xzf clipsync-v0.1.0-linux-x86_64.tar.gz && ./clipsync --version` imprime `clipsync 0.1.0`.
    - O `.exe` num Windows real é prova M, pendente: o adaptador segue experimental até lá.

## 10. Aberto (decisão fora do time)
1. **Licença** antes do primeiro release público (v1.12; a 0.1.0 sai em repositório privado sem LICENSE). MIT/Apache-2.0 sugerido; GPL-3 só se copiar Lan Mouse. Usuário decide.
2. Chave privada em arquivo 0600 no perfil até existir cofre (Secret Service no Linux). Registrar na ADR de stack.
3. Nome do monitor nas zonas do portal: o InputCapture não dá nome; Vector resolve por geometria (Mutter DisplayConfig ou wl_output). Se não der hoje, `monitor = "#0"`.
4. ADRs (stack, transporte, licença) e `docs/matriz-compatibilidade.md`: sem dono neste contrato; o Maestro atribui.
5. Captura contínua guiada por barreiras no agente (produto 2B) fica fora de hoje; hoje só loopback e probes.
6. Decisão do Keel a confirmar com o usuário: "conteúdo inalterado não é cópia nova" (§7 regra 3). O anexo §15 pede que recopiar o mesmo texto possa valer como ação nova; aqui não vale, porque a tomada de posse do Mutter (app de origem morre) gera evento idêntico a qualquer tempo e reenviaria conteúdo velho por cima de cópia nova do par. Saída para o usuário: `clipsync send`.

## 11. S5 · Windows e release (v1.11)
Escopo da 0.1.0, decidido pelo Maestro:
- no Windows entram a malha QUIC, o pareamento e o módulo clipboard;
- teclado e mouse ficam "indisponível nesta versão".

O binário Windows usa o alvo `x86_64-pc-windows-msvc` (CI e release). Aqui só se checa `x86_64-pc-windows-gnu`, sem linkar (§9 S5 item 1).

Posse na S5:
- **Rivet:** `crates/clipsync-proto/**` e o `crates/clipsync-core/**` inteiro. A S5 devolve ao Rivet `src/{profile,control,diag}.rs` e `tests/{profile,control}.rs`. Motivo: o named pipe mexe em control, profile e agent ao mesmo tempo, e a porta Windows do core precisa de um dono só.
- **Flux:** `crates/clipsync-clipboard/**` e `crates/clipsync-cli/**`.
- **Vector:** `crates/clipsync-input/**`.
- **Bolt:** `Cargo.toml` raiz, `Cargo.lock`, `.github/workflows/{ci,release}.yml` e `README.md`.
- **Keel:** `docs/contrato.md`, `docs/workboard.md`, `docs/decisoes/**` e `docs/matriz-compatibilidade.md`.

Regras:
1. **Sem unsafe novo e sem dep nova.** O que a S5 precisa já está no workspace: `tokio::net::windows::named_pipe` (feature `net`), o backend Windows do arboard e a std. O que exigiria Win32 direto (`windows-sys`, FFI) fica fora da 0.1.0. Antes de tentar, pergunte ao Keel.
2. **cfg por plataforma:**
   - `cfg(unix)`: socket Unix, modos 0600 e 0700, rustix;
   - `cfg(windows)`: named pipe e adaptador `win-poll`;
   - `cfg(target_os = "linux")`: x11rb, ashpd, reis e portal-tokens.

   Dep de uma plataforma só vai em `[target.'cfg(...)'.dependencies]` do crate dono, com `workspace = true`. Código que uma plataforma não usa fica atrás do mesmo cfg. Nada de `#[allow(dead_code)]` para calar o outro alvo.
3. **Relógio (proto, já feito pelo Rivet).** No Unix, `mono_us()` segue no CLOCK_MONOTONIC. No Windows, conta µs desde um `Instant` fixado na primeira chamada do processo. Esse valor só vale dentro do processo. O único leitor entre processos é o loopback de input, e ele não existe no Windows.
4. **Perfil no Windows (Rivet, profile.rs).**
   - O padrão é `%LOCALAPPDATA%\clipsync`, não o Roaming: a chave privada não pode viajar para um servidor de domínio.
   - `--profile` e `CLIPSYNC_PROFILE` funcionam igual ao Unix. Sem `LOCALAPPDATA`, sai `CoreError::Profile` pedindo `--profile`.
   - **A proteção vem da ACL herdada do usuário.** Nada de chmod, e `load_identity` não confere modo no Windows.
   - Risco aceito na 0.1.0: um `--profile` em pasta compartilhada expõe a chave, e o README avisa disso. Conferir que a ACL é só do dono exige Win32: fica como pendência.
5. **Canal de controle no Windows (Rivet: control.rs e agent.rs).** Named pipe só local:
   - a primeira instância usa `ServerOptions::first_pipe_instance(true)`, e todas usam `reject_remote_clients(true)`;
   - o nome é sorteado a cada `serve`: `\\.\pipe\clipsync-` + 32 hex de 16 bytes do `rand`;
   - o nome é publicado em `DIR\agent.pipe`, com escrita atômica (tmp + rename). **O nome não é segredo** (v1.13, S5-04): qualquer usuário local lista `\\.\pipe\`. Quem protege é a DACL padrão do pipe, que não deixa outro usuário conectar para escrita nem criar instância nova, junto com o nome sorteado na hora;
   - `request` lê o arquivo e conecta só a esse nome. No Windows, `Profile::socket_path()` devolve `DIR\agent.pipe`. As assinaturas da §5 não mudam;
   - se `agent.pipe` existe e o connect funciona, é `AgentAlreadyRunning`. Se o connect dá NotFound, o arquivo é velho e o `serve` o regrava. Qualquer outro erro sobe sem apagar nada. O `serve` apaga `agent.pipe` ao sair;
   - `agent.rs` não abre `UnixStream` direto. A checagem de agente vivo usa a mesma rotina do control.rs, uma por plataforma;
   - o quadro é o mesmo do Unix: u32 BE + JSON, no máximo MAX_FRAME_BYTES, uma requisição por conexão.

   Por que o nome é sorteado: o namespace `\\.\pipe\` é global na máquina, e qualquer usuário lista os nomes. Com nome fixo, ou derivado do caminho do perfil, outro usuário local criaria o pipe antes do agente. Aí receberia o texto de `clipsync send --stdin`. Com o nome sorteado na hora, ninguém consegue criá-lo enquanto o agente está vivo.

   **Lacunas conhecidas na 0.1.0** (multiusuário apenas; num PC de um usuário só não há atacante):
   - **S5-01, MÉDIO:** depois de uma saída suja (console fechado, logoff, `taskkill`), o nome velho fica em `agent.pipe`. Outro usuário que já o listou pode criar o pipe e receber o próximo `send --stdin`.
   - **Desenho aceito pelo Keel, para implementar depois da 0.1.0:**
     - `agent.pipe` ganha uma 2ª linha com 16 bytes aleatórios de segredo;
     - o cliente abre a conexão mandando um nonce de 16 bytes;
     - o servidor responde `HMAC-SHA256(segredo, nonce)`;
     - o cliente confere em tempo constante e só então manda o quadro de sempre. Se a resposta não bater, aborta sem mandar o texto e diz `AgentNotRunning` com aviso;
     - o HMAC vem de `ring::hmac`: o ring já está no grafo, e a dep direta no core fica aprovada desde já.
   - **S5-04, BAIXO:** conexões só de leitura de outro usuário podem prender tarefas do controle. Limitar tarefas simultâneas e reduzir o timeout de leitura no pipe (pendência).
6. **Clipboard no Windows (Flux).** Adaptador `win-poll` (`WinClipboard`):
   - lê e escreve pelo arboard, com `WatchMode::Poll { interval_ms: 250 }`;
   - a cada 250 ms lê o texto, e só gera `ChangeEvent` quando o conteúdo muda. É a mesma regra da §7 regra 3: recópia idêntica não é cópia nova;
   - clipboard ocupado por outro app (`Busy`): tenta de novo no próximo tique, sem evento e sem erro para o usuário. Só texto;
   - `adapter = "auto"` escolhe `win-poll` no Windows. `"x11"` no Windows é erro de config;
   - as regras 1 a 12 da §7 valem do mesmo jeito: livro de escritas, Lamport, caixa, pausa, e conteúdo nunca em log;
   - (v1.13, S5-02, b81b08b) texto recebido de par é escrito com `exclude_from_history()` e `exclude_from_cloud()` do arboard: não entra no Win+V nem na nuvem da Microsoft;
   - marcador de gerenciador de senha (`ExcludeClipboardContentFromMonitorProcessing`) ainda não é lido (S5-03, pendência). Ler esse marcador pede `clipboard-win` como dep direta, e isso passa pelo Keel.

   **É experimental até o teste em máquina real.** O `diagnose` e o README dizem "experimental", e a matriz continua "não testada" na linha Windows.
7. **Teclado e mouse no Windows (Vector, entregue em b4f472a).** ashpd e reis só existem no Linux. Fora do Linux:
   - `build` devolve `ModuleError::Adapter(msg::UNAVAILABLE_ON_PLATFORM)`;
   - `probe input` sai **3** com o mesmo texto (v1.13, decisão do Maestro: 3 = recusado ou indisponível, `Fail::refused`, igual ao `probe clipboard` fora do Linux; 1 fica para erro de execução);
   - o `diagnose` mostra o item.

   O Hello anuncia só os módulos que de fato rodam (Rivet confere no agent.rs). Assim, o par Linux recebe `RemoteModuleDisabled` ao tentar mandar input para o Windows.
8. **cli no Windows (Flux).**
   - Fora do Linux, o `init` grava `input = false` sem perguntar. A pergunta do clipboard mantém o padrão NÃO. Se não sobrar módulo utilizável, o `init` recusa com motivo, sai 3 e não grava nada em disco.
   - Com `input = true` fora do Linux, o `run` avisa `INPUT_UNAVAILABLE` e segue sem o módulo. Se não sobrar nenhum módulo, o `run` sai 1 com `RUN_NO_MODULE`.
   - Cumprida em 707c780 (Flux), com testes puros que recebem a plataforma como parâmetro.
9. **Release (Bolt).**
   - `[profile.release]`: `lto = true`, `codegen-units = 1`, `strip = true` (já no Cargo.toml).
   - `release.yml` dispara em tag `v*`. Ele falha se a tag sem o `v` diferir do `version` do `[workspace.package]`.
   - O build Linux roda em `ubuntu-24.04` fixo, não `ubuntu-latest`. A glibc do runner é o mínimo que o binário exige, e o README diz isso.
   - O build Windows usa `x86_64-pc-windows-msvc` com `RUSTFLAGS=-C target-feature=+crt-static`. Sem isso, o `.exe` depende do `VCRUNTIME140.dll` do VC++ Redistributable, que um Windows limpo pode não ter.
   - **Sem cache no release:** nada de `Swatinem/rust-cache` nem `actions/cache` no `release.yml`. Um cache restaurado de execução de PR pode envenenar o binário publicado. No CI, o cache continua liberado.
   - O `release.yml` só usa actions de `actions/*`. O toolchain vem do rustup do runner, que lê o `rust-toolchain.toml` (1.99.1).
   - Artefatos: `clipsync-<tag>-linux-x86_64.tar.gz` (binário `clipsync` + README), `clipsync-<tag>-windows-x86_64.zip` (`clipsync.exe` + README) e `SHA256SUMS` dos dois.
   - **Licença (v1.12, decisão do Maestro):** a 0.1.0 sai num repositório privado, sem LICENSE, e o README segue com "a definir". Antes do primeiro release público (repositório público ou release visível fora da conta), o usuário decide a licença. Sem essa decisão, nada vira público. v1.20: até a escolha, o README (a9f69a8) e as Preferências do app dizem "Todos os direitos reservados", no lugar de "a definir".
10. **CI (Bolt).** O job Windows (`windows-latest`) roda `cargo clippy --workspace --all-targets --locked -- -D warnings` e `cargo test --workspace --locked`.
    - Esse clippy é o único lugar onde o código `cfg(windows)` passa pelo lint.
    - O e2e `roteiro.rs` já é `cfg(unix)`.
    - Os testes de malha (`mesh`) e de controle do core rodam no Windows e provam lá o named pipe e o QUIC.
11. **README (Bolt).** O quadro por plataforma segue esta seção:
    - no Windows, clipboard "experimental (polling de 250 ms, só texto, não testado em máquina real)" e teclado e mouse "indisponível nesta versão";
    - no Linux, só o que a matriz valida.

    O README também diz onde fica o perfil no Windows e avisa para não pôr `--profile` em pasta compartilhada. E informa a glibc mínima do binário Linux.
12. **git na S5** (decisão do Maestro; substitui a §0 regra 1 durante a fatia):
    - cada builder commita só os próprios arquivos, na main local, com caminho explícito: `git add <arquivos> && git commit -m "<msg>" -- <arquivos>`;
    - nunca `git add -A`, `git add .` nem `commit -a`;
    - se `.git/index.lock` existir, espere e tente de novo. Nunca apague esse arquivo;
    - mensagem em português, minúscula, no imperativo, sem co-author;
    - push e tag: só o usuário.

    O Bolt commita o `Cargo.lock` por último. A S5 não adiciona dep, então o lock não deve mudar. Se mudar, avise o Keel.

## 12. S6 · interface visual (v1.14)
Meta do usuário: a interface visual completa. App desktop Tauri 2 (`clipsync-app`) com o agente no mesmo processo, ícone na bandeja e uma janela com as 7 telas de `docs/telas.md` e do Figma (`vkXY3Q37AaGe1argTA2STv`). A CLI continua. Agente do app e agente da CLI nunca rodam juntos no mesmo perfil: o socket ou pipe já garante, e o segundo vira cliente.

### 12.1 Posse e ordem
```
crates/clipsync-ui/   lógica das telas, sem Tauri; cargo test no Linux sem sessão gráfica
  src/{lib,api,agent,status,onboarding,devices,pairing,clipboard,events,msg}.rs             [Rivet]
  src/{layout,input}.rs                                                                     [Vector]
  src/{autostart,prefs,diag}.rs                                                             [Flux]
  tests/fixtures/*.json   um exemplo JSON de cada resposta e evento                         [Rivet]
  tests/                  cada dono, os testes dos próprios arquivos
crates/clipsync-app/      Cargo.toml, build.rs, tauri.conf.json, capabilities/, icons/, assets/, src/**   [Flux]
crates/clipsync-app/ui/       frontend estático (HTML, CSS, JS puros), fonts/; é o frontendDist          [Pixel]
crates/clipsync-app/ui-dev/   mock.js, mock/ e a galeria (componentes.html, js/galeria.js), fora do frontendDist (v1.17)   [Pixel]
```
v1.17: nesta rodada de correções da S6, `prefs.rs` e `diag.rs` do `clipsync-ui` passam do Rivet ao Flux, com os testes deles (`tests/prefs.rs` e `tests/diag.rs`).
v1.22: na rodada de fidelidade ao Figma (§12.9 item 5), o front fica assim:
```
crates/clipsync-app/ui/js/telas/{clipboard,diagnostico,preferencias}.js     [Flux]
crates/clipsync-app/ui/css/telas/{clipboard,diagnostico,preferencias}.css  [Flux]
crates/clipsync-app/ui/js/telas/layout.js e ui/css/telas/layout.css         [Vector]
crates/clipsync-app/ui/   todo o resto, inclusive os compartilhados: index.html, css/{tokens,app,components,fonts}.css, js/*.js fora de telas/, fonts/   [Pixel]
crates/clipsync-app/ui-dev/**                                                [Pixel]
```
Quem precisa mudar um arquivo compartilhado (token, componente, `app.js`, mock) pede ao Pixel e não edita. As regras da §12.2 e da §12.7 (Pixel) valem para os três donos.
Grafo: ui -> core + clipboard + input; app -> ui + tauri. O grafo da cli não muda.
Fora dos crates novos: Rivet `clipsync-proto/**` e `clipsync-core/**`; Flux `clipsync-clipboard/**` e `clipsync-cli/**`; Vector `clipsync-input/**`; Maestro `Cargo.toml` raiz, `Cargo.lock`, `.github/workflows/**`, `README.md`, release, instalação local e `scripts/servir-ui-dev.py` (servidor de dev, v1.17); Keel `docs/contrato.md`, `docs/workboard.md`, `docs/telas.md`, `docs/decisoes/**`. Argus dá o veredito por tela e pelo artefato. Sentinel revisa a webview e a superfície de comandos. Git: igual à §11 regra 12.

Ordem: (1) o Rivet entrega o proto da §12.6 primeiro (até 30 min) e marca "proto S6 pronto" no workboard; (2) o resto anda em paralelo. O Pixel não espera ninguém: trabalha com o mock e os tipos da §12.5.

### 12.2 Regras
1. A UI fala com o agente SEMPRE por `control::request`, mesmo com o agente dentro do app. Se um `clipsync run` já está de pé no perfil, o app fica em modo `external` (cliente) e não sobe outro.
2. O agente sobe sozinho quando o app abre e o perfil existe. `agent_start` segue explícito.
3. Texto vindo do par (nome, motivo, preview) passa pelo `clean` da CLI dentro do crate. O front escreve dado só com `textContent`; `innerHTML` com dado é proibido.
4. Comando recebe id, nunca nome: par = `peer_id` (64 hex), com `short` só para exibir; item da caixa = `id` (32 hex) + `short`. Hora sempre em `*_unix_ms`; o front formata no fuso local. Campo sem valor sai como `null`, nunca omitido.
5. Arquivos novos no perfil, gravados por tmp + rename: `ui.toml` (preferências, `prefs.rs`) e `layout.toml` (layout de telas, Vector). O `config.toml` não ganha chave nova.
6. Escopo visível: o que não tem backend nesta versão aparece na tela desabilitado, com o texto "indisponível nesta versão", nunca escondido. A fonte é `app_state.features`. Comando de recurso desligado responde `unavailable`.
7. Notificação nunca leva conteúdo de clipboard. É o único modo.
8. Permissão inicial no pareamento: tudo desligado, o usuário liga (prompt-mestre §2.1, §3 deste contrato). `input_in` exige `acknowledge_full_trust: true`.
9. A montagem dos módulos (`plan` + `build`) fica duplicada no `clipsync-ui` (Rivet), com os 3 testes da CLI. É dívida: unificar depois.
10. Sem unsafe, sem npm, sem bundler, sem conteúdo remoto.

### 12.3 Superfície Rust do clipsync-ui (Rivet)
```rust
pub struct UiOptions {
    pub profile: Option<PathBuf>,       // None = Profile::resolve(None), igual à CLI
    pub poll: Duration,                 // 1 s; testes usam menos
    pub listen: Option<SocketAddr>,     // testes: porta própria no onboarding_create
    pub fake_adapters: bool,            // testes
    pub autostart_dir: Option<PathBuf>, // testes; None = padrão da plataforma
    pub downloads_dir: Option<PathBuf>, // testes; None = XDG_DOWNLOAD_DIR, ~/Downloads ou %USERPROFILE%\Downloads
}
#[derive(Clone)] pub struct Ui { /* Arc por dentro */ }
impl Ui {
    /// Exige runtime tokio multi-thread no contexto (a casca usa o do Tauri). Sobe o agente se o perfil existe e liga a consulta.
    pub async fn start(opts: UiOptions) -> Result<Ui, UiError>;
    /// Entrada única: o JSON de UiRequest. Forma errada = bad_request; cmd desconhecido = unknown_command.
    pub async fn call(&self, req: serde_json::Value) -> Result<serde_json::Value, UiError>;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<UiEvent>;
    /// Para a consulta, a tentativa de pareamento e o agente in_app (os pares recebem Bye).
    pub async fn shutdown(&self);
}
#[derive(Serialize, Debug, Clone)]
pub struct UiError { pub code: String, pub message: String, pub field: Option<String>, pub details: Option<serde_json::Value> }
// UiRequest: #[serde(tag = "cmd", content = "args", rename_all = "snake_case")]; cada args com deny_unknown_fields.
// UiEvent:   #[serde(tag = "event", content = "data", rename_all = "snake_case")].
```

### 12.4 Fio entre o front e a casca
- Um comando Tauri, `clipsync`, com um argumento `req`. JS: `await window.__TAURI__.core.invoke("clipsync", { req: { cmd: "pair_finish", args: { attempt_id: 3, grants: { clipboard_in: true, clipboard_out: false, input_in: false } } } })`. Comando sem argumento: `{ cmd: "app_state" }`.
- Resposta: o objeto de retorno. Erro: a promessa rejeita com `{ code, message, field, details }`. A `message` já vem em português, pronta para a tela.
- Eventos: um nome só, `clipsync://event`, payload `{ "event": "<nome>", "data": {...} }`. JS: `window.__TAURI__.event.listen("clipsync://event", e => ...)`.
- Erros comuns a todo comando: `bad_request`, `unknown_command`, `not_initialized`, `agent_not_running`, `agent_external`, `unknown_peer`, `needs_confirmation`, `unavailable`, `io`, `config`, `internal`.

### 12.5 Comandos, tipos e eventos
Formato: `cmd {args} -> retorno · erros próprios`.

**Comuns (barra lateral e bandeja)**
- `app_state -> AppState`
- `agent_start -> {agent: "in_app"|"external"} · no_usable_module, module_build` (`AgentAlreadyRunning` não é erro: vira `external`)
- `agent_stop -> {agent: "stopped"}` (com agente `external` dá `agent_external`)
- `pause_set {paused} -> {paused}`

**1 · Onboarding**
- `onboarding_start -> {default_name, fingerprint_short, input_supported, already_initialized, defaults: {input, clipboard: false}}`. Gera a identidade em memória: o passo 1 mostra a impressão digital antes de gravar.
- `onboarding_check_name {name} -> {ok: true} · invalid_name`
- `onboarding_create {name, modules: {input, clipboard}, port: 47800, recreate: false} -> {device: DeviceSelf, listen, modules} · invalid_name, no_module, no_usable_module, needs_confirmation`. Grava a mesma identidade do `onboarding_start` e sobe o agente. Perfil existente com `recreate: false` dá `needs_confirmation`: a tela avisa "os pares terão de parear de novo" e reenvia com `true`.
- `onboarding_self_test -> {results: [{module: "clipboard"|"input", status: "validated"|"experimental"|"unavailable"|"not_tested", detail}]}`. Com agente de pé, roda com ele pausado e depois restaura o estado.

**2 · Dispositivos**
- `devices_list -> {agent_running, devices: [Device]}`. Agente parado: lista do `peers.toml` com `state: "offline"`.
- `device_set_perms {peer_id, perms: Perms, acknowledge_full_trust: false} -> {device: Device} · module_off`
- `device_revoke {peer_id, typed_name} -> {removed: true} · name_mismatch`
- `device_pause {peer_id, paused}`: sempre `unavailable` nesta versão (feature `peer_pause`).
- Reparear não é comando: o front abre a tela 3.

**3 · Pareamento** (uma tentativa por vez; no core, o código e o pedido de decisão chegam no mesmo momento)
- `pair_host_start {port: 0, window_secs: 120} -> {attempt_id, port, addresses: ["192.168.0.10:53122"], expires_unix_ms} · pair_busy, bind`. `window_secs` vai de 1 a `PAIR_WINDOW_SECS` (120, v1.17); fora disso dá `bad_request` com `field: "window_secs"`.
- `pair_join_start {addr: "IP:PORTA"} -> {attempt_id} · bad_addr, pair_busy`
- `pair_code_matches {attempt_id} -> {step: "permissions", available: {clipboard, input}, defaults: Perms} · no_attempt, wrong_step` (defaults tudo false)
- `pair_finish {attempt_id, grants: Perms, acknowledge_full_trust: false} -> {step: "waiting_peer"} · no_attempt, wrong_step`
- `pair_cancel {attempt_id} -> {cancelled: true} · no_attempt`
- `pair_state -> {step: "idle"|"listening"|"connecting"|"code"|"permissions"|"waiting_peer", attempt_id, port, addresses, expires_unix_ms, code, peer}`
- Fluxo: evento `pair_code_ready`, depois "São iguais" chama `pair_code_matches`, e "Concluir" chama `pair_finish`. "São diferentes, cancelar" chama `pair_cancel`. O fim vem sempre pelo evento `pair_result`. Sem decisão em 120 s = recusa. Pareou: o crate manda `ReloadPeers`.
- Sair da tela (v1.17, decisão do Pixel aceita pelo Maestro): com a tentativa em `listening`, `connecting`, `code` ou `permissions`, o front chama `pair_cancel`. Em `waiting_peer` não cancela: a decisão local já foi enviada, e o fim fica com a outra máquina e chega pelo `pair_result`.
- Cancelar (v1.23, 13255e3): em `code` ou `permissions`, o `pair_cancel` recusa pelo core (§5 regra 3h): `Confirm{false}`, ack até 2 s e só então o fechamento. Quem cancela recebe `pair_result` `cancelled`; o outro lado recebe `declined_peer`, sem esperar o usuário dele. Em `listening` e `connecting` não há decisão a mandar (v1.24, d7a4d79): a tentativa é largada, e as guardas no drop da §5 regra 3i soltam o endpoint e a porta de pareamento e fecham a conexão com PairFailed nos dois lados. Quem cancela recebe `cancelled`, e a porta fica livre para um novo `pair_host_start`. Depois do cancelamento, `pair_code_matches` e `pair_finish` dessa tentativa dão `no_attempt`.

**4 · Layout de telas** (Vector). Toda escrita grava `layout.toml` e manda `InputReload` ao agente de pé (in_app ou external). Vale na hora, sem reiniciar.
- `input_off` (v1.17) vale para todo comando desta tela, inclusive `lock_to_screen`, `switch_to_device` e `input_test`: módulo de input desligado no `config.toml`, plataforma sem input ou o agente respondendo que o módulo está desligado.
- `layout_get -> Layout · input_off`
- `layout_set_link {id: null|"l2", a: Endpoint, b: Endpoint, base_rev} -> {link: Link, layout_rev, warnings: [{code: "trimmed_by_dead_corner", side: "a"|"b"}]} · same_device, internal_edge_locked, empty_segment, segment_out_of_range, segment_overlap, stale_rev, link_not_found` (validados nesta ordem; `id: null` cria; `link_not_found`, na v1.17, é o `id` que não existe, conferido depois do `stale_rev`)
- `layout_remove_link {id, base_rev} -> {layout_rev} · link_not_found, stale_rev`
- `layout_set_device {peer_id, x, y, hotkey: null|1..9, base_rev} -> {device: LayoutDevice, layout_rev} · hotkey_taken, stale_rev` (o crate encaixa x e y em `snap_px`)
- `layout_set_options {dead_corner_px: 0..32, base_rev} -> {options, layout_rev} · out_of_range, empty_segment, stale_rev` (`empty_segment` lista as ligações em `details.links` e não aplica nada)
- `layout_copy_to_peers {peer_ids: null|[...]} -> {rev, results: [{peer_id, name, status: "offered"|"offline"|"denied"|"unsupported", reason}]} · no_layout` (v1.17: `layout.toml` ainda não gravado)
- `layout_offer_answer {accept, from_peer_id, rev} -> {layout_rev} · no_offer, stale_offer` (v1.19, S6-18, 490335f). `from_peer_id` e `rev` são os da `pending_offer` que a tela mostrou. Sem oferta pendente dá `no_offer`. Pendente de outro par ou de outra revisão dá `stale_offer`, tanto ao aceitar quanto ao recusar, com a oferta pendente em `details: {from_peer_id, rev}`, e nada é aplicado. Aceitar grava no `layout.toml` a oferta conferida, nunca uma que chegou depois.
- `lock_to_screen {locked} -> {screen_locked}` (estado do agente, não persiste)
- `switch_to_device {peer_id} -> {active_peer_id} · offline, not_linked, denied` (v1.17: tela travada, par com versão 1 ou recusa, do par ou do portal, dá `denied`; par pareado e desconectado dá `offline`)
- `input_test {peer_id, seconds: 5..60} -> {stats: {received, lost, out_of_order, p50_us, p95_us, max_us}} · denied, offline` (loopback do §8; pode abrir o diálogo do portal)
- Borda interna não é comando: `layout_get` calcula `locked_edges`, e uma ligação que passou a cruzar borda interna (troca de monitor) vem com `problem: "internal_edge_locked"` para o usuário remover.

**5 · Clipboard**
- `clipboard_get -> {enabled, adapter, classification: "validated"|"experimental"|"unavailable"|"test", mode: "auto"|"manual", last_event, last_block, inbox: [InboxItem], retention: {max_items: 50, ttl_ms: 600000}}`. A lista não leva preview.
- `clipboard_preview {id} -> {preview} · no_item` (até 200 caracteres, limpo)
- `clipboard_set_mode {mode: "auto"|"manual"} -> {mode, restart_needed}`. Grava `[clipboard] mode` no `config.toml`. Agente in_app: o crate reinicia e devolve `false`. Agente external: `true`, e a tela pede para reiniciar o do terminal.
- `clipboard_copy_item {id} -> {state: "applied"} · no_item, wrong_state`. Só item `in_inbox` ou `rejected` (Stale) com texto. Vale pausado, porque é ação explícita, e não gera oferta.
- `clipboard_discard_item {id} -> {removed: true} · no_item`
- `clipboard_send_now {peer_id: null} -> {outcomes: [{peer_id, name, state, reason}]} · no_target, empty_clipboard`
- Desabilitados com "indisponível nesta versão": o modo "com confirmação" (o seletor mostra as três opções), "Pausar captura" e "Modo privado".

**6 · Diagnóstico**
- `diagnostics_get -> {items: [DiagItem], network: {listen, addresses, peers: [{peer_id, name, state, addr}]}, blocks: [Block]}`
- `support_bundle_preview -> {text, redactions: [{kind: "peer_id"|"peer_name"|"ip"|"device_name", count}]}`
- `support_bundle_export -> {path, bytes} · io`. Grava o mesmo texto em `<downloads>/clipsync-suporte-AAAAMMDD-HHMMSS.txt`, sem diálogo. Nunca entram a chave, o `peers.toml` cru, o `portal-tokens.toml` nem conteúdo de clipboard.

**7 · Preferências**
- `prefs_get -> Prefs`
- `prefs_set {autostart, notifications, appearance} -> {prefs: Prefs} · io` (os três opcionais; outro campo dá `bad_request`). Autostart (Flux, `autostart.rs`): no Linux `~/.config/autostart/clipsync.desktop` com `Exec` = `$APPIMAGE` quando existir, senão o executável, mais `--hidden`; no Windows `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` por `reg.exe`. Padrão desligado (consentimento explícito).
- Atalhos aparecem como texto, com "indisponível nesta versão" (feature `global_shortcuts`): no Wayland não há tecla global sem o portal GlobalShortcuts. As ações existem como botões: pausar (barra lateral e bandeja), travar nesta tela e ir para dispositivo (tela 4).

**Tipos**
```
AppState    = {initialized, profile_dir, device: DeviceSelf|null, agent: "in_app"|"external"|"stopped"|"failed", agent_error,
               paused, screen_locked, mesh: {connected, total}, modules: {clipboard, input}, input_supported,
               version: "0.2.0", platform: "linux"|"windows", features: Features}
DeviceSelf  = {peer_id, short, name}
Features    = {input, peer_monitors, layout_links, dead_corners, screen_lock, switch_device, layout_copy, os_notifications,
               global_shortcuts, peer_pause, peer_controls_column, peer_rtt, peer_last_activity, peer_addr_history,
               clipboard_confirm_mode, clipboard_pause_capture, clipboard_private_mode, pair_peer_declined_notice}
               // os 7 primeiros seguem o input do agente (Linux, módulo ligado, InputState respondendo); a linha de baixo é false nesta versão
Perms       = {clipboard_in, clipboard_out, input_in}
Device      = {peer_id, short, name, state: "connected"|"connecting"|"offline"|"blocked", blocked_reason, addr, perms: Perms,
               remote_modules: {clipboard, input}, remote_version, paired_at}
InboxItem   = {id, short, from, received_unix_ms, mime, bytes, state: "received"|"applied"|"in_inbox"|"rejected"|"expired"|"unreachable", reason}
DiagItem    = {area, name, status: "ok"|"warn"|"missing"|"error", detail}
Block       = {at_unix_ms, peer_id, peer_name, module: "clipboard"|"input"|null, reason}
Layout      = {rev, self_id, snap_px, options: {dead_corner_px}, screen_locked, active_peer_id, devices: [LayoutDevice],
               locked_edges: [Endpoint], links: [Link], pending_offer: {from_peer_id, from_name, rev, links}|null}
LayoutDevice= {peer_id, name, is_self, state, x, y, hotkey, monitors_source: "live"|"cached"|"none", monitors: [Monitor]}
Monitor     = {ref, name, index, x, y, width, height, scale}   // ref = "DP-1" ou "#0": o que identifica o monitor no layout
Endpoint    = {peer_id, monitor, edge: "left"|"right"|"top"|"bottom", from, to}   // monitor = ref; from e to em fração 0..1
Link        = {id, a: Endpoint, b: Endpoint, problem}
Prefs       = {shortcuts: {open: "Win+Alt+C", pause_all: "Win+Alt+P", send_once: "Win+Alt+V", lock_screen: "Win+Alt+L",
                           go_to_device: ["Win+Alt+1", "Win+Alt+2", "Win+Alt+3"]},
               autostart: false, notifications: {item_received: true, device_connected: false, block: true},
               appearance: {theme: "system"|"light"|"dark", reduce_motion: false}, language: "pt-BR", version, license: "Todos os direitos reservados"}   // v1.20: até o usuário escolher a licença
```

**Eventos** (`{event, data}` em `clipsync://event`)
1. `peer_connected {peer_id, short, name, addr}`
2. `peer_disconnected {peer_id, short, name}`
3. `peer_blocked {peer_id, name, module, reason, at_unix_ms}`
4. `clip_received {id, short, from, bytes, state}` (sem preview)
5. `pause_changed {paused}` (pega também o `clipsync pause` do terminal)
6. `agent_state {agent, error}`
7. `pair_code_ready {attempt_id, code: "123 456", peer: {name, short, modules: {clipboard, input}}}`
8. `pair_result {attempt_id, outcome: "paired"|"declined_local"|"declined_peer"|"failed"|"expired"|"cancelled", device: Device|null, reason}`
9. `notification {kind: "item_received"|"device_connected"|"block"|"device_removed", title, body}` (já filtrada pelas preferências, sem conteúdo; `device_removed` na v1.16, §12.6)
10. `prefs_changed {prefs}`
11. `resync {}` (o receptor atrasou; o front busca tudo de novo)
12. `layout_changed {rev, reason: "saved"|"zones"|"offer"}`
13. `screen_lock_changed {screen_locked, active_peer_id}`

Fonte: 1 a 6, 12 (zones e offer) e 13 vêm da consulta de 1 s (`Status`; `ClipInbox` com clipboard ligado; `InputState` com input ligado), comparada com a anterior. Funciona igual com agente external. 7 a 11 e 12 (saved) nascem no crate.

### 12.6 Proto, core e módulos
**Proto (Rivet, primeiro; variante nova sempre no fim do enum):**
```rust
// limits.rs
pub const PROTOCOL_VERSION: u16 = 2;   // MIN_PROTOCOL_VERSION segue 1 (decisão 8 da §12.10)
// wire.rs (v1.16)
pub enum CloseCode { /* atuais, 0 a 7 */ Revoked = 8 }   // ALL com 9 itens; reason() = b"revoked"; from_code(8) = Some(Revoked)
pub const MAX_ZONES: usize = 16;
pub const MAX_LAYOUT_LINKS: usize = 64;
// module.rs
pub struct PeerInfo { /* campos atuais */ pub remote_version: u16 }      // do Hello do par
pub enum SendError { /* atuais */ #[error("peer too old")] RemoteTooOld }
pub fn min_version(msg: &ModuleMsg) -> u16;   // 2 para Zones, LayoutOffer e EnterAt; 1 para o resto
// input.rs
pub enum InputMsg { /* atuais */
    Zones { zones: Vec<ZoneInfo> },
    LayoutOffer(LayoutOffer),
    EnterAt { session: u32, edge: Edge, monitor: Option<String>, pos: f32 },
}
pub struct ZoneInfo { pub index: u32, pub name: Option<String>, pub x: i32, pub y: i32, pub width: u32, pub height: u32, pub scale: f64 }
pub struct LayoutOffer { pub rev: u64, pub devices: Vec<OfferDevice>, pub links: Vec<OfferLink> }
pub struct OfferDevice { pub peer: PeerId, pub x: i32, pub y: i32 }
pub struct OfferLink { pub a: OfferEndpoint, pub b: OfferEndpoint }
pub struct OfferEndpoint { pub peer: PeerId, pub monitor: String, pub edge: Edge, pub from: f32, pub to: f32 }
// control.rs
pub enum ModuleCommand { /* atuais */
    ClipApply { id: EventId }, ClipDiscard { id: EventId }, ClipSendCurrent { to: Option<PeerId> },
    InputState, InputReload, InputLock { locked: bool }, InputSwitch { to: PeerId },
    InputOffer { to: Vec<PeerId> }, InputOfferClear,
}
pub enum ModuleReply { /* atuais */ Done, ClipApplied(DeliveryState), InputState(InputState), InputOffered(Vec<OfferOutcome>) }
pub struct InputState { pub local: Vec<ZoneInfo>, pub peers: Vec<PeerZones>, pub screen_locked: bool,
                        pub active_peer: Option<PeerId>, pub pending_offer: Option<PendingOffer> }
pub struct PeerZones { pub peer: PeerId, pub zones: Vec<ZoneInfo> }
pub struct PendingOffer { pub from: PeerId, pub offer: LayoutOffer }
pub struct OfferOutcome { pub peer: PeerId, pub state: OfferState }
pub enum OfferState { Offered, Offline, Denied, Unsupported }
pub struct PeerStatus { /* atuais */ #[serde(default)] pub remote_version: u16 }
pub struct AgentStatus { /* atuais */ #[serde(default)] pub recent_blocks: Vec<BlockRecord> }
pub struct BlockRecord { pub at_unix_ms: u64, pub peer: Option<PeerId>, pub peer_name: Option<String>,
                         pub module: Option<ModuleKind>, pub reason: String }   // reason em português, sem conteúdo
```
- `validate()`: no máximo `MAX_ZONES` zonas e dispositivos, `MAX_LAYOUT_LINKS` ligações; `width` e `height` > 0; `scale` finito e > 0; `pos`, `from` e `to` em 0..=1, com `from < to`; nomes de monitor pela regra do `validate_name`, até `MAX_NAME_BYTES`.
- `authorize_*`, mesma ordem de hoje: `Zones` só SAI para par a quem concedi `input_in` (NotAuthorized); `EnterAt` e `LayoutOffer` só ENTRAM com `input_in`; a pausa bloqueia `EnterAt` como o `Enter`; `Zones` e `LayoutOffer` ignoram a pausa e exigem só o módulo local ligado.
- `MemHub`: `remote_version` padrão 2, com um setter para teste, e as mesmas regras.

**Core (Rivet):**
1. `send_reliable` checa `remote_version >= min_version(msg)` antes de abrir o fluxo; senão `RemoteTooOld` e nada sai. `PeerInfo` e `PeerStatus` levam a versão do Hello do par.
2. `Profile::init_with(opts, identity) -> Result<Identity, CoreError>` (L1); `init` passa a chamar `init_with(opts, Identity::generate()?)`.
3. `recent_blocks`: anel de 20 em memória, o mais novo primeiro. Fontes: entrada recusada pelo `authorize_inbound`, `PeerState::Blocked` e `last_block` novo de módulo.
4. Prazos no `ask_module`: `ClipSendCurrent` = `CLIP_SEND_TIMEOUT`; `InputOffer` 5 s; os demais, `STATUS_TIMEOUT`.
5. Gravar `[clipboard] mode` no `config.toml`: função no core (nome à escolha do Rivet), tmp + rename, mantendo o resto.

**Revogação avisada ao par (v1.16, decisão do Maestro; dono: Rivet, em proto, core e clipsync-ui):**
1. `CloseCode::Revoked = 8`, no fim do enum.
2. No `device_revoke` com o nome certo, A grava o `peers.toml` como hoje e, antes de fechar a conexão, manda `Bye{code: Revoked}` no fluxo de controle, com a mesma folga do `Bye` de shutdown. O fechamento segue o de hoje (`CloseCode::UnknownPeer` no `ReloadPeers`). O gatilho dentro do core é escolha do Rivet.
3. Só par com `remote_version >= 2` recebe `Bye{Revoked}` (decisão 7 da §12.10: o 0.1.0 não decodifica a variante nova e fecharia com `Protocol`). Com par v1, A só fecha.
4. B, ao receber `Bye{Revoked}` de par autenticado (malha, ALPN_MESH, pino conferido), remove A do próprio `peers.toml` pelo caminho do revoke local: grava e recarrega como no `ReloadPeers`. Quem faz é o core, então vale com agente in_app e external. Fora da malha (ALPN_PAIR), nada é removido.
5. A `Ui` de B emite `peer_disconnected` e uma `notification {kind: "device_removed", title: "Dispositivo removido", body: "<nome> removeu este dispositivo"}`, com o nome limpo de A como estava no `peers.toml`. A notificação segue a preferência `notifications.block` e sai uma vez só: a remoção não gera também `peer_blocked`. Como o crate fica sabendo da remoção é escolha do Rivet.
6. B offline na hora do revoke (v1.18, 7c770ef): quando B volta, A recusa o certificado de B, e B não remove A. Registra quem foi recusado: quando um par conhecido rejeita o certificado deste dispositivo (CONNECTION_CLOSE com um dos textos fixos de certificado inválido do rustls, `tls::HANDSHAKE_REJECT_REASONS`, no connect ou já no Hello), entra em `recent_blocks` uma entrada com o par, `module: null` e o motivo "recusou a conexão: este dispositivo não está mais pareado lá" (`msg::PEER_REFUSED_US` do core). É uma por par até uma conexão com ele voltar. Quem recusa não registra nada, porque uma chave desconhecida pode ser qualquer um na rede. Falha de rede não conta. Só conta par que provou a identidade (v1.21, d43a13b, S6-21): o texto do CONNECTION_CLOSE não é autenticado antes do fim do handshake, então, no erro de conexão, o servidor precisa ter passado no pino e assinado o handshake com a chave dele (`PinVerifier::proven`). Depois do handshake, no Hello, a identidade já está provada. Fechamento com o texto certo e sem essa prova não registra nada.

**Clipboard (Flux):**
1. Itens `in_inbox` e `rejected` (Stale) guardam o texto completo até o ttl, o descarte ou a aplicação. Debug redigido.
2. `ClipApply` escreve pelo adaptador com o livro de escritas, marca `Applied` e atualiza o hash atual. Não gera oferta e vale pausado.
3. `ClipDiscard` remove o item e o texto.
4. `ClipSendCurrent` lê o clipboard atual e segue o caminho do `ClipSend` (§7 regra 12). Vazio dá `ModuleReply::Error` com a const pública de `msg.rs`; o crate da UI mapeia pelas consts públicas.
5. `self_test()` estruturado (L7), com a assinatura comum abaixo: escreve texto sintético, confere a leitura e devolve o conteúdo anterior quando havia texto.

**Input (Vector):**
1. `clipsync_input::layout`: tipos, leitura e escrita do `layout.toml`, usados pelo `clipsync-ui/src/layout.rs`. Formato:
   ```toml
   version = 1
   rev = 7
   [options]
   dead_corner_px = 8
   [[device]]
   peer = "<64 hex>"
   name = "pc-pessoal"
   x = 0
   y = 0
   hotkey = 2
   [[link]]
   id = "l1"
   a = { peer = "<64 hex>", monitor = "HDMI-1", edge = "right", from = 0.5, to = 1.0 }
   b = { peer = "<64 hex>", monitor = "eDP-1", edge = "left", from = 0.0, to = 1.0 }
   ```
   Teto de posição (v1.19, S6-19 do Sentinel, e4e6308): `Layout::validate` recusa dispositivo com `x` ou `y` fora de -1.000.000 a 1.000.000 (`clipsync_input::layout::MAX_CANVAS_PX`). O teto vale para o `layout_set_device`, para a oferta aceita no `layout_offer_answer` e para a leitura do arquivo (`invalid_field`). A coordenada da oferta é saturada, nunca dá a volta até zero.
2. Captura contínua: com input ligado e pelo menos uma ligação local no `layout.toml`, o agente instala as barreiras derivadas e captura sozinho. Sem ligação, vale o §8 regra 1 (só loopback).
3. `[[input.barrier]]` manual e ligação no `layout.toml` juntos dão `ModuleError::Config` com motivo.
4. Barreira ganha `peer_monitor`, `peer_from` e `peer_to` opcionais. Posição no destino: `peer_from + t * (peer_to - peer_from)`, com `t = (pos - from) / (to - from)`.
5. `Zones` vai no PeerUp de par com versão 2 e a cada mudança de monitor. `EnterAt` quando a ligação nomeia o monitor de destino. `InputLock`, `InputSwitch`, `InputOffer`, `InputOfferClear` e `InputState` como na §12.5. `InputReload` relê o `layout.toml` e reinstala as barreiras.
6. Listar os monitores locais não pode abrir diálogo do portal só para isso.
7. `self_test` do input (L7), com a assinatura comum abaixo.

**self_test (L7, v1.16): assinatura comum.** O clipboard (Flux) e o input (Vector) exportam, cada um na raiz do próprio crate, a mesma forma:
```rust
pub async fn self_test() -> SelfTestOutcome;
pub enum SelfTestOutcome { Validated(String), Experimental(String), Unavailable(String) }
```
A `String` é o detalhe em português, sem conteúdo do clipboard. O Rivet mapeia no `onboarding_self_test` (§12.5): `Validated` dá `validated`, `Experimental` dá `experimental` e `Unavailable` dá `unavailable`, com a `String` em `detail`. `not_tested` só sai quando o Rivet não chamou o `self_test` daquele módulo.

### 12.7 Casca Tauri e frontend
**Flux:**
1. `crates/clipsync-app/tauri.conf.json`: `productName` "ClipSync", `identifier` "dev.montesouza.clipsync", sem `version` (vem do Cargo.toml), `build.frontendDist` "./ui", `app.withGlobalTauri` true, janela `main` 1280×800, mínimo 960×640, `visible: false` até o front carregar.
2. CSP: `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; base-uri 'none'; form-action 'none'`.
3. Capabilities (v1.17, mínimas; informativo I-S6-1 do Sentinel em 7a722ca, aceito pelo Maestro): só a janela `main`, com `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`, mais só o que o front usar de fato. Lista final (v1.18, c68d980 do Flux, conferida no `capabilities/default.json`): exatamente `allow-clipsync`, `core:event:allow-listen` e `core:event:allow-unlisten`. Permissão nova exige grep no front e mudança de contrato. Sem `core:default` e sem `notification:default`: a notificação sai pelo Rust da casca, que não passa pela ACL. Nenhuma URL remota. O comando `clipsync` vai declarado no manifesto do app (`tauri_build::Attributes::app_manifest`). Regra (v1.15): o Tauri 2.12.5 exige permissão explícita para todo comando declarado no `app_manifest`. Por isso o `allow-clipsync` (gerado pelo build) entra na capability; comando novo no manifesto só funciona com o `allow-<comando>` dele, e isso é mudança de contrato. Navegação para fora do app é bloqueada.
4. Sem a feature `devtools` (o build de debug já tem o inspetor). Sem plugin além do de notificação.
5. Bandeja (`tray-icon`): menu Abrir, "Pausar tudo"/"Retomar" (o texto segue o `pause_changed`) e Sair. No Linux, o clique no ícone não gera evento: tudo vai pelo menu.
6. Fechar a janela só esconde. Sair chama `Ui::shutdown` (Bye aos pares) e encerra. SIGINT e SIGTERM vão pelo mesmo caminho. `--hidden` (autostart) abre sem mostrar a janela.
7. Laço `subscribe` → `emit_to("main", "clipsync://event", ev)`. O `notification` vira notificação do sistema.
8. Os lints do workspace valem. Se uma macro do Tauri esbarrar em `unsafe_code = "forbid"`, pare e fale com o Keel: nada de afrouxar.
9. Ícones: `assets/icon.svg` (quadrado arredondado na cor do acento, duas setas, sem gradiente) → `cargo tauri icon` (tauri-cli =2.12.4) gera `icons/`, commitado.

**Pixel:**
1. `crates/clipsync-app/ui/`: `index.html`, CSS e JS em arquivos próprios (a CSP bloqueia inline), ES modules. Evolui de `docs/telas/index.html` e segue o Figma: Inter no corpo, Geist nos títulos, Geist Mono em código e valores (`tabular-nums`), escala de 8 pt, raio 8 px, uma elevação, tema claro e escuro (`appearance.theme`, `reduce_motion`), textos em português, sem travessão em texto de interface, foco visível, `<meta name="author">` e "feito por montesouza.dev" em Preferências.
2. Fontes woff2 variáveis em `ui/fonts/`, com `OFL.txt` e `FONTS.md` (URL de origem, versão, SHA-256). Sem CDN.
3. Mock e galeria (v1.17, S6-03 do Sentinel): moram em `crates/clipsync-app/ui-dev/` (`mock.js`, `mock/`, `componentes.html` e `js/galeria.js`), fora do `frontendDist`, então não entram no pacote; `ui/` não tem nenhum desses arquivos. O `api.js` falha fechado: sem `window.__TAURI__`, só importa o mock em `http:` com host `127.0.0.1` ou `localhost`; em qualquer outra origem, inclusive a do app, rejeita com erro visível e não importa nada. O mock implementa `invoke("clipsync", {req})` e `listen` com os tipos da §12.5 (depois, com os exemplos de `crates/clipsync-ui/tests/fixtures/`) e cenários por `?cenario=` (vazio, sem perfil, erro, pareamento).
4. Todo botão assíncrono tem estado de carregamento. Toda tela tem estado vazio e de erro, com o `UiError.message`. Recurso com feature false fica visível, desabilitado e com "indisponível nesta versão".
5. Portal (v1.17): `python3 -I scripts/servir-ui-dev.py [porta]` (Maestro, porta padrão 5174, só em 127.0.0.1, sem cache) serve `ui/` por cima de `ui-dev/`: o arquivo de `ui/` vence, e o que não existe lá vem de `ui-dev/` na mesma URL. É exceção à §0 regra 1, só enquanto confere: derrube no fim.

### 12.8 Build, CI e release (Maestro)
1. `default-members` = todos os crates menos `clipsync-app`. Localmente, `cargo clippy --all-targets -- -D warnings` e `cargo test` SEM `--workspace`. O `-p clipsync-app` só roda no CI.
2. Deps novas aprovadas pelo Keel, fixadas com `=`: `tauri =2.12.5` (feature `tray-icon`, padrão mantido), `tauri-build =2.7.4` (build-dependency), `tauri-plugin-notification =2.7.0` e `if-addrs =0.15.0` (só clipsync-ui). Fora desta fatia: os plugins opener, dialog, global-shortcut e autostart, `toml_edit` e a feature `devtools`.
3. CI Linux (`ubuntu-24.04`): apt `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev pkg-config build-essential file`, depois `cargo clippy --workspace --all-targets --locked -- -D warnings` e `cargo test --workspace --locked`. Windows: igual ao de hoje, agora com o app.
4. Release `v0.2.0`, com o `version` do workspace em 0.2.0: os dois pacotes da CLI de hoje mais `cargo install tauri-cli --version =2.12.4 --locked` e `cargo tauri build --bundles appimage,deb` (Linux) ou `--bundles msi,nsis` (Windows). Nomes: `clipsync-app-v0.2.0-linux-x86_64.AppImage`, `clipsync-app-v0.2.0-linux-amd64.deb`, `clipsync-app-v0.2.0-windows-x86_64-setup.exe` e `clipsync-app-v0.2.0-windows-x86_64.msi`, todos no `SHA256SUMS`. A §11 regra 9 continua valendo (sem cache, só `actions/*`, tag = versão). O instalador Windows sai sem assinatura, e o SmartScreen avisa.
5. Instalação aqui: AppImage em `~/.local/bin/clipsync-app.AppImage` (+x) e `~/.local/share/applications/clipsync.desktop` com `Exec`, `Icon` e `Categories=Utility;`. Esta máquina tem libfuse2, WebKitGTK 4.1, libayatana-appindicator3 e a extensão de AppIndicator do Ubuntu ativa.

### 12.9 Critérios de pronto (passa/não passa)
Comandos da raiz, com `export PATH=$HOME/.cargo/bin:$PATH`.
1. **Rivet, proto e core:** `cargo test -p clipsync-proto` e `cargo test -p clipsync-core` verdes com:
   - (a) round-trip postcard de `Zones`, `LayoutOffer` e `EnterAt`;
   - (b) `validate` recusa 17 zonas, `scale` NaN, `pos` 1.5 no `EnterAt` e `from >= to` na oferta;
   - (c) tabela de `authorize_*` dos três (Zones sai só com input_in; EnterAt e LayoutOffer entram só com input_in; pausa bloqueia EnterAt e não Zones);
   - (d) `send_reliable` de `Zones` para par com versão 1 dá `RemoteTooOld`, e nada sai;
   - (e) Hello v2 aceito pela regra de compatibilidade do 0.1.0;
   - (f) `init_with` grava a identidade dada (mesmo id);
   - (g) `recent_blocks` guarda no máximo 20, o mais novo primeiro.
2. **Rivet, clipsync-ui (critério a):** `cargo test -p clipsync-ui` verde, sem sessão gráfica, com agentes reais em perfis temporários, 127.0.0.1 e adaptadores fake. Um teste por fluxo:
   - (1) onboarding: `onboarding_start` e `onboarding_create` dão a mesma impressão digital; perfil existente sem `recreate` dá `needs_confirmation`;
   - (2) dispositivos: `input_in` sem confirmação dá `needs_confirmation`; com confirmação grava o `peers.toml` e o par vê a permissão nova; `device_revoke` com nome errado dá `name_mismatch`, e com o nome certo o par some do status dos dois lados; revogação avisada (v1.16): com B de pé, em até 3 s A sai do `peers.toml` de B, e a `Ui` de B emite `peer_disconnected` e uma única `notification`, de kind `device_removed` e corpo "<nome de A> removeu este dispositivo"; com B parado na hora do revoke, o `peers.toml` de B continua com A depois que B volta;
   - (3) pareamento: duas `Ui` recebem `pair_code_ready` com o MESMO código; `pair_finish` antes de `pair_code_matches` dá `wrong_step`; aceite dos dois lados grava exatamente os grants (padrão tudo false) e emite `pair_result` paired; `pair_cancel` emite cancelled e não grava; (v1.23) `pair_cancel` na tela do código, com os pacotes do host perdidos por 300 ms, dá `cancelled` de um lado e `declined_peer` do outro, sem nada gravado (`cancel_reaches_the_other_side_even_when_packets_are_lost`); (v1.24) `pair_cancel` em `listening`, com o handshake do par travado, dá `cancelled` e a porta de pareamento volta a aceitar `bind` (`cancel_before_the_code_frees_the_pairing_port`);
   - (4) layout: `layout_get` traz o próprio dispositivo (o resto é do Vector);
   - (5) clipboard: B em manual recebe o item (`clip_received` sem preview); `clipboard_get` sem preview; `clipboard_preview` devolve o texto; `clipboard_copy_item` aplica no fake; `clipboard_discard_item` remove;
   - (6) diagnóstico: `support_bundle_preview` não contém nome, id (64 hex ou curto) nem IP dos pares, nem o nome do dispositivo;
   - (7) preferências: `prefs_set` grava o `ui.toml` e sobrevive a uma `Ui` nova; campo desconhecido dá `bad_request`.

   Também: cada `tests/fixtures/*.json` desserializa no tipo do crate; JSON ruim dá `bad_request` e cmd desconhecido dá `unknown_command`; `peer_connected` chega em até 3 s; com outro `agent::run` no mesmo perfil, `app_state` dá `agent: "external"` e `agent_stop` dá `agent_external`.
3. **Vector:** `cargo test -p clipsync-input` e `cargo test -p clipsync-ui --test layout` verdes com:
   - barreiras derivadas das ligações nos dois lados; `[[input.barrier]]` + ligação dá erro de config com motivo; mapeamento `peer_from`/`peer_to`;
   - `Zones` não sai para par sem input_in (MemHub `sent_count` 0) nem para par com versão 1; `LayoutOffer` de par sem input_in é recusado;
   - `InputLock`: ativação não manda `Enter`; `InputSwitch` manda `EnterAt` ao par ligado;
   - validações do `layout_set_link` na ordem da §12.5, `stale_rev`, e o `layout.toml` relido igual depois de gravado.

   M: `cargo test -p clipsync-input -- --ignored portal_loopback` com uma ligação do `layout.toml` no lugar da barreira padrão.
4. **Flux:**
   - `cargo test -p clipsync-clipboard` verde com `ClipApply` (in_inbox aplica no fake sem oferta nova; applied ou expired dá erro), `ClipDiscard` (o item some e o texto sai da memória), `ClipSendCurrent` (manda o conteúdo atual aos pares com clipboard_out; vazio dá erro), texto da caixa descartado no ttl e Debug sem conteúdo;
   - `cargo test -p clipsync-ui --test autostart` verde (`.desktop` com `Exec` = `$APPIMAGE` quando definido, e `--hidden`);
   - CI: `cargo clippy -p clipsync-app --all-targets --locked -- -D warnings` verde nos dois runners.
5. **Pixel (critério b):**
   - as 9 telas (3 passos do onboarding mais 6), nos temas claro e escuro, no portal com o mock, cada uma capturada ao lado do quadro do Figma, com veredito do Argus;
   - (v1.22) a captura é em 1280×800, o tamanho da janela (§12.7 Flux item 1), e a disposição segue o quadro: o que o Figma põe em duas colunas ou numa faixa lado a lado não sai empilhado numa coluna só. Referências do Maestro, app e Figma lado a lado: `~/.cache/clipsync-telas-ref/pares/<tela>-<tema>.png` (fora do repositório). Cada tela responde ao dono dela na §12.1;
   - `grep -rnE 'innerHTML|style=' crates/clipsync-app/ui` não acha nada, e todo `<script>` tem `src`;
   - `grep -rn '—' crates/clipsync-app/ui` não acha nada em texto de interface;
   - nenhuma requisição fora da origem no log do portal; `document.fonts` mostra Inter, Geist e Geist Mono carregadas;
   - cenários vazio e erro conferidos em cada tela.
6. **App real (critérios c e d, Maestro e Argus).** O AppImage do CI roda nesta máquina e:
   - o onboarding cria o perfil;
   - o pareamento com `clipsync pair join` (outro perfil, adaptador fake) mostra o mesmo código nos dois lados;
   - `clipsync send --stdin` chega à caixa do app em modo manual, e "Aplicar" põe o texto no clipboard real; uma cópia no sistema chega ao `clipsync inbox` do outro perfil;
   - o diagnóstico lista os blocos, e as preferências sobrevivem a fechar e abrir;
   - o ícone aparece na bandeja com o menu, e "ClipSync" aparece na barra de aplicações.
7. **Release (critério e):** `gh release view v0.2.0` lista os pacotes da CLI e do app da §12.8 e o `SHA256SUMS`; `sha256sum -c SHA256SUMS` dá OK.
8. **Sentinel (critério f):** revisão da webview (CSP, capabilities, navegação, mock fora do app) e da superfície `clipsync` (validação, limites, textos do par limpos) sem bloqueante.

### 12.10 Decisões da S6
1. Maestro: Tauri 2 num processo só, agente dentro, bandeja, CLI mantida; lógica num crate puro (`clipsync-ui`); casca fina.
2. Maestro: `UiRequest` tipado com um comando Tauri só; erros com código estável; ids em vez de nomes; horas em unix_ms; `clean` no crate e `textContent` no front; UI sempre pelo socket de controle; `ui.toml` separado; eventos por consulta de 1 s; `if-addrs`.
3. Maestro: permissão inicial no pareamento tudo desligado (`docs/telas.md` corrigida). O agente sobe sozinho com perfil existente.
4. Maestro: o que não tem backend aparece desabilitado com "indisponível nesta versão" (L4, L8, coluna "controla", RTT, pausa por par, modo com confirmação, pausar captura, modo privado, atalhos globais).
5. Maestro: `layout.toml` é a fonte única do layout; `[[input.barrier]]` manual continua valendo, mas conflita com ligação. `ModuleCommand::InputReload`. Comandos `set_layout_options`, `lock_to_screen` e `switch_to_device`.
6. Keel, nomes: `set_layout_options` virou `layout_set_options` (prefixo da tela); `move_device` e o atalho do dispositivo viraram `layout_set_device`; `lock_internal_edges` não é comando (o `layout_get` calcula); `InputZones` (L2) virou `InputState`, que também traz trava, par ativo e oferta pendente. L3 (monitores do par, travar, cantos mortos, copiar layout) entra pela decisão 5; enquanto o backend não responde, as features ficam false e a tela mostra "indisponível nesta versão".
7. **Keel, ajuste à decisão do Maestro "variante no fim, sem bump".** O 0.1.0 fecha a conexão com `CloseCode::Protocol` quando um frame de módulo não decodifica (`crates/clipsync-core/src/transport.rs`, leitura do fluxo de módulo). Um 0.2.0 mandando `Zones` a um 0.1.0 derrubaria a conexão a cada reconexão. Por isso as variantes vão no fim, `PROTOCOL_VERSION` sobe para 2 com `MIN_PROTOCOL_VERSION` em 1 (o 0.1.0 aceita: 2 >= 1 e 1 >= 1), e o core só manda variante nova a par com versão 2 (`RemoteTooOld`). O `Enter` não muda de forma: um campo novo quebraria a leitura do `Enter` do 0.1.0 pelo 0.2.0. O monitor vai na variante `EnterAt`.
8. Keel: `Zones` só sai para par a quem concedi `input_in` (só revelo meus monitores a quem pode me controlar); `EnterAt` e `LayoutOffer` só entram com `input_in`.
9. Keel: a captura contínua entra na S6. Com ligação no `layout.toml`, o agente captura sozinho; sem ligação, vale o §8 regra 1.
10. Keel: notificação do sistema por `tauri-plugin-notification`. Sem diálogo de arquivo (o pacote de suporte vai para Downloads) e sem plugin de autostart (feito à mão pelo Flux).
11. Lacunas: L1 `init_with` (Rivet); L2 `InputState` (Vector); L5 `ClipApply`, `ClipDiscard` e `ClipSendCurrent` (Flux); L6 `recent_blocks` (Rivet); L7 `self_test` (Flux no clipboard, Vector no input).
12. Maestro: `plan` + `build` duplicados no `clipsync-ui` por agora. É dívida registrada.
13. Maestro (v1.16): a revogação avisa o par v2 com `Bye{Revoked}`, e o par se remove sozinho (§12.6, dono Rivet). O `self_test` tem uma assinatura só no clipboard e no input, e o Rivet faz o mapeamento.
14. Maestro (v1.17, rodada de correções): capabilities mínimas (§12.7 item 3); mock e galeria em `ui-dev/`, fora do pacote, `api.js` falhando fechado e portal por `scripts/servir-ui-dev.py` (§12.7 Pixel itens 3 e 5); `prefs.rs` e `diag.rs` com o Flux (§12.1); códigos do Vector e teto de `window_secs` registrados na §12.5. Decisão do Pixel, aceita: sair da tela 3 em `waiting_peer` não cancela a tentativa.
15. Maestro (v1.18): com B offline na hora do revoke, quem é recusado registra o par em `recent_blocks`, uma vez por par, e quem recusa não registra (§12.6 revogação, item 6).
16. Maestro (v1.19): a resposta à oferta de layout diz qual oferta o usuário viu (`from_peer_id` e `rev`), e oferta trocada dá `stale_offer` (código do Vector). O teto de posição de 1.000.000 fica no `Layout::validate`, para valer em todo caminho.
17. Maestro (v1.20): até o usuário escolher a licença, o app e o README exibem "Todos os direitos reservados" (Flux no `prefs.rs`, Pixel no mock, Maestro no README).
18. Keel (v1.21): o registro do lado recusado (decisão 15) exige par com identidade provada, pino e assinatura do handshake (S6-21, d43a13b).
19. Maestro (v1.22): rodada de fidelidade ao Figma. O Maestro comparou as 9 telas em 1280×800 com os quadros, e o app empilha numa coluna o que o Figma põe em duas. A posse do front se divide por tela (§12.1): Flux com Clipboard, Diagnóstico e Preferências; Vector com Layout; Pixel com o resto e os compartilhados. Keel: §12.9 item 5 ganha o tamanho da captura e a regra das colunas.
20. Maestro (v1.23): no pareamento, a primeira recusa de qualquer lado encerra os dois na hora. Quem cancela depois do código manda `Confirm{false}` e espera o ack (até 2 s) antes de fechar, e o outro lado vê `declined_peer` (§5 regra 3h e §12.5 tela 3; Rivet, 13255e3, a partir da falha do CI 40680756334).
21. Maestro (v1.24): cancelar em `listening` ou `connecting` solta o endpoint e a porta e fecha a conexão com PairFailed nos dois lados, por guardas no drop (§5 regra 3i e §12.5 tela 3; Rivet, d7a4d79).

## 13. S7 · layout de telas de verdade (v1.25)
Pedido do usuário (2026-10-28, depois de aprovar a S6): "a parte de layout de telas". O cenário real (prompt mestre §3) é o PC Windows (192.168.115.30, 2 monitores, teclado e mouse físicos) controlando o notebook Linux (1 monitor, `eDP-1`) e o terceiro PC (sistema a confirmar, §13.14). Hoje o input só existe no Linux. Meta da S7: o módulo Input nas máquinas do cenário, para o Input Leap poder sair.

Entra: captura e emulação no Windows; teclado entre Windows e Linux (ABNT2 e US, acentos, AltGr); `Zones` em tempo real nos dois sistemas; volta só pelo trecho ligado, paredes e cantos mortos; atalhos no Windows; soltar e reconciliar teclas na troca; arrasto através da borda; tela bloqueada e área de trabalho segura; interface; provas e release v0.3.0.

Fora: X11 e macOS (entram se o terceiro PC pedir); ir de um destino a outro sem passar pela máquina do teclado físico (ex.: notebook direto ao terceiro PC); atalhos globais no Linux (portal GlobalShortcuts); atalho configurável; movimento relativo para jogos; arrastar arquivo ou janela entre máquinas.

### 13.1 Posse e ordem
```
crates/clipsync-win/ (novo)                                            [Flux]    chamadas Win32, unsafe isolado (§13.3), API da §13.5
crates/clipsync-input/src/keymap.rs e tests/keymap.rs                  [Flux]    tradução de teclas e botões, pura (§13.7)
crates/clipsync-input/src/adapter/win/** (cfg(windows))                [Flux]    WinCapture, WinEmulate e o self_test do Windows (§13.6)
crates/clipsync-input/src/adapter/winmath.rs e tests/winmath.rs        [Flux]    decisões puras do adaptador Windows, testadas nos dois CIs
crates/clipsync-input/src/adapter/wincore.rs e tests/wincore.rs        [Flux]    v1.30: núcleo de decisão da captura no Windows (a559bdb, 57dcde0), puro, testado nos dois CIs
crates/clipsync-input/** (o resto: adapter/{mod,fake,portal,session}.rs, lib.rs, module.rs, capture.rs, layout.rs, probe/**, self_test.rs)   [Vector]
crates/clipsync-ui/src/{layout,input}.rs e os testes deles             [Vector]
crates/clipsync-proto/**, crates/clipsync-core/** e o resto do clipsync-ui, inclusive src/{diag,prefs}.rs e tests/{diag,prefs}.rs, que voltam do Flux (v1.26)   [Rivet]
crates/clipsync-ui/src/autostart.rs e tests/autostart.rs               [Flux]
crates/clipsync-cli/**                                                 [Flux]
crates/clipsync-app/ui/** e ui-dev/**                                  [Pixel]   (a divisão por tela da v1.22 acabou com a rodada)
Cargo.toml raiz, Cargo.lock, .github/workflows/**, README.md, release  [Maestro]
docs/contrato.md, docs/workboard.md, docs/matriz-compatibilidade.md, docs/decisoes/**   [Keel]
```
Ordem:
0. O Sentinel revisa a decisão da §13.3 antes da primeira linha do `clipsync-win`. Todo o resto começa já.
1. Primeiro, em até 30 min cada, com marca no workboard: Vector, as mudanças de trait e de fake da §13.6 e o `SUPPORTED` ("trait S7 pronto"); Rivet, o proto v3 da §13.4 ("proto S7 pronto"). O Flux começa pelo `keymap.rs` e pelo `winmath.rs`, que não dependem de ninguém.
2. Depois, em paralelo: Flux (`clipsync-win` depois do aval do Sentinel, adaptador Windows, cli), Vector (módulo e Linux), Rivet (core e ui), Pixel (front com o mock).
3. Argus e Sentinel; Maestro com o v0.3.0; o usuário com o roteiro da §13.12.

### 13.2 Regras
1. O prompt mestre §8 vale inteiro. O destino nunca emula com tela bloqueada, área de trabalho segura (UAC, Ctrl+Alt+Del, logon) ou pausa: responde `Refused` (§13.4), e a origem devolve o cursor. A origem encerra a sessão (`Leave` e `ReleaseAll`) quando a própria tela bloqueia ou a área de trabalho de entrada muda.
2. **Hooks de baixo nível (privacidade).** Eles veem todas as teclas, então:
   - (a) só existem com o módulo input ligado e pelo menos uma ligação local no `layout.toml` (a captura contínua da §12.6 item 2). Máquina que só recebe não instala hook nenhum. Vale a cada mudança, não só na subida (v1.26, R2 do Sentinel): o hook cai na hora quando a última ligação local sai do `layout.toml`, quando o par da ligação é bloqueado ou revogado e quando o módulo desliga. Ligação para par que não está pareado não conta;
   - (b) evento marcado como injetado (`LLKHF_INJECTED`, `LLMHF_INJECTED`) passa direto, sem ser lido: nem o que o ClipSync injeta nem o de outro programa vira captura;
   - (c) fora da captura ativa, o callback devolve "passa". O de mouse só confere o cruzamento das barreiras. O de teclado só atualiza duas coisas: o conjunto do que está pressionado agora (esvazia ao soltar) e o reconhecimento dos atalhos. Do callback só saem `Activated`, `Hotkey` e, na ativação, o pressionar do que está seguro. Esse "seguro" é só modificador e botão, nunca letra. O estado do teclado fora da captura tem tamanho fixo (v1.26, R3): o atalho é função do conjunto pressionado agora mais a tecla que chegou, sem `Vec`, fila, janela de tempo ou contador por tecla, e o tipo do estado é `Copy` e não aloca;
   - (d) durante a captura ativa, cada evento é bloqueado e vira `InputEvent` no canal do módulo, sem cópia;
   - (e) proibido: log, `Debug`, `Display` ou `tracing` com vk, scan code, código evdev ou caractere; buffer, histórico, contador por tecla, arquivo ou telemetria. `Debug` de evento de tecla é redigido, como no `InputEvent`. Por prova, não só por regra (v1.26, R5): `impl Debug` manual de `RawEvent` e da tecla, com teste de que `format!("{:?}")` não contém vk, scan nem extended; no CI, nenhum `tracing::*!`, `println!`, `eprintln!` ou `dbg!` com argumento em `crates/clipsync-win/src/` e em `crates/clipsync-input/src/adapter/win/`; `assert!` e `assert_eq!` com evento só em teste. Prova no CI (v1.30, S7-04, 96c89fa): `scripts/conferir-privacidade.sh`, nos dois sistemas, no código do `clipsync-win`, do `keymap.rs` e de `adapter/win*.rs` e `adapter/win/**`: proíbe `println!`, `eprintln!`, `print!`, `eprint!` e `dbg!`, e só aceita macro de log com um texto fixo, sem argumento;
   - (f) o callback não espera nada (sem lock bloqueante, sem await, sem I/O): manda por `try_send` num canal **limitado** a 8 000 eventos e retorna (v1.26, R4, no lugar do canal sem limite). Se o canal encher, a captura termina (`Leave` e `ReleaseAll`), a fila é descartada e nada sai depois. Na saída de qualquer sessão, o que sobrou na fila nunca é enviado. O Windows remove em silêncio o hook que estoura o `LowLevelHooksTimeout`;
   - (g) parar o módulo, pausar tudo ou sair remove os hooks (Drop do `Hooks`). **Retomar depois da pausa (v1.26, decisão do Maestro sobre o R1 do Sentinel):** Win+Alt+P só pausa, pelo hook. Enquanto está pausado, o adaptador registra Win+Alt+P pelo `RegisterHotKey` do sistema, que não lê tecla e só avisa a combinação exata; o aviso retoma, e o registro sai ao retomar (por qualquer caminho). Se o Windows recusar o registro (I-S7-2 do Sentinel), a retomada fica só pela bandeja e pelo botão Pausar tudo do app, e o app avisa uma vez: "o Windows não liberou Win+Alt+P para retomar: use a bandeja ou o botão Pausar tudo".
3. Unidades: `InputEvent::Motion` e `Scroll` continuam em px lógicos (§4). `ZoneInfo` fica no espaço da plataforma: px lógicos no GNOME; no Windows, px físicos do desktop virtual, com `scale` = DPI/96. Só o dono das zonas converte; a origem não precisa saber o espaço do destino, porque a volta é decidida no destino (§13.6 Vector item 2).
4. Escopo visível (§12.2 regra 6): o que não existe numa plataforma aparece desabilitado, com "indisponível nesta versão".
5. `cfg(windows)` só no `clipsync-win` (funções) e em `adapter/win/`. Todo o resto do código novo compila e testa nos dois sistemas.

### 13.3 Decisão: `unsafe_code = "forbid"` e o Win32 (item 4 do pedido)
**(a) Crate maduro com API segura.** Levantamento de 2026-10-28, na API do crates.io e no código de cada crate:
- `rdev` 0.5.3 (2023-06): bloqueia pelo `grab` (feature `unstable_grab`), mas o hook não para, o callback mora num `static mut`, depende do `winapi` 0.3.9 (parado desde 2020), o mouse absoluto não desconta a origem do desktop virtual (erra com monitor à esquerda do principal) e o teclado sai só por VK. A 0.6 nunca foi publicada;
- `rdevin` 0.1.0 (2025-05): fork de um release só, que se declara instável, com o mesmo `winapi`;
- `willhook` 0.6.3 (2023-12): não bloqueia, sempre chama `CallNextHookEx`;
- `inputbot` 0.6.0 (2023-08): bloqueia só tecla ou botão com bind; movimento, roda e soltura passam;
- `enigo` 0.6.1 (2025-08, ativo): só emula; o absoluto é normalizado pelo monitor principal, sem `VIRTUALDESK`; DPI v1;
- `winapi-easy` 0.3.0 (2025-11): bloqueia, mas não tem `SendInput` de movimento, scan code nem monitores;
- `display-info` 0.5.10: lista monitores, sem aviso de mudança;
- `input-capture` e `input-emulation` 0.4.0 (lan-mouse): cobrem quase tudo, mas são GPL-3.0 (proibido, prompt mestre §5);
- nenhum chama `OpenInputDesktop` nem `WTSRegisterSessionNotification`: a regra do secure desktop exigiria `unsafe` nosso mesmo com (a).

**(b) Crate `windows` da Microsoft, `=0.62.2`,** a versão mais nova e a mesma que o Tauri já puxa no `Cargo.lock`, num lugar só, pequeno e auditado.

Limite técnico, provado pelo Keel com o rustc do projeto: `#[allow(unsafe_code)]` num módulo não compila sob `forbid` (erro E0453), e o Cargo recusa `[lints] workspace = true` junto com `[lints.rust]` no mesmo crate. Então (b) é um crate próprio.

**Recomendação: (b).** Critério: a menor superfície de `unsafe` que cobre o requisito inteiro, com licença compatível e mantida por quem mantém a API do sistema. A opção (a) não cobre o requisito (nenhum crate bloqueia tudo e emula certo no desktop virtual), traz de 28 a 263 blocos `unsafe` de terceiros pouco mantidos e ainda não dispensa `unsafe` nosso.

Como fica:
1. Crate novo `crates/clipsync-win` (Flux). Os tipos compilam em todo sistema; as funções ficam em `cfg(windows)`. A dep `windows =0.62.2` fica declarada com as features só no `Cargo.toml` raiz (7040a2f, v1.27), e o `clipsync-win` a usa com `workspace = true` em `[target.'cfg(windows)'.dependencies]`. Features, exatamente estas: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_Graphics_Gdi`, `Win32_Devices_Display`, `Win32_UI_HiDpi`, `Win32_System_StationsAndDesktops`, `Win32_System_RemoteDesktop`, `Win32_System_Threading` e `Win32_System_LibraryLoader`. Feature nova é mudança de contrato.
2. Lints próprios, sem `workspace = true`: `[lints.rust] unsafe_code = "deny"` e `unsafe_op_in_unsafe_fn = "deny"`; `[lints.clippy]` com as seis do workspace mais `undocumented_unsafe_blocks = "deny"` e `multiple_unsafe_ops_per_block = "deny"`.
3. `unsafe` só em `crates/clipsync-win/src/ffi.rs`, que tem o único `#![allow(unsafe_code)]` do repositório. Um bloco por chamada, cada um com `// SAFETY:` dizendo por que os ponteiros e handles valem. O resto do crate não tem `unsafe` e expõe só a API segura da §13.5.
4. Toda decisão (cruzamento, tradução, coordenadas, atalhos, o que bloquear) fica no `clipsync-input`, sob `forbid`. O `clipsync-win` só chama o sistema e converte tipos.
5. Prova (CI nos dois runners): `grep -rn 'allow(unsafe_code)' crates/` acha exatamente uma linha, em `crates/clipsync-win/src/ffi.rs`; `grep -rnE '\bunsafe\s*(\{|fn|impl|extern|trait)' crates --include='*.rs'` só acha linhas desse arquivo.
6. O Sentinel revisa esta decisão antes da primeira linha do `clipsync-win` e revisa o `ffi.rs` bloco a bloco antes do v0.3.0. **Revisada em 7cd67a3 (`docs/seguranca.md`, "S7 · decisão unsafe"): aprovada com condições.** Condição aberta na revisão bloco a bloco é BLOQUEANTE.
7. **Condições do Sentinel (v1.26):**
   - **C1, conferência do isolamento (Maestro, feita em 2bd35a9 e 3027c09).** O `scripts/conferir-unsafe.sh` roda no CI dos dois sistemas e falha com: `unsafe_code` em qualquer linha `.rs` fora do `ffi.rs` (allow, expect, várias linhas); a palavra `unsafe` no código fora do `ffi.rs` (comentário `//` não conta); crate de `crates/` sem `[lints]` com `workspace = true`, menos o `clipsync-win`; `ffi.rs` sem o `#![allow(unsafe_code)]` ou com mais de uma linha `unsafe_code`; `pub unsafe fn` no `ffi.rs`; e (v1.28, 3027c09) `Cargo.toml` do `clipsync-win` sem `deny` em `unsafe_code` e `unsafe_op_in_unsafe_fn` (`[lints.rust]`) ou em `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block` e nas seis do workspace (`[lints.clippy]`). O Maestro testou afrouxar um e o script acusa; o Keel rodou e passa. C1 fechado.
   - **C2, o `ffi.rs` não exporta `unsafe`.** `mod ffi;` privado. Toda função de lá é segura e fecha a invariante dentro do bloco: nenhum `pub unsafe fn` nem `pub(crate) unsafe fn`. Cada `// SAFETY:` diz por que ponteiro, tamanho de buffer e handle valem, e não só repete a chamada.
   - **C3, estado dos callbacks sem ponteiro cru.** O `Box<dyn FnMut>` dos callbacks mora num `thread_local!` (`RefCell` ou `Cell`), na thread dos hooks, que é a do laço de mensagens. Proibidos `static mut`, `Box::into_raw` e `GWLP_USERDATA` com ponteiro cru. Reentrada: `try_borrow_mut`; se falhar, "passa". Cada `extern "system"` roda dentro de `catch_unwind`: pânico devolve "passa", marca o hook como morto e manda `Notice::HooksLost`, que encerra a sessão (§13.5, v1.27). Pânico que cruza o limite C aborta o processo.
   - **C4, handles com RAII.** `HHOOK`, `HWND`, `HDESK` e o registro do WTS fecham no `Drop`; o `input_desktop_is_default`, chamado antes de cada lote do `WinEmulate`, fecha o `HDESK` que abre. Teste no CI Windows: 10 000 chamadas sem o `process_handle_count()` subir.
   - **Cargo.lock (Maestro, §13.10).** Ao entrar o membro novo, o diff do `Cargo.lock` mostra só o pacote `clipsync-win` e as dependências dele; nenhuma versão muda e nenhum pacote externo aparece. Builds com `--locked`.

   Os informativos I-S7-1 a I-S7-5 estão em `docs/seguranca.md`. Dois viram regra aqui: o teste de injetado cobre `LLKHF_INJECTED` e `LLKHF_LOWER_IL_INJECTED` (I-S7-1); e, com captura ativa, a thread dos hooks confere a cada 1 s se o `GetCursorPos` ainda é o ponto estacionado e, se não for, manda `Notice::HooksLost`, que encerra a captura, porque o hook pode ter sido removido em silêncio (I-S7-3). Na ativação, o conjunto de teclas é reconciliado com `GetAsyncKeyState`, só por `modifier_down` e `button_down` (v1.30, S7-01: nenhuma outra tecla pode ser lida assim).

### 13.4 Proto v3 (Rivet)
`PROTOCOL_VERSION = 3`, `MIN_PROTOCOL_VERSION = 1`. Variantes novas no fim do `InputMsg`:
```rust
/// v3. Entrada com o trecho de volta. `pos`, `back_from` e `back_to` vão de 0 a 1 ao longo da
/// borda inteira do monitor de destino; a origem já descontou os cantos mortos.
EnterSpan { session: u32, edge: Edge, monitor: Option<String>, pos: f32, back_from: f32, back_to: f32 },
/// v3. Destino para a origem: o cursor saiu pelo trecho de volta, em `pos` (0 a 1 na borda inteira).
Returned { session: u32, pos: f32 },
/// v3. Destino para a origem: não emulo nesta sessão.
Refused { session: u32, reason: RejectReason },
```
1. `validate`: `pos`, `back_from` e `back_to` finitos, de 0 a 1; `back_from < back_to`; `monitor` passa no `validate_monitor_name`. O módulo só manda `Refused` com `Locked`, `Paused` ou `ApplyFailed`.
2. `min_version` das três é 3. Mandar a par com versão menor dá `RemoteTooOld`, e nada sai (§12.10 decisão 7).
3. Autorização: `EnterSpan` como `EnterAt` (só entra com `input_in` concedido ao remetente; a pausa bloqueia). `Returned` e `Refused` como `Stats`: passam sem `input_in`, e o módulo só age se a `session` for uma que este nó abriu para aquele par.
4. `ModuleHost` ganha `fn request_pause(&self, paused: bool);`, pelo mesmo caminho do `pause` e do `resume` do controle (`state.toml`, `ModuleInput::Paused` aos módulos, evento `pause_changed`). O `MemHub` implementa e conta as chamadas. Quem usa é o atalho Win+Alt+P.
6. v1.28 (decisão do Vector): `MemHub::unlink` (Rivet, só na feature `testing`) desfaz a ligação entre dois nós, para o teste do gatilho "par revogado" do R2 (§13.2 regra 2a).
7. v1.29 (decisão do Maestro, I-S7-2, a pedido do Pixel em 020451d): campo novo `resume_hotkey_refused: bool` no fim do `InputState` (`control.rs`), com `#[serde(default)]`, que dá false. Agente mais velho que o campo é lido como false. Uso: §13.9 decisão 3.
8. v1.31 (decisão do Maestro, a pedido do Vector em 87da922): campo novo `switch_supported: bool` no fim do `InputState`, com `#[serde(default = ...)]` que dá **true**: agente mais velho que o campo é lido como true, porque antes do campo a troca não era recusada. Uso: §13.9 decisão 4.
5. Hello v3 é aceito pela regra do 0.1.0 e do 0.2.x (3 >= 1).

### 13.5 `clipsync-win`: API segura (Flux)
```rust
// crates/clipsync-win/src/lib.rs. Os tipos compilam em todo sistema; as funções, só em cfg(windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum WinButton { Left, Right, Middle, X1, X2 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum WinKey { Scan { code: u8, extended: bool }, Vk(u16) }
/// O que o hook viu, já sem os eventos injetados. `Debug` de `Key` é redigido.
pub enum RawEvent {
    Move { x: i32, y: i32 },                               // MSLLHOOKSTRUCT.pt, px físicos do desktop virtual
    Button { button: WinButton, down: bool },
    Wheel { delta: i32, horizontal: bool },                // em unidades de WHEEL_DELTA (120)
    Key { vk: u32, scan: u32, extended: bool, up: bool },  // KBDLLHOOKSTRUCT
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Verdict { Pass, Block }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Notice { DisplayChanged, SessionLocked, SessionUnlocked, DesktopSwitched, HooksLost }   // v1.27: HooksLost no lugar do HookDied; sem ResumeHotkey aqui
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor { pub friendly: String, pub device: String, pub path: String, pub x: i32, pub y: i32, pub width: u32, pub height: u32, pub dpi: u32, pub primary: bool }
pub enum Synth { MoveAbs { nx: i32, ny: i32 }, Button { button: WinButton, down: bool }, Wheel { delta: i32, horizontal: bool }, Key { key: WinKey, up: bool } }
#[derive(thiserror::Error, Debug)] #[error("win32 {call}: {code}")] pub struct WinError { pub call: &'static str, pub code: u32 }

#[cfg(windows)] pub struct Hooks;     // Drop: tira os dois hooks, fecha a janela oculta e encerra a thread
#[cfg(windows)] impl Hooks {
    /// Thread própria com janela oculta de nível superior (janela só de mensagem não recebe o
    /// broadcast de WM_DISPLAYCHANGE), WTSRegisterSessionNotification, os hooks WH_MOUSE_LL e
    /// WH_KEYBOARD_LL e o laço de mensagens. `on_input` roda dentro do callback (§13.2 regra 2f).
    /// v1.27: um WM_TIMER de 500 ms confere `input_desktop_is_default()` e manda `DesktopSwitched`
    /// quando o valor muda, porque o Windows não avisa a troca de área de trabalho (o UAC não gera
    /// mensagem). `HooksLost`: pânico num callback (C3) ou o relógio do I-S7-3 (cursor fora do ponto
    /// estacionado com captura ativa); o callback devolve "passa", o hook fica marcado como morto, e
    /// o aviso sai uma vez.
    pub fn start(on_input: Box<dyn FnMut(&RawEvent) -> Verdict + Send>, on_notice: Box<dyn FnMut(Notice) + Send>) -> Result<Hooks, WinError>;
}
#[cfg(windows)] pub struct Watcher;   // igual ao Hooks, sem hook: só Notice (máquina que só recebe), com o mesmo laço de 500 ms
#[cfg(windows)] impl Watcher { pub fn start(on_notice: Box<dyn FnMut(Notice) + Send>) -> Result<Watcher, WinError>; }
#[cfg(windows)] pub fn monitors() -> Result<Vec<Monitor>, WinError>;   // EnumDisplayMonitors, GetMonitorInfoW, GetDpiForMonitor, QueryDisplayConfig
#[cfg(windows)] pub fn virtual_screen() -> (i32, i32, i32, i32);      // SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN
#[cfg(windows)] pub fn cursor_pos() -> Result<(i32, i32), WinError>;
#[cfg(windows)] pub fn send(inputs: &[Synth]) -> Result<usize, WinError>;   // SendInput; MoveAbs com ABSOLUTE|VIRTUALDESK; Key por SCANCODE (+EXTENDEDKEY) ou VK; envio parcial conta como falha (v1.30, aac844e)
#[cfg(windows)] pub fn input_desktop_is_default() -> bool;           // OpenInputDesktop + GetUserObjectInformationW(UOI_NAME) == "Default"; falha = false
#[cfg(windows)] pub struct CursorPark; // enquanto vive: cursor estacionado em (x, y) e escondido (janela própria sob o ponto com cursor vazio)
#[cfg(windows)] impl CursorPark { pub fn new(x: i32, y: i32) -> Result<CursorPark, WinError>; }
#[cfg(windows)] pub fn keyboard_layout() -> Option<String>;           // KLID do GetKeyboardLayoutNameW, só para diagnóstico
#[cfg(windows)] pub struct ResumeHotkey; // v1.27: thread própria com RegisterHotKey(Win+Alt+P) enquanto o módulo está pausado; Drop faz UnregisterHotKey
#[cfg(windows)] impl ResumeHotkey { pub fn register(on_press: Box<dyn FnMut() + Send>) -> Result<ResumeHotkey, WinError>; }   // Err = o Windows recusou: retomada só pela bandeja (I-S7-2)
// v1.30 (S7-01, aac844e): no lugar do async_key_down(vk) livre, que lia qualquer tecla, um tipo fechado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Modifier { LeftShift, RightShift, LeftCtrl, RightCtrl, LeftAlt, RightAlt, LeftWin, RightWin }
#[cfg(windows)] pub fn modifier_down(modifier: Modifier) -> bool;    // GetAsyncKeyState, só para reconciliar na ativação (I-S7-3)
#[cfg(windows)] pub fn button_down(button: WinButton) -> bool;       // idem, para os botões do mouse
#[cfg(windows)] pub fn hotkey_registrable(vk: u32) -> Result<(), WinError>;   // registra Win+Alt+vk e solta: sonda para o CI e o diagnóstico, fora do comportamento do módulo
#[cfg(windows)] pub fn process_handle_count() -> Result<u32, WinError>;   // GetProcessHandleCount, só para o teste do C4
```
Regras: toda função que lê ou escreve coordenadas põe a thread em `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` na entrada e devolve o contexto anterior na saída (o processo não muda; a CLI não tem manifesto). A normalização do `MoveAbs` (0 a 65535 sobre o desktop virtual, com origem negativa) é do `winmath.rs`, não daqui.

### 13.6 `clipsync-input`
**Traits e fakes (Vector, primeiro):**
```rust
pub enum CaptureEvent {                // as quatro de hoje, mais:
    Hotkey(Hotkey),                    // atalho reconhecido; as teclas dele não vão a lugar nenhum
    ZonesChanged(Vec<Zone>),           // monitores mudaram (§13.8)
    SessionLocked(bool),               // tela desta máquina bloqueou (true) ou voltou; área de trabalho segura conta como bloqueio
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Hotkey { GoTo(u8), ToggleLock, TogglePause }
pub enum InputError { /* as de hoje, mais: */ Locked }   // tela bloqueada ou área segura: nada foi emulado
pub trait CaptureAdapter {   /* o de hoje, mais: */
    /// Troca as barreiras sem recriar a sessão (no portal, sem diálogo novo).
    async fn set_barriers(&mut self, barriers: &[BarrierSpec]) -> Result<(), InputError> { Err(InputError::Other("unsupported".into())) }
    /// Devolve o cursor na barreira `barrier`, em `pos` (0 a 1), dentro da zona (no Windows, no
    /// segundo pixel a partir da borda, v1.32).
    async fn release_at(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> { let _ = (barrier, pos); self.release().await }
    /// v1.28 (Vector, no lugar do `set_paused` da v1.26): true quando o adaptador segura hooks do
    /// sistema enquanto roda (Windows). Só então pausar para o adaptador (§13.2 regra 2g); o portal
    /// do Linux não perde a sessão na pausa.
    fn holds_hooks(&self) -> bool { false }
    /// v1.30 (Flux, aprovado pelo Maestro): começa a capturar sem cruzamento, na barreira `barrier`
    /// e em `pos`. O módulo chama depois de um `open()` bem-sucedido por troca (atalho `GoTo` ou o
    /// botão "Ir para dispositivo"), nunca por `Activated`. Windows: o mesmo que um cruzamento.
    /// Linux: o Vector decide. Padrão: nada.
    async fn begin(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> { let _ = (barrier, pos); Ok(()) }
    /// v1.31 (Vector, 87da922): false quando a plataforma só começa a captura por cruzamento (o
    /// portal do Wayland). Aí a troca por atalho ou botão deixaria teclado e mouse nesta máquina,
    /// então é recusada com "ir para o dispositivo sem cruzar a borda: indisponível nesta versão
    /// neste sistema". Padrão true; false no `PortalCapture`, true no `WinCapture` e no fake.
    fn can_begin(&self) -> bool { true }
    /// Pausado e parado: escuta só a combinação de retomar (Win+Alt+P) por um atalho do sistema que
    /// não lê tecla. `Ok(None)`: não existe aqui (retomada pela bandeja ou pelo botão); `Err`: o
    /// sistema recusou a combinação (mesma saída). O receptor entrega `Hotkey(TogglePause)`.
    async fn watch_resume(&mut self) -> Result<Option<mpsc::Receiver<CaptureEvent>>, InputError> { Ok(None) }
    /// Desfaz o que o `watch_resume` registrou.
    async fn unwatch_resume(&mut self) {}
}
pub trait EmulateAdapter {   /* o de hoje, mais: */
    /// Põe o cursor na entrada. Padrão: a rajada relativa de hoje (§8 regra 8); o `entry_events_at`
    /// sai do module.rs para o adapter.
    async fn enter(&mut self, zone: &Zone, zones: &[Zone], edge: Edge, pos: f32) -> Result<(), InputError> { self.emit(&entry_events_at(edge, pos, zone, zones)).await }
    /// Posição real do cursor, quando a plataforma diz (Windows). Portal: None.
    fn cursor(&self) -> Option<(i32, i32)> { None }
    /// v1.27 (Keel, para fechar a §13.9 decisão 1): o layout do teclado desta máquina, quando
    /// conhecido. Portal: o `xkb_symbols` que o EIS anuncia; Windows: o KLID. O módulo copia para o
    /// `InputState.keyboard_layout`.
    fn keyboard_layout(&self) -> Option<String> { None }
}
impl FakeCaptureHandle { pub fn hotkey(&self, h: Hotkey); pub fn set_zones(&self, z: Vec<Zone>); pub fn session_locked(&self, on: bool); pub fn barriers(&self) -> Vec<BarrierSpec>; }
impl FakeEmulateHandle { pub fn set_locked(&self, on: bool); pub fn events(&self) -> Vec<InputEvent>; }
pub const SUPPORTED: bool = cfg!(any(target_os = "linux", windows));   // lib.rs; substitui os dois INPUT_SUPPORTED (cli e ui)
#[cfg(windows)] pub use adapter::{WinCapture, WinEmulate};
```
No `build`, `auto` vira o portal no Linux e o Windows no Windows. `AdapterChoice` ganha `Windows` (`"windows"` no `config.toml`).

**Módulo (Vector):**
1. Destino v3: no `Enter`, `EnterAt` ou `EnterSpan`, se pausado responde `Refused{Paused}`; emulador sem permissão, `Refused{ApplyFailed}`; `enter` ou `emit` com `InputError::Locked`, `Refused{Locked}`, e a sessão local fecha com `release_all`. Datagrama de sessão recusada é descartado e contado. Par v2 não recebe `Refused` (fica como hoje).
2. Volta, no destino (`EnterSpan`): o módulo acompanha o cursor no espaço das zonas locais, com a posição real (`emu.cursor()`) quando houver, senão com a estimativa (ponto de entrada mais a soma dos `Motion`, convertidos ao espaço local e presos à união das zonas). Quando o cursor está no trecho `back_from..back_to` da borda `edge` do monitor de entrada e o movimento empurra para fora mais de `RETURN_SLACK_PX` somados, o destino manda `Returned{session, pos}`, para de emular a sessão e solta tudo. A linha do trecho vale como borda mesmo que outro monitor local encoste ali. Fora do trecho, toda borda é parede.
3. Volta, na origem: para par v3, a origem manda `EnterSpan` com `pos` e o trecho `peer_from..peer_to` da ligação (§12.6 item 4), já sem os cantos mortos (`trim_dead_corner` no comprimento da zona de destino), e não usa o `VirtualCursor`. Ao receber `Returned{pos}`, faz `t = (pos - peer_from) / (peer_to - peer_from)`, chama `release_at(barreira, from + t * (to - from))` e fecha a sessão com `Leave`. Com a trava ligada, `Returned` é ignorado. Para par v2, fica como hoje (`EnterAt` e `VirtualCursor`).
4. Troca: logo depois do `Activated`, o adaptador de captura emite um `Input` de pressionar para cada modificador e botão ainda seguro (assim Shift e arrasto atravessam), e o destino aplica depois de posicionar o cursor. Na volta e em toda troca, o destino solta tudo (`release_all`), como hoje.
5. Atalhos: `GoTo(n)` vira `InputSwitch` para o dispositivo com `hotkey = n` no `layout.toml`; o `n` do próprio dispositivo devolve o cursor. Depois de um `open()` bem-sucedido por troca (o atalho ou o botão "Ir para dispositivo"), o módulo chama `cap.begin(barreira, pos)` (v1.30); por `Activated` não chama, porque o cruzamento já começou a captura. `ToggleLock` inverte o `InputLock`. `TogglePause` chama `host.request_pause(!host.paused())`.
6. `SessionLocked(true)` na origem fecha a sessão com `Leave` e `ReleaseAll` e não deixa barreira ativar até `SessionLocked(false)`.
7. `Zones` em tempo real (§13.8).
8. Tela bloqueada no Linux (`adapter/session.rs`, cfg linux): `LockedHint` da sessão no logind (`org.freedesktop.login1`, a sessão de `XDG_SESSION_ID` ou de `GetSessionByPID`), lido na subida e acompanhado por `PropertiesChanged`. O `PortalCapture` emite `SessionLocked`, e o `PortalEmulate` devolve `InputError::Locked`. Sem logind, trata como desbloqueado e o diagnóstico avisa.
9. Textos novos em `msg.rs`, sem travessão: "<nome> está com a tela bloqueada: o controle voltou para esta máquina", "<nome> está pausado: o controle voltou para esta máquina", "<nome> não deixou emular teclado e mouse".

**Adaptador Windows (Flux, `src/adapter/win/`):**
1. `WinCapture` (nome `"win-hooks"`):
   - `zones()` e `known_zones()` vêm de `monitors()`, sem diálogo; o nome de cada zona sai do `winmath`;
   - `start` sobe o `Hooks`. A barreira é o segmento da §8 regra 7 sobre a zona. Ativa quando o cursor está no último pixel do segmento e o evento empurra para fora, ou quando o ponto novo cruza a linha do segmento (borda entre dois monitores locais). O evento que ativa é bloqueado. v1.27 (Flux, 40070d3): chegar ou ficar no último pixel do trecho já conta como empurrão, porque o hook pode cortar o ponto na borda do desktop; com isso, andar junto à borda pode ativar (a afinar no roteiro, §13.12 passo 2);
   - durante a captura ativa: `CursorPark` no centro do monitor da barreira; delta = ponto do hook menos o ponto estacionado, em px lógicos pela escala desse monitor; roda vira `ScrollDiscrete` com `dy = -delta` no vertical e `dx = delta` no horizontal;
   - na ativação, solta localmente (`send` de soltar, que volta marcado como injetado e o hook ignora) tudo o que estava pressionado, e emite o pressionar dos seguros (item 4 do módulo). Na volta, pressiona de novo localmente o que ainda está fisicamente seguro;
   - `release_at` desfaz o `CursorPark` e põe o cursor na barreira, no segundo pixel a partir da borda, um antes do pixel que ativa a barreira, para o próximo movimento curto não reativar (`winmath::inside_point`, v1.32);
   - `begin(barreira, pos)` (v1.30) faz o mesmo que um cruzamento: `CursorPark` no centro do monitor da barreira, e o local passa a ser capturado e bloqueado;
   - teclas por `keymap::from_windows`; atalhos Win+Alt+1 a 9, Win+Alt+L e Win+Alt+P, com ou sem captura ativa: as teclas do atalho são engolidas, e um toque de tecla neutra evita que o Win solto abra o menu Iniciar. Win+Alt+C e Win+Alt+V ficam "indisponível nesta versão";
   - `Notice::DisplayChanged` vira `ZonesChanged`; `SessionLocked`, `SessionUnlocked` e `DesktopSwitched` viram `SessionLocked(bool)` (`input_desktop_is_default()` decide o valor depois de `DesktopSwitched`);
   - `HooksLost` (v1.31, decisão do Maestro sobre o S7-02, no lugar do "`SessionLocked(true)` mais `Error`" da v1.27): a sessão em curso termina (`Leave` e `ReleaseAll`), e o `WinCapture` recria o `Hooks` na hora. Se cair mais de 3 vezes em 10 s, desiste: para a captura, emite `Error` com o texto fixo "a captura do Windows parou: os hooks do sistema caíram 4 vezes em 10 s; reinicie o módulo ou recarregue o layout" (sem tecla nem código), que o módulo registra como bloqueio do input, e só volta com o módulo reiniciado ou com `InputReload`. A conta das quedas guarda só os horários das últimas quedas, num vetor de tamanho fixo, e zera quando a captura volta;
   - pausa (v1.28, no lugar do `set_paused`): `holds_hooks()` dá true. Ao pausar, o módulo chama `stop` (o `Hooks` sai) e depois `watch_resume`, que faz `ResumeHotkey::register` com um `on_press` que manda `Hotkey(TogglePause)` no receptor devolvido. Ao retomar, por qualquer caminho, o módulo chama `unwatch_resume` (o `ResumeHotkey` sai) e sobe a captura de novo se a regra 2a da §13.2 ainda valer. Se o `register` falhar, `watch_resume` devolve `Err`: a retomada fica só pela bandeja e pelo botão Pausar tudo, e o módulo avisa como bloqueio com o texto da §13.2 regra 2g.
2. `WinEmulate` (nome `"win-sendinput"`):
   - `start` sobe o `Watcher`;
   - antes de cada lote, `input_desktop_is_default()` e a sessão desbloqueada; senão `Err(InputError::Locked)`, sem injetar nada;
   - `Motion`: posição nova = cursor atual mais delta vezes a escala do monitor atual, presa à união dos monitores, mandada como `MoveAbs`. Não usa movimento relativo, para a aceleração do Windows não agir duas vezes;
   - `enter`: `MoveAbs` direto no ponto (monitor, borda, `pos`), no segundo pixel a partir da borda (`winmath::inside_point`, v1.32);
   - teclas por `keymap::to_windows`; `ScrollDiscrete` vira roda com o sinal trocado no vertical; `Scroll` em px vira roda com 120 por 15 px, acumulando o resto (valor inicial, a afinar no roteiro);
   - `release_all` solta o que este emulador pressionou; `cursor()` = `cursor_pos()`.

   **Entregue em c2e73bc (v1.32):** `WinCapture`, `WinEmulate` e o `self_test` do Windows sobre o `clipsync-win`, com `begin`, `RestartPolicy` (a política do `HooksLost`) e `resolve_barriers` no `wincore` (21 testes em `tests/wincore.rs`). Escolhas do Flux fora do texto acima, a afinar no roteiro §13.12:
   - tecla repetida (o KEYDOWN repetido do Windows) vai ao destino uma vez só, e o destino repete sozinho; só as transições saem;
   - ao voltar, o cursor cai no segundo pixel a partir da borda (acima), para não reativar a barreira;
   - ao ativar, solta localmente tudo o que estava pressionado e leva ao destino só modificadores e botões (R3), reconciliados com `GetAsyncKeyState` pelo `modifier_down` e o `button_down`;
   - roda suave em px vira 120 unidades por 15 px, com o resto acumulado;
   - o Win+Alt+P do `RegisterHotKey` reage também a input sintético (o `RegisterHotKey` não distingue) e só alterna a pausa, nada mais.

   **Decisão do Maestro (v1.32), risco registrado:** o trabalhador do adaptador Windows (`adapter/win/`) não ganha trait de plataforma nesta versão, então não roda com um `clipsync-win` falso. A lógica pura fica coberta pelo `wincore` e pelo `winmath` nos dois CIs; o fluxo completo (hooks reais, estacionar o cursor, `SendInput`, troca de área de trabalho) se prova no CI Windows (`crates/clipsync-win/tests/windows.rs`, com a medida de handles separada do aquecimento em a9d0b00) e no roteiro §13.12. Risco: um defeito na costura entre o trabalhador e o `clipsync-win` só aparece no CI Windows ou na máquina do usuário.
3. `adapter::win::self_test()`, chamado pelo `self_test.rs` do Vector: sobe e desce os hooks, lê os monitores e confere a área de trabalho. Dá `Experimental("captura e emulação do Windows prontas, ainda sem prova nesta máquina")` até a matriz registrar o roteiro, ou `Unavailable(motivo)`.
4. `winmath.rs` (puro, todo sistema): nome estável da zona (o nome amigável do `QueryDisplayConfig`; vazio, o `device` sem `\\.\`; repetido ganha " 2", " 3" na ordem do `path`; corte em 64 bytes na fronteira de caractere); zonas a partir dos monitores; cruzamento de barreira; normalização do `MoveAbs`; px físicos para lógicos por DPI; reconhecimento dos atalhos a partir da sequência de teclas.

   Decisões do Flux em 40070d3 (v1.27), além do texto acima:
   - `normalize_abs`: `n = ceil(pixel * 65536 / largura)`, com o pixel preso ao desktop virtual e `n` no máximo 65535. Supõe que o Windows leva `n` ao pixel `floor(n * largura / 65536)`. Pode errar 1 px, a conferir no roteiro (§13.12 passo 3);
   - `barrier_crossing`: chegar ou ficar no último pixel do trecho conta como empurrão (ver `WinCapture` acima; a afinar no roteiro, §13.12 passo 2);
   - `HotkeyTracker` trabalha em código evdev e diz, a cada tecla, `swallow` (engolir), `shortcut` (o atalho reconhecido) e `neutral_tap` (tocar a tecla neutra antes de soltar). A tecla neutra é `VK 0xE8` (`NEUTRAL_KEY`); um toque cobre o Win e o Alt, então nem o menu Iniciar nem o menu do Alt abrem. O estado é `Copy`, de tamanho fixo e sem alocação (R3).

### 13.7 Teclado entre plataformas
1. **Decisão: o fio leva a posição física da tecla** (código evdev, como no §4), e quem recebe aplica o próprio layout. Dead keys, AltGr e ç são do layout do destino, então o destino precisa estar configurado com o layout do teclado físico da origem (§13.14). Motivo: o libei não aceita keymap do cliente (o keymap vai do EIS ao cliente), e o `SendInput` por scan code também usa o layout do Windows de destino. Isso corrige o prompt mestre §2.3 item 7, que supunha mandar o keymap da origem. Traduzir caractere por caractere não cabe nesta versão.
2. `keymap.rs` (Flux, puro, testado nos dois CIs):
   ```rust
   pub fn from_windows(vk: u32, scan: u32, extended: bool) -> Option<u32>;   // None = descartar
   pub fn to_windows(code: u32) -> Option<clipsync_win::WinKey>;             // None = sem tecla no Windows: descartado e contado
   pub fn button_from_windows(b: clipsync_win::WinButton) -> u32;
   pub fn button_to_windows(code: u32) -> Option<clipsync_win::WinButton>;
   ```
   - bloco principal (v1.27, Flux 40070d3): sem E0, aceita 0x01..=0x53 e 0x56..=0x58, iguais ao código evdev (de 0x01 `KEY_ESC` a 0x58 `KEY_F12`), mais 0x73 e 0x7E (ABNT2, abaixo). Sem VK que os explique, 0x54 e 0x55 são descartados: o 0x54 é o Alt+PrintScreen, que entra pelo VK, e o 0x55 não tem tecla nesses teclados;
   - com E0: 0x1C `KEY_KPENTER` (96), 0x1D `KEY_RIGHTCTRL` (97), 0x35 `KEY_KPSLASH` (98), 0x37 `KEY_SYSRQ` (99), 0x38 `KEY_RIGHTALT` (100, AltGr), 0x47 `KEY_HOME` (102), 0x48 `KEY_UP` (103), 0x49 `KEY_PAGEUP` (104), 0x4B `KEY_LEFT` (105), 0x4D `KEY_RIGHT` (106), 0x4F `KEY_END` (107), 0x50 `KEY_DOWN` (108), 0x51 `KEY_PAGEDOWN` (109), 0x52 `KEY_INSERT` (110), 0x53 `KEY_DELETE` (111), 0x5B `KEY_LEFTMETA` (125), 0x5C `KEY_RIGHTMETA` (126), 0x5D `KEY_COMPOSE` (127);
   - ABNT2: 0x73 `KEY_RO` (89, "/ ?"), 0x7E `KEY_KPCOMMA` (121, o "." extra do teclado numérico), 0x56 `KEY_102ND` (86, "\ |");
   - por VK, nos dois sentidos: `VK_PAUSE` com `KEY_PAUSE` (119), `VK_NUMLOCK` com `KEY_NUMLOCK` (69) e, desde a v1.27 (Flux 40070d3), `VK_SNAPSHOT` com `KEY_SYSRQ` (99). Na captura, o `VK_SNAPSHOT` é lido pelo VK, porque o Alt+PrintScreen chega com scan 0x54, que não tem evdev; na emulação, `KEY_SYSRQ` sai como `VK_SNAPSHOT`;
   - descartado: scan code com o bit 0x200 (o LCtrl falso que o Windows gera com o AltGr, 0x21D, e os Shift falsos);
   - botões: `BTN_LEFT` 0x110, `BTN_RIGHT` 0x111, `BTN_MIDDLE` 0x112, `BTN_SIDE` 0x113 (X1, voltar), `BTN_EXTRA` 0x114 (X2, avançar).

   O Flux confere cada valor em `linux/input-event-codes.h` e na Keyboard Scan Code Specification da Microsoft. Testes: ida e volta de toda tecla física do ABNT2 e do US; casos nomeados ç (0x27 com `KEY_SEMICOLON` 39), acento agudo e crase (0x1A com `KEY_LEFTBRACE` 26), til e circunflexo (0x28 com `KEY_APOSTROPHE` 40), aspas (0x29 com `KEY_GRAVE` 41), AltGr, "/ ?" e o LCtrl falso descartado.
3. Linux: a emulação já manda evdev pelo libei (`portal.rs`), sem tradução. O emulador lê o keymap que o EIS anuncia só para o diagnóstico ("layout do teclado nesta máquina", o nome do `xkb_symbols` quando vier). No Windows, o diagnóstico mostra o KLID (`00010416` é Português (Brasil ABNT2); `00000409` é EUA).
4. **P4b** (roteiro §13.12 passos 4 e 8): no editor do destino, o teclado da origem digita ``ação É ç Ç à ü ã õ â ê ~ ^ ´ ` / ? \ |``, e com AltGr: AltGr+Q dá `/`, AltGr+W dá `?`, AltGr+1 dá `¹`, AltGr+2 dá `²`. Passa quando o texto sai igual ao que o teclado do próprio destino produz, nos dois sentidos.

### 13.8 `Zones` em tempo real
1. O módulo republica `Zones` quando o conteúdo muda (nome, posição, tamanho ou escala de qualquer zona), não só a quantidade, como faz hoje o `on_note`, e reinstala as barreiras por `set_barriers`, sem recriar a sessão: no InputCapture v1, sessão nova é diálogo novo.
2. Fontes: o módulo relê `known_zones()` a cada 2 s em toda plataforma, o que cobre o Linux sem sessão (estimativa do `/sys/class/drm`) e a máquina que só recebe. Com sessão de captura, o `ZonesChanged` do adaptador adianta a mudança: no Linux, pelo sinal `ZonesChanged` do InputCapture e um `GetZones`; no Windows, pelo `WM_DISPLAYCHANGE`.
3. A barreira cuja zona sumiu fica inativa até a zona voltar, e a ligação continua no `layout.toml`.

### 13.9 Interface
- **Rivet (`clipsync-ui`):** `INPUT_SUPPORTED` dá lugar a `clipsync_input::SUPPORTED`; `features.global_shortcuts` fica true no Windows com input ligado; o onboarding libera o input no Windows, e o `onboarding_self_test` chama o self_test do input lá; o diagnóstico ganha "layout do teclado" e, no Windows, "atalhos ativos".
- **Vector (`clipsync-ui/src/layout.rs`):** a tela 4 no Windows lista os monitores reais pelo `known_zones()`; `tests/layout_windows.rs` reescrito para input ligado com adaptadores falsos; `input_off` só com o módulo desligado.
- **Flux (cli):** `INPUT_SUPPORTED` dá lugar a `clipsync_input::SUPPORTED`; `clipsync init` oferece input no Windows; `probe input --capture` e `--emulate` passam a existir no Windows (o probe é do Vector).
- **Pixel:** a tela 4 mostra os monitores do Windows com nome e escala quando for diferente de 100%; onboarding e Dispositivos sem "indisponível" no input quando `features` disser; Preferências mostram os atalhos como texto fixo, "ativos nesta máquina" no Windows e "indisponível nesta versão" no Linux; as mensagens de recusa aparecem como bloqueio; o mock ganha `?cenario=windows` (2 monitores, um com escala 1,5).

Decisões do Maestro (v1.27), a pedido do Rivet:
1. **Layout do teclado.** Campo novo `keyboard_layout: Option<String>` no fim do `InputState` (proto, Rivet), com `#[serde(default)]`. Quem preenche é o módulo (Vector): no Linux, o `portal.rs` com o nome do `xkb_symbols` que o EIS anuncia; no Windows, o `WinEmulate` com o KLID de `clipsync_win::keyboard_layout()`. Os dois entregam pelo `EmulateAdapter::keyboard_layout()` (§13.6). O `diag.rs` (Rivet) mostra a partir do `InputState`; com `None`, mostra "agente parado" ou "ainda não anunciado".
2. **Atalhos ativos no Windows.** O diagnóstico diz "ativos" com o agente rodando, o input ligado e pelo menos uma ligação local no `layout.toml`, que é quando o hook existe (§13.2 regra 2a). Senão, diz "inativos" com o motivo. O `features.global_shortcuts` segue o que está acima: Windows com input ligado.
3. **Recusa do Win+Alt+P vira dado (v1.29, I-S7-2, a pedido do Pixel em 020451d).** Ninguém depende do texto do bloqueio para saber que o Windows recusou o atalho de retomar. Ordem: o Rivet primeiro, depois Vector e Pixel.
   - Rivet: `resume_hotkey_refused` no fim do `InputState` (§13.4 item 7) e `features.resume_hotkey_refused` no `clipsync-ui`, copiado do `InputState`; false sem agente;
   - Vector: o módulo põe true quando o `watch_resume` do adaptador devolve `Err`, e volta a false ao retomar ou quando o registro der certo. O bloqueio de input com o texto da §13.2 regra 2g continua, para o Diagnóstico;
   - Pixel: Preferências leem `features.resume_hotkey_refused`, e não procuram mais o texto; o mock `?cenario=windows-recusa` liga o campo.
4. **Troca sem cruzar a borda (v1.31, a pedido do Vector em 87da922).** No Linux, o portal só ativa a captura ao cruzar a barreira, então `can_begin()` é false e "Ir para dispositivo" (e o atalho, onde existir) é recusado com "ir para o dispositivo sem cruzar a borda: indisponível nesta versão neste sistema". Ordem: o Rivet primeiro, depois Vector e Pixel.
   - Rivet: `switch_supported` no fim do `InputState` (§13.4 item 8) e `features.switch_device` no `clipsync-ui`, copiado do `InputState`; false sem agente;
   - Vector: o módulo preenche `switch_supported` com o `can_begin()` do adaptador (false no `PortalCapture`, true no `WinCapture` e no fake por padrão);
   - Pixel: com `features.switch_device` false, os botões "Ir para dispositivo" da tela Layout ficam desabilitados com "indisponível nesta versão neste sistema", o mesmo texto do erro; o mock ganha o campo, false nos cenários Linux e true no Windows.

### 13.10 Deps, CI e release (Maestro)
1. Deps aprovadas pelo Keel, fixadas com `=`: `windows =0.62.2` (só no `clipsync-win`, só `cfg(windows)`, features da §13.3 item 1) e `zbus =5.19.0` (só no `clipsync-input`, só Linux, `default-features = false`, feature `tokio`). As duas já estão no `Cargo.lock`, e nenhuma versão pode mudar lá.
2. Membro novo `crates/clipsync-win` no workspace e no `default-members` (no Linux compila só os tipos).
3. CI: Linux e Windows como hoje; o clippy do Windows cobre o `cfg(windows)`. Passo novo nos dois runners: os greps da §13.3 item 5.
4. Release v0.3.0 (workspace em 0.3.0): os mesmos 6 pacotes, com o nome v0.3.0, e o `SHA256SUMS`. A tag sai com os critérios 1 a 7 da §13.11 passando; o roteiro do usuário (critério 8) pode vir depois do release, como na S6.

### 13.11 Critérios de pronto (passa/não passa)
1. **Rivet:** `cargo test -p clipsync-proto` e `cargo test -p clipsync-core` verdes com: (a) round-trip postcard de `EnterSpan`, `Returned` e `Refused`; (b) `validate` recusa `pos` 1.5, NaN, `back_from >= back_to` e nome de monitor inválido; (c) `EnterSpan` só entra com `input_in` e sem pausa; `Returned` e `Refused` passam sem `input_in`; (d) as três a par com versão 2 dão `RemoteTooOld`, e nada sai; (e) Hello v3 aceito com mínimo 1; (f) `request_pause(true)` pausa os dois módulos e emite `pause_changed`, como o controle.
2. **Vector:** `cargo test -p clipsync-input` e `cargo test -p clipsync-ui --test layout` verdes nos dois CIs, com adaptadores falsos:
   - (a) destino bloqueado (`set_locked(true)`) responde `Refused{Locked}` ao `EnterSpan`, nada é emitido, e a origem fecha a sessão com o texto da §13.6 item 9;
   - (b) empurrar para fora dentro do trecho de volta gera `Returned` com o `pos` certo, e a origem chama `release_at` na posição mapeada; empurrar a mesma borda fora do trecho, ou outra borda, não gera nada; com a trava, `Returned` é ignorado;
   - (c) botão seguro na ativação chega ao destino como pressionar depois do posicionamento; na volta, `pressed_now() == 0` no destino;
   - (d) `hotkey(GoTo(n))` abre sessão no dispositivo com `hotkey = n`; o `n` do próprio fecha a sessão; `ToggleLock` alterna o `InputLock`; `TogglePause` chama `request_pause` (contado no `MemHub`);
   - (e) `set_zones` com a mesma quantidade e outra resolução faz o par v2 com `input_in` receber `Zones` novo em até 3 s, e `barriers()` mostra a geometria nova; sem mudança, nada sai;
   - (f) `session_locked(true)` na origem fecha a sessão com `Leave` e `ReleaseAll` e impede ativação até `false`;
   - (g) par v2: a origem manda `EnterAt` e usa o `VirtualCursor`, sem variante v3;
   - (h) no CI Windows, `layout_windows.rs`: com input ligado e adaptadores falsos, os comandos da tela 4 funcionam; `input_off` só com o módulo desligado;
   - (i) v1.28, pausa: com o fake segurando hooks, pausar chama `stop` e `watch_resume`, e retomar chama `unwatch_resume` e sobe a captura de novo; com o fake sem hooks (como o portal), pausar não para a captura; `watch_resume` com `Err` deixa o aviso da §13.2 regra 2g e (v1.29) `InputState.resume_hotkey_refused` true, que volta a false ao retomar;
   - (j) v1.28, R2: com o fake segurando hooks, a captura para quando a última ligação local sai do `layout.toml`, quando o par da ligação é revogado (`MemHub::unlink`) e quando o módulo desliga;
   - (k) v1.30: `GoTo(n)` com `open()` bem-sucedido chama `begin` no fake uma vez, com a barreira da ligação; o mesmo pelo botão "Ir para dispositivo"; `Activated` não chama `begin`; `open()` que falha não chama;
   - (l) v1.31: com o fake em `can_begin` false (como o portal), o atalho e o botão "Ir para dispositivo" são recusados com o texto da §13.6, sem chamar `begin`, e `InputState.switch_supported` é false; com o padrão, true.
3. **Flux:** `cargo test -p clipsync-input --test keymap --test winmath` verdes nos dois CIs, com os casos da §13.7 item 2 e do `winmath` (nomes estáveis e repetidos; `MoveAbs` nos quatro cantos de um desktop virtual com origem negativa; px lógicos a 96, 120, 144 e 192 dpi; cruzamento no último pixel e na borda entre monitores; Win+Alt+N, L e P reconhecidos e o toque neutro gerado); `cargo clippy` verde no CI Windows com o `clipsync-win` e o adaptador; o `scripts/conferir-unsafe.sh` verde nos dois CIs (§13.3 itens 5 e 7, C1 e C2); v1.26: o estado de teclado fora da captura é `Copy` e sem alocação, e só modificador e botão saem como seguros (R3); `format!("{:?}")` de `RawEvent` e da tecla sem vk, scan nem extended (R5); canal cheio encerra a captura e descarta a fila (R4); no CI Windows, 10 000 chamadas de `input_desktop_is_default()` sem o `process_handle_count()` subir (C4); `grep -rn 'INPUT_SUPPORTED' crates/` não acha nada (com o Rivet); (v1.31, S7-02) teste com o `ffi` falso (no `wincore` ou no `adapter/win/` com o `Hooks` trocado por um falso): 3 `HooksLost` em 10 s recriam o `Hooks`, e cada um fecha a sessão em curso com `Leave` e `ReleaseAll`; a 4ª dentro de 10 s para a captura, não recria e emite o `Error` com o texto da §13.6; depois disso, `InputReload` volta a capturar.
4. **Rivet (ui):** `cargo test -p clipsync-ui` verde nos dois CIs; no Windows, `features.global_shortcuts` true com input ligado, e `onboarding_self_test` traz o input diferente de `not_tested`; (v1.29) `InputState` sem o campo `resume_hotkey_refused` desserializa com false, e `features.resume_hotkey_refused` copia o `InputState` e dá false sem agente; (v1.31) `InputState` sem o campo `switch_supported` desserializa com true, e `features.switch_device` copia o `InputState` e dá false sem agente.
5. **Pixel:** os greps do §12.9 item 5 seguem passando; no portal com o mock, `?cenario=windows` mostra na tela 4 os dois monitores com nome e escala, Preferências mostram os atalhos ativos (e "indisponível nesta versão" no cenário Linux), e o onboarding libera teclado e mouse; (v1.29) `?cenario=windows-recusa` mostra o aviso na linha do Win+Alt+P das Preferências, lido de `features.resume_hotkey_refused`, e nenhum JS das telas procura o texto do bloqueio; (v1.31) nos cenários Linux, os botões "Ir para dispositivo" da tela Layout aparecem desabilitados com "indisponível nesta versão neste sistema", e no `?cenario=windows` ficam habilitados, lidos de `features.switch_device`; veredito do Argus.
6. **Maestro:** CI verde nos dois runners na base da tag; v0.3.0 com os 6 pacotes e `sha256sum -c SHA256SUMS` OK; o diff do `Cargo.lock` da S7 só com o `clipsync-win` e as dependências dele, sem versão mudada (v1.26).
7. **Sentinel:** a decisão da §13.3 revisada antes do `clipsync-win` (feito em 7cd67a3); na revisão final, sem bloqueante, com C1 a C4 e R2 a R5 fechados (condição aberta é BLOQUEANTE) e mais: cada `unsafe` do `ffi.rs` com `SAFETY` correto, os hooks contra a §13.2 regra 2, `Returned` e `Refused` só para sessão do mesmo par, e o bloqueio de tela nos dois sistemas.
8. **Usuário:** o roteiro da §13.12, com a resposta por passo registrada pelo Argus; a matriz ganha a linha de input do Windows.

### 13.12 Roteiro manual do usuário (PC Windows e notebook)
Antes: parar o Input Leap nos dois (servidor no PC, cliente Flatpak no notebook); instalar o v0.3.0 nos dois; PC e notebook pareados; no notebook, em Dispositivos, ligar "pode controlar este dispositivo" para o PC (e no PC para o notebook, para o passo 8); no PC, na tela Layout, ligar a borda inferior do monitor 2 ao topo do `eDP-1`, com o atalho 1 no PC e o 2 no notebook, e copiar o layout para o notebook.

1. Abrir Layout no PC. Deve ver: os 2 monitores do PC com nome e o `eDP-1` do notebook.
2. Levar o mouse à borda inferior do monitor 2; depois à do monitor 1 e aos cantos. Deve ver: o cursor some do PC e aparece no topo do notebook, na posição proporcional; no monitor 1 e nos cantos, nada. Ponto de atenção (v1.27): andar com o mouse rente à borda inferior do monitor 2, sem empurrar para baixo, pode trocar de máquina; anotar se acontece.
3. No notebook, empurrar as bordas esquerda, direita e de baixo; depois subir pela de cima. Deve ver: nas três, nada; pela de cima, o cursor volta ao monitor 2 no ponto correspondente. Ponto de atenção (v1.27): anotar se o cursor volta 1 px fora do ponto esperado.
4. No notebook, num editor, digitar a frase da P4b (§13.7 item 4) e rolar a roda. Deve ver: o texto igual; a página rolando no sentido certo.
5. Segurar Shift no PC, cruzar e digitar uma letra; soltar, voltar e digitar no PC. Depois apertar o botão perto da borda, cruzar segurando e soltar no notebook. Deve ver: maiúscula no notebook e minúscula no PC; a seleção começa na entrada do notebook, e nada fica preso no PC. Também (v1.33, I-S7-16): segurar Shift, atravessar, voltar sem soltar e digitar. Deve ver: a letra sai maiúscula no PC só se o Shift ainda estiver fisicamente apertado. Anotar se sair minúscula com o Shift ainda apertado: o reaperto na volta depende do `GetAsyncKeyState`, e o Flux avisou que ele pode não ver a tecla (falha para o lado seguro, nada fica preso).
6. Win+Alt+2, Win+Alt+1, Win+Alt+L duas vezes, Win+Alt+P; para retomar, Win+Alt+P de novo, ou, se o app avisou que o Windows não liberou Win+Alt+P, a bandeja ou o botão Pausar tudo (v1.26). Deve ver: vai ao notebook e volta; as bordas param e voltam a trocar; pausa (a bandeja mostra Retomar) e retoma; o menu Iniciar não abre.
7. Com o cursor no notebook, bloquear o notebook (Super+L) e tentar cruzar de novo; depois, com o cursor no notebook, Win+L no PC. Deve ver: o cursor volta ao PC com o aviso de tela bloqueada, e a nova travessia é recusada; depois do Win+L, nada é digitado no notebook.
8. Pelo teclado do notebook, cruzar para o PC (o diálogo do portal aparece uma vez) e digitar a frase da P4b no Bloco de Notas; depois abrir algo como administrador no PC e tentar controlar o aviso do UAC. Deve ver: o texto igual; nada injetado na tela do UAC.
9. Com os dois apps abertos, mudar a escala ou a resolução de um monitor do PC. Deve ver: a tela Layout do notebook muda em até 3 s, e a ligação do monitor 2 continua.

O usuário responde por passo ("ok" ou o que viu), e o Argus registra.

### 13.13 Decisões da S7
1. Keel (item 4 do pedido): opção (b), crate próprio `clipsync-win` com o `windows =0.62.2` e `unsafe` só no `ffi.rs` (§13.3). Depende da revisão do Sentinel antes dos builders do Win32.
2. Keel: teclado pela posição física, com o layout do destino valendo (§13.7 item 1). Corrige o prompt mestre §2.3 item 7.
3. Keel: com par v3, a volta é decidida no destino, que conhece as próprias zonas e unidades (`EnterSpan` e `Returned`); a origem só mapeia a posição. Com par v2, fica o `VirtualCursor`.
4. Keel: hook só com ligação local; máquina que só recebe não tem hook.
5. Keel: no Windows, o movimento emulado é absoluto, calculado a partir do delta, para não acelerar duas vezes.
6. Keel: atalhos fixos e só no Windows nesta versão (Win+Alt+1 a 9, L e P).
7. Maestro: proto v3 com o Rivet; release v0.3.0.
8. Sentinel (v1.26, 7cd67a3): a §13.3 (b) e a §13.2 regra 2 aprovadas com as condições C1 a C4 e R2 a R5, escritas na §13.3 item 7 e na §13.2 regra 2. O C1 foi feito pelo Maestro (2bd35a9).
9. Maestro (v1.26), sobre o R1 do Sentinel: pausar remove os hooks, e a §13.2 regra 2g vale. Win+Alt+P só pausa, pelo hook; retomar com Win+Alt+P usa o `RegisterHotKey` enquanto estiver pausado, e o registro sai ao retomar. Se o Windows recusar a combinação, a retomada fica pela bandeja e pelo botão Pausar tudo, e o roteiro §13.12 passo 6 diz isso. Keel: o trait ganha `set_paused` (§13.6) e o `clipsync-win` ganha `ResumeHotkey` (§13.5; na v1.27, `ResumeHotkey::register` com thread própria, no lugar do `Notice::ResumeHotkey`).
10. Flux (v1.27, 40070d3), aceitas pelo Maestro: `VK_SNAPSHOT` entra pelo VK; 0x54 e 0x55 sem VK são descartados; o bloco principal é 0x01..=0x53 e 0x56..=0x58, mais 0x73 e 0x7E (§13.7 item 2). `normalize_abs` com `ceil(pixel * 65536 / largura)`, que pode errar 1 px; `barrier_crossing` conta chegar ou ficar no último pixel como empurrão, o que pode ativar andando junto à borda; `HotkeyTracker` em evdev com `swallow`, `shortcut` e `neutral_tap`, pela tecla neutra `VK 0xE8` (§13.6 item 4 do adaptador Windows). Os dois pontos que podem errar entram como atenção no roteiro (§13.12 passos 2 e 3).
11. Maestro (v1.27), a pedido do Flux para cumprir C3, I-S7-3 e o R1 (§13.5): `Notice::HooksLost` no lugar do `HookDied` (pânico num callback ou o relógio do I-S7-3; o adaptador trata como `SessionLocked(true)` mais `Error`); `ResumeHotkey::register(on_press)` com thread própria, no lugar do `ResumeHotkey::new(&Watcher)` e do `Notice::ResumeHotkey`; `DesktopSwitched` sai de um laço de 500 ms (`WM_TIMER`) que confere `input_desktop_is_default()`. A dep `windows` tem as features só na raiz (7040a2f), e o `clipsync-win` usa `workspace = true`.
12. Maestro (v1.27), a pedido do Rivet (§13.9): `keyboard_layout` no fim do `InputState`, preenchido pelo módulo e mostrado pelo diagnóstico; atalhos "ativos" no diagnóstico do Windows só quando o hook existe.
13. Vector (v1.28), aceito pelo Maestro: no lugar do `set_paused` da v1.26 (decisão 9), o `CaptureAdapter` ganha `holds_hooks`, `watch_resume` e `unwatch_resume`, com padrão vazio e `holds_hooks` true no Windows. A pausa só para o adaptador que segura hooks, e o portal do Linux não perde a sessão. O Flux implementa os três no `WinCapture`. O Rivet põe `MemHub::unlink` na feature `testing` para o teste do gatilho "par revogado" (§13.4 item 6).
14. Maestro (v1.28): o C1 está fechado (2bd35a9 e 3027c09), e o script confere também os lints do `clipsync-win`.
15. Maestro (v1.29, I-S7-2, a pedido do Pixel em 020451d): a recusa do Win+Alt+P vira o campo `resume_hotkey_refused` no `InputState` e em `features`, sem depender do texto do bloqueio (§13.4 item 7 e §13.9 decisão 3). O Rivet vai primeiro e avisa Vector e Pixel.
16. Flux (v1.30), aprovado pelo Maestro: `CaptureAdapter::begin(barrier, pos)`, com padrão vazio, chamado depois de um `open()` bem-sucedido por troca e nunca por `Activated`. No Windows, faz o mesmo que um cruzamento. O Vector põe no trait e decide o Linux (§13.6).
17. Sentinel (v1.30, revisão do `ffi.rs` em c8cf755, sem bloqueante, com quatro BAIXO): S7-01 (o `async_key_down` lia qualquer tecla) virou `Modifier`, `modifier_down` e `button_down`; S7-03 (ponteiro da consulta de monitores) e a parte do crate do S7-02 (teste do pânico nos callbacks) foram corrigidos pelo Flux em aac844e; S7-04 (grep de log no CI) virou o `scripts/conferir-privacidade.sh` do Maestro em 96c89fa (§13.2 regra 2e). Do S7-02 falta a parte do adaptador.
18. Maestro (v1.31): o `adapter/wincore.rs` e o `tests/wincore.rs` ficam com o Flux (§13.1). S7-02, parte do adaptador: `HooksLost` termina a sessão em curso (`Leave` e `ReleaseAll`) e o `WinCapture` recria o `Hooks` na hora; mais de 3 quedas em 10 s param a captura com um bloqueio de texto fixo, até reiniciar o módulo ou `InputReload` (§13.6). Substitui o "`SessionLocked(true)` mais `Error`" da v1.27 (decisão 11).
19. Vector (v1.31, 87da922) e Maestro: `CaptureAdapter::can_begin()`, false no portal do Linux, que só captura ao cruzar a borda; a troca é recusada lá com texto fixo (§13.6). A interface recebe isso como dado: `InputState::switch_supported` (padrão true para agente antigo) e `features.switch_device` (false sem agente), e a tela Layout desabilita "Ir para dispositivo" (§13.4 item 8 e §13.9 decisão 4). O Rivet vai primeiro e avisa Vector e Pixel.
20. Flux (v1.32, c2e73bc), a afinar no roteiro: tecla repetida sai uma vez só; a volta cai no segundo pixel a partir da borda; a ativação solta tudo localmente e leva só modificadores e botões; a roda suave usa 120 por 15 px com resto acumulado; o Win+Alt+P do `RegisterHotKey` reage a input sintético e só alterna a pausa (§13.6).
21. Maestro (v1.32): o trabalhador do adaptador Windows não ganha trait de plataforma nesta versão. O fluxo completo se prova no CI Windows (`tests/windows.rs`) e no roteiro §13.12; o risco fica registrado na §13.6.

### 13.14 Perguntas que só o usuário responde
1. O terceiro PC: sistema e versão, sessão gráfica (Windows, GNOME Wayland, KDE ou X11) e quantos monitores? KDE e X11 ficam fora da S7.
2. Qual o layout do teclado em cada máquina? O ClipSync manda a posição da tecla, e a máquina que recebe usa o próprio layout; com ABNT2 em todas, acento e ç saem certos.
3. A disposição das telas: confirma "borda inferior do monitor 2 do PC leva ao topo do notebook"? Onde entra o terceiro PC?
4. Precisa ir do notebook direto ao terceiro PC, sem passar pelo PC? Fica fora da S7.
5. Precisa controlar, de outra máquina, programas abertos como administrador no PC? O Windows bloqueia essa entrada sem elevar o ClipSync, e a S7 não eleva.


## 14. S8 · diagnóstico e configuração da travessia (v1.34)

Pedido de 2026-10-28: estudar os documentos deixados pelo Claude e realizar os itens pendentes. Escopo desta rodada: os seis itens de `docs/handoff-codex.md` §7 e a investigação da §6. Mantém a stack existente (Rust, Tauri 2, HTML/CSS/JS sem framework). A hipótese de ausência de ligação no PC não é causa confirmada: sem as saídas do Windows e a travessia observada, o defeito real continua em investigação.

### 14.1 Posse e limites

| Responsável da rodada | Arquivos |
|---|---|
| Backend | `clipsync-ui/src/{status,traversal,lib,diag,events}.rs` e testes/fixtures correspondentes; `clipsync-input/src/{diag,lib}.rs`; `clipsync-cli/src/main.rs` e testes correspondentes; `clipsync-app/src/{tray,events,msg}.rs` e testes correspondentes |
| Frontend | `crates/clipsync-app/ui/**` e `ui-dev/**` |
| Arquiteto | `docs/contrato.md`, `docs/workboard.md`, `CONTEXTO.md`, `docs/superpowers/plans/2026-10-28-travessia.md` |
| Input, depois de liberação do Maestro | `clipsync-input/src/module.rs` e teste de regressão do loopback; não disputa `diag.rs`/`lib.rs` com backend |
| Warden e segurança | Novas seções em `docs/verdicts.md` e `docs/seguranca.md`; nunca reescrevem rodadas antigas |
| Maestro | Integração, processos, verificação visual e comandos de validação; nenhum commit, push, tag, release ou instalação nesta rodada |

Os caminhos de crates na tabela são relativos a `crates/`. Não há mudança de dependência, de capability, de `Hello`, de mensagem postcard, de `PROTOCOL_VERSION = 3` ou de `MIN_PROTOCOL_VERSION = 1`. Nenhuma leitura/gravação de `.env`, `config.json`, chaves ou tokens para diagnosticar. Não executar teste de input nem aceitar permissão do sistema automaticamente.

### 14.2 Estado de saída, com evidência disponível

`app_state` ganha o campo tipado `traversal`, com `#[serde(default)]` para leitura dos JSON antigos. Forma:

```text
Traversal = {code, detail, local_links, last_block: string|null, peers: [TraversalPeer]}
TraversalPeer = {peer_id, code, detail, protocol_warning: string|null}
```

`code` é enum serializado em snake_case; `detail` é português limpo, pronto para exibição. Códigos: `not_initialized`, `agent_stopped`, `input_off`, `paused`, `screen_locked`, `layout_error`, `no_local_link`, `peer_offline`, `remote_input_off`, `peer_too_old`, `checking`, `unverified`, `active`. Padrão de campo ausente: `checking`, nunca um estado de sucesso.

Regras:

1. Ordem das condições locais: perfil ausente; agente parado/falho; input desligado ou indisponível; pausa; trava manual; layout ilegível/inválido; nenhuma ligação desta máquina; estado do input não recebido. Só depois avaliar os pares conectados e suas configurações. Um bloqueio local vale para a saída a qualquer par, mesmo se existir uma sessão antiga anunciada.
2. `local_links` conta apenas ligações com uma ponta desta máquina e outra de um par ainda conhecido. Ligações entre dois pares remotos, ou para identidade revogada, não resolvem a falta de saída local. É contagem de configuração, não confirmação de barreira instalada, monitor presente ou hook saudável.
3. `no_local_link`: explicar que o mouse desta máquina não sai pelas bordas. Configurar o layout no computador que tem o mouse físico; uma máquina que só recebe não precisa de ligação local para receber.
4. Por par: offline antes de módulo remoto desligado; protocolo 1 não suporta a travessia contínua por monitor (`peer_too_old`); ausência de ligação entre esta máquina e esse par continua `no_local_link`. Protocolo remoto ausente/desconhecido nunca é inventado.
5. Ligação local, módulos e conexão disponíveis resultam em `unverified`: configuração encontrada, falta testar. `active` significa apenas que o agente anuncia uma sessão de saída para esse par, não prova recebimento de eventos nem travessia funcional.
6. `screen_locked` é a trava manual do ClipSync. Não a apresentar como bloqueio da sessão do sistema. Hooks, saúde da captura, permissão atual do portal e bloqueio do SO não têm telemetria própria nesta rodada; não inferir que estão ativos/liberados pela configuração.
7. `last_block` contém o último bloqueio histórico de input, limpo e identificado como histórico. Não o interpretar por busca em texto nem transformá-lo em falha atual. Uma falha antiga não substitui o estado local atual.
8. `perms.input_in` permite que o par controle ESTA máquina. Não é autorização de saída e não pode bloquear a classificação de saída. Orientar a conceder a permissão na máquina de destino; nunca conceder por conta própria.
9. Dispositivos, Layout e bandeja usam esse mesmo estado. A ação sugerida é escolhida pelo `code` (abrir Layout/Diagnóstico/Preferências, retomar, destravar ou testar), sem comando novo. O polling existente emite o novo evento local `traversal_changed {traversal}` somente quando o resumo muda, inclusive por pausa, trava, agente, pares e layout. Não é mensagem de rede. Frontend e bandeja o consomem; evento perdido provoca recarga pelo `resync` existente.

### 14.3 Ligação simples por posição e estado vazio

Direção nova do usuário nesta rodada: diminuir escolhas, usar a lógica simples do Input Leap; escolher borda não deve ser obrigatório. Esta instrução substitui a exigência de escolha manual de borda no caminho padrão da S7. O suporte a trechos/monitores continua disponível no modo avançado.

1. Depois de parear e de conceder `input_in`, oferecer configuração da posição relativa do computador: à esquerda, à direita, acima ou abaixo. O usuário escolhe destino e posição, sem selecionar monitor ou borda na via padrão.
2. O app deriva a ligação em bordas externas dos monitores reais, na direção escolhida e na direção oposta no destino. Prioriza o monitor alinhado ao outro computador pela distância entre o centro do monitor e o centro da caixa do outro computador, no eixo perpendicular à passagem; empates usam o extremo na direção desejada, o índice e a referência. Assim, abaixo do monitor 2 continua usando sua borda externa mesmo quando o monitor 1 é mais alto. Usa o maior trecho livre do primeiro candidato com espaço útil, excluindo bordas internas travadas, trechos ocupados e cantos mortos; se o candidato não tem espaço, tenta o seguinte. Um trecho só é útil se permanece não vazio após o corte dos cantos mortos e a conversão para f32 usada pelo agente. A prévia mostra os nomes e o efeito da ligação calculada.
3. O botão explícito `Ligar nesta posição` usa os comandos existentes: primeiro `layout_set_device`, depois `layout_set_link` com a revisão retornada pela primeira gravação. Criação usa `id: null`; havendo uma única ligação local com esse par, atualiza seu id. Múltiplas ligações avançadas são preservadas e encaminham ao modo avançado, sem simplificação automática. Se a posição salvar e a ligação for recusada antes da gravação, como em stale_rev, mostrar claramente “posição salva; ligação não alterada”, recarregar o estado e preservar a ligação anterior. Falha ao recarregar o agente ou resposta indeterminada pode ocorrer depois da gravação do arquivo: nesses casos, informar que a posição foi salva e não foi possível confirmar a atualização da ligação, preservar a mensagem do backend e recarregar, sem repetir a escrita automaticamente nem afirmar que a ligação anterior permaneceu. Arrastar pode preencher a sugestão, mantendo confirmação; arrasto/pareamento/permissão/abertura não gravam ligações por conta própria.
4. O perfil gravado é o local: explicar que a configuração deve ser feita no computador com o mouse físico. `input_in` local não identifica essa origem. Não conceder controle, copiar layout ou aceitar oferta automaticamente.
5. Se não houver monitores reais conhecidos ou um trecho externo livre, mostrar o motivo e permitir o editor avançado, sem inventar monitor ou substituir ligação existente. Todas as validações/cantos mortos/stale_rev da §12.5 continuam.
6. O editor manual de monitores/bordas/trechos fica em opção avançada. O fluxo simples funciona sem entrar nele. Sem ligação LOCAL, explicar que o mouse não sai desta tela e orientar o computador com mouse físico; ligações só entre remotos não escondem o aviso.

### 14.4 Aviso de protocolo

`Device.remote_version` é a versão do protocolo da conexão, não a versão do app. Trocar o rótulo ambíguo “Versão do par” por “Protocolo do par”. `protocol_warning` informa quando o protocolo conhecido é menor que 3: protocolo 2 usa a compatibilidade sem o retorno por trecho da S7; protocolo 1 não tem a configuração por monitor. Orientar atualização dos dois apps, sem atribuir `0.2.1` ou qualquer release exato ao número do protocolo. Campo ausente aparece como desconhecido.

### 14.5 Teste guiado de entrada

1. Reusar `input_test {peer_id, seconds: 5}`. O comando já existe; o trabalho é torná-lo descoberto no fluxo de Layout e explicar a ação. Antes do clique, orientar o usuário a empurrar a borda ligada e mover o mouse durante o teste; avisar que pode aparecer o diálogo do portal, que só ele aceita.
2. O teste nunca começa ao entrar na tela, parear, conceder permissão ou selecionar uma sugestão. Botão assíncrono mostra andamento e impede disparo duplicado enquanto aguarda.
3. O loopback usa captura e envio reais e pode controlar o destino. Não prova o retorno pelo trecho da S7 nem o posicionamento no monitor do `EnterSpan`: o teste atual usa `Enter`. Não apresentar p50/p95 entre máquinas como medição de latência de rede confiável, pois os relógios monotônicos dos processos não compartilham origem.
4. `received > 0` comprova somente eventos contabilizados no teste. `received == 0`, ausência de ativação, recusa ou timeout não viram “funcionando”: mostrar resultado inconclusivo ou o motivo da falha e apontar o Diagnóstico. Perdas e fora de ordem continuam visíveis.
5. Correção no `drive` do loopback: `CaptureEvent::SessionLocked(true)` na origem encerra o teste, devolve o cursor, envia `Leave` e `ReleaseAll` quando houver sessão e descarta o restante da fila. Nenhum evento posterior segue ao destino. A falha é a const de tela bloqueada, sem tecla nem conteúdo no log. `SessionLocked(false)` não recomeça um teste encerrado.

### 14.6 CLI com contexto do perfil

Mantém `clipsync_input::diagnose()`. API nova no mesmo crate, sem dependência no core:

```rust
pub async fn diagnose_for(
    profile_dir: &std::path::Path,
    self_id: Option<clipsync_proto::PeerId>,
    status: Option<&clipsync_proto::AgentStatus>,
) -> Vec<clipsync_proto::DiagItem>;
```

A CLI passa o perfil resolvido por `--profile`/ambiente/padrão, consulta `Status` pelo controle e passa identidade quando conhecida. A UI usa o mesmo helper para os itens de configuração local. Sem perfil, identidade, agente ou layout legível, a coleta de sistema continua e explica o que não pôde conferir. O diagnóstico não cria layout, abre permissão, instala hooks nem inicia captura. O item de atalhos do Windows aparece uma vez e descreve as condições encontradas, sem declarar saúde dos hooks a partir delas. Pausa e falta de ligação local precisam aparecer no diagnóstico com contexto.

### 14.7 Critérios de pronto

1. **Backend:** testes de `app_state.traversal` cobrem ausência de perfil/agente/input, pausa, trava, layout inválido/ausente, ligação entre remotos, identidade revogada, par offline, input remoto desligado, protocolos 1/2/3/desconhecido, InputState ausente, configuração sem prova e sessão anunciada. Com `input_in` local false, uma saída configurada não vira recusada. Bloqueio histórico não vira condição atual; strings do par seguem `clean`.
2. **Frontend:** cenários reproduzíveis para nenhuma ligação local, receptor sem layout, protocolo antigo e teste com zero eventos. Conferir posição relativa sem seleção obrigatória de monitor/borda, derivação de trechos externos livres, editor avançado e salvamento somente no clique; `stale_rev` não apaga ligação concorrente. Sem `innerHTML`, `style=`, script inline ou travessão em texto de interface.
3. **CLI/UI:** testes do helper cobrem layout ausente/inválido, identidade ausente, agente parado, módulo desligado, pausa e ligação local; `diagnose --profile` mantém as checagens de sistema sem agente. No Windows, exatamente um item de atalhos e nenhuma afirmação de hook ativo sem observação.
4. **Input:** regressão com fake e MemHub prova encerramento no `SessionLocked(true)` antes e depois de ativar, liberação do que estava pressionado e nenhum evento posterior entregue. O teste deve falhar antes da correção e passar depois.
5. **Integração:** `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, checagem cruzada do Windows dos crates alcançáveis e os scripts de unsafe/privacidade. Tauri e testes `cfg(windows)` só ficam verificados se rodarem no ambiente correspondente; falta de headers/runner é registrada, não convertida em aprovação.
6. **Revisão:** warden por critério e segurança da nova superfície de diagnóstico/configuração e do encerramento do teste. Novas seções em `docs/verdicts.md`/`docs/seguranca.md`, sem reescrever o histórico.
7. **Manual, pendente do usuário:** saídas de versão/status/diagnose e ligação no PC Windows; atualização instalada no notebook; roteiro §13.12; instaladores numa máquina Windows real e as perguntas §13.14. Código e simulador aprovados não fecham esta prova.

### 14.8 Reconciliação do histórico e pendências fora desta rodada

S6 e releases 0.2.x já concluídos; `CONTEXTO.md` automático de 07:54 é anterior à S7. O S7-08 tem correção em `b683c56`, com guarda de parada antes de recriar hooks e remoção depois do join; não reimplementar como se estivesse aberto. Os critérios 1 a 7 da S7 são registrados como aprovados no handoff/vereditos; o critério 8 é manual e continua pendente. A conferência desta rodada ainda não substitui a revisão histórica por uma afirmação de execução real.

Continuam fora deste recorte: decisão de licença e recópia; segurança antiga S1/S5 ainda aberta; cofre de segredos/ACL; centralização de `write_private`; novos sistemas/terceiro PC; localização completa do clap; mudanças de CI/release e publicação. Permanecem no workboard, sem serem marcadas como resolvidas.

### 14.9 Preparação do release v0.3.1

Pedido posterior do usuário: “faça o commit, quero testar no release, instalando a nova versão”. Autoriza commitar a S8 e preparar os pacotes. Versão do workspace e dos oito pacotes locais passa a 0.3.1, sem atualizar dependências. O Maestro possui Cargo.toml/Cargo.lock, README, notas e workflow de release nesta etapa. A restrição de não commitar da implementação inicial não se aplica a este novo pedido.

As notas ficam em `docs/releases/v0.3.1.md`; o job de publicação faz checkout da tag sem persistir credenciais e lê `docs/releases/${TAG}.md`. O job de publicação exige evento push de tag v*; workflow_dispatch nunca publica, mesmo quando aponta para uma tag. Antes da publicação: suíte local, CI Linux/Windows no commit e empacotamento manual do mesmo commit. O push da tag v0.3.1 mantém a confirmação prevista em AGENTS.md. A instalação .deb exige o usuário; esta etapa não altera perfil nem aprova diálogo do sistema.

Publicação confirmada pelo usuário com “publique”, após o CI e o empacotamento manual aprovarem o commit `f700caa`. A tag `v0.3.1` aponta para esse commit; a autorização de publicação exigida por AGENTS.md foi satisfeita.

Nesta publicação, os seis artefatos da run manual aprovada `40783058074` são promovidos ao release pela CLI do GitHub, sem reconstruir nem alterar seus bytes. A tag existente é conferida com `--verify-tag`; o release permanece rascunho durante o upload. O SHA256SUMS é calculado sobre esses arquivos e os hashes recebidos pelo GitHub são conferidos antes de publicar. A execução duplicada de push `40789025116` foi cancelada; CI e empacotamento aprovados do mesmo commit permanecem como evidência. Não há mudança nas regras de build ou no workflow para releases futuros.


## 15. S9 · correções das auditorias e regressões

### 15.1 Escopo, execução e posse
João autorizou os itens 2, 3 e 4 após os relatórios de SEG, Acessibilidade e UX/UI: finalizar documentação, corrigir o app e executar testes. A base de código é `d41aa65` (igual a `f700caa`/v0.3.1); os 16 documentos entregues por DOC são preservados. Os achados mantêm ID e causa, sem somar duplicatas. Hipóteses, decisões de produto que não corrigem um defeito demonstrado e provas físicas ficam identificadas, não aprovadas por inferência.

- O Maestro é dono deste contrato, dos planos e do runtime. DOC encerrou sua posse após revisão dos dois ajustes finais; nova edição documental exige coordenação.
- Primeira fatia: H-01 e B-02, com implementador nativo backend e revisão independente Sentinel/warden. Posse exclusiva: `clipsync-core/src/transport.rs`; `clipsync-proto/src/{module,testing,lib}.rs` (tipos/API LOCAL e MemHub, sem wire); `clipsync-input/src/{module,capture,msg,network,lib}.rs` e `adapter/fake.rs` (fronteira stop/join/fila do adaptador de testes), `adapter/session.rs` e `adapter/win/capture.rs` (snapshot inicial verdadeiro/falso de disponibilidade, sem mudar ffi), `input/src/diag.rs` somente texto do fallback fechado; `input/src/adapter/portal.rs` somente ordem watcher antes do produtor/enable; `clipsync-win/src/{lib,ffi}.rs` somente consulta segura do estado WTS (§15.9); `clipsync-clipboard/src/module.rs` somente padrões de envelope; testes/fixtures afetados em proto/core/input/clipboard. `network.rs` é novo; `lib.rs` só registro/reexportação necessária. Mudança adicional de arquivo exige coordenação.
- A próxima fatia de módulos/UX/acessibilidade recebe contrato próprio antes do código; inspeção e matriz de cobertura podem ocorrer em paralelo. O andar TESTES prepara cenários e executa a janela final combinada, sem implementar nos arquivos dos builders.
- Sem mudança de dependências, versão de rede, permissões de pares ou versão publicada nesta primeira fatia. Sem commit/push/tag/release. Perfis reais, segredos e consentimentos do sistema não entram nas sondas automatizadas.

### 15.2 H-01 · perda de autorização de entrada
A especificação técnica vinculante é `docs/superpowers/specs/2026-10-28-s9-seguranca.md`, incluindo seu adendo de admissão. Não há mudança de wire, versões ou dependências; há ampliação de API LOCAL entre host e módulos.

1. `ReceiveContext` carrega geração de conexão, época local de autorização e índice de stream. O host oferece snapshot coerente da autoridade vigente, envio fixado à conexão e rejeição antiga fixada à conexão. O envelope é capturado no ingresso, antes de leitura/entrega bloqueantes; não usar caches `self.peers/self.paused` como autoridade. Core e MemHub reproduzem as mesmas fronteiras.
2. Revogar entrada, pausar, substituir conexão ou remover/reintroduzir identidade invalidam épocas antes dos eventos. Conferir contexto/autoridade antes e depois de `emu.start/enter/emit` assíncronos. Sessão invalidada é retirada e os pressionados liberados antes da notificação; reautorizar não recupera sessão antiga. `PeerUp` atrasado reconcilia com o host atual. `input_in` não governa saída. Se o evento de reconciliação for retido/perdido por contrapressão, on_tick revalida a autoridade vigente a cada 2 s e solta sessão inválida antes de rede; esse fallback limitado não é reação instantânea nem prova física.
3. Mensagens admitidas localmente antes de revogar/reautorizar não reabrem sessão. Sem campo wire, não se conhece a época remota de mensagem que só chega depois da nova concessão. Movimento síncrono que coincide com revogação não é desfeito; a conclusão inválida não deixa sessão/pressionados ativos nem autoriza os próximos eventos.
4. v3 usa `Refused { session, reason: NotAuthorized }`; v1/v2 conservam `Frame::Rejected/NotAuthorized` existente. A restrição v3 não suprime rejeição compatível. Envios velhos nunca usam a conexão substituta. Rejeição antiga sem sessão pode cancelar conservadoramente a tentativa atual do par, conforme limite do wire.
5. Aceite: Enter consumido antes de PeerUp e após mudança no host; revogar/reautorizar em cada await; tecla/botão/datagrama antigo; conexão substituída; novo Enter válido; PeerUp atrasado; versões 1/2/3; saída independente. Adaptadores falsos provam transições, não soltura física.

### 15.3 B-02 · prazos, cancelamento e liberação local
1. Envio QUIC tem prazo TOTAL de 2.000 ms desde antes de `open_uni`, incluindo escrita/finalização. Erro, cancelamento ou timeout reseta stream parcial; retorno de finish não é ACK. Mensagens de erro são fixas, sem payload. Reset não desfaz Enter completo já entregue.
2. Tentativa conserva id, par, conexão e sessão até cancelamento/limpeza. Cancelar invalida a tentativa, libera captura/estado local e resolve Switch com erro antes de rede; aborta e aguarda abertura. Sucesso tardio é descartado. Leave limitado continua necessário quando a conexão existe; nova abertura do mesmo par respeita a ordem do encerramento anterior. ReleaseAll não soma esperas seriais nem equivale a Leave.
3. Receptor guarda fronteiras monotônicas `closed_through/latest_enter` por par/conexão usando índice do stream QUIC. Leave antes de Enter tardio impede ressuscitar; Enter velho não substitui o novo; Leave velho não fecha sessão nova. Fronteira persiste pela reautorização na mesma conexão. Receptor antigo não tem essa proteção: garantia completa exige ambos atualizados, sem inferir patch por versão wire 3.
4. Fila temporária contém até 256 eventos INPUT; controles/ciclo de vida têm processamento prioritário fora dela. Cancelar cobre pausa, Stop, trava manual/sessão, PeerDown, Home, Refused/Rejected, retarget que invalida destino, Deactivated, erro/EOF do adaptador e overflow. Descartar o rx velho, parar/aguardar produtor e criar rx novo antes de aceitar nova ativação; não basta limpar VecDeque. Aplicar também ao loopback; falha de rearmar não reutiliza receptor duvidoso. Exceção delimitada: após SessionLocked do SO, parar/aguardar o produtor antigo e criar novo rx/produtor apenas para observar ciclo de vida/desbloqueio, mantendo o gate session_locked fechado e descartando Input/Activated sem abrir sessão. Pausa global, parada e trava manual não usam essa exceção. Ao retomar, o watcher entrega snapshot inicial explícito, tanto bloqueado quanto desbloqueado, antes de aceitar ativação; estado desconhecido/falha de consulta nunca vira desbloqueado por padrão.
5. Cada controlador possui no máximo uma abertura e um encerramento; encerramento tem orçamento total próprio de 2 s. Bridge LOCAL de Switch/hotkey pode aguardar até 5 s (encerramento anterior 2 s + abertura 2 s + grace 400 ms), cancelável imediatamente por controle/ciclo de vida. Timeout/drop invalida a tentativa e responde erro, nunca sucesso tardio. Esse limite não amplia os 2 s de nenhum envio/reply/cleanup QUIC. Respostas/anúncios têm dono (`JoinSet`) limitado a 32; migrar os spawns/awaits de rede pertinentes, inclusive send_v3/Stats/Zones e bridges. Ao encerrar: liberar localmente, resolver respostas, cancelar e aguardar filhos. Nenhuma fila, resposta ou tarefa migra para outro par/sessão. Uma future possuída pelo laço pode substituir tarefa spawnada: cancelamento por drop deve executar cleanup/reset RAII e não deixar trabalho filho; join é obrigatório onde houver tarefa criada.
6. `uni_task` tem até 32 filhos por conexão. Cada filho tem prazo TOTAL de 2.000 ms desde admissão, cobrindo leitura E entrega ao módulo. Timeout fecha a conexão, invalida geração no host, propaga PeerDown e cancela/aguarda filhos; não descarta Leave silenciosamente mantendo conexão viva. Saturação não impede observar prazos/término. O adendo da spec define a política e os limites.
7. Aceite: sete grupos de regressões da spec, incluindo crédito esgotado, escrita parcial/reset/drop, sucesso de Enter simultâneo ao cancelamento, Leave/Enter fora de ordem, 255/256/257 eventos com controles prioritários, rx velho, Switch/grace, Leave bloqueado seguido de Stop sem esperas seriais, zero tarefas órfãs e saturação de 32 leitores/entregas. Redes externas, adaptadores reais e incidente físico permanecem pendentes.

### 15.4 S9 · interface e acessibilidade
A especificação `docs/superpowers/specs/2026-10-28-s9-ux-acessibilidade.md` e a matriz `2026-10-28-s9-cobertura.md` registram a cobertura e os limites das auditorias. Regras desta fatia prevalecem sobre propostas conflitantes de notas: manter o fluxo simples por posição, consentimento explícito e distinção entre configuração, eventos e prova física. Os seis P1 de UX e os seis fluxos de foco têm aceites próprios; 42 entradas brutas não são 42 defeitos únicos.

1. Operação assíncrona mantém foco e bloqueia reentrada sem desabilitar involuntariamente o controle ocupado. Preservar indisponibilidade real. Não restaurar foco se o usuário o moveu de propósito, trocou de rota ou abriu outro diálogo.
2. Repintura usa chaves lógicas estáveis, mantém seleção/details/scroll e alvo lógico. Fechamento de diálogo volta ao gatilho conectado ou ao próximo alvo adequado; confirmação de confiança começa em Cancelar. Nunca usar identificador remoto como seletor sem escape.
3. Avisos vivem em regiões persistentes, inicialmente vazias. Erro assertivo; progresso/resultado polido e único. Polling silencioso não repete anúncio nem rouba foco. Código de pareamento não usa nome acessível em paragraph.
4. Campos habilitados têm contorno pelo menos 3:1 nos dois temas; foco/seleção usam indicador distinto além de cor. Rótulos, estados e expansão mantêm semântica compatível. A tabela não vira grid para satisfazer a alegação refutada Access P2-1.
5. Caption assistivo não pode criar rolagem externa que desloca a navegação. Conteúdo real longo continua rolável; reflow em 960, 640 e 320 preserva ações e textos, fora das exceções delimitadas de canvas/tabela.
6. Editor avançado deverá oferecer clique simples equivalente ao arrasto, com origem/destino/trechos e posição fina, mantendo revisão, geometria e limites atuais; o caminho simples não ganha borda obrigatória. `pointercancel` desfaz sem salvar.
7. C01 exige os ajustes da revisão técnica antes do código. C14 foi aprovado com condições registradas na §15.5. Demais alterações de comportamento entram em adendo desta seção antes da fatia consumidora. Provas assistivas, zoom Tauri e Windows/Linux físicos permanecem explicitamente pendentes sem acesso real.

Posse da primeira fatia visual (builder nativo frontend): `ui/js/{acessibilidade.js,componentes.js,areas.js,dom.js,app.js,estado.js}`, `ui/index.html`, `ui/css/{tokens.css,components.css,app.css}`, telas `clipboard.js` e `diagnostico.js` e CSS próprio dessas duas, mais testes novos de base. Não editar layout/pareamento/dispositivos/onboarding/preferencias, mocks, Rust, Tauri/capabilities nem docs do contrato. Consumidores recebem interfaces estáveis depois. Backend de segurança conserva sua posse da §15.1.

### 15.5 C14 · ampliação no app real
Sentinel conferiu a implementação local fixada (Tauri 2.12.5, utils 2.10.2, wry 2.12.3) e aprovou a exceção mínima às capabilities da §12/invariante 7. Habilitar `zoomHotkeysEnabled: true` somente na configuração da janela main e acrescentar somente `core:webview:allow-set-webview-zoom` à capability existente com `windows=[main]`, `remote` ausente. Manter as três permissões anteriores, CSP, dependências e negação de navegação externa/novas janelas intactas.

`windows=[main]` restringe o invocador; não constitui escopo de destino do setter, que aceita label opcional. Esta decisão se aplica ao app atual de uma janela e deve ser revista se forem introduzidas outras webviews. O intervalo 20%–1000% pertence ao polyfill não Windows, não à ACL. Configuração passa pelo `WebviewWindowBuilder::from_config` existente.

Posse exclusiva C14: Rivet · slice, apenas `crates/clipsync-app/tauri.conf.json` e `crates/clipsync-app/capabilities/default.json`. Aceite local: JSON válido e diff exato; nenhum teste que só espelha esses dois valores. Aceite posterior real: Ctrl+/Ctrl-/Ctrl+0 e reflow Tauri Linux/Windows, explicitamente não provados por resize no simulador nem por leitura do código. Sem modificar política de consentimento, atalhos de controle de input ou wire.

### 15.6 Consumidores UX · clareza, passagem e foco
Aplicar spec UX §§3–7 e matriz de cobertura, sem reabrir o incidente físico. Posse da fatia de consumidores: builder UX designado pelo Maestro, `ui/js/{layout-simples,travessia,travessia-vista}.js`, `ui/js/telas/{layout,pareamento,dispositivos,onboarding,preferencias}.js`, CSS próprio dessas cinco telas, `ui/js/api.js` apenas wrapper de modules_set, testes UX próprios/geométricos. Não editar base acessível, mocks, Rust ou docs. C01 nos consumidores só depois do schema final; arquivos base dependem da interface liberada pelo frontend.

1. Permissão nomeia quem controla ESTE computador. Conceder input_in não define papel persistente: orientar configuração no computador de origem; CTA local explícito Configurar saída deste computador. Confiança continua explícita, foco inicial Cancelar.
2. Atalhos mostram condições observadas por ação; capacidade não prova hook ativo. Não anunciar Retomar por atalho quando recusado; não habilitar ação indisponível. Recuperação de agente reconsulta estado/pares, descarta resposta antiga e apresenta uma ação principal sem avisos contraditórios.
3. Prévia nomeia monitor e bordas reais de saída/entrada, texto e canvas derivados da mesma geometria final. Posição continua simples, sem exigir borda. Reproduzir geometria diagonal antes de corrigir: preservar alinhamento perpendicular válido com monitor real, ou projetar na borda candidata mais próxima, sem centralizar todo o conjunto.
4. Em 960×640/632, posição, prévia e Ligar ficam antes do canvas alto; em 320, reflow vertical sem ação coberta. Avançado oferece clique simples equivalente ao arrasto (origem/destino/trechos/posição fina), controles pelo menos 24×24; sem alvos SVG sobrepostos. Home/End respeitam limites válidos; pointercancel restaura sem salvar.
5. Integrar helpers da base para foco estável em canvas, modal, operação demorada, repaint, popover Escape e Ligar; não roubar foco movido/rota/modal posterior. Preservar details/seleção/scroll e anúncios únicos. Alterações fora da posse são solicitadas ao dono.
6. Textos respeitam plataforma, condição real e nome do par; par bloqueado não concede controle; recebimento de eventos não prova travessia física. Pareamento avisa uma vez aos 30 s e ao expirar; recuperação usa nova tentativa/código, prazo atual 120 s. Não declarar WCAG 2.2.1 resolvido.

Aceite: regressões de geometria e estado, seis fluxos de foco completos e jornadas no portal, cenários negativos (vazio/sem monitores/input off/sem ligação/recusa/protocolo antigo/erro), dois temas e reflow 1280/960/640/320. Prova de leitor/teclado nativo/zoom Tauri e Windows/Linux físico continua separada.

### 15.7 C01 · módulos editáveis sem recriar identidade
Desenho vinculante: `docs/superpowers/specs/2026-10-29-s9-modulos.md` §§1–8 e10, aprovado por Sentinel após ajustar concorrência/posse/falhas. Ele substitui §2 e os trechos incompatíveis de C01 da spec UX anterior. Nenhuma variante QUIC/ControlRequest, dependência, grant ou subcomando CLI novo.

1. API local `modules_set {modules:{input,clipboard}, expected:{input,clipboard}}`, booleanos completos, campos desconhecidos recusados; wrapper `api.modulesSet`. Sucesso `{changed,modules,effects,observed}`; erro `UiError.details={effects,observed}`. Tipos/campos/enum de effects e observed estão fixados na §6 da spec e fazem parte deste contrato. Leitura corrente continua app_state.modules.
2. ConfigEdit reserva `config.toml.lock`, faz uma leitura/validação completa, compara expected e conserva bytes originais até salvar. `set_clipboard_mode` migra para o mesmo lock/leitura única. Antes do rename, comparar conteúdo integral para detectar editor que não coopera. No-op sem escrita/pausa/parada/start; conflito não restaura snapshot sobre alteração concorrente.
3. Guard de operação específico por perfil, entre processos, exclusivo na mudança/init; compartilhado no startup, tentativa inteira de pairing e Resume/InputLock. Try-lock sem bloquear executor; helpers internos recebem guard do mesmo perfil, sem reaquisição recursiva. Ordem: mutex async UI, operação, config, locks curtos. Pause permanece permitido e nunca é revertido automaticamente.
4. Preservar semanticamente todas as tabelas fora dos dois módulos; chave/certificado/pares/layout/UI/tokens não são escritos pela transação. Pausa deliberada atualiza somente paused sob lock state, mantendo campos/versão suportados; estado inválido/desconhecido falha sem truncar. Não prometer exclusão de binários antigos/escritores que ignoram locks.
5. Diferenciar próprio, externo, ausente comprovado e desconhecido. Timeout não é ausência. Running/JoinHandle permanecem possuídos até término observado, incluindo timeout de stop/start. Readiness vem da tarefa própria e listener reservado, não de Status respondido por outro processo. Consulta de Status usada pela observação UI tem prazo total de 3 s; erro, timeout ou resposta inesperada mantém agent_error e não confirma pausa persistida. Sem tarefa própria, essa observação usa agent=failed; com tarefa própria, conserva agent=in_app. Status válido posterior volta a informar o estado observado, sem apagar aviso de persistência por polling.
6. Após pausa confirmada/persistida, capturar trava; parar e confirmar tarefa; reservar listener antes do commit e consumi-lo sem rebind. Configuração concorrente, externo ou estado desconhecido falham fechados com efeitos reais. Restart próprio inicia PAUSADO; restaurar trava no bootstrap interno com ack antes de servir/readiness. Falha mantém posse/reserva até estado determinado; nunca enviar lock/Resume ao externo. Retomar depois exige ação explícita.
7. A transação é job gerenciado até resultado/limpeza, mesmo se o chamador desaparecer. Relatar efeitos de pausa/parada também antes de salvar; null/unknown para observação indeterminada. Não repetir nem retomar automaticamente após erro. Emitir resync para reconsulta, sem segredo/snapshot nos detalhes/logs.
8. Dependência de integração no dono /root/backend: enquanto paused, ignorar ToggleLock/GoTo atrasados; preservar TogglePause, cujo pedido de Resume passa pelo guard no core. Rivet não edita input/module concorrente. C01 não fecha sem essa dependência.

Posse Rivet: core `profile.rs`, `lib.rs` (exports), `agent.rs`, `control.rs`, `pairing.rs`; CLI `agent.rs`/`init.rs` se necessário; UI Rust `api,lib,modules(new),agent,clipboard,onboarding,input,status,msg,pairing`; testes relacionados; mock.js/mock/* e testes exclusivos de módulos/mocks. Sem transport/proto/input/clipboard da fatia segurança, base JS/CSS, consumidores UX ou docs. Mudança adicional requer coordenação.

Aceites: §8 da spec, incluindo preservação/no-op, concorrência real entre processos/métodos, intervalo preflight/commit, externos/desconhecidos, gatilhos pairing/init/start, pausa concorrente, falhas em cada etapa, cancelamento do chamador, posse no timeout e bootstrap. Perfis fake isolados; nunca perfil real/consentimento. Provas Tauri/Windows físicas permanecem pendentes.

### 15.8 B09 · pausa persistida ao mudar modo e feedback da bandeja
Proposta em `docs/superpowers/specs/2026-10-29-s9-seg-complemento.md`, seção B09, aprovada por Sentinel. `clipboard_set_mode` reutiliza a transação reservada C01, com edição tipada de modo/módulos, sem duplicar lifecycle. No-op não pausa/grava/reinicia; próprio exige Pause confirmada E persistida antes de salvar, trava após pausa, stop confirmado, listener reservado, restart pausado+trava. Falha de persistência impede save/stop/start, sem reverter pausa volátil. Parado salva sem iniciar; Unknown recusa; External conserva somente config+restart_needed, sem Pause/Stop/Lock/Resume nesse processo e sem alegar modo aplicado ao runtime.

ModeReply conserva mode/restart_needed e acrescenta effects/observed opcionais com serde(default). Erros carregam efeitos/estado real como C01. `app_state.pause_warning: Option<String>` é aditivo/default: aviso guardado no Ctx após falha de pause_set, resync; descreve a ação solicitada REAL (Pause ou Resume), distinguindo memória alterada/persistência falha de comunicação incerta. Limpar somente pela confirmação de persistência correspondente, com ordenação para resposta antiga não apagar falha nova; polling não limpa.

A bandeja, em erro, reconsulta o estado e abre a janela para o aviso persistente; falha de reconsulta mantém unknown/aviso, nunca inventa confirmação ou muda rótulo pelo valor solicitado. Não depender de notificação do SO. Front mostra aviso por textContent/região persistente sem roubar foco e explica antes da troca de modo que reiniciar deixa a malha pausada. Retomar é explícito.

Posse: Rivet acrescenta aos arquivos C01 `clipsync-app/src/{tray,events,msg}.rs` somente feedback B09 e testes relacionados; não editar ui/events.rs da futura agregação B01 nem frontend JS. Dono da próxima Task2 UI recebe `ui/js/{app,estado}.js` e `ui/js/telas/clipboard.js` somente aviso/feedback B09, além de Preferências/onboarding; nenhuma concorrência com frontend-base. Aceites: falha controlada de state.toml com config gravável bloqueia troca e não reinicia; reparar permite restart pausado/trava; no-op/externo/unknown; pausa concorrente e erro anterior/posterior; bandeja mostra aviso mesmo com notificações desligadas; erro de reconsulta não fabrica estado. Testes Tauri físicos continuam separados.

### 15.9 Snapshot Windows · sessão WTS e desktop são distintos
O snapshot inicial distingue sessão WTS bloqueada de disponibilidade do desktop de entrada. `clipsync_win::input_desktop_is_default()` continua medindo apenas desktop e NÃO prova sessão desbloqueada. Adicionar safe API `session_locked() -> Result<bool, WinError>` em clipsync-win, implementação exclusivamente ffi.rs via WTSQuerySessionInformationW(WTSSessionInfoEx) para sessão corrente. Validar sucesso, ponteiro/tamanho, Level e flags LOCK/UNLOCK; UNKNOWN ou falha mantém estado fechado. Liberar o buffer em todos os caminhos com WTSFreeMemory. Nenhum nome/conta/conteúdo da estrutura é exposto/logado.

Win32_System_RemoteDesktop já está habilitada; sem feature/dependência nova. Suporte Win10+ permanece (inversão dos flags em Win7/Server2008R2 fora do suporte). lib.rs exporta somente função segura; sem unsafe fora ffi.rs, cada bloco com SAFETY e lints preservados. Windows usa snapshot WTS explícito antes de hooks/captura, além de condição independente de desktop; não confundir eventos de desktop com estado de sessão. Linux inicia watcher antes do produtor/enable para que snapshot inicial preceda input; falha desconhecida permanece fechada.

Posse backend exclusiva nos arquivos mínimos de §15.1. Aceites: parser/flags/tamanho/Level/erro, cross-clippy input/win e scripts unsafe/privacidade; startup pausado, snapshot antes de entrada, unlock durante pausa seguido de retomada e precedência Stop. Revisão Sentinel obrigatória de unsafe/semântica. Teste fake/compilação não prova WTS/desktop/SendInput físicos.

Posse da implementação Task2 UX: /root/preferencias recebe Preferências/onboarding e CSS próprio, api.js wrapper, testes exclusivos, e somente o aviso B09 em app.js/estado.js/clipboard.js. Frontend-base libera esses três JS para esta tarefa; demais helpers/CSS continuam na posse anterior. O novo implementador preserva os deltas revisados da base/FIX1. Task1 Layout/Pareamento/Dispositivos permanece /root/ux.

### 15.10 B01 · grupos de negativas e cadência de avisos

`recent_blocks` conserva no máximo 20 grupos pela chave `(peer, module, reason)`; nome é apresentação. Chegada nova/repetida move o grupo ao início; causa nova expulsa o último, sem ordenar pelo relógio de parede. `at_unix_ms` é o primeiro instante, `occurrences: u64` tem default 1 e soma saturante, `last_unix_ms: u64` tem default 0 e leitura normalizada para `at_unix_ms` quando ausente. `u64::MAX` significa pelo menos. Controle JSON/UI/Diagnóstico são aditivos/default, sem alteração do fio QUIC. UI calcula diferenças por chave/início/contagem, aceita recriação sem subtração negativa e preserva baseline em polls sem agente. Uma ocorrência observada abre aviso; repetições acumulam para resumo após pelo menos 30 s monotônicos do aviso anterior, globalmente, sem reset por resync. Resumo diz ocorrências/causas/pares observados; diversidade guarda no máximo 20 chaves/20 pares e usa pelo-menos quando excedida. Casca aplica o mesmo teto a `kind=block`, respeitando preferências e sem afetar remoção real de dispositivo/outros kinds. Diagnóstico conserva história observada independentemente da supressão de notificações. Posse e aceites: Task 3 do plano 2026-10-29-s9-seg-complemento.

### 15.11 B03 · relógio lógico do clipboard

Manter `(lamport, PeerId)` e contador por instância, sem persistência nova. `MAX_REMOTE_AHEAD = 1_000_000` é restrição explícita de compatibilidade entre instâncias, não tempo/taxa: 0, MAX e salto acima da janela são `Invalid`. Conteúdo/tamanho/duplicação/pausa/TTL são validados antes de admitir relógio; oferta inadmissível não muda `current/last/lamport_max` nem entra na caixa por rejeição de relógio. Incremento local verificado nunca emite MAX: falha em MAX-1 e MAX, com erro explícito nas ações, diagnóstico fixo e laço vivo. Remoto admitido avança somente `lamport_max`; `last` representa maior par aplicado/copied localmente e só avança após escrita confirmada, nunca por InInbox/falha/conflito. ClipApply stale mantém permissão existente, sem fazer `last` retroceder. Todos os criadores tratam Result; sem reset automático ou nova variante wire. Posse e aceites: Task 1 do plano complementar.

### 15.12 B05 · soltura conservadora no emulador Windows

Conjuntos representam códigos possivelmente pressionados. Para cada lote, calcular `desired` na ordem dos eventos e `possible = anterior ∪ downs tentados`; sucesso integral confirma desired, qualquer erro conserva possible e marca cleanup_pending. release_all manda ups da cópia e só limpa após sucesso integral; falha retém tudo. emit E enter reais tentam limpeza pendente antes de qualquer injeção nova e recusam a nova operação se ela falhar, sem replay de movimento/down. Em erro, zerar restos fracionários. Stop tenta soltura e devolve falha; não promete recuperação após destruição da instância nem soltura física. FFI/Result, desktop seguro e política H01/B02 ficam intactos. Aceites devem atravessar os métodos reais com send falso, além de testes de bookkeeping. Posse: Task 2 do plano complementar.

### 15.13 B07 · lifecycle independente dos dados de captura

Notice Windows usa mailbox fixa de atomics/latches; lock/desktop/hooks invalidam e fecham flags locais imediatamente, sem I/O/consulta/espera no callback. Worker reconcilia antes dos dados, em timeout e durante contrapressão de saída; overflow descarta somente dados. `CaptureAdapter::lifecycle_receiver() -> Option<watch::Receiver<CaptureLifecycle>>`, default None, publica por instância o último estado e uma geração monotônica de invalidação. Schema interno abaixo, sem wire/FFI. Consumidor lê o valor atual imediatamente ao adquirir/reobter receiver, conserva geração já reconciliada e fecha gate até snapshot válido. Invalidação encerra sessão/abertura/batch e descarta produtor/rx antigos pelo B02, mesmo se o último estado já for desbloqueado. Unlock habilita somente ativação nova. Windows mantém WTS e desktop separados (§15.9); None conserva obrigatoriamente snapshots SessionLocked iniciais do caminho atual Linux/Windows. Stop/pausa/trava prevalecem; somente bloqueio do SO permite produtor novo sem barreiras para observar unlock. Stop não apaga snapshot/geração antes do consumo nem recria hooks. Posse e aceites: Task 4 do plano complementar.


Schemas, posse e aceites detalhados: docs/superpowers/specs/2026-10-29-s9-seg-execucao.md. B07 exige revisão desse schema antes de implementação e handoff do FIX1 H01/B02; B03/B05 têm desenho já aprovado por Sentinel. Nenhuma edição de arquivo ainda sob outro dono.


### 15.14 Integração final · pausa independente da fila de dados

Adendo v1.38, Maestro, 30/10/2026. Correção I1 da revisão integrada: prazo de entrega na fila de módulo não pode descartar a invalidação de pausa nem deixar captura local ativa após a pausa. Não altera wire, versão publicada, dependências ou autorização de release.

1. `ModuleHost` fornece observação local por `watch`, separada da fila limitada de `ModuleInput`, com pausa atual e geração monotônica de invalidação. Mesh publica coerentemente com a autoridade, antes do broadcast de compatibilidade. Pause seguido de Resume antes do consumo conserva invalidação da sessão anterior; concorrência não permite publicar snapshot antigo por cima do novo. Mudança de conexão/par também acorda a reconciliação da conexão vigente. MemHub reproduz as mesmas fronteiras; o modelo não depende de input_in local para autorizar saída.
2. Módulo Input, controlador contínuo e loopback observam essa fonte. Captura deve cancelar abertura/sessão/batch, liberar controle local, parar e aguardar produtor e descartar receptor antigo mesmo quando o estado principal está aguardando emulador e sua fila está cheia. Rearme exige snapshot vigente, gate aberto e nova ativação válida; mensagem Paused atrasada na fila provoca reconciliação, não restauração do valor antigo. Snapshot desconhecido ou fonte encerrada não habilita sessão.
3. Preservar cancelamento com dono, prazos de rede e progresso independente de módulos; não introduzir tarefa solta nem substituir a espera limitada por espera sem orçamento. Stop, pausa e trava mantêm prioridade; `CaptureLifecycle` do adaptador (§15.13) continua fonte separada para sessão do SO/desktop. Esta correção não reabre o FIX1 Windows já aprovado.
4. Aceite focal: captura de saída ativa, emulação pendente, fila Input realmente saturada e pausa por mais de dois segundos; comprovar liberação/parada sem depender da drenagem da fila principal. Cobrir também Pause/Resume coalescidos, descarte de dados/sessão anteriores e ausência de reativação sem nova ativação. Fakes provam controle de fluxo, não soltura física. Gate integrado deve completar após adaptar teste QUIC B01 para um grupo com 23 ocorrências da mesma chave, preservando redação de segredos e cobertura explícita do teto em caso apropriado.
5. Posse exclusiva desta onda: nativo `/root/s9_final_fix`, `core/src/transport.rs`, `core/tests/{mesh,reliable_backpressure}.rs` e suporte; `proto/src/{module,testing}.rs`; `input/src/{module,capture}.rs` e suporte; `cli/tests/pause_backpressure.rs` para composição Mesh/Input reais, pois CLI já depende de ambos. `lib.rs` dos crates somente reexportação local necessária, registrada no relatório. DOC/contrato/contexto continuam sob Maestro; frontend49 congelado. Relatório, delta e revisão independente em `.superpowers/retomada-s9-2026-10-30/revisao-final/`.

Schema local da §15.14: `HostLifecycle { paused: bool, pause_generation: u64, stopped: bool }`, Copy/Default/Eq, não serializado. `ModuleHost::lifecycle_receiver() -> Option<watch::Receiver<HostLifecycle>>`, default None para implementações legadas; Mesh e MemHub sempre Some. Fonte Some encerrada equivale a parado. None conserva caminho legado de autoridade/controle e não recebe a promessa de retenção de épocas coalescidas; wrappers usados para validar Mesh/MemHub devem encaminhar a fonte. `pause_generation` invalida ao pausar, não dá wrap; esgotamento de u64 mantém falha fechada. Snapshot inicial real deve ser publicado antes de servir o receptor; `Default` não é evidência de runtime ativo. Alteração de pares/conexões acorda o watch e exige consulta da autoridade vigente, comparando geração de conexão independentemente da permissão de entrada local. Estado stopped prevalece sobre resume.

Extensão focal do gate integrado (30/10): `FakeCapture::standalone(zones: Vec<Zone>) -> Self` mantém o sender da fonte dentro do adaptador e publica `CaptureLifecycle` inicial conhecido e válido do fake (desbloqueado, desktop disponível e zonas), antes de disponibilizar observação. Somente `build(AdapterChoice::Fake)` usa esse modo. `FakeCapture::new` continua retornando adaptador/handle separado e conserva EOF ao perder o injector, para testes controlados. Stop deve encerrar e aguardar o produtor; restart exige novo receptor e snapshot. Isso não abre gates nem presume estado de adaptadores reais. Posse ampliada ao mesmo implementador em `input/src/adapter/fake.rs`, `input/src/lib.rs` e testes focais. A falha UI de switch deve ser corrigida no ciclo do fake, sem retry/sleep ou redução de assert no consumidor. Wrappers de MemHub usados na prova de contrapressão encaminham `lifecycle_receiver`, conforme o schema acima.


## 16. S10 · clareza e monitores independentes

Adendo v1.39, Maestro, 30/10/2026. João autorizou executar os dois recortes apresentados e publicar release **somente após tudo finalizado**. Designs vinculantes: [clareza](superpowers/specs/2026-10-30-clareza-disponibilidade-design.md) e [monitores independentes](superpowers/specs/2026-10-30-monitores-independentes-design.md). A S9 e seus invariantes continuam vigentes.

### 16.1 Clareza de disponibilidade e concessão no destino

Nas telas Dispositivos/Pareamento, distinguir suporte explicitamente ausente, módulo local explicitamente desligado e estado não confirmado usando os dados existentes. Dentro de ramo já desabilitado, mudar apenas a descrição; preservar equivalência dos motivos nulos/não nulos, prioridade de confiança bloqueada, etapa, handlers e confirmação de confiança total. Ausência de observação remota não prova módulo removido; disponibilidade agregada não atribui causa exclusiva ao par.

O placeholder sempre falso “Controlar {nome} a partir deste computador” passa a orientação estática no mesmo grupo, sem interruptor e sem ação de escrita: “Para controlar {nome} a partir deste computador, autorize este computador na tela Dispositivos de {nome}. O ClipSync não informa aqui se essa autorização já foi concedida.” Esta é exceção explícita à apresentação anterior do placeholder; o controle real de conceder entrada nesta máquina e sua confirmação permanecem. Nenhum grant remoto/local é criado por orientação, navegação ou teste de texto.

Clipboard: enabled=false apresenta a classificação de plataforma como não avaliada com módulo desligado, sem inferir incompatibilidade; ausência de enabled booleano permanece não confirmada; enabled=true conserva a classificação recebida. Diagnóstico vazio não promete latência, e a ausência dessa medição é explicada. Demais recursos futuros permanecem visíveis/desabilitados. Nenhuma API, dependência, capability ou protocolo muda nesta fatia.

Posse de implementação: nativo s10_clareza, quatro telas dispositivos/pareamento/clipboard/diagnostico, helper disponibilidade.js e testes focais conforme plano 2026-10-30-clareza-disponibilidade.md. Contrato/documentação/release só Maestro. Aceite: regressões de causas/gates/etapas/consentimento/texto seguro; revisão independente de spec e qualidade; visual focal nos temas/larguras do plano, com limitações explícitas.

### 16.2 Mapa por monitor, identidade e saídas por sessão

O primeiro aceite é desktop Windows com dois monitores e notebook GNOME Wayland com uma tela, usando mouse/teclado físicos do desktop. O mapa organiza A ↔ notebook ↔ B sem mudar a disposição nativa do SO, sem bordas técnicas obrigatórias e sem novo peer/grant. Movimento é rascunho; aplicação explícita valida/grava o arranjo inteiro e informa separadamente persistência/aplicação observada/travessia comprovada. Notebook como origem e captura de divisória interna em Linux multitelas mantêm os limites declarados no design.

Registro privado monitor-identities.toml independente de layout: UID não pode apontar silenciosamente para outra tela; correspondência ambígua/falha/versão futura preserva dados e não anuncia identidade estável suposta. O receptor pode fornecer inventário sem criar/migrar layout. Layout v2 e passagens derivadas usam transação única com base_rev e inventários vigentes, backup privado obrigatório antes da primeira migração v1, recuperação explícita e preservação de links/atalhos/opções legados não substituídos. Abrir não migra.

Evolução de protocolo para v4, mínimo de conexão1: variantes novas só ao fim; estruturas/índices das mensagens antigas preservados. Entrada oferece até duas rotas de retorno validadas/vinculadas à conexão/sessão/inventário. A origem conserva a tabela local imutável, o destino escolhe somente rota oferecida; monitor ausente/ambíguo não cai em primeira tela. Inventário mantém privacidade de Zones e autorização input_in. Oferta nova conserva posições individuais e aceitação explícita; par antigo não recebe fallback silencioso para rota errada. S9 mantém invalidação/soltura local independente da rede e fronteira de filas/sessões.

Detalhes de tipos, arquivos e testes serão fixados no plano de implementação correspondente antes da posse de código de cada tarefa. Geometria Linux exige observação real, sem tratar DRM estimado como autorização de ativação; preparação de portal não aceita diálogo do SO automaticamente.

Precisão da persistência S10: erro anterior ao rename preserva o arquivo anterior; erro na sincronização do diretório após rename identifica explicitamente a revisão nova já visível, com durabilidade não confirmada. Write e recuperação não prometem rollback nem repetem escrita automaticamente. A transação mantém passagens suspensas, responde revisão publicada/ativação não confirmada/durabilidade não confirmada, e a UI distingue isso de erro sem gravação. Registro de identidades não publica snapshot após falha de confirmação de persistência. O Debug público também omite aliases potencialmente sensíveis, não apenas Evidence.

### 16.3 Finalização e publicação

Release está autorizado somente após conclusão dos dois recortes, revisões sem pendência impeditiva, verificações locais/integradas, CI Linux/Windows no mesmo commit, empacotamento e hashes conferidos, e aceites obrigatórios visuais/físicos do novo percurso. Preparar candidato de teste não equivale a publicar release. Fakes/cross-compilação/CI não fecham prova física. Limite de acesso a Windows, portal ou diálogo deve permanecer visível; nenhuma pendência é reclassificada como sucesso para publicar. O incidente anterior de teclado continua adiado, separado dos testes de regressão do novo recurso.

O gate inclui execução dos consumidores Node no CI do candidato: job separado `ui-node`, Node 24, todos `ui-dev/testes/*.test.mjs`, heap 512 MiB por processo e concorrência 2, sem npm/navegador. Usa actions oficiais e somente `contents: read`, preservando os jobs Rust e o workflow de release. A revisão do YAML não comprova execução remota.


### §16 · precisões de integração da interface

A API da interface normaliza os códigos internos de inventário para `stale_inventory` e `inventory_required`. As funções Rust que recebem o contexto privado permanecem `pub(crate)`; a superfície externa continua sendo `Ui::call`, sem comando Tauri adicional.

`layout_get` acrescenta `v1_backup_available` booleano com default conservador false. True exige leitura e validação de backup v1 no perfil, sem expor caminho ou conteúdo. Trata-se de observação: a restauração revalida o backup, a revisão e a exclusão sob lock no serviço; organização ausente desativa recuperação na tela.

Para monitores ausentes, preservar UID/posição e mostrar o último rótulo disponível na sessão. Sem rótulo persistido ou observado, apresentar ausência e UID, sem fabricar nome/geometria ou acrescentar registro de nomes a esta etapa. Dados de apresentação em cache não habilitam aplicar nem substituem inventários atuais.

Precisão dos cantos mortos em v2: ligações geridas persistem segmentos efetivos validados na aplicação. Alterar a opção local não recorta novamente esses segmentos nem altera endpoints recebidos de uma oferta aceita; a opção vigente entra na próxima compilação explícita de prévia/aplicação. Ligações avançadas conservam a validação própria. A interface não deve prometer recomputação retroativa das passagens existentes.


### 16.4 Fecho da integração S10

Preparar telas espera consentimento humano: `InputPrepareInventory` tem prazo de 110 segundos no dispatch do agente, inferior aos 120 segundos do controle externo. Consultas de status mantêm 2 segundos. Timeout, pausa, revogação e encerramento conservam o cancelamento e impedem publicar inventário de preparação invalidada; não autorizam consentimento automático.

Aceitar organização e restaurar v1 apresentam separadamente ativação e durabilidade. Quando `durability=unconfirmed`, informar também que a revisão foi publicada mas a persistência após uma queda não foi confirmada. A recarga e a saída do estado ocupado conservam esse aviso visível e acessível, sem repetir a escrita. Esta precisão aplica o §16.2, sem mudança de wire ou persistência.
