//! Pure compiler: independent virtual positions, native dead corners, explicit legacy adoption.
use crate::{
    layout_document::{
        LayoutDocument, LegacyBinding, links_overlap, valid_position, valid_segment,
    },
    monitor_identities::valid_geometry,
};
use clipsync_proto::{
    CoordinateSpace, Edge, GeometrySource, MonitorEndpoint, MonitorInfo, MonitorInventory,
    MonitorKey, MonitorLink, MonitorPosition, PeerId,
};
use std::collections::{BTreeSet, HashSet};
#[derive(Debug, Clone)]
pub struct InventoryView {
    pub peer: PeerId,
    pub inventory: MonitorInventory,
    pub aliases: Vec<LegacyBinding>,
}
#[derive(Debug, Clone)]
pub struct OrganizationDraft {
    pub positions: Vec<MonitorPosition>,
    pub replace_link_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledOrganization {
    pub document: LayoutDocument,
    pub effective_links: Vec<MonitorLink>,
    pub preserved_link_ids: Vec<String>,
    pub replaced_link_ids: Vec<String>,
    /// Legacy links kept on disk, inactive until both endpoint identities resolve.
    pub unresolved_link_ids: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OrganizationError {
    #[error("geometria inválida")]
    InvalidGeometry,
    #[error("monitor ausente")]
    MonitorMissing,
    #[error("identidade ambígua")]
    IdentityAmbiguous,
    #[error("trechos sobrepostos")]
    SegmentOverlap,
    #[error("trecho vazio")]
    EmptySegment,
    #[error("topologia não suportada")]
    UnsupportedTopology,
    #[error("conflito com ligação avançada")]
    AdvancedLinkConflict,
}
impl OrganizationError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidGeometry => "invalid_geometry",
            Self::MonitorMissing => "monitor_missing",
            Self::IdentityAmbiguous => "identity_ambiguous",
            Self::SegmentOverlap => "segment_overlap",
            Self::EmptySegment => "empty_segment",
            Self::UnsupportedTopology => "unsupported_topology",
            Self::AdvancedLinkConflict => "advanced_link_conflict",
        }
    }
}
struct Card<'a> {
    position: &'a MonitorPosition,
    monitor: &'a MonitorInfo,
    width: f64,
    height: f64,
}
impl Card<'_> {
    fn right(&self) -> f64 {
        self.position.x + self.width
    }
    fn bottom(&self) -> f64 {
        self.position.y + self.height
    }
}

