//! Local control plane. No private monitor evidence belongs here.
use crate::{LayoutOfferV4, MonitorInventory, MonitorLink, MonitorPosition, PeerId};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryStamp {
    pub peer: PeerId,
    pub connection: u64,
    pub rev: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveInventory {
    pub stamp: InventoryStamp,
    pub inventory: MonitorInventory,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationRequest {
    pub base_rev: u64,
    pub inventories: Vec<InventoryStamp>,
    pub positions: Vec<MonitorPosition>,
    pub replace_link_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationPreview {
    pub base_rev: u64,
    pub inventories: Vec<InventoryStamp>,
    pub positions: Vec<MonitorPosition>,
    pub effective_links: Vec<MonitorLink>,
    pub preserved_link_ids: Vec<String>,
    pub replaced_link_ids: Vec<String>,
    pub unresolved_link_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationState {
    Confirmed,
    Unconfirmed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurabilityState {
    Confirmed,
    Unconfirmed,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationApplied {
    pub layout_rev: u64,
    pub activation: ActivationState,
    pub durability: DurabilityState,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationFailure {
    pub code: String,
    pub current_rev: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingMonitorOffer {
    pub from: PeerId,
    pub connection: u64,
    pub offer: LayoutOfferV4,
}
