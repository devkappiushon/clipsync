//! 5 · Clipboard (§12.5): the box of received items, preview, mode and sending. Owned by Rivet.

use clipsync_proto::DeliveryState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemState {
    Received,
    Applied,
    InInbox,
    Rejected,
    Expired,
    Unreachable,
}

impl From<DeliveryState> for ItemState {
    fn from(s: DeliveryState) -> Self {
        match s {
            DeliveryState::Received => ItemState::Received,
            DeliveryState::Applied => ItemState::Applied,
            DeliveryState::InInbox => ItemState::InInbox,
            DeliveryState::Rejected(_) => ItemState::Rejected,
            DeliveryState::Expired => ItemState::Expired,
            DeliveryState::Unreachable => ItemState::Unreachable,
        }
    }
}
