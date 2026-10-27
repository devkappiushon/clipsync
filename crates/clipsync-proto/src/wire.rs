//! Framing: u32 big-endian length + postcard body.
//! Readers check the length against `max` BEFORE allocating, then decode, then `validate()`.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::clip::ClipMsg;
use crate::ids::{EventId, ModuleKind, ModuleSet};
use crate::input::{InputDatagram, InputMsg};
use crate::limits::{MAX_DATAGRAM_BYTES, MAX_NAME_BYTES};

#[derive(Serialize, Deserialize, Debug)]
pub enum Frame {
    Hello(Hello),
    Bye {
        code: CloseCode,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    /// Only on ALPN_PAIR connections; on ALPN_MESH it is `CloseCode::Protocol`.
    Pair(PairMsg),
    Module(ModuleMsg),
    Rejected {
        kind: ModuleKind,
        ref_id: Option<EventId>,
        reason: RejectReason,
    },
}

/// Pairing commitment (B = joiner, A = host), on the control stream, in this order:
/// B->A Commit{H(nB)}; A->B Nonce{nA}; B->A Reveal{nB}; A checks H(nB).
/// Code = u64 BE of export_keying_material(8 bytes, PAIR_EXPORTER_LABEL, context nB||nA) mod 1_000_000.
/// Then each side sends Confirm{accept} with the user's decision.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PairMsg {
    Commit { h_nb: [u8; 32] },
    Nonce { na: [u8; 32] },
    Reveal { nb: [u8; 32] },
    Confirm { accept: bool },
}

/// QUIC application close code. The reason sent to the peer is ONLY `reason()`:
/// a short fixed ASCII token. Never Debug/Display text from rustls, quinn or io.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CloseCode {
    Normal = 0,
    Version = 1,
    UnknownPeer = 2,
    Duplicate = 3,
    Protocol = 4,
    Shutdown = 5,
    PairFailed = 6,
    KeyChanged = 7,
}

impl CloseCode {
    pub const ALL: [CloseCode; 8] = [
        CloseCode::Normal,
        CloseCode::Version,
        CloseCode::UnknownPeer,
        CloseCode::Duplicate,
        CloseCode::Protocol,
        CloseCode::Shutdown,
        CloseCode::PairFailed,
        CloseCode::KeyChanged,
    ];

    pub fn code(self) -> u32 {
        self as u32
    }

    pub fn reason(self) -> &'static [u8] {
        match self {
            CloseCode::Normal => b"normal",
            CloseCode::Version => b"version",
            CloseCode::UnknownPeer => b"unknown-peer",
            CloseCode::Duplicate => b"duplicate",
            CloseCode::Protocol => b"protocol",
            CloseCode::Shutdown => b"shutdown",
            CloseCode::PairFailed => b"pair-failed",
            CloseCode::KeyChanged => b"key-changed",
        }
    }

    pub fn from_code(c: u64) -> Option<Self> {
        Self::ALL.into_iter().find(|k| u64::from(k.code()) == c)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello {
    pub version: u16,
    pub min_version: u16,
    pub device_name: String,
    pub modules: ModuleSet,
    pub listen_port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModuleMsg {
    Clip(ClipMsg),
    Input(InputMsg),
}

impl ModuleMsg {
    pub fn kind(&self) -> ModuleKind {
        match self {
            ModuleMsg::Clip(_) => ModuleKind::Clipboard,
            ModuleMsg::Input(_) => ModuleKind::Input,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    ModuleDisabled,
    NotAuthorized,
    Paused,
    TooLarge,
    Invalid,
    Duplicate,
    Stale,
    Expired,
    ApplyFailed,
    Locked,
    VersionMismatch,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Datagram {
    Input(InputDatagram),
}

/// Structural checks run after every decode (and before every encode).
pub trait Validate {
    fn validate(&self) -> Result<(), WireError>;
}

#[derive(thiserror::Error, Debug)]
pub enum WireError {
    #[error("frame too large: {len} > {max}")]
    TooLarge { len: usize, max: usize },
    #[error("decode")]
    Decode,
    #[error("invalid: {0}")]
    Invalid(&'static str),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl Validate for Frame {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            Frame::Hello(h) => h.validate(),
            Frame::Module(m) => m.validate(),
            Frame::Bye { .. }
            | Frame::Ping { .. }
            | Frame::Pong { .. }
            | Frame::Pair(_)
            | Frame::Rejected { .. } => Ok(()),
        }
    }
}

impl Validate for Hello {
    fn validate(&self) -> Result<(), WireError> {
        validate_name(&self.device_name)?;
        if self.min_version > self.version {
            return Err(WireError::Invalid("hello min_version > version"));
        }
        Ok(())
    }
}

impl Validate for ModuleMsg {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            ModuleMsg::Clip(c) => c.validate(),
            ModuleMsg::Input(i) => i.validate(),
        }
    }
}

impl Validate for Datagram {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            Datagram::Input(d) => d.validate(),
        }
    }
}

