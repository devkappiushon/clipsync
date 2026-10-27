//! Capture/emulation adapters (trait, fake, portal), module, probe and diagnostics
//! (contract §8).
pub mod adapter;
pub mod config;
mod diag;
mod module;
pub mod msg;
pub mod probe;

use clipsync_proto::{Module, ModuleError};
use std::path::Path;

pub use adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, FakeCapture, FakeCaptureHandle,
    FakeEmulate, FakeEmulateHandle, InputError, TokenStore, Zone, ZoneRef,
};
#[cfg(target_os = "linux")]
pub use adapter::{PortalCapture, PortalEmulate};
pub use config::{AdapterChoice, BarrierConfig, InputConfig};
pub use diag::diagnose;
pub use module::build_with_adapters;

/// Builds the module with the adapters picked by `cfg`. `auto` means the Wayland portals.
pub fn build(cfg: InputConfig, profile_dir: &Path) -> Result<Box<dyn Module>, ModuleError> {
    let cap: Box<dyn CaptureAdapter> = match cfg.capture {
        AdapterChoice::Fake => Box::new(FakeCapture::new(Vec::new()).0),
        AdapterChoice::Auto | AdapterChoice::Portal => portal_capture()?,
    };
    let emu: Box<dyn EmulateAdapter> = match cfg.emulate {
        AdapterChoice::Fake => Box::new(FakeEmulate::new().0),
        AdapterChoice::Auto | AdapterChoice::Portal => portal_emulate(profile_dir)?,
    };
    Ok(build_with_adapters(cfg, cap, emu))
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
    Err(ModuleError::Adapter(
        "sem adaptador de captura nesta plataforma".into(),
    ))
}
#[cfg(not(target_os = "linux"))]
fn portal_emulate(_: &Path) -> Result<Box<dyn EmulateAdapter>, ModuleError> {
    Err(ModuleError::Adapter(
        "sem adaptador de emulação nesta plataforma".into(),
    ))
}
