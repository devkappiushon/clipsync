//! Capture/emulation adapters (trait, fake, portal), module, probe and diagnostics
//! (contract §8).
pub mod adapter;
mod capture;
pub mod config;
mod diag;
pub mod keymap;
pub mod layout;
mod module;
pub mod msg;
pub mod probe;
mod self_test;
mod span;

use clipsync_proto::{Module, ModuleError};
use std::path::Path;

pub use adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, FakeCapture, FakeCaptureHandle,
    FakeEmulate, FakeEmulateHandle, Hotkey, InputError, TokenStore, Zone, ZoneRef,
};
#[cfg(target_os = "linux")]
pub use adapter::{PortalCapture, PortalEmulate};
pub use config::{AdapterChoice, BarrierConfig, InputConfig, map_pos, zone_ref};
pub use diag::diagnose;
pub use layout::{
    DerivedBarrier, Endpoint, Layout, LayoutDevice, LayoutError, LayoutOptions, Link,
    trim_dead_corner,
};
pub use module::{build_with_adapters, build_with_layout};
pub use self_test::{SelfTestOutcome, self_test};

pub const LAYOUT_FILE: &str = "layout.toml";

/// Whether this build has an input backend (the portals on Linux, the Win32 adapters on Windows).
/// The one source for the CLI and the UI (contract §13.6).
pub const SUPPORTED: bool = cfg!(any(target_os = "linux", windows));

/// Builds the module with the adapters picked by `cfg`. `auto` means the Wayland portals on Linux
/// and the Win32 adapters on Windows.
/// Reads `DIR/layout.toml`: links there and manual `[[input.barrier]]` together are a
/// `ModuleError::Config` (contract §12.6 item 3).
pub fn build(cfg: InputConfig, profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError> {
    let cap: Box<dyn CaptureAdapter> = match cfg.capture {
        AdapterChoice::Fake => Box::new(FakeCapture::new(vec![fake_zone()]).0),
        AdapterChoice::Portal => portal_capture()?,
        AdapterChoice::Windows => windows_capture()?,
        AdapterChoice::Auto if cfg!(windows) => windows_capture()?,
        AdapterChoice::Auto => portal_capture()?,
    };
    let emu: Box<dyn EmulateAdapter> = match cfg.emulate {
        AdapterChoice::Fake => Box::new(FakeEmulate::new().0),
        AdapterChoice::Portal => portal_emulate(profile_dir)?,
        AdapterChoice::Windows => windows_emulate()?,
        AdapterChoice::Auto if cfg!(windows) => windows_emulate()?,
        AdapterChoice::Auto => portal_emulate(profile_dir)?,
    };
    build_with_layout(cfg, cap, emu, profile_dir)
}

/// The one monitor of `capture = "fake"`: a layout needs an edge to link.
pub fn fake_zone() -> Zone {
    Zone {
        index: 0,
        name: Some("fake-0".to_owned()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}

#[cfg(target_os = "linux")]
fn portal_capture() -> Result<Box<dyn CaptureAdapter>, ModuleError> {
    Ok(Box::new(PortalCapture::new()))
}
#[cfg(target_os = "linux")]
fn portal_emulate(dir: &Path) -> Result<Box<dyn EmulateAdapter>, ModuleError> {
    Ok(Box::new(PortalEmulate::new(TokenStore::new(dir))))
}
#[cfg(not(target_os = "linux"))]
fn portal_capture() -> Result<Box<dyn CaptureAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::UNAVAILABLE_ON_PLATFORM.into()))
}
#[cfg(not(target_os = "linux"))]
fn portal_emulate(_: &Path) -> Result<Box<dyn EmulateAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::UNAVAILABLE_ON_PLATFORM.into()))
}

// The Win32 adapters land with `adapter/win/` (Flux, §13.6); until then every target answers
// "unavailable" and `WinCapture`/`WinEmulate` replace these two bodies.
fn windows_capture() -> Result<Box<dyn CaptureAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::UNAVAILABLE_ON_PLATFORM.into()))
}
fn windows_emulate() -> Result<Box<dyn EmulateAdapter>, ModuleError> {
    Err(ModuleError::Adapter(msg::UNAVAILABLE_ON_PLATFORM.into()))
}
