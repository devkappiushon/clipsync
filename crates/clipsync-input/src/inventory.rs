//! Observations come from a platform, identities from the private store, validity from this catalog.
use crate::{
    InputError, Zone,
    monitor_identities::{Evidence, Observation},
};
use clipsync_proto::{
    CoordinateSpace, GeometrySource, InventoryStamp, LiveInventory, MonitorCapabilities,
    MonitorInventory, NativeRect, PeerId, Validate,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub struct ObservedMonitors {
    pub observations: Vec<Observation>,
    pub space: CoordinateSpace,
    pub source: GeometrySource,
    pub capabilities: MonitorCapabilities,
}
pub(crate) fn unavailable() -> InputError {
    InputError::Other("inventário de monitores indisponível".into())
}

/// Shared only with the capture owner; UI snapshots can never establish validity.
#[derive(Clone, Default)]
pub struct InventoryCatalog(Arc<Mutex<HashMap<PeerId, LiveInventory>>>);
impl InventoryCatalog {
    pub fn snapshot(&self) -> Vec<LiveInventory> {
        let mut entries: Vec<_> = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect();
        entries.sort_by_key(|v| v.stamp.peer);
        entries
    }
    pub fn matches(&self, stamps: &[InventoryStamp]) -> bool {
        let entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let mut unique = HashSet::new();
        !stamps.is_empty()
            && stamps.iter().all(|s| {
                unique.insert(s.peer) && entries.get(&s.peer).is_some_and(|v| v.stamp == *s)
            })
    }
    pub(crate) fn get(&self, peer: PeerId) -> Option<LiveInventory> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&peer)
            .cloned()
    }
    pub(crate) fn insert(
        &self,
        local: PeerId,
        stamp: InventoryStamp,
        inventory: MonitorInventory,
    ) -> Result<bool, InputError> {
        if stamp.rev != inventory.rev
            || stamp.rev == 0
            || (stamp.peer == local) != (stamp.connection == 0)
            || inventory.source != GeometrySource::Observed
            || inventory.validate().is_err()
        {
            return Err(unavailable());
        }
        let mut entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let next = LiveInventory { stamp, inventory };
        let changed = entries.get(&stamp.peer) != Some(&next);
        entries.insert(stamp.peer, next);
        Ok(changed)
    }
    pub(crate) fn invalidate(&self, peer: PeerId) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&peer);
    }
}

pub fn windows_observation(
    monitors: &[clipsync_win::Monitor],
) -> Result<ObservedMonitors, InputError> {
    let mut paths = HashSet::new();
    if monitors.is_empty()
        || monitors.len() > clipsync_proto::MAX_ZONES
        || monitors
            .iter()
            .any(|m| m.path.trim().is_empty() || !paths.insert(&m.path))
    {
        return Err(unavailable());
    }
    let zones = crate::adapter::winmath::zones(monitors);
    Ok(ObservedMonitors {
        observations: monitors
            .iter()
            .zip(zones)
            .map(|(m, z)| Observation {
                evidence: Evidence::WindowsPath(m.path.clone()),
                label: z.name.clone().unwrap_or_else(|| "Monitor".into()),
                rect: NativeRect {
                    x: m.x,
                    y: m.y,
                    width: m.width,
                    height: m.height,
                },
                scale: z.scale,
                legacy_aliases: z.name.into_iter().collect(),
            })
            .collect(),
        space: CoordinateSpace::Physical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: true,
            positioned_return: true,
            route_receiver: true,
        },
    })
}

/// Correlates only an unambiguous singleton with a compositor-observed region.
/// Connector path and raw EDID never leave the private identity input.
pub fn linux_observation(root: &Path, zones: &[Zone]) -> Result<ObservedMonitors, InputError> {
    if zones.len() != 1 {
        return Err(unavailable());
    }
    let mut connectors = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|_| unavailable())? {
        let entry = entry.map_err(|_| unavailable())?;
        let path = entry.path();
        let status = match std::fs::read_to_string(path.join("status")) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(unavailable()),
        };
        if status.trim() != "connected" {
            continue;
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(unavailable)?
            .to_owned();
        if !name.contains('-') {
            return Err(unavailable());
        }
        let edid_sha256 = match std::fs::read(path.join("edid")) {
            Ok(bytes) if !bytes.is_empty() => Some(Sha256::digest(bytes).into()),
            Ok(_) => None,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(unavailable()),
        };
        connectors.push(Evidence::LinuxSingle {
            connector: name,
            edid_sha256,
        });
    }
    if connectors.len() != 1 {
        return Err(unavailable());
    }
    let z = &zones[0];
    Ok(ObservedMonitors {
        observations: vec![Observation {
            evidence: connectors.remove(0),
            label: z.name.clone().unwrap_or_else(|| "Monitor".into()),
            rect: NativeRect {
                x: z.x,
                y: z.y,
                width: z.width,
                height: z.height,
            },
            scale: z.scale,
            legacy_aliases: z.name.clone().into_iter().collect(),
        }],
        space: CoordinateSpace::Logical,
        source: GeometrySource::Observed,
        capabilities: MonitorCapabilities {
            internal_edges: false,
            positioned_return: false,
            route_receiver: true,
        },
    })
}

pub(crate) fn same_observation(a: &ObservedMonitors, b: &ObservedMonitors) -> bool {
    a.space == b.space
        && a.source == b.source
        && a.capabilities == b.capabilities
        && a.observations.len() == b.observations.len()
        && a.observations.iter().zip(&b.observations).all(|(a, b)| {
            a.evidence == b.evidence
                && a.rect == b.rect
                && a.scale == b.scale
                && a.label == b.label
                && a.legacy_aliases == b.legacy_aliases
        })
}

/// DRM supplies possible dimensions, never permission to activate a route.
pub fn linux_estimated_observation(root: &Path) -> Result<ObservedMonitors, InputError> {
    let mut observation = linux_observation(root, &crate::adapter::outputs::sysfs_zones_at(root))?;
    observation.source = GeometrySource::Estimated;
    observation.capabilities = MonitorCapabilities {
        internal_edges: false,
        positioned_return: false,
        route_receiver: false,
    };
    Ok(observation)
}
