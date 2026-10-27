//! Exit codes: 0 ok, 1 error, 2 usage (clap), 3 refused/not confirmed, 4 agent not running.
use crate::msg;
use clipsync_core::CoreError;

#[derive(Debug)]
pub struct Fail {
    pub code: u8,
    pub message: String,
}

impl Fail {
    pub fn error(message: impl Into<String>) -> Self {
        Self { code: 1, message: message.into() }
    }
    pub fn refused(message: impl Into<String>) -> Self {
        Self { code: 3, message: message.into() }
    }
    pub fn not_running() -> Self {
        Self { code: 4, message: msg::AGENT_STOPPED.into() }
    }
}

impl From<CoreError> for Fail {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::AgentNotRunning => Fail::not_running(),
            other => Fail::error(other.to_string()),
        }
    }
}

impl From<std::io::Error> for Fail {
    fn from(e: std::io::Error) -> Self {
        Fail::error(e.to_string())
    }
}
