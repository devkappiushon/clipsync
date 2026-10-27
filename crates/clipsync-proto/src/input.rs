use std::fmt;

use serde::{Deserialize, Serialize};

use crate::limits::MAX_EVENTS_PER_DATAGRAM;
use crate::redact::Redacted;
use crate::wire::{Validate, WireError};

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
    Enter { session: u32, edge: Edge, pos: f32 },
    Leave { session: u32 },
    ReleaseAll { session: u32 },
    StatsRequest { session: u32 },
    Stats(LoopbackStats),
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

impl Validate for InputMsg {
    fn validate(&self) -> Result<(), WireError> {
        match self {
            InputMsg::Enter { pos, .. } => {
                if pos.is_finite() && (0.0..=1.0).contains(pos) {
                    Ok(())
                } else {
                    Err(WireError::Invalid("pos outside 0..=1"))
                }
            }
            InputMsg::Leave { .. }
            | InputMsg::ReleaseAll { .. }
            | InputMsg::StatsRequest { .. }
            | InputMsg::Stats(_) => Ok(()),
        }
    }
}
