use clipsync_proto::{MAX_CLIP_TEXT_BYTES, ModuleError};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterChoice {
    Auto,
    X11,
    Fake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardConfig {
    pub adapter: AdapterChoice,
    pub mode: ClipMode,
    pub max_text_bytes: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    adapter: Option<AdapterChoice>,
    mode: Option<ClipMode>,
    max_text_bytes: Option<usize>,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            adapter: AdapterChoice::Auto,
            mode: ClipMode::Auto,
            max_text_bytes: MAX_CLIP_TEXT_BYTES,
        }
    }
}

impl ClipboardConfig {
    /// `None` (no `[clipboard]` table) means all defaults. Unknown keys are an error.
    pub fn from_table(t: Option<&toml::Table>) -> Result<Self, ModuleError> {
        let Some(t) = t else {
            return Ok(Self::default());
        };
        let raw: Raw = t
            .clone()
            .try_into()
            .map_err(|e| ModuleError::Config(e.to_string()))?;
        let d = Self::default();
        let max = raw.max_text_bytes.unwrap_or(d.max_text_bytes);
        if max == 0 || max > MAX_CLIP_TEXT_BYTES {
            return Err(ModuleError::Config(format!(
                "max_text_bytes deve estar entre 1 e {MAX_CLIP_TEXT_BYTES}"
            )));
        }
        Ok(Self {
            adapter: raw.adapter.unwrap_or(d.adapter),
            mode: raw.mode.unwrap_or(d.mode),
            max_text_bytes: max,
        })
    }
}
