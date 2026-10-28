//! Portuguese text the agent puts in logs and in `status` (contract §2.1). Owned by Rivet.

/// Server log when the handshake refuses an unpinned key; followed by the short id.
pub const UNKNOWN_PEER_LOG: &str = "par desconhecido recusado:";
/// Dialer log when a known address presents another key.
pub const KEY_CHANGED_LOG: &str = "chave do par mudou; repareie";
/// PeerState::Blocked reason after a key change (rule 7).
pub const KEY_CHANGED: &str = "chave do par mudou; repareie";
/// PeerState::Blocked reason when the protocol versions do not overlap.
pub const VERSION_MISMATCH: &str = "versão de protocolo incompatível";
/// Dialer log when the peer refused our certificate (rule 6).
pub const REFUSED_BY_PEER: &str = "recusado pelo par: certificado não confiável";
/// recent_blocks entry on the side that was refused (v1.16): the peer no longer has us paired.
/// The UI shows it after the peer's name.
pub const PEER_REFUSED_US: &str = "recusou a conexão: este dispositivo não está mais pareado lá";
/// ControlResponse::Error when a module's channel is closed.
pub const MODULE_GONE: &str = "módulo encerrado";
/// ControlResponse::Error / last_block when a module does not answer in time.
pub const MODULE_TIMEOUT: &str = "módulo não respondeu a tempo";
/// Pause/resume applied in memory, but state.toml could not be written.
pub const STATE_NOT_SAVED: &str =
    "aplicado agora, mas não foi possível gravar state.toml: não sobrevive a reinício";
/// ReloadPeers could not read peers.toml.
pub const PEERS_NOT_LOADED: &str = "não foi possível ler peers.toml";
