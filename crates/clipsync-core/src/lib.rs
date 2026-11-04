//! ClipSync core: identity, profile, pairing, QUIC mesh, agent and local IPC.
//!
//! Ownership (contract §5, v1.6): Rivet owns Cargo.toml, lib.rs (CoreError), tls.rs, pairing.rs,
//! transport.rs, agent.rs and msg.rs; Flux owns profile.rs, control.rs and diag.rs.

use clipsync_proto::CloseCode;

pub mod agent;
pub mod control;
pub mod diag;
pub mod logging;
pub mod msg;
pub mod pairing;
pub mod profile;
pub mod tls;
pub mod transport;

pub use profile::*;

/// The single error of the core, for both halves. Facts only; Portuguese text lives in the CLI.
/// A new variant is requested from Rivet (contract §5, boundary rule 1).
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid config.toml, both modules false, version != 1.
    #[error("config: {0}")]
    Config(String),
    /// Missing key, bad PEM, key mode != 0600.
    #[error("identity: {0}")]
    Identity(String),
    /// Profile not initialized or already initialized, bad peers/state.
    #[error("profile: {0}")]
    Profile(String),
    #[error("pairing: {0}")]
    Pairing(String),
    #[error("transport: {0}")]
    Transport(String),
    #[error("agent not running")]
    AgentNotRunning,
    /// control::serve found another live agent on the socket.
    #[error("agent already running")]
    AgentAlreadyRunning,
    #[error("closed: {0:?}")]
    Closed(CloseCode),
}
