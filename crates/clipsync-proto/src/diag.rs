use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagStatus {
    Ok,
    Warn,
    Missing,
    Error,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiagItem {
    pub area: String,
    pub name: String,
    pub status: DiagStatus,
    pub detail: String,
}
