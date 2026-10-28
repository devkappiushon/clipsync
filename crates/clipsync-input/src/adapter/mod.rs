//! Capture and emulation adapters (contract §8).
mod barrier;
mod fake;
pub(crate) mod outputs;
#[cfg(target_os = "linux")]
mod portal;
mod tokens;

use async_trait::async_trait;
use clipsync_proto::{Edge, InputEvent};
use tokio::sync::mpsc;

pub use barrier::{Segment, barrier_segment, edge_position};
pub use fake::{FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle};
#[cfg(target_os = "linux")]
pub use portal::{PortalCapture, PortalEmulate};
pub use tokens::TokenStore;

#[derive(Debug, Clone)]
pub struct Zone {
    pub index: u32,
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

#[derive(Debug, Clone)]
pub enum ZoneRef {
    Name(String),
    Index(u32),
}

#[derive(Debug, Clone)]
pub struct BarrierSpec {
    pub zone: ZoneRef,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}

/// `Debug` is safe: `InputEvent` redacts key and button codes.
#[derive(Debug)]
pub enum CaptureEvent {
    Activated {
        barrier: usize,
        pos: f32,
        t_mono_us: u64,
    },
    Input {
        t_mono_us: u64,
        ev: InputEvent,
    },
    Deactivated,
    Error(String),
}

#[derive(thiserror::Error, Debug)]
pub enum InputError {
    #[error("portal unavailable: {0}")]
    PortalUnavailable(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("zone not found: {0}")]
    ZoneNotFound(String),
    #[error("eis: {0}")]
    Eis(String),
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait CaptureAdapter: Send + 'static {
    /// `"fake"` | `"portal-inputcapture"`.
    fn name(&self) -> &'static str;
    /// Asks the platform; on the portal this creates the session (dialog on first use).
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError>;
    /// What is known without asking the user (contract §12.6 item 6): the live zones once a
    /// session exists, else a quiet estimate; empty when there is none.
    fn known_zones(&self) -> Vec<Zone> {
        Vec::new()
    }
    /// Installs barriers and starts delivering events. A portal dialog may appear here.
    async fn start(
        &mut self,
        barriers: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError>;
    /// Gives the cursor back to the local desktop.
    async fn release(&mut self) -> Result<(), InputError>;
    async fn stop(&mut self) -> Result<(), InputError>;
}

#[async_trait]
pub trait EmulateAdapter: Send + 'static {
    /// `"fake"` | `"portal-remotedesktop"`.
    fn name(&self) -> &'static str;
    /// A portal dialog may appear here (not with a valid restore token).
    async fn start(&mut self) -> Result<(), InputError>;
    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError>;
    /// Releases every pressed key and button.
    async fn release_all(&mut self) -> Result<(), InputError>;
    async fn stop(&mut self) -> Result<(), InputError>;
}
