use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    #[error("invalid hex")]
    Hex,
    #[error("invalid length")]
    Length,
}

/// Device identity: SHA-256 of the certificate's SubjectPublicKeyInfo DER.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    pub fn from_spki_der(spki: &[u8]) -> Self {
        Self(Sha256::digest(spki).into())
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, IdError> {
        if s.len() != 64 {
            return Err(IdError::Length);
        }
        let mut out = [0u8; 32];
        hex::decode_to_slice(s, &mut out).map_err(|_| IdError::Hex)?;
        Ok(Self(out))
    }

    /// First 8 bytes as `ab12-cd34-ef56-7890`.
    pub fn short(&self) -> String {
        let h = hex::encode(&self.0[..8]);
        format!("{}-{}-{}-{}", &h[0..4], &h[4..8], &h[8..12], &h[12..16])
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short())
    }
}

impl fmt::Debug for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub [u8; 16]);

impl EventId {
    pub fn random() -> Self {
        Self(rand::random())
    }

    /// First 4 bytes as 8 hex chars.
    pub fn short(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short())
    }
}

impl fmt::Debug for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.short())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleKind {
    Clipboard,
    Input,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSet {
    pub clipboard: bool,
    pub input: bool,
}

impl ModuleSet {
    pub fn contains(&self, k: ModuleKind) -> bool {
        match k {
            ModuleKind::Clipboard => self.clipboard,
            ModuleKind::Input => self.input,
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.clipboard && !self.input
    }
}
