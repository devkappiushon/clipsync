//! Versioned layout, keeping the legacy data model intact.
use crate::{Layout, LayoutError, config::EdgeRaw};
use clipsync_proto::{
    MonitorEndpoint, MonitorKey, MonitorLink, MonitorPosition, MonitorUid, PeerId,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, path::Path};
#[derive(Clone, PartialEq, Eq)]
pub struct LegacyBinding {
    pub peer: PeerId,
    pub alias: String,
    pub uid: MonitorUid,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Organization {
    pub positions: Vec<MonitorPosition>,
    pub links: Vec<MonitorLink>,
    pub legacy_bindings: Vec<LegacyBinding>,
}
#[derive(Clone, Default, PartialEq)]
pub struct LayoutDocument {
    pub legacy: Layout,
    pub organization: Option<Organization>,
}
impl fmt::Debug for LegacyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LegacyBinding")
            .field("peer", &self.peer)
            .field("uid", &self.uid)
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for LayoutDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Legacy endpoint.monitor also carries an alias, so do not delegate to
        // Layout's Debug and accidentally reintroduce a private path here.
        f.debug_struct("LayoutDocument")
            .field("rev", &self.legacy.rev)
            .field("options", &self.legacy.options)
            .field("devices", &self.legacy.devices)
            .field("legacy_link_count", &self.legacy.links.len())
            .field("organization", &self.organization)
            .finish_non_exhaustive()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    peer: String,
    monitor_uid: MonitorUid,
    x: f64,
    y: f64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    peer: String,
    monitor_uid: MonitorUid,
    edge: EdgeRaw,
    from: f32,
    to: f32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Link {
    id: String,
    a: Endpoint,
    b: Endpoint,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    peer: String,
    alias: String,
    monitor_uid: MonitorUid,
}
// Keep the unsigned revision outside toml::Value, whose integer is signed.
#[derive(Deserialize)]
struct Parts {
    version: u32,
    #[serde(default)]
    rev: u64,
    #[serde(flatten)]
    rest: toml::Table,
}
fn parse_error(e: impl std::fmt::Display) -> LayoutError {
    LayoutError::Parse(e.to_string())
}
fn invalid() -> LayoutError {
    LayoutError::InvalidField("organização de monitores inválida".into())
}
fn peer(s: &str) -> Result<PeerId, LayoutError> {
    PeerId::from_hex(s).map_err(|_| invalid())
}
impl Endpoint {
    fn parse(self) -> Result<MonitorEndpoint, LayoutError> {
        Ok(MonitorEndpoint {
            key: MonitorKey {
                peer: peer(&self.peer)?,
                uid: self.monitor_uid,
            },
            edge: self.edge.into(),
            from: self.from,
            to: self.to,
        })
    }
    fn from(e: &MonitorEndpoint) -> Self {
        Self {
            peer: e.key.peer.to_hex(),
            monitor_uid: e.key.uid,
            edge: e.edge.into(),
            from: e.from,
            to: e.to,
        }
    }
}
impl LayoutDocument {
    pub fn read(path: &Path) -> Result<Option<Self>, LayoutError> {
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_toml_str(&s).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(LayoutError::Io("falha ao ler layout".into())),
        }
    }
    pub fn from_toml_str(text: &str) -> Result<Self, LayoutError> {
        let Parts {
            version,
            rev,
            rest: mut table,
        } = toml::from_str(text).map_err(parse_error)?;
        if version != 1 && version != 2 {
            return Err(LayoutError::UnsupportedVersion(version));
        }
        let organization = if version == 2 {
            let mut org = Organization::default();
            if let Some(v) = table.remove("monitor_position") {
                let values: Vec<Position> = v.try_into().map_err(parse_error)?;
                for p in values {
                    org.positions.push(MonitorPosition {
                        key: MonitorKey {
                            peer: peer(&p.peer)?,
                            uid: p.monitor_uid,
                        },
                        x: p.x,
                        y: p.y,
                    });
                }
            }
            if let Some(v) = table.remove("monitor_link") {
                let values: Vec<Link> = v.try_into().map_err(parse_error)?;
                for l in values {
                    org.links.push(MonitorLink {
                        id: l.id,
                        a: l.a.parse()?,
                        b: l.b.parse()?,
                    });
                }
            }
            if let Some(v) = table.remove("legacy_binding") {
                let values: Vec<Binding> = v.try_into().map_err(parse_error)?;
                for b in values {
                    org.legacy_bindings.push(LegacyBinding {
                        peer: peer(&b.peer)?,
                        alias: b.alias,
                        uid: b.monitor_uid,
                    });
                }
            }
            Some(org)
        } else {
            None
        };
        let doc = Self {
            legacy: Layout::from_toml_str(&format!(
                "version = 1\nrev = {rev}\n{}",
                toml::to_string(&table).map_err(parse_error)?
            ))?,
            organization,
        };
        doc.validate()?;
        Ok(doc)
    }
    pub fn to_toml_string(&self) -> Result<String, LayoutError> {
        self.validate()?;
        let text = self.legacy.to_toml_string()?;
        let Some(org) = &self.organization else {
            return Ok(text);
        };
        let Parts {
            rest: mut table, ..
        } = toml::from_str(&text).map_err(parse_error)?;
        let positions: Vec<_> = org
            .positions
            .iter()
            .map(|p| Position {
                peer: p.key.peer.to_hex(),
                monitor_uid: p.key.uid,
                x: p.x,
                y: p.y,
            })
            .collect();
        let links: Vec<_> = org
            .links
            .iter()
            .map(|l| Link {
                id: l.id.clone(),
                a: Endpoint::from(&l.a),
                b: Endpoint::from(&l.b),
            })
            .collect();
        let bindings: Vec<_> = org
            .legacy_bindings
            .iter()
            .map(|b| Binding {
                peer: b.peer.to_hex(),
                alias: b.alias.clone(),
                monitor_uid: b.uid,
            })
            .collect();
        table.insert(
            "monitor_position".into(),
            toml::Value::try_from(positions).map_err(parse_error)?,
        );
        table.insert(
            "monitor_link".into(),
            toml::Value::try_from(links).map_err(parse_error)?,
        );
        table.insert(
            "legacy_binding".into(),
            toml::Value::try_from(bindings).map_err(parse_error)?,
        );
        Ok(format!(
            "version = 2\nrev = {}\n{}",
            self.legacy.rev,
            toml::to_string(&table).map_err(parse_error)?
        ))
    }
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.legacy.validate()?;
        let Some(org) = &self.organization else {
            return Ok(());
        };
        let known = |p| self.legacy.devices.iter().any(|d| d.peer == p);
        let mut positions = HashSet::new();
        for p in &org.positions {
            if !known(p.key.peer)
                || p.key.uid.0 == [0; 16]
                || !positions.insert(p.key)
                || !valid_position(p.x)
                || !valid_position(p.y)
            {
                return Err(invalid());
            }
        }
        let mut ids: HashSet<_> = self.legacy.links.iter().map(|l| l.id.as_str()).collect();
        for (i, l) in org.links.iter().enumerate() {
            if l.id.is_empty() || !ids.insert(&l.id) || l.a.key.peer == l.b.key.peer {
                return Err(invalid());
            }
            for e in [&l.a, &l.b] {
                if !known(e.key.peer) || e.key.uid.0 == [0; 16] || !valid_segment(e.from, e.to) {
                    return Err(invalid());
                }
            }
            if org.links[..i].iter().any(|other| links_overlap(l, other)) {
                return Err(LayoutError::SegmentOverlap {
                    link: l.id.clone(),
                    other: "monitor".into(),
                });
            }
        }
        let mut aliases = HashSet::new();
        for b in &org.legacy_bindings {
            if !known(b.peer)
                || b.alias.is_empty()
                || b.uid.0 == [0; 16]
                || !aliases.insert((b.peer, &b.alias))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
pub(crate) fn valid_position(v: f64) -> bool {
    v.is_finite() && v.abs() <= 1_000_000.
}
pub(crate) fn valid_segment(from: f32, to: f32) -> bool {
    from.is_finite() && to.is_finite() && from >= 0. && to <= 1. && from < to
}
pub(crate) fn links_overlap(a: &MonitorLink, b: &MonitorLink) -> bool {
    [&a.a, &a.b].iter().any(|a| {
        [&b.a, &b.b]
            .iter()
            .any(|b| a.key == b.key && a.edge == b.edge && a.from < b.to && b.from < a.to)
    })
}
