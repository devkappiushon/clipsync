//! Protocol 4: public inventories and a fixed table of return routes.
use std::{collections::HashSet, fmt};

use crate::{
    Edge, MonitorInventory, MonitorLink, MonitorPosition, MonitorUid, OfferDevice, OfferLink,
    Validate, WireError,
    limits::{BoundedVecSeed, MAX_LAYOUT_LINKS, MAX_MONITOR_POSITIONS, MAX_ZONES},
    validate_monitor_name,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteId(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryRevisions {
    /// Opaque nonzero echo at the receiver; checked against local inventory by the source.
    pub source: u64,
    /// Checked against the receiver's current local inventory by the runtime.
    pub target: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExitRoute {
    pub id: RouteId,
    pub monitor: MonitorUid,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}
/// Exactly one required route and one optional route; no remote-controlled vector length.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRoutes {
    pub first: ExitRoute,
    pub second: Option<ExitRoute>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnterRoutes {
    pub session: u32,
    pub inventories: InventoryRevisions,
    pub monitor: MonitorUid,
    pub edge: Edge,
    pub pos: f32,
    pub routes: SessionRoutes,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnedRoute {
    pub session: u32,
    pub inventories: InventoryRevisions,
    pub route: RouteId,
    pub pos: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutOfferV4 {
    pub rev: u64,
    pub devices: Vec<OfferDevice>,
    pub positions: Vec<MonitorPosition>,
    pub links: Vec<MonitorLink>,
    pub legacy_links: Vec<OfferLink>,
}

impl<'de> Deserialize<'de> for LayoutOfferV4 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["rev", "devices", "positions", "links", "legacy_links"];
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Rev,
            Devices,
            Positions,
            Links,
            LegacyLinks,
        }
        struct OfferVisitor;
        impl<'de> Visitor<'de> for OfferVisitor {
            type Value = LayoutOfferV4;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded monitor layout offer")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let rev = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::missing_field("rev"))?;
                let devices = seq
                    .next_element_seed(BoundedVecSeed::new(MAX_ZONES))?
                    .ok_or_else(|| de::Error::missing_field("devices"))?;
                let positions = seq
                    .next_element_seed(BoundedVecSeed::new(MAX_MONITOR_POSITIONS))?
                    .ok_or_else(|| de::Error::missing_field("positions"))?;
                let links: Vec<MonitorLink> = seq
                    .next_element_seed(BoundedVecSeed::new(MAX_LAYOUT_LINKS))?
                    .ok_or_else(|| de::Error::missing_field("links"))?;
                let legacy_links = seq
                    .next_element_seed(BoundedVecSeed::new(MAX_LAYOUT_LINKS - links.len()))?
                    .ok_or_else(|| de::Error::missing_field("legacy_links"))?;
                Ok(LayoutOfferV4 {
                    rev,
                    devices,
                    positions,
                    links,
                    legacy_links,
                })
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut rev, mut devices, mut positions) = (None, None, None);
                let mut links: Option<Vec<MonitorLink>> = None;
                let mut legacy_links: Option<Vec<OfferLink>> = None;
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Rev => {
                            if rev.is_some() {
                                return Err(de::Error::duplicate_field("rev"));
                            }
                            rev = Some(map.next_value()?);
                        }
                        Field::Devices => {
                            if devices.is_some() {
                                return Err(de::Error::duplicate_field("devices"));
                            }
                            devices = Some(map.next_value_seed(BoundedVecSeed::new(MAX_ZONES))?);
                        }
                        Field::Positions => {
                            if positions.is_some() {
                                return Err(de::Error::duplicate_field("positions"));
                            }
                            positions = Some(
                                map.next_value_seed(BoundedVecSeed::new(MAX_MONITOR_POSITIONS))?,
                            );
                        }
                        Field::Links => {
                            if links.is_some() {
                                return Err(de::Error::duplicate_field("links"));
                            }
                            links = Some(map.next_value_seed(BoundedVecSeed::new(
                                MAX_LAYOUT_LINKS - legacy_links.as_ref().map_or(0, Vec::len),
                            ))?);
                        }
                        Field::LegacyLinks => {
                            if legacy_links.is_some() {
                                return Err(de::Error::duplicate_field("legacy_links"));
                            }
                            legacy_links = Some(map.next_value_seed(BoundedVecSeed::new(
                                MAX_LAYOUT_LINKS - links.as_ref().map_or(0, Vec::len),
                            ))?);
                        }
                    }
                }
                Ok(LayoutOfferV4 {
                    rev: rev.ok_or_else(|| de::Error::missing_field("rev"))?,
                    devices: devices.ok_or_else(|| de::Error::missing_field("devices"))?,
                    positions: positions.ok_or_else(|| de::Error::missing_field("positions"))?,
                    links: links.ok_or_else(|| de::Error::missing_field("links"))?,
                    legacy_links: legacy_links
                        .ok_or_else(|| de::Error::missing_field("legacy_links"))?,
                })
            }
        }
        d.deserialize_struct("LayoutOfferV4", FIELDS, OfferVisitor)
    }
}