pub fn compile_organization(
    current: &LayoutDocument,
    draft: &OrganizationDraft,
    inventories: &[InventoryView],
) -> Result<CompiledOrganization, OrganizationError> {
    current
        .validate()
        .map_err(|_| OrganizationError::InvalidGeometry)?;
    if !(2..=3).contains(&draft.positions.len()) {
        return Err(OrganizationError::UnsupportedTopology);
    }
    let peers: BTreeSet<_> = draft.positions.iter().map(|p| p.key.peer).collect();
    if peers.len() != 2 {
        return Err(OrganizationError::UnsupportedTopology);
    }
    let mut cards = vec![];
    let mut keys = HashSet::new();
    let mut selected = vec![];
    for peer in &peers {
        let matches: Vec<_> = inventories.iter().filter(|i| i.peer == *peer).collect();
        if matches.len() > 1 {
            return Err(OrganizationError::IdentityAmbiguous);
        }
        let view = matches
            .first()
            .copied()
            .ok_or(OrganizationError::MonitorMissing)?;
        if view.inventory.source != GeometrySource::Observed || view.inventory.rev == 0 {
            return Err(OrganizationError::UnsupportedTopology);
        }
        let mut uids = HashSet::new();
        for m in &view.inventory.monitors {
            if m.uid.0 == [0; 16] || !uids.insert(m.uid) {
                return Err(OrganizationError::IdentityAmbiguous);
            }
            if !valid_geometry(m.rect, m.scale) {
                return Err(OrganizationError::InvalidGeometry);
            }
        }
        selected.push(view);
    }
    let origin = selected
        .iter()
        .find(|i| {
            i.inventory.space == CoordinateSpace::Physical
                && i.inventory.capabilities.internal_edges
                && i.inventory.capabilities.positioned_return
        })
        .ok_or(OrganizationError::UnsupportedTopology)?;
    let receiver = selected
        .iter()
        .find(|i| {
            i.peer != origin.peer
                && i.inventory.capabilities.route_receiver
                && i.inventory.monitors.len() == 1
        })
        .ok_or(OrganizationError::UnsupportedTopology)?;
    if draft
        .positions
        .iter()
        .filter(|p| p.key.peer == receiver.peer)
        .count()
        != 1
        || !(1..=2).contains(
            &draft
                .positions
                .iter()
                .filter(|p| p.key.peer == origin.peer)
                .count(),
        )
    {
        return Err(OrganizationError::UnsupportedTopology);
    }
    for p in &draft.positions {
        if !keys.insert(p.key) {
            return Err(OrganizationError::IdentityAmbiguous);
        }
        if !valid_position(p.x) || !valid_position(p.y) {
            return Err(OrganizationError::InvalidGeometry);
        }
        let inventory = &selected
            .iter()
            .find(|i| i.peer == p.key.peer)
            .ok_or(OrganizationError::MonitorMissing)?
            .inventory;
        let m = inventory
            .monitors
            .iter()
            .find(|m| m.uid == p.key.uid)
            .ok_or(OrganizationError::MonitorMissing)?;
        let divisor = if inventory.space == CoordinateSpace::Physical {
            m.scale
        } else {
            1.
        };
        let (width, height) = (
            f64::from(m.rect.width) / divisor,
            f64::from(m.rect.height) / divisor,
        );
        if !width.is_finite()
            || !height.is_finite()
            || width <= 0.
            || height <= 0.
            || width > 2_000_000.
            || height > 2_000_000.
            || p.x + width <= p.x
            || p.y + height <= p.y
        {
            return Err(OrganizationError::InvalidGeometry);
        }
        cards.push(Card {
            position: p,
            monitor: m,
            width,
            height,
        });
    }
    // Canonical order stabilizes IDs and output regardless of input ordering.
    cards.sort_by_key(|c| c.position.key);
    let mut generated = vec![];
    for (i, a) in cards.iter().enumerate() {
        for b in &cards[i + 1..] {
            if a.position.x < b.right()
                && b.position.x < a.right()
                && a.position.y < b.bottom()
                && b.position.y < a.bottom()
            {
                return Err(OrganizationError::InvalidGeometry);
            }
            if a.position.key.peer == b.position.key.peer {
                continue;
            }
            let contact = if a.right() == b.position.x {
                Some((a, b, Edge::Right, Edge::Left, true))
            } else if b.right() == a.position.x {
                Some((b, a, Edge::Right, Edge::Left, true))
            } else if a.bottom() == b.position.y {
                Some((a, b, Edge::Bottom, Edge::Top, false))
            } else if b.bottom() == a.position.y {
                Some((b, a, Edge::Bottom, Edge::Top, false))
            } else {
                None
            };
            if let Some((a, b, ae, be, vertical)) = contact
                && let Some(link) = contact_link(
                    a,
                    b,
                    ae,
                    be,
                    vertical,
                    current.legacy.options.dead_corner_px,
                )?
            {
                generated.push(link);
            }
        }
    }
    if generated.is_empty() || generated.len() > 2 {
        return Err(OrganizationError::UnsupportedTopology);
    }
    for (i, l) in generated.iter().enumerate() {
        if generated[..i].iter().any(|o| links_overlap(l, o)) {
            return Err(OrganizationError::SegmentOverlap);
        }
    }
    let mut document = current.clone();
    let org = document.organization.get_or_insert_with(Default::default);
    let in_pair = |l: &MonitorLink| peers.contains(&l.a.key.peer) && peers.contains(&l.b.key.peer);
    let mut replacements = HashSet::new();
    for id in &draft.replace_link_ids {
        if !replacements.insert(id) || !org.links.iter().any(|l| l.id == *id && in_pair(l)) {
            return Err(OrganizationError::AdvancedLinkConflict);
        }
    }
    if org
        .links
        .iter()
        .filter(|l| in_pair(l))
        .any(|l| !replacements.contains(&l.id))
    {
        return Err(OrganizationError::AdvancedLinkConflict);
    }
    let mut replaced_link_ids: Vec<_> = org
        .links
        .iter()
        .filter(|l| in_pair(l))
        .map(|l| l.id.clone())
        .collect();
    org.links.retain(|l| !in_pair(l));
    // Persist only confirmed bindings. Duplicate aliases with different UIDs fail closed.
    for view in &selected {
        for b in &view.aliases {
            if b.peer != view.peer
                || b.alias.is_empty()
                || !view.inventory.monitors.iter().any(|m| m.uid == b.uid)
            {
                return Err(OrganizationError::IdentityAmbiguous);
            }
            if let Some(old) = org
                .legacy_bindings
                .iter()
                .find(|old| old.peer == b.peer && old.alias == b.alias)
            {
                if old.uid != b.uid {
                    return Err(OrganizationError::IdentityAmbiguous);
                }
            } else {
                org.legacy_bindings.push(b.clone());
            }
        }
    }
    let mut unresolved_link_ids = Vec::new();
    let mut resolved_legacy = Vec::new();
    // Resolve and validate the complete preserved set before adopting anything.
    // Different aliases can name the same UID, so the v1 string-based validation
    // alone cannot establish disjoint segments or a one-to-one migration.
    for old in &document.legacy.links {
        let a = resolve_endpoint(&old.a, &org.legacy_bindings, inventories)?;
        let b = resolve_endpoint(&old.b, &org.legacy_bindings, inventories)?;
        let (Some(a), Some(b)) = (a, b) else {
            unresolved_link_ids.push(old.id.clone());
            continue;
        };
        // An unresolved link is inactive as a whole. Only now may trimming
        // reject the geometry of a fully identified legacy passage.
        let resolved = MonitorLink {
            id: old.id.clone(),
            a: effective_endpoint(a, current.legacy.options.dead_corner_px)?,
            b: effective_endpoint(b, current.legacy.options.dead_corner_px)?,
        };
        if resolved_legacy
            .iter()
            .chain(org.links.iter())
            .any(|other| links_overlap(&resolved, other))
        {
            return Err(OrganizationError::SegmentOverlap);
        }
        resolved_legacy.push(resolved);
    }
    let mut adopted = HashSet::new();
    let mut adopted_routes = HashSet::new();
    for resolved in &resolved_legacy {
        if current.organization.is_none()
            && let Some(index) = generated
                .iter()
                .position(|new| same_geometry(new, resolved))
        {
            if !adopted_routes.insert(index) {
                return Err(OrganizationError::IdentityAmbiguous);
            }
            generated[index].id = resolved.id.clone();
            adopted.insert(resolved.id.clone());
            replaced_link_ids.push(resolved.id.clone());
            continue;
        }
        if generated.iter().any(|new| links_overlap(new, resolved)) {
            return Err(OrganizationError::AdvancedLinkConflict);
        }
    }
    document.legacy.links.retain(|l| !adopted.contains(&l.id));
    for l in &generated {
        if org.links.iter().any(|o| links_overlap(l, o))
            || org.links.iter().any(|o| l.id == o.id)
            || document.legacy.links.iter().any(|o| l.id == o.id)
        {
            return Err(OrganizationError::AdvancedLinkConflict);
        }
    }
    let mut preserved_link_ids: Vec<_> = document
        .legacy
        .links
        .iter()
        .map(|l| l.id.clone())
        .chain(org.links.iter().map(|l| l.id.clone()))
        .collect();
    for p in &draft.positions {
        if let Some(existing) = org.positions.iter_mut().find(|old| old.key == p.key) {
            *existing = p.clone();
        } else {
            org.positions.push(p.clone());
        }
    }
    org.links.extend(generated.clone());
    org.positions.sort_by_key(|p| p.key);
    org.links.sort_by(|a, b| a.id.cmp(&b.id));
    preserved_link_ids.sort();
    replaced_link_ids.sort();
    unresolved_link_ids.sort();
    document
        .validate()
        .map_err(|_| OrganizationError::InvalidGeometry)?;
    Ok(CompiledOrganization {
        document,
        effective_links: generated,
        preserved_link_ids,
        replaced_link_ids,
        unresolved_link_ids,
    })
}
fn contact_link(
    a: &Card<'_>,
    b: &Card<'_>,
    ae: Edge,
    be: Edge,
    vertical: bool,
    dead: u32,
) -> Result<Option<MonitorLink>, OrganizationError> {
    let (start_a, start_b, len_a, len_b, native_a, native_b) = if vertical {
        (
            a.position.y,
            b.position.y,
            a.height,
            b.height,
            a.monitor.rect.height,
            b.monitor.rect.height,
        )
    } else {
        (
            a.position.x,
            b.position.x,
            a.width,
            b.width,
            a.monitor.rect.width,
            b.monitor.rect.width,
        )
    };
    let start = start_a.max(start_b);
    let end = (start_a + len_a).min(start_b + len_b);
    if start >= end {
        return Ok(None);
    }
    // Intersect both usable native edges in virtual coordinates before computing either fraction.
    let start = start
        .max(start_a + f64::from(dead) / f64::from(native_a) * len_a)
        .max(start_b + f64::from(dead) / f64::from(native_b) * len_b);
    let end = end
        .min(start_a + (1. - f64::from(dead) / f64::from(native_a)) * len_a)
        .min(start_b + (1. - f64::from(dead) / f64::from(native_b)) * len_b);
    if start >= end {
        return Err(OrganizationError::EmptySegment);
    }
    let af = ((start - start_a) / len_a) as f32;
    let at = ((end - start_a) / len_a) as f32;
    let bf = ((start - start_b) / len_b) as f32;
    let bt = ((end - start_b) / len_b) as f32;
    if !valid_segment(af, at) || !valid_segment(bf, bt) {
        return Err(OrganizationError::EmptySegment);
    }
    let id = managed_link_id(a.position.key, ae, b.position.key, be);
    Ok(Some(MonitorLink {
        id,
        a: MonitorEndpoint {
            key: a.position.key,
            edge: ae,
            from: af,
            to: at,
        },
        b: MonitorEndpoint {
            key: b.position.key,
            edge: be,
            from: bf,
            to: bt,
        },
    }))
}
fn same_geometry(a: &MonitorLink, b: &MonitorLink) -> bool {
    (a.a == b.a && a.b == b.b) || (a.a == b.b && a.b == b.a)
}
struct ResolvedEndpoint<'a> {
    endpoint: MonitorEndpoint,
    monitor: &'a MonitorInfo,
}
fn resolve_endpoint<'a>(
    e: &crate::Endpoint,
    bindings: &[LegacyBinding],
    inventories: &'a [InventoryView],
) -> Result<Option<ResolvedEndpoint<'a>>, OrganizationError> {
    let matches: Vec<_> = bindings
        .iter()
        .filter(|b| b.peer == e.peer && b.alias == e.monitor)
        .collect();
    if matches.len() > 1 {
        return Err(OrganizationError::IdentityAmbiguous);
    }
    let uid = if let Some(b) = matches.first() {
        Some(b.uid)
    } else {
        let views: Vec<_> = inventories.iter().filter(|i| i.peer == e.peer).collect();
        if views.len() > 1 {
            return Err(OrganizationError::IdentityAmbiguous);
        }
        views.first().and_then(|v| {
            let i = &v.inventory;
            if i.source == GeometrySource::Observed
                && i.rev > 0
                && i.monitors.len() == 1
                && !e.monitor.starts_with('#')
                && i.monitors[0].label == e.monitor
            {
                Some(i.monitors[0].uid)
            } else {
                None
            }
        })
    };
    let Some(uid) = uid else { return Ok(None) };
    let views: Vec<_> = inventories.iter().filter(|i| i.peer == e.peer).collect();
    if views.len() > 1 {
        return Err(OrganizationError::IdentityAmbiguous);
    }
    let Some(view) = views.first() else {
        return Ok(None);
    };
    if view.inventory.source != GeometrySource::Observed || view.inventory.rev == 0 {
        return Ok(None);
    }
    let monitors: Vec<_> = view
        .inventory
        .monitors
        .iter()
        .filter(|m| m.uid == uid)
        .collect();
    if monitors.len() > 1 {
        return Err(OrganizationError::IdentityAmbiguous);
    }
    let Some(monitor) = monitors.first() else {
        return Ok(None);
    };
    Ok(Some(ResolvedEndpoint {
        endpoint: MonitorEndpoint {
            key: MonitorKey { peer: e.peer, uid },
            edge: e.edge,
            from: e.from,
            to: e.to,
        },
        monitor,
    }))
}
fn effective_endpoint(
    resolved: ResolvedEndpoint<'_>,
    dead_corner_px: u32,
) -> Result<MonitorEndpoint, OrganizationError> {
    let ResolvedEndpoint {
        mut endpoint,
        monitor,
    } = resolved;
    if !valid_geometry(monitor.rect, monitor.scale) {
        return Err(OrganizationError::InvalidGeometry);
    }
    let len = match endpoint.edge {
        Edge::Left | Edge::Right => monitor.rect.height,
        Edge::Top | Edge::Bottom => monitor.rect.width,
    };
    // Legacy endpoints are raw. Match the actual legacy runtime's f32 trim;
    // generated monitor links are already effective and must not be trimmed again.
    let (from, to) = crate::trim_dead_corner(endpoint.from, endpoint.to, len, dead_corner_px)
        .ok_or(OrganizationError::EmptySegment)?;
    endpoint.from = from;
    endpoint.to = to;
    Ok(endpoint)
}