/// Device names: 1..=MAX_NAME_BYTES bytes of UTF-8.
pub(crate) fn validate_name(name: &str) -> Result<(), WireError> {
    if name.is_empty() {
        return Err(WireError::Invalid("empty device name"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(WireError::Invalid("device name too long"));
    }
    Ok(())
}

/// Encodes with the length prefix. Fails if the body exceeds `max` or `validate()` fails.
pub fn encode_frame(f: &Frame, max: usize) -> Result<Vec<u8>, WireError> {
    f.validate()?;
    let body = postcard::to_allocvec(f).map_err(|_| WireError::Invalid("encode"))?;
    let len = body.len();
    let too_large = WireError::TooLarge { len, max };
    if len > max {
        return Err(too_large);
    }
    let prefix = u32::try_from(len).map_err(|_| too_large)?;
    let mut out = Vec::with_capacity(4 + len);
    out.extend_from_slice(&prefix.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decodes one body (without prefix) and validates it. Trailing bytes are an error.
pub fn decode_frame_body(body: &[u8]) -> Result<Frame, WireError> {
    let frame: Frame = decode_exact(body)?;
    frame.validate()?;
    Ok(frame)
}

pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R, max: usize) -> Result<Frame, WireError> {
    let mut hdr = [0u8; 4];
    r.read_exact(&mut hdr).await?;
    let len = usize::try_from(u32::from_be_bytes(hdr)).unwrap_or(usize::MAX);
    if len > max {
        return Err(WireError::TooLarge { len, max });
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).await?;
    decode_frame_body(&body)
}

pub async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W,
    f: &Frame,
    max: usize,
) -> Result<(), WireError> {
    let buf = encode_frame(f, max)?;
    w.write_all(&buf).await?;
    w.flush().await?;
    Ok(())
}

/// Fails if the encoded datagram exceeds MAX_DATAGRAM_BYTES.
pub fn encode_datagram(d: &Datagram) -> Result<Vec<u8>, WireError> {
    d.validate()?;
    let out = postcard::to_allocvec(d).map_err(|_| WireError::Invalid("encode"))?;
    if out.len() > MAX_DATAGRAM_BYTES {
        return Err(WireError::TooLarge {
            len: out.len(),
            max: MAX_DATAGRAM_BYTES,
        });
    }
    Ok(out)
}

pub fn decode_datagram(b: &[u8]) -> Result<Datagram, WireError> {
    if b.len() > MAX_DATAGRAM_BYTES {
        return Err(WireError::TooLarge {
            len: b.len(),
            max: MAX_DATAGRAM_BYTES,
        });
    }
    let d: Datagram = decode_exact(b)?;
    d.validate()?;
    Ok(d)
}

fn decode_exact<'a, T: Deserialize<'a>>(b: &'a [u8]) -> Result<T, WireError> {
    let (value, rest) = postcard::take_from_bytes::<T>(b).map_err(|_| WireError::Decode)?;
    if !rest.is_empty() {
        return Err(WireError::Decode);
    }
    Ok(value)
}
