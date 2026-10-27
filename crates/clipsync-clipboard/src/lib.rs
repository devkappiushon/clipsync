//! Clipboard adapter (trait, fake, arboard), module, probe and diagnostics (contract §7).
pub mod adapter;
pub mod config;
pub mod diag;
mod ledger;
mod module;
pub mod msg;
pub mod probe;

use clipsync_proto::{Module, ModuleError};
use std::path::Path;

pub use adapter::{
    ClipError, ClipSnapshot, ClipboardAdapter, FakeClipboard, FakeClipboardHandle, X11Clipboard,
};
pub use config::{AdapterChoice, ClipMode, ClipboardConfig};
pub use diag::diagnose;
pub use module::build_with_adapter;

/// Builds the module with the adapter picked by `cfg.adapter`. `auto` and `x11` need XWayland (DISPLAY).
pub fn build(cfg: ClipboardConfig, _profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError> {
    let adapter: Box<dyn ClipboardAdapter> = match cfg.adapter {
        AdapterChoice::Fake => Box::new(FakeClipboard::new().0),
        AdapterChoice::Auto | AdapterChoice::X11 => match X11Clipboard::new() {
            Ok(a) => Box::new(a),
            Err(ClipError::NoDisplay) => return Err(ModuleError::Adapter(msg::NO_DISPLAY.into())),
            Err(e) => return Err(ModuleError::Adapter(e.to_string())),
        },
    };
    Ok(build_with_adapter(cfg, adapter))
}
