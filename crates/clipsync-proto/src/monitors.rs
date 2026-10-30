//! Opaque monitor identities and observed geometry. No hardware evidence travels here.
use crate::{Edge, PeerId, WireError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MonitorUid(pub [u8; 16]);
impl MonitorUid {
    pub fn random() -> Self {
        loop {
            let uid = Self(rand::random());
            if uid.0 != [0; 16] {
                return uid;
            }
        }
    }
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
    pub fn from_hex(s: &str) -> Result<Self, WireError> {
        let mut bytes = [0; 16];
        if s.len() != 32 || hex::decode_to_slice(s, &mut bytes).is_err() || bytes == [0; 16] {
            return Err(WireError::Invalid("monitor uid"));
        }
        Ok(Self(bytes))
    }
}
impl Serialize for MonitorUid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.serialize_str(&self.to_hex())
        } else {
            self.0.serialize(s)
        }
    }
}
impl<'de> Deserialize<'de> for MonitorUid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let text = String::deserialize(d)?;
            Self::from_hex(&text).map_err(serde::de::Error::custom)
        } else {
            let bytes = <[u8; 16]>::deserialize(d)?;
            if bytes == [0; 16] {
                return Err(serde::de::Error::custom("monitor uid"));
            }
            Ok(Self(bytes))
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateSpace {
    Physical,
    Logical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometrySource {
    Observed,
    Estimated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub uid: MonitorUid,
    pub label: String,
    pub rect: NativeRect,
    pub scale: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorCapabilities {
    pub internal_edges: bool,
    pub positioned_return: bool,
    pub route_receiver: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInventory {
    pub rev: u64,
    pub space: CoordinateSpace,
    pub source: GeometrySource,
    pub capabilities: MonitorCapabilities,
    pub monitors: Vec<MonitorInfo>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonitorKey {
    pub peer: PeerId,
    pub uid: MonitorUid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorPosition {
    pub key: MonitorKey,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorEndpoint {
    pub key: MonitorKey,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorLink {
    pub id: String,
    pub a: MonitorEndpoint,
    pub b: MonitorEndpoint,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn uid_is_hex_for_humans_and_exactly_sixteen_bytes_on_wire() {
        let uid = MonitorUid([0xab; 16]);
        assert_eq!(
            serde_json::to_string(&uid).unwrap(),
            "\"abababababababababababababababab\""
        );
        assert_eq!(postcard::to_allocvec(&uid).unwrap(), vec![0xab; 16]);
        assert_eq!(
            postcard::from_bytes::<MonitorUid>(&[0xab; 16]).unwrap(),
            uid
        );
        assert!(postcard::from_bytes::<MonitorUid>(&[0; 16]).is_err());
        assert!(MonitorUid::from_hex("00000000000000000000000000000000").is_err());
        assert!(MonitorUid::from_hex("abab").is_err());
        assert!(MonitorUid::from_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }
}