fn fraction(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
fn nonzero(value: u64) -> Result<(), WireError> {
    if value == 0 {
        Err(WireError::Invalid("zero session or revision"))
    } else {
        Ok(())
    }
}
fn span(from: f32, to: f32) -> Result<(), WireError> {
    if fraction(from) && fraction(to) && from < to {
        Ok(())
    } else {
        Err(WireError::Invalid("segment not 0 <= from < to <= 1"))
    }
}
fn position(pos: f32) -> Result<(), WireError> {
    if fraction(pos) {
        Ok(())
    } else {
        Err(WireError::Invalid("pos outside 0..=1"))
    }
}
fn canvas(value: f64) -> bool {
    value.is_finite() && (-1_000_000.0..=1_000_000.0).contains(&value)
}
impl Validate for MonitorUid {
    fn validate(&self) -> Result<(), WireError> {
        if self.0 == [0; 16] {
            Err(WireError::Invalid("monitor uid"))
        } else {
            Ok(())
        }
    }
}
impl Validate for MonitorInventory {
    fn validate(&self) -> Result<(), WireError> {
        nonzero(self.rev)?;
        if self.monitors.len() > MAX_ZONES {
            return Err(WireError::Invalid("too many monitors"));
        }
        let mut ids = HashSet::new();
        for monitor in &self.monitors {
            monitor.uid.validate()?;
            validate_monitor_name(&monitor.label)?;
            if !ids.insert(monitor.uid) {
                return Err(WireError::Invalid("duplicate monitor uid"));
            }
            if monitor.rect.width == 0
                || monitor.rect.height == 0
                || !monitor.scale.is_finite()
                || monitor.scale <= 0.0
            {
                return Err(WireError::Invalid("invalid monitor geometry"));
            }
        }
        Ok(())
    }
}
impl Validate for RouteId {
    fn validate(&self) -> Result<(), WireError> {
        if self.0 <= 1 {
            Ok(())
        } else {
            Err(WireError::Invalid("route id"))
        }
    }
}
impl Validate for InventoryRevisions {
    fn validate(&self) -> Result<(), WireError> {
        nonzero(self.source)?;
        nonzero(self.target)
    }
}
impl Validate for ExitRoute {
    fn validate(&self) -> Result<(), WireError> {
        self.id.validate()?;
        self.monitor.validate()?;
        span(self.from, self.to)
    }
}
impl Validate for SessionRoutes {
    fn validate(&self) -> Result<(), WireError> {
        self.first.validate()?;
        if let Some(second) = &self.second {
            second.validate()?;
            if second.id == self.first.id {
                return Err(WireError::Invalid("duplicate route id"));
            }
        }
        Ok(())
    }
}
impl Validate for EnterRoutes {
    fn validate(&self) -> Result<(), WireError> {
        nonzero(u64::from(self.session))?;
        self.inventories.validate()?;
        self.monitor.validate()?;
        position(self.pos)?;
        self.routes.validate()
    }
}
impl Validate for ReturnedRoute {
    fn validate(&self) -> Result<(), WireError> {
        nonzero(u64::from(self.session))?;
        self.inventories.validate()?;
        self.route.validate()?;
        position(self.pos)
    }
}
impl Validate for LayoutOfferV4 {
    fn validate(&self) -> Result<(), WireError> {
        nonzero(self.rev)?;
        if self.devices.len() > MAX_ZONES
            || self.positions.len() > MAX_MONITOR_POSITIONS
            || self.links.len().saturating_add(self.legacy_links.len()) > MAX_LAYOUT_LINKS
        {
            return Err(WireError::Invalid("too many layout elements"));
        }
        let mut peers = HashSet::new();
        for device in &self.devices {
            if !peers.insert(device.peer)
                || !canvas(f64::from(device.x))
                || !canvas(f64::from(device.y))
            {
                return Err(WireError::Invalid("invalid offer device"));
            }
        }
        let mut keys = HashSet::new();
        for pos in &self.positions {
            pos.key.uid.validate()?;
            if !peers.contains(&pos.key.peer)
                || !keys.insert(pos.key)
                || !canvas(pos.x)
                || !canvas(pos.y)
            {
                return Err(WireError::Invalid("invalid monitor position"));
            }
        }
        // The same opaque UID may exist on distinct peers; each peer's keys are unique.
        let mut ids = HashSet::new();
        for link in &self.links {
            validate_monitor_name(&link.id)?;
            if !ids.insert(&link.id) {
                return Err(WireError::Invalid("duplicate link id"));
            }
            for endpoint in [&link.a, &link.b] {
                endpoint.key.uid.validate()?;
                if !peers.contains(&endpoint.key.peer) {
                    return Err(WireError::Invalid("unknown endpoint peer"));
                }
                span(endpoint.from, endpoint.to)?;
            }
        }
        for link in &self.legacy_links {
            for endpoint in [&link.a, &link.b] {
                if !peers.contains(&endpoint.peer) {
                    return Err(WireError::Invalid("unknown endpoint peer"));
                }
                endpoint.validate()?;
            }
        }
        Ok(())
    }
}
