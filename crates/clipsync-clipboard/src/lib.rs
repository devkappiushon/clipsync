//! Clipboard adapter (trait, fake, arboard), module, probe and diagnostics (contract §7).
pub mod adapter;
pub mod config;
pub mod diag;
mod ledger;
mod module;
pub mod msg;
#[cfg(target_os = "linux")]
pub mod probe;

use clipsync_proto::{Module, ModuleError};
use std::path::Path;

#[cfg(windows)]
pub use adapter::WinClipboard;
#[cfg(target_os = "linux")]
pub use adapter::X11Clipboard;
pub use adapter::{ClipError, ClipSnapshot, ClipboardAdapter, FakeClipboard, FakeClipboardHandle};
pub use config::{AdapterChoice, ClipMode, ClipboardConfig};
pub use diag::diagnose;
pub use module::build_with_adapter;

/// Builds the module with the adapter picked by `cfg.adapter`. `auto` is the XWayland bridge (needs
/// DISPLAY) on Linux and arboard polling on Windows.
pub fn build(cfg: ClipboardConfig, _profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError> {
    let adapter: Box<dyn ClipboardAdapter> = match cfg.adapter {
        AdapterChoice::Fake => Box::new(FakeClipboard::new().0),
        AdapterChoice::Auto => native()?,
        AdapterChoice::X11 => x11()?,
    };
    Ok(build_with_adapter(cfg, adapter))
}

#[cfg(target_os = "linux")]
fn native() -> Result<Box<dyn ClipboardAdapter>, ModuleError> {
    x11()
}

#[cfg(target_os = "linux")]
fn x11() -> Result<Box<dyn ClipboardAdapter>, ModuleError> {
    match X11Clipboard::new() {
        Ok(a) => Ok(Box::new(a)),
        Err(ClipError::NoDisplay) => Err(ModuleError::Adapter(msg::NO_DISPLAY.into())),
        Err(e) => Err(ModuleError::Adapter(e.to_string())),
    }
}

#[cfg(windows)]
fn native() -> Result<Box<dyn ClipboardAdapter>, ModuleError> {
    WinClipboard::new()
        .map(|a| Box::new(a) as Box<dyn ClipboardAdapter>)
        .map_err(|e| ModuleError::Adapter(e.to_string()))
}

#[cfg(not(target_os = "linux"))]
fn x11() -> Result<Box<dyn ClipboardAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::X11_ONLY_LINUX.into()))
}

#[cfg(not(any(target_os = "linux", windows)))]
fn native() -> Result<Box<dyn ClipboardAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::NO_ADAPTER.into()))
}
