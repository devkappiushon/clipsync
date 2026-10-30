//! One receiver position, shared by all offered exits. Coordinates are explicit, never host-cfg guessed.
use crate::InputError;
use clipsync_proto::{
    CoordinateSpace, Edge, EnterRoutes, ExitRoute, GeometrySource, InputEvent, MonitorInventory,
    NativeRect, RouteId, Validate,
};
#[derive(Debug, Clone)]
pub struct RouteTrack {
    rect: NativeRect,
    factor: f64,
    x: f64,
    y: f64,
    routes: Vec<ExitRoute>,
    push: Option<(RouteId, f64)>,
}
impl RouteTrack {
    pub fn new(inventory: &MonitorInventory, entry: &EnterRoutes) -> Result<Self, InputError> {
        if inventory.validate().is_err()
            || entry.validate().is_err()
            || inventory.source != GeometrySource::Observed
            || inventory.rev != entry.inventories.target
            || !inventory.capabilities.route_receiver
        {
            return Err(crate::inventory::unavailable());
        }
        let monitor = inventory
            .monitors
            .iter()
            .find(|m| m.uid == entry.monitor)
            .ok_or_else(crate::inventory::unavailable)?;
        let routes: Vec<_> = std::iter::once(entry.routes.first.clone())
            .chain(entry.routes.second.clone())
            .collect();
        if routes.iter().any(|r| r.monitor != monitor.uid) {
            return Err(crate::inventory::unavailable());
        }
        let r = monitor.rect;
        let (left, top, right, bottom) = (
            f64::from(r.x),
            f64::from(r.y),
            f64::from(r.x) + f64::from(r.width) - 1.0,
            f64::from(r.y) + f64::from(r.height) - 1.0,
        );
        let p = f64::from(entry.pos);
        let (x, y) = match entry.edge {
            Edge::Left => (left + 1.0, top + p * (bottom - top)),
            Edge::Right => (right - 1.0, top + p * (bottom - top)),
            Edge::Top => (left + p * (right - left), top + 1.0),
            Edge::Bottom => (left + p * (right - left), bottom - 1.0),
        };
        Ok(Self {
            rect: r,
            factor: if inventory.space == CoordinateSpace::Physical {
                monitor.scale
            } else {
                1.0
            },
            x: x.clamp(left, right),
            y: y.clamp(top, bottom),
            routes,
            push: None,
        })
    }
    pub fn sync(&mut self, x: i32, y: i32) {
        self.x = f64::from(x);
        self.y = f64::from(y);
        self.clamp();
    }
    fn clamp(&mut self) {
        self.x = self.x.clamp(
            f64::from(self.rect.x),
            f64::from(self.rect.x) + f64::from(self.rect.width) - 1.0,
        );
        self.y = self.y.clamp(
            f64::from(self.rect.y),
            f64::from(self.rect.y) + f64::from(self.rect.height) - 1.0,
        );
    }
    pub fn step(&mut self, events: &mut Vec<InputEvent>) -> Option<(RouteId, f32)> {
        for i in 0..events.len() {
            let InputEvent::Motion { dx, dy } = events[i] else {
                continue;
            };
            if !dx.is_finite() || !dy.is_finite() {
                continue;
            }
            let (nx, ny) = (
                self.x + f64::from(dx) * self.factor,
                self.y + f64::from(dy) * self.factor,
            );
            let mut selected = None;
            for r in &self.routes {
                let vertical = matches!(r.edge, Edge::Left | Edge::Right);
                let line = match r.edge {
                    Edge::Left => f64::from(self.rect.x),
                    Edge::Right => f64::from(self.rect.x) + f64::from(self.rect.width) - 1.0,
                    Edge::Top => f64::from(self.rect.y),
                    Edge::Bottom => f64::from(self.rect.y) + f64::from(self.rect.height) - 1.0,
                };
                let sign = if matches!(r.edge, Edge::Left | Edge::Top) {
                    -1.0
                } else {
                    1.0
                };
                let next = if vertical { nx } else { ny };
                let tangent = if vertical { ny } else { nx };
                let (origin, size) = if vertical {
                    (self.rect.y, self.rect.height)
                } else {
                    (self.rect.x, self.rect.width)
                };
                let pos = (tangent - f64::from(origin)) / (f64::from(size) - 1.0).max(1.0);
                if (next - line) * sign > 0.0 && pos >= f64::from(r.from) && pos <= f64::from(r.to)
                {
                    selected = Some((r.id, pos as f32, vertical, line, (next - line) * sign));
                    break;
                }
            }
            if let Some((id, pos, vertical, line, overshoot)) = selected {
                let total = overshoot
                    + self
                        .push
                        .filter(|(old, _)| *old == id)
                        .map_or(0.0, |(_, v)| v);
                self.push = Some((id, total));
                events[i] = if vertical {
                    InputEvent::Motion {
                        dx: ((line - self.x) / self.factor) as f32,
                        dy,
                    }
                } else {
                    InputEvent::Motion {
                        dx,
                        dy: ((line - self.y) / self.factor) as f32,
                    }
                };
                self.x = if vertical { line } else { nx };
                self.y = if vertical { ny } else { line };
                self.clamp();
                if total > f64::from(crate::capture::RETURN_SLACK_PX) {
                    events.truncate(i + 1);
                    return Some((id, pos));
                }
            } else {
                self.push = None;
                self.x = nx;
                self.y = ny;
                self.clamp();
            }
        }
        None
    }
}