// Hash only public routing keys. Fixed byte widths and domain separation make this
// reproducible across architectures and input ordering; it is not hardware evidence.
fn managed_link_id(a: MonitorKey, ae: Edge, b: MonitorKey, be: Edge) -> String {
    use sha2::{Digest, Sha256};
    let edge = |e| match e {
        Edge::Left => 0u8,
        Edge::Right => 1,
        Edge::Top => 2,
        Edge::Bottom => 3,
    };
    let mut ends = [(a, edge(ae)), (b, edge(be))];
    ends.sort();
    let mut digest = Sha256::new();
    digest.update(b"clipsync.monitor-link.v1\0");
    for (key, edge) in ends {
        digest.update(key.peer.0);
        digest.update(key.uid.0);
        digest.update([edge]);
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn link_hash_is_direction_independent_and_domain_separated() {
        let a = MonitorKey {
            peer: PeerId([1; 32]),
            uid: clipsync_proto::MonitorUid([1; 16]),
        };
        let b = MonitorKey {
            peer: PeerId([2; 32]),
            uid: clipsync_proto::MonitorUid([3; 16]),
        };
        assert_eq!(
            managed_link_id(a, Edge::Right, b, Edge::Left),
            managed_link_id(b, Edge::Left, a, Edge::Right)
        );
        assert_eq!(
            managed_link_id(a, Edge::Right, b, Edge::Left),
            "13b975e3cefc3a227417d779a38344974e3857d71b76532addb9b8fd9481ed03"
        );
    }
}
