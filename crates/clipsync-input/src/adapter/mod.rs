//! Capture and emulation adapters (contract §8).
mod barrier;
mod entry;
mod fake;
pub(crate) mod outputs;
#[cfg(target_os = "linux")]
mod portal;
mod tokens;
pub mod wincore;
pub mod winmath;
mod zones;

use async_trait::async_trait;
use clipsync_proto::{Edge, InputEvent};
use tokio::sync::mpsc;

pub use barrier::{Segment, barrier_segment, edge_position};
pub(crate) use entry::{entry_events, entry_events_at};
pub use fake::{FakeCapture, FakeCaptureHandle, FakeEmulate, FakeEmulateHandle};
#[cfg(target_os = "linux")]
pub use portal::{PortalCapture, PortalEmulate};
pub use tokens::TokenStore;
pub(crate) use zones::same_zones;

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

/// A shortcut the capture side recognized (contract §13.6). The keys of a shortcut go nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    /// Go to the device whose `hotkey` is this number (1 to 9).
    GoTo(u8),
    ToggleLock,
    TogglePause,
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
    /// A shortcut was recognized.
    Hotkey(Hotkey),
    /// The monitors changed (§13.8).
    ZonesChanged(Vec<Zone>),
    /// This machine's screen locked (`true`) or came back; the secure desktop counts as locked.
    SessionLocked(bool),
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
    /// Screen locked or secure desktop: nothing was emulated.
    #[error("locked")]
    Locked,
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
    /// Swaps the barriers without recreating the session (on the portal, no new dialog).
    async fn set_barriers(&mut self, barriers: &[BarrierSpec]) -> Result<(), InputError> {
        let _ = barriers;
        Err(InputError::Other("unsupported".into()))
    }
    /// True when the adapter holds system-wide input hooks while it runs (Windows). Pausing the
    /// module then stops it, so no hook stays installed while paused (§13.2 rule 2g).
    fn holds_hooks(&self) -> bool {
        false
    }
    /// While paused and stopped: listen only for the resume combination (Win+Alt+P), through a
    /// system hotkey that reads no keys. `Ok(None)`: no such thing here (resume from the tray or
    /// the app button); `Err`: the system refused the combination (same fallback).
    async fn watch_resume(&mut self) -> Result<Option<mpsc::Receiver<CaptureEvent>>, InputError> {
        Ok(None)
    }
    /// Drops what `watch_resume` registered.
    async fn unwatch_resume(&mut self) {}
    /// Gives the cursor back at barrier `barrier`, at `pos` (0 to 1), one pixel inside the zone.
    async fn release_at(&mut self, barrier: usize, pos: f32) -> Result<(), InputError> {
        let _ = (barrier, pos);
        self.release().await
    }
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
    /// Puts the cursor at the entry. Default: the relative burst of §8 rule 8.
    async fn enter(
        &mut self,
        zone: &Zone,
        zones: &[Zone],
        edge: Edge,
        pos: f32,
    ) -> Result<(), InputError> {
        self.emit(&entry_events_at(edge, pos, zone, zones)).await
    }
    /// Real cursor position when the platform says (Windows). Portal: `None`.
    fn cursor(&self) -> Option<(i32, i32)> {
        None
    }
    /// The keyboard layout this machine types with, for the diagnostics only: the `xkb_symbols`
    /// name the EIS announces (Linux) or the KLID (Windows). `None` until known.
    fn keyboard_layout(&self) -> Option<String> {
        None
    }
    async fn stop(&mut self) -> Result<(), InputError>;
}
