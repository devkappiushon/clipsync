use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::EventId;
use crate::limits::{MAX_CLIP_TEXT_BYTES, MAX_MIME_BYTES};
use crate::redact::ByteLen;
use crate::wire::{RejectReason, Validate, WireError};

pub const MIME_TEXT_UTF8: &str = "text/plain;charset=utf-8";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClipMsg {
    Offer(ClipOffer),
    Ack(ClipAck),
}

/// `Debug` is manual and redacted: never prints `text`.
#[derive(Serialize, Deserialize, Clone)]
pub struct ClipOffer {
    pub id: EventId,
    pub lamport: u64,
    pub mime: String,
    pub text: String,
    /// Display only.
    pub created_unix_ms: u64,
    pub ttl_ms: u32,
}

impl fmt::Debug for ClipOffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipOffer")
            .field("id", &self.id)
            .field("lamport", &self.lamport)
            .field("mime", &self.mime)
            .field("text", &ByteLen(self.text.len()))
            .field("created_unix_ms", &self.created_unix_ms)
            .field("ttl_ms", &self.ttl_ms)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipAck {
    pub id: EventId,
    pub state: DeliveryState,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    Received,
    Applied,
    InInbox,
    Rejected(RejectReason),
    Expired,
}

impl Validate for ClipMsg {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            ClipMsg::Offer(o) => o.validate(),
            ClipMsg::Ack(_) => Ok(()),
        }
    }
}

impl Validate for ClipOffer {
    fn validate(&self) -> Result<(), WireError> {
        if self.mime.len() > MAX_MIME_BYTES {
            return Err(WireError::Invalid("mime too long"));
        }
        if self.mime != MIME_TEXT_UTF8 {
            return Err(WireError::Invalid("unsupported mime"));
        }
        if self.text.len() > MAX_CLIP_TEXT_BYTES {
            return Err(WireError::Invalid("clip text too long"));
        }
        Ok(())
    }
}
