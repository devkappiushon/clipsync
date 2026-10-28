use crate::adapter::ZoneRef;
use clipsync_proto::{Edge, ModuleError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterChoice {
    Auto,
    Portal,
    Windows,
    Fake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EdgeRaw {
    Left,
    Right,
    Top,
    Bottom,
}

impl From<EdgeRaw> for Edge {
    fn from(e: EdgeRaw) -> Self {
        match e {
            EdgeRaw::Left => Edge::Left,
            EdgeRaw::Right => Edge::Right,
            EdgeRaw::Top => Edge::Top,
            EdgeRaw::Bottom => Edge::Bottom,
        }
    }
}

impl From<Edge> for EdgeRaw {
    fn from(e: Edge) -> Self {
        match e {
            Edge::Left => EdgeRaw::Left,
            Edge::Right => EdgeRaw::Right,
            Edge::Top => EdgeRaw::Top,
            Edge::Bottom => EdgeRaw::Bottom,
        }
    }
}

/// A manual `[[input.barrier]]`. `peer` is the peer's device name.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierConfig {
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
    pub peer: String,
    pub peer_edge: Edge,
    /// Destination monitor (connector name or `"#idx"`). `None`: the destination picks zone 0.
    pub peer_monitor: Option<String>,
    /// Destination segment on `peer_edge` (contract §12.6 item 4). `None`: the whole edge.
    pub peer_from: Option<f32>,
    pub peer_to: Option<f32>,
}

impl BarrierConfig {
    /// Destination segment: `peer_from..peer_to`, or the same fractions as the local one when
    /// they are not given (the cursor keeps its height, as in 0.1.0).
    pub fn remote_segment(&self) -> (f32, f32) {
        (
            self.peer_from.unwrap_or(self.from),
            self.peer_to.unwrap_or(self.to),
        )
    }

    /// Position on the destination edge for a cursor at `pos` on the local edge.
    pub fn map_pos(&self, pos: f32) -> f32 {
        map_pos((self.from, self.to), self.remote_segment(), pos)
    }
}

/// `peer_from + t * (peer_to - peer_from)`, with `t = (pos - from) / (to - from)` clamped to 0..=1.
pub fn map_pos((from, to): (f32, f32), (peer_from, peer_to): (f32, f32), pos: f32) -> f32 {
    let span = to - from;
    let t = if span > 0.0 {
        ((pos - from) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (peer_from + t * (peer_to - peer_from)).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputConfig {
    pub capture: AdapterChoice,
    pub emulate: AdapterChoice,
    pub barriers: Vec<BarrierConfig>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBarrier {
    monitor: String,
    edge: EdgeRaw,
    from: Option<f32>,
    to: Option<f32>,
    peer: String,
    peer_edge: EdgeRaw,
    peer_monitor: Option<String>,
    peer_from: Option<f32>,
    peer_to: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    capture: Option<AdapterChoice>,
    emulate: Option<AdapterChoice>,
    #[serde(default)]
    barrier: Vec<RawBarrier>,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            capture: AdapterChoice::Auto,
            emulate: AdapterChoice::Auto,
            barriers: Vec::new(),
        }
    }
}

fn segment_ok(from: f32, to: f32) -> bool {
    let ok = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
    ok(from) && ok(to) && from < to
}

impl InputConfig {
    /// `None` (no `[input]` table) means all defaults. Unknown keys are an error.
    pub fn from_table(t: Option<&toml::Table>) -> Result<Self, ModuleError> {
        let Some(t) = t else {
            return Ok(Self::default());
        };
        let raw: Raw = t
            .clone()
            .try_into()
            .map_err(|e| ModuleError::Config(e.to_string()))?;
        let d = Self::default();
        let mut barriers = Vec::new();
        for b in raw.barrier {
            let (from, to) = (b.from.unwrap_or(0.0), b.to.unwrap_or(1.0));
            if !segment_ok(from, to) {
                return Err(ModuleError::Config(
                    "barreira: from e to devem estar em 0..1 com from < to".into(),
                ));
            }
            if (b.peer_from.is_some() || b.peer_to.is_some())
                && !segment_ok(b.peer_from.unwrap_or(0.0), b.peer_to.unwrap_or(1.0))
            {
                return Err(ModuleError::Config(
                    "barreira: peer_from e peer_to devem estar em 0..1 com peer_from < peer_to"
                        .into(),
                ));
            }
            if b.peer_monitor.as_deref() == Some("") {
                return Err(ModuleError::Config(
                    "barreira: peer_monitor não pode ser vazio".into(),
                ));
            }
            barriers.push(BarrierConfig {
                monitor: b.monitor,
                edge: b.edge.into(),
                from,
                to,
                peer: b.peer,
                peer_edge: b.peer_edge.into(),
                peer_monitor: b.peer_monitor,
                peer_from: b.peer_from,
                peer_to: b.peer_to,
            });
        }
        Ok(Self {
            capture: raw.capture.unwrap_or(d.capture),
            emulate: raw.emulate.unwrap_or(d.emulate),
            barriers,
        })
    }
}

/// `"#0"` ('#' plus digits) is a zone index; anything else is a monitor name (`"eDP-1"`).
pub fn zone_ref(monitor: &str) -> ZoneRef {
    monitor
        .strip_prefix('#')
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|d| d.parse::<u32>().ok())
        .map_or_else(|| ZoneRef::Name(monitor.to_owned()), ZoneRef::Index)
}
