/// 2 since S6: `Zones`, `LayoutOffer` and `EnterAt` exist. A 0.1.0 peer (1) still connects:
/// the core never sends it a message newer than its version (`min_version`, `RemoteTooOld`).
pub const PROTOCOL_VERSION: u16 = 2;
pub const MIN_PROTOCOL_VERSION: u16 = 1;
pub const ALPN_MESH: &[u8] = b"clipsync/1";
pub const ALPN_PAIR: &[u8] = b"clipsync-pair/1";
pub const PAIR_EXPORTER_LABEL: &[u8] = b"EXPORTER-clipsync-pair-v1";
/// H(nB) = SHA-256(PAIR_COMMIT_DOMAIN || nB).
pub const PAIR_COMMIT_DOMAIN: &[u8] = b"clipsync-pair-commit-v1";
/// 1 MiB + 64 KiB, module streams.
pub const MAX_FRAME_BYTES: usize = 1_114_112;
/// Control stream and local socket.
pub const MAX_CONTROL_FRAME_BYTES: usize = 65_536;
pub const MAX_CLIP_TEXT_BYTES: usize = 1_048_576;
pub const MAX_NAME_BYTES: usize = 64;
pub const MAX_MIME_BYTES: usize = 128;
pub const MAX_REASON_BYTES: usize = 256;
pub const MAX_DATAGRAM_BYTES: usize = 1_200;
pub const MAX_EVENTS_PER_DATAGRAM: usize = 32;
pub const MAX_INBOX_ITEMS: usize = 50;
/// Ring of `AgentStatus::recent_blocks`.
pub const MAX_RECENT_BLOCKS: usize = 20;
pub const CLIP_TTL_MS: u32 = 600_000;
pub const SEEN_EVENTS_CAP: usize = 1_024;
pub const PAIR_WINDOW_SECS: u64 = 120;
pub const MAX_INPUT_DATAGRAMS_PER_SEC: u32 = 2_000;
/// Monitors in one `Zones`, and devices in one `LayoutOffer`.
pub const MAX_ZONES: usize = 16;
pub const MAX_LAYOUT_LINKS: usize = 64;
