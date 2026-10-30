/// 2 since S6: `Zones`, `LayoutOffer` and `EnterAt` exist; 3 since S7: `EnterSpan`, `Returned`
/// and `Refused`; 4 adds monitor inventories, routes and monitor layout offers. Older peers
/// still connect: the core never sends them a message newer
/// than their version (`min_version`, `RemoteTooOld`).
pub const PROTOCOL_VERSION: u16 = 4;
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
/// `AgentStatus::recent_removals`: peers that removed this device lately (v1.16).
pub const MAX_RECENT_REMOVALS: usize = 20;
pub const CLIP_TTL_MS: u32 = 600_000;
pub const SEEN_EVENTS_CAP: usize = 1_024;
pub const PAIR_WINDOW_SECS: u64 = 120;
pub const MAX_INPUT_DATAGRAMS_PER_SEC: u32 = 2_000;
/// Monitors in one `Zones`, and devices in one `LayoutOffer`.
pub const MAX_ZONES: usize = 16;
pub const MAX_LAYOUT_LINKS: usize = 64;

/// At most sixteen monitors for each of sixteen peers.
pub const MAX_MONITOR_POSITIONS: usize = MAX_ZONES * MAX_ZONES;

/// A sequence seed that rejects an untrusted length before reserving or reading elements.
/// Unknown lengths are checked incrementally and never use the sender's capacity hint.
pub(crate) struct BoundedVecSeed<T> {
    max: usize,
    marker: std::marker::PhantomData<T>,
}
impl<T> BoundedVecSeed<T> {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            max,
            marker: std::marker::PhantomData,
        }
    }
}
impl<'de, T: serde::Deserialize<'de>> serde::de::DeserializeSeed<'de> for BoundedVecSeed<T> {
    type Value = Vec<T>;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Vec<T>, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for BoundedVecSeed<T> {
    type Value = Vec<T>;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at most {} elements", self.max)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
        use serde::de::Error;
        if seq.size_hint().is_some_and(|len| len > self.max) {
            return Err(A::Error::custom("too many elements"));
        }
        let mut values = Vec::new();
        while values.len() < self.max {
            let Some(value) = seq.next_element()? else {
                return Ok(values);
            };
            values.push(value);
        }
        // SeqAccess can still report an exact end without decoding an element. If one
        // exists, this seed fails without invoking T or consuming any of its fields.
        match seq.next_element_seed(RejectExcessElement)? {
            None => Ok(values),
            Some(never) => match never {},
        }
    }
}

/// Detect an element beyond the budget even in non-self-describing formats such as
/// postcard: neither deserialize_any nor a DTO/String deserializer needs to be called.
struct RejectExcessElement;
impl<'de> serde::de::DeserializeSeed<'de> for RejectExcessElement {
    type Value = std::convert::Infallible;
    fn deserialize<D: serde::Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
        Err(serde::de::Error::custom("too many elements"))
    }
}

pub(crate) fn deserialize_monitors<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::MonitorInfo>, D::Error> {
    serde::de::DeserializeSeed::deserialize(BoundedVecSeed::new(MAX_ZONES), d)
}
