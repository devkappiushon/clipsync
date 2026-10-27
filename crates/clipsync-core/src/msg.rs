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
