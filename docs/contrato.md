# Contrato · ClipSync MVP de hoje (v1.19, Keel, 2026-10-28)

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
   - **Licença (v1.12, decisão do Maestro):** a 0.1.0 sai num repositório privado, sem LICENSE, e o README segue com "a definir". Antes do primeiro release público (repositório público ou release visível fora da conta), o usuário decide a licença. Sem essa decisão, nada vira público.
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
               appearance: {theme: "system"|"light"|"dark", reduce_motion: false}, language: "pt-BR", version, license: "a definir"}
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
6. B offline na hora do revoke (v1.18, 7c770ef): quando B volta, A recusa o certificado de B, e B não remove A. Registra quem foi recusado: quando um par conhecido rejeita o certificado deste dispositivo (CONNECTION_CLOSE com um dos textos fixos de certificado inválido do rustls, `tls::HANDSHAKE_REJECT_REASONS`, no connect ou já no Hello), entra em `recent_blocks` uma entrada com o par, `module: null` e o motivo "recusou a conexão: este dispositivo não está mais pareado lá" (`msg::PEER_REFUSED_US` do core). É uma por par até uma conexão com ele voltar. Quem recusa não registra nada, porque uma chave desconhecida pode ser qualquer um na rede. Falha de rede não conta.

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
   - (3) pareamento: duas `Ui` recebem `pair_code_ready` com o MESMO código; `pair_finish` antes de `pair_code_matches` dá `wrong_step`; aceite dos dois lados grava exatamente os grants (padrão tudo false) e emite `pair_result` paired; `pair_cancel` emite cancelled e não grava;
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
