# Contrato · ClipSync MVP de hoje (v1.6, Keel, 2026-10-27)

v1.1: identidade passa de Ed25519 para ECDSA P-256 (§3 e §5 regra 1; motivo na ADR 0003).
v1.2: seis ajustes do Rivet no proto aceitos (§4: sha2, IdError, SendError com #[error], ordem de autorização, MemHub, validate); consequência no §8 regra 2 (módulo input confere dono da sessão).
v1.3: `DeliveryState::Unreachable` novo (§4, Rivet); ClipSent lista TODO par alvo (§7 regra 9, Flux); código de saída do `send` (§6, Rivet); ttl conta do recebimento local e vale para a caixa (§7 regra 6); ClipSend avança Lamport sem contar como cópia local (§7 regra 12).
v1.4: `CloseCode::ALL` (§4, já entregue pelo Rivet); spike input 2 do Vector no §8 (regras 7 a 11): barreira em x = largura da zona, Enter por movimento relativo, restore_token do RemoteDesktop em `DIR/portal-tokens.toml`, proibido `supported_capabilities()`, P4c automatizado com retry; §3 ganha `portal-tokens.toml`; §9 S3 ganha itens 6 e 7.
v1.5: dep `futures-util =0.3.34` (§1, pedido do Vector).
v1.6: posse dividida (Maestro): core/{profile,control,diag}.rs e o crate cli inteiro passam ao Flux; o Rivet fica com core/{lib,tls,pairing,transport,agent,msg}.rs. Tipos: Identity/Config/InitOptions/Peers/PeerEntry/State/Profile em profile.rs (Flux); PairOutcome/PairPrompt/PairDecision em pairing.rs (Rivet); CoreError único em lib.rs (Rivet). Keel acrescentou: `tls::pin_of` como fonte única do pino, `CoreError::AgentAlreadyRunning`, `ControlHandler`/`control::serve`. `pairing::host` ganha `on_listen`. Dep `quinn-proto =0.11.19` (Rivet troca o reason do CONNECTION_CLOSE). §9: S1 dividido; nova S4 (Flux); I passa ao Flux.

Fonte: docs/prompt-mestre.md v4 (§1, §2, §5, §6, §13, §14). Este contrato vence qualquer suposição de builder. Dúvida: `maestri ask "Keel · esqueleto" "..."`. Mudança de contrato: Keel edita AQUI primeiro, depois avisa.

## 0. Regras do time
1. Ninguém roda git (o Maestro commita). Ninguém sobe processo de longa duração fora de teste automatizado (`cargo test` pode subir `clipsync run` como filho e matar no fim).
2. Um dono por crate/arquivo (§2). Editar arquivo de outra fatia = pedir ao dono.
3. Sempre `source ~/.cargo/env` antes de cargo. Toolchain fixado: 1.99.1. Sem sudo, sem Node, sem Tauri.
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

## 3. Perfil (`--profile DIR`)
`--profile` é global (qualquer posição). Padrão: `$XDG_CONFIG_HOME/clipsync` (ou `~/.config/clipsync`); env `CLIPSYNC_PROFILE`. DIR criado com 0700.

```
DIR/
  config.toml         configuração versionada (usuário edita)
  identity.key.pem    chave privada ECDSA P-256 PKCS#8, modo 0600 (ADR 0003)
  identity.cert.pem   certificado autoassinado (rcgen, chamada do spike: SAN "clipsync"; nome do dispositivo NÃO vai no cert)
  peers.toml          pares confiáveis + permissões (escrito por pair; usuário pode editar)
  state.toml          estado persistido pelo agente: paused
  agent.sock          socket de controle (Unix, 0600) enquanto `run` está de pé
  portal-tokens.toml  restore_token do portal RemoteDesktop, modo 0600; escrito e lido SÓ por clipsync-input (Vector)
```
`portal-tokens.toml`: `version = 1` · `remote_desktop = "<token>"`. Token inválido ou recusado: apaga a chave e pede o diálogo de novo. InputCapture v1 não tem token: 1 diálogo por sessão do agente (documentado na matriz).
Segredo em arquivo 0600 é provisório (cofre da plataforma fica para depois; registrado em Aberto).

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
/// CLOCK_MONOTONIC em µs (rustix). Comparável entre processos SÓ na mesma máquina (loopback).
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

// control.rs: IPC CLI <-> agente. Socket Unix DIR/agent.sock, quadro = u32 BE + JSON (serde_json),
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
3. `Cargo.toml` do core ganha `serde_json`, `async-trait` e `quinn-proto` (todos `workspace = true`).

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
    pub fn resolve(arg: Option<PathBuf>) -> Result<Self, CoreError>;      // --profile > CLIPSYNC_PROFILE > XDG
    pub fn init(&self, opts: InitOptions) -> Result<Identity, CoreError>; // DIR 0700, key 0600, config, peers vazio, state
    pub fn load_config(&self) -> Result<Config, CoreError>;
    pub fn load_identity(&self) -> Result<Identity, CoreError>;           // recusa key com modo != 0600
    pub fn load_peers(&self) -> Result<Peers, CoreError>;
    pub fn save_peers(&self, p: &Peers) -> Result<(), CoreError>;         // escrita atômica (tmp + rename)
    pub fn load_state(&self) -> Result<State, CoreError>;                 // ausente = State::default()
    pub fn save_state(&self, s: &State) -> Result<(), CoreError>;
    pub fn socket_path(&self) -> PathBuf;                                 // DIR/agent.sock
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
/// Cria o socket com 0600. Socket velho: se connect falha, apaga e segue; se connect funciona, AgentAlreadyRunning.
/// Uma conexão = uma requisição + uma resposta (quadro u32 BE + JSON, máx MAX_FRAME_BYTES). Apaga o socket ao sair.
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
6. **quinn-proto (v1.6):** o quinn manda o `e.to_string()` do rustls no CONNECTION_CLOSE quando o handshake falha. Por isso o Rivet embrulha `quinn::crypto::{ServerConfig, ClientConfig, Session}` para trocar esse reason por `CloseCode::reason()` (regras 1, 6 e 9 abaixo). O teste tls_ (c) prova isso.

Regras de transporte (implementação livre dentro delas):
1. Identidade: ECDSA P-256 (rcgen; ADR 0003), PeerId = SHA-256(SPKI DER). **Um verificador único** (`core::tls::PinVerifier`) implementa `ClientCertVerifier` e `ServerCertVerifier`: valida o X.509 com o próprio cert como âncora (autoassinado), verifica a assinatura TLS 1.3 real com o provider ring, e o pino é SHA-256 do SPKI. Modo malha: pino precisa estar em peers.toml. Modo pareamento (ALPN_PAIR): aceita qualquer pino, mas exige posse da chave. Cert de cliente obrigatório nos dois modos (TLS mútuo).
2. **Retomada TLS 1.3 desligada (obrigatório)**, senão o verificador não roda na reconexão. Servidor: `session_storage = NoServerSessionStorage`, `send_tls13_tickets = 0`. Cliente: `resumption = Resumption::disabled()`. Vale para malha e pareamento.
3. Pareamento: host escuta em porta efêmera (`--port 0` padrão), imprime `Aguardando em <addr>:<porta> (expira em 2 min)`. No fluxo de controle aberto pelo join (B): Hello B->A, Hello A->B, depois o compromisso de `PairMsg` (§4): B envia Commit{H(nB)}, A envia Nonce{nA}, B revela nB, A confere H(nB). Código = u64 BE de `export_keying_material(8 bytes, PAIR_EXPORTER_LABEL, contexto nB||nA)` mod 1_000_000, "NNN NNN". nA e nB: 32 bytes aleatórios novos por tentativa. **Uma tentativa por janela**: qualquer falha (hash não confere, recusa, timeout, frame fora de ordem) fecha com CloseCode::PairFailed e encerra `pair host` (sai 3). Conexões extras durante a tentativa são recusadas. Cada lado mostra código + nome + id curto e pergunta permissões só dos módulos ligados (input_in com aviso de confiança total). Cada lado envia Confirm{accept}; grava peers.toml **só se os dois aceitarem**. Se o agente estiver rodando, manda `ReloadPeers`.
4. Malha: `run` escuta em `network.listen`, disca cada par (backoff 1 s até 30 s, jitter ±20%). Fluxo bidi de controle aberto por quem disca. **Conectado só depois de Hello de aplicação autenticado nos DOIS sentidos** (o `connect()` do quinn volta antes de o servidor validar o cert do cliente): antes disso nada de PeerUp, nada de módulo, status = Connecting. Versão incompatível fecha com CloseCode::Version. Mensagens de módulo: um fluxo uni por frame. Datagramas: `encode_datagram`. keep-alive 5 s, idle 15 s, datagramas ligados.
5. Duplicada: se há duas conexões com o mesmo par, fica a discada pelo menor PeerId; a outra fecha com CloseCode::Duplicate.
6. Par desconhecido: handshake recusado; servidor loga `par desconhecido recusado: <id curto>`; cliente desconhecido vê a mensagem de `msg.rs` "recusado pelo par: certificado não confiável".
7. Chave trocada: ao discar `addr` de um par conhecido e receber outro SPKI: aborta (CloseCode::KeyChanged), PeerState::Blocked{"chave do par mudou; repareie"}, alerta em warn e em `status`. Não reconecta até novo pareamento.
8. Pausa: core guarda (state.toml), aplica authorize_* e propaga `ModuleInput::Paused`.
9. **CONNECTION_CLOSE e Bye levam só `CloseCode::code()` + `reason()`** (token curto fixo). Nunca texto Debug/Display de rustls, quinn ou io vai ao par. Erro detalhado só em log local.
10. Frame acima do limite do fluxo: o leitor para antes de alocar e reseta o fluxo (`stop`/RESET com CloseCode::Protocol). Fluxo de controle: MAX_CONTROL_FRAME_BYTES (64 KiB); fluxo de módulo: MAX_FRAME_BYTES.

## 6. clipsync-cli (Flux desde v1.6): comandos
`clipsync [--profile DIR] <cmd>` (`--profile` global). Saída em português de `src/msg.rs`. Códigos de saída: 0 ok, 1 erro, 2 uso (clap), 3 recusado/não confirmado, 4 agente não está rodando.
```
init [--name N] [--modules input,clipboard] [--port P] [--fake-adapters]   pergunta o que faltar; pergunta de clipboard com padrão NÃO
status                    agente de pé: AgentStatus; parado: config + "agente parado" e sai 4
pair host [--port P] [--timeout 120s] [--allow LISTA]
pair join ADDR [--allow LISTA]       LISTA: clipboard-in,clipboard-out,input-in (pré-responde permissões; o CÓDIGO sempre é confirmado por stdin "s/N")
run [--listen ADDR]       foreground até Ctrl-C
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
1. MVP de hoje: o agente **não captura sozinho**. Captura só sob `ModuleCommand::InputLoopback{to, seconds}`: usa `[[input.barrier]]` (ou, sem barreira, metade superior da borda direita da zona 0), ao `Activated` manda `InputMsg::Enter`, cada evento vira datagrama (lote até MAX_EVENTS_PER_DATAGRAM, sem esperar), ao fim `Leave` + `StatsRequest`, responde `ModuleReply::InputLoopback(stats)`.
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
3. TLS: `cargo test -p clipsync-core tls_` verde com: (a) **contador do PinVerifier sobe nas DUAS conexões** ao conectar, desconectar e reconectar (prova de retomada desligada); (b) cliente com cert desconhecido: servidor nunca marca Connected nem entrega PeerUp, mesmo com `connect()` do cliente tendo retornado Ok; (c) reason de todo CONNECTION_CLOSE recebido pertence ao conjunto de `CloseCode::reason()` (nenhum texto de rustls).
4. P2 canal: teste `stream_and_datagram` verde: no mesmo Connection, um frame em fluxo uni e um datagrama chegam ao outro lado; frame de controle de 64 KiB + 1 recusado com RESET antes de alocar. Medição registrada no workboard (spike: datagrama 64 B p95 < 100 µs; fluxo 1 KiB p95 < 80 µs); passa se p95 < 1 ms nos dois. Se `tshark` ou `tcpdump` existir sem sudo, captura em lo sem o texto sintético em claro; senão registrar "não verificado".
5. Malha no core, sem cli: `cargo test -p clipsync-core mesh_` verde com dois `agent::run` no mesmo processo, perfis em tempdir, sem MemHub (QUIC real em 127.0.0.1). Os casos: (a) Connected e PeerUp só depois do Hello nos dois sentidos; (b) uma terceira identidade desconhecida é recusada e nunca chega a Connected; (c) derrubar e subir B reconecta, com o PinVerifier rodando de novo; (d) B com identidade nova no mesmo addr vira PeerState::Blocked e o reason recebido é `key-changed`.
6. Comportamento que os testes do cli (S4 itens 3 e 4) exercitam: se falharem na camada de pairing ou agent, o conserto é do Rivet.
7. `cargo clippy --workspace --all-targets -- -D warnings` e `cargo fmt --all --check` verdes.

### S4 · Flux (perfil, controle, diag e cli; nova em v1.6)
1. Perfil: `cargo test -p clipsync-core profile_` verde com: (a) `init` cria DIR 0700, key 0600 e `peers.toml` vazio; (b) `load_identity` recusa a key com modo 0644; (c) `Identity::id() == tls::pin_of(&cert_der())`; (d) round-trip do peers.toml com id em 64 hex; (e) config com os dois módulos false é recusada com `CoreError::Config`.
2. Controle: `cargo test -p clipsync-core control_` verde com: (a) o socket nasce com 0600; (b) socket velho (sem dono) é apagado e o serve sobe; (c) um segundo serve com o primeiro vivo dá `AgentAlreadyRunning`; (d) request sem agente dá `AgentNotRunning`; (e) quadro acima de MAX_FRAME_BYTES recusado antes de alocar.
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

## 10. Aberto (decisão fora do time)
1. **Licença** antes do primeiro commit (MIT/Apache-2.0 sugerido; GPL-3 só se copiar Lan Mouse). Usuário decide.
2. Chave privada em arquivo 0600 no perfil até existir cofre (Secret Service no Linux). Registrar na ADR de stack.
3. Nome do monitor nas zonas do portal: o InputCapture não dá nome; Vector resolve por geometria (Mutter DisplayConfig ou wl_output). Se não der hoje, `monitor = "#0"`.
4. ADRs (stack, transporte, licença) e `docs/matriz-compatibilidade.md`: sem dono neste contrato; o Maestro atribui.
5. Captura contínua guiada por barreiras no agente (produto 2B) fica fora de hoje; hoje só loopback e probes.
6. Decisão do Keel a confirmar com o usuário: "conteúdo inalterado não é cópia nova" (§7 regra 3). O anexo §15 pede que recopiar o mesmo texto possa valer como ação nova; aqui não vale, porque a tomada de posse do Mutter (app de origem morre) gera evento idêntico a qualquer tempo e reenviaria conteúdo velho por cima de cópia nova do par. Saída para o usuário: `clipsync send`.
