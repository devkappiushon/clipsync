//! Shared types, limits, wire codec and the core <-> module contract.
//!
//! Every public item is also re-exported at the crate root, so both
//! `clipsync_proto::PeerId` and `clipsync_proto::ids::PeerId` work.

pub mod clip;
pub mod control;
pub mod diag;
pub mod ids;
pub mod input;
pub mod limits;
pub mod module;
pub mod msg;
mod redact;
#[cfg(feature = "testing")]
pub mod testing;
pub mod time;
pub mod wire;

pub use clip::*;
pub use control::*;
pub use diag::*;
pub use ids::*;
pub use input::*;
pub use limits::*;
pub use module::*;
#[cfg(feature = "testing")]
pub use testing::*;
pub use time::*;
pub use wire::*;
