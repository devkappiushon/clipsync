//! Immutable source-side return authorization. A return never names a new destination.
use clipsync_proto::{InventoryRevisions, MonitorKey, PeerId, ReturnedRoute, RouteId};
#[derive(Debug, Clone)]
pub struct LocalReturn {
    pub id: RouteId,
    pub key: MonitorKey,
    pub barrier: usize,
    pub local: (f32, f32),
    pub remote: (f32, f32),
}
#[derive(Debug, Clone)]
pub struct RouteSession {
    pub peer: PeerId,
    pub connection: u64,
    pub session: u32,
    pub inventories: InventoryRevisions,
    pub returns: Vec<LocalReturn>,
}
impl RouteSession {
    pub fn resolve(
        &self,
        from: PeerId,
        connection: u64,
        returned: &ReturnedRoute,
        current: InventoryRevisions,
    ) -> Option<(usize, f32)> {
        if self.peer != from
            || self.connection != connection
            || connection == 0
            || self.session != returned.session
            || self.inventories != returned.inventories
            || self.inventories != current
            || self.returns.is_empty()
            || self.returns.len() > 2
        {
            return None;
        }
        let mut found = self.returns.iter().filter(|r| r.id == returned.route);
        let r = found.next()?;
        if found.next().is_some()
            || !returned.pos.is_finite()
            || returned.pos < r.remote.0
            || returned.pos > r.remote.1
            || r.remote.0 >= r.remote.1
        {
            return None;
        }
        let pos = crate::map_pos(r.remote, r.local, returned.pos);
        pos.is_finite().then_some((r.barrier, pos))
    }
}
#[derive(Clone)]
pub(crate) struct RoutePlan {
    pub entry: clipsync_proto::EnterRoutes,
    pub session: RouteSession,
}
