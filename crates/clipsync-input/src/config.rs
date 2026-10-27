use clipsync_proto::{Edge, ModuleError};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterChoice {
    Auto,
    Portal,
    Fake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum EdgeRaw {
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

#[derive(Debug, Clone, PartialEq)]
pub struct BarrierConfig {
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
    pub peer: String,
    pub peer_edge: Edge,
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
            let ok = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
            if !ok(from) || !ok(to) || from >= to {
                return Err(ModuleError::Config(
                    "barreira: from e to devem estar em 0..1 com from < to".into(),
                ));
            }
            barriers.push(BarrierConfig {
                monitor: b.monitor,
                edge: b.edge.into(),
                from,
                to,
                peer: b.peer,
                peer_edge: b.peer_edge.into(),
            });
        }
        Ok(Self {
            capture: raw.capture.unwrap_or(d.capture),
            emulate: raw.emulate.unwrap_or(d.emulate),
            barriers,
        })
    }
}
