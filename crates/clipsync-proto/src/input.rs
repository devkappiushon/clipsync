use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::PeerId;
use crate::limits::{MAX_EVENTS_PER_DATAGRAM, MAX_LAYOUT_LINKS, MAX_ZONES};
use crate::redact::Redacted;
use crate::wire::{Validate, WireError, validate_monitor_name};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// `Debug` is manual: Key and Button never print their code.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Logical px, relative.
    Motion { dx: f32, dy: f32 },
    /// evdev BTN_*.
    Button { code: u32, pressed: bool },
    /// Smooth, logical px.
    Scroll { dx: f32, dy: f32 },
    /// v120 units.
    ScrollDiscrete { dx: i32, dy: i32 },
    /// evdev KEY_*; NEVER in logs.
    Key { code: u32, pressed: bool },
}

impl fmt::Debug for InputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputEvent::Motion { dx, dy } => f
                .debug_struct("Motion")
                .field("dx", dx)
                .field("dy", dy)
                .finish(),
            InputEvent::Button { .. } => f.debug_tuple("Button").field(&Redacted).finish(),
            InputEvent::Scroll { dx, dy } => f
                .debug_struct("Scroll")
                .field("dx", dx)
                .field("dy", dy)
                .finish(),
            InputEvent::ScrollDiscrete { dx, dy } => f
                .debug_struct("ScrollDiscrete")
                .field("dx", dx)
                .field("dy", dy)
                .finish(),
            InputEvent::Key { .. } => f.debug_tuple("Key").field(&Redacted).finish(),
        }
    }
}

/// `Debug` is manual: only session, seq and the event count.
#[derive(Serialize, Deserialize, Clone)]
pub struct InputDatagram {
    pub session: u32,
    pub seq: u32,
    pub t_mono_us: u64,
    pub events: Vec<InputEvent>,
}

impl fmt::Debug for InputDatagram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputDatagram")
            .field("session", &self.session)
            .field("seq", &self.seq)
            .field("events", &self.events.len())
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InputMsg {
    Enter {
        session: u32,
        edge: Edge,
        pos: f32,
    },
    Leave {
        session: u32,
    },
    ReleaseAll {
        session: u32,
    },
    StatsRequest {
        session: u32,
    },
    Stats(LoopbackStats),
    // ---- protocol 2 (S6); new variants only at the end: postcard tags are positional ----
    /// My monitors, sent only to peers I granted `input_in` (they may control me).
    Zones {
        zones: Vec<ZoneInfo>,
    },
    /// "Copy the layout to the other devices": the receiver decides whether to apply it.
    LayoutOffer(LayoutOffer),
    /// `Enter` that names the destination monitor (`Enter` keeps its 0.1.0 shape).
    EnterAt {
        session: u32,
        edge: Edge,
        monitor: Option<String>,
        pos: f32,
    },
}

/// One local monitor, in logical px of the desktop.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ZoneInfo {
    pub index: u32,
    /// Connector name ("DP-1") when the platform knows it.
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LayoutOffer {
    pub rev: u64,
    pub devices: Vec<OfferDevice>,
    pub links: Vec<OfferLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OfferDevice {
    pub peer: PeerId,
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OfferLink {
    pub a: OfferEndpoint,
    pub b: OfferEndpoint,
}

/// A segment of one monitor edge; `from` and `to` are fractions of the edge, `from < to`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OfferEndpoint {
    pub peer: PeerId,
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoopbackStats {
    pub session: u32,
    pub received: u64,
    pub lost: u64,
    pub out_of_order: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub max_us: u64,
}

impl Validate for InputEvent {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            InputEvent::Motion { dx, dy } | InputEvent::Scroll { dx, dy } => {
                if dx.is_finite() && dy.is_finite() {
                    Ok(())
                } else {
                    Err(WireError::Invalid("non-finite float"))
                }
            }
            InputEvent::Button { .. }
            | InputEvent::ScrollDiscrete { .. }
            | InputEvent::Key { .. } => Ok(()),
        }
    }
}

impl Validate for InputDatagram {
    fn validate(&self) -> Result<(), WireError> {
        if self.events.len() > MAX_EVENTS_PER_DATAGRAM {
            return Err(WireError::Invalid("too many events"));
        }
        self.events.iter().try_for_each(Validate::validate)
    }
}

fn unit_fraction(v: f32) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}

impl Validate for InputMsg {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            InputMsg::Enter { pos, .. } => {
                if unit_fraction(*pos) {
                    Ok(())
                } else {
                    Err(WireError::Invalid("pos outside 0..=1"))
                }
            }
            InputMsg::Leave { .. }
            | InputMsg::ReleaseAll { .. }
            | InputMsg::StatsRequest { .. }
            | InputMsg::Stats(_) => Ok(()),
            InputMsg::Zones { zones } => {
                if zones.len() > MAX_ZONES {
                    return Err(WireError::Invalid("too many zones"));
                }
                zones.iter().try_for_each(Validate::validate)
            }
            InputMsg::LayoutOffer(o) => o.validate(),
            InputMsg::EnterAt { monitor, pos, .. } => {
                if !unit_fraction(*pos) {
                    return Err(WireError::Invalid("pos outside 0..=1"));
                }
                monitor.as_deref().map_or(Ok(()), validate_monitor_name)
            }
        }
    }
}

impl Validate for ZoneInfo {
    fn validate(&self) -> Result<(), WireError> {
        if self.width == 0 || self.height == 0 {
            return Err(WireError::Invalid("empty zone"));
        }
        if !(self.scale.is_finite() && self.scale > 0.0) {
            return Err(WireError::Invalid("zone scale not finite and positive"));
        }
        self.name.as_deref().map_or(Ok(()), validate_monitor_name)
    }
}

impl Validate for LayoutOffer {
    fn validate(&self) -> Result<(), WireError> {
        if self.devices.len() > MAX_ZONES {
            return Err(WireError::Invalid("too many devices in offer"));
        }
        if self.links.len() > MAX_LAYOUT_LINKS {
            return Err(WireError::Invalid("too many links in offer"));
        }
        self.links
            .iter()
            .try_for_each(|l| l.a.validate().and_then(|()| l.b.validate()))
    }
}

impl Validate for OfferEndpoint {
    fn validate(&self) -> Result<(), WireError> {
        if !(unit_fraction(self.from) && unit_fraction(self.to) && self.from < self.to) {
            return Err(WireError::Invalid("segment not 0 <= from < to <= 1"));
        }
        validate_monitor_name(&self.monitor)
    }
}
