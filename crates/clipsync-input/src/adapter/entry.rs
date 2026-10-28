//! Where the cursor lands on entry, with relative motion only (contract §8 rule 8). The default of
//! `EmulateAdapter::enter`; the Windows emulator places the cursor with an absolute move instead.
use super::Zone;
use clipsync_proto::{Edge, InputEvent};

/// Reference length of the entry edge (no destination size is known): 1080p.
const ENTRY_REF_LEN_PX: f32 = 1080.0;
/// Saturating burst toward an edge/corner.
const SATURATE_PX: f32 = 4000.0;

/// Relative-only positioning (the EIS has no absolute motion): saturate into the entry corner,
/// then walk along the edge to `pos`.
pub(crate) fn entry_events(edge: Edge, pos: f32) -> Vec<InputEvent> {
    let toward_edge = match edge {
        Edge::Left => (-SATURATE_PX, 0.0),
        Edge::Right => (SATURATE_PX, 0.0),
        Edge::Top => (0.0, -SATURATE_PX),
        Edge::Bottom => (0.0, SATURATE_PX),
    };
    let toward_start = match edge {
        Edge::Left | Edge::Right => (0.0, -SATURATE_PX),
        Edge::Top | Edge::Bottom => (-SATURATE_PX, 0.0),
    };
    let walk = pos.clamp(0.0, 1.0) * ENTRY_REF_LEN_PX;
    let along = match edge {
        Edge::Left | Edge::Right => (0.0, walk),
        Edge::Top | Edge::Bottom => (walk, 0.0),
    };
    let mk = |(dx, dy): (f32, f32)| InputEvent::Motion { dx, dy };
    vec![mk(toward_edge), mk(toward_start), mk(along)]
}

/// `EnterAt` with a known monitor: saturate into the desktop corner on the entry side, walk to
/// the start of that monitor's edge, then along it to `pos` (relative motion only, §8 rule 8).
pub(crate) fn entry_events_at(
    edge: Edge,
    pos: f32,
    zone: &Zone,
    zones: &[Zone],
) -> Vec<InputEvent> {
    let min_x = zones.iter().map(|z| z.x).min().unwrap_or(zone.x) as f32;
    let min_y = zones.iter().map(|z| z.y).min().unwrap_or(zone.y) as f32;
    let max_x = zones
        .iter()
        .map(|z| z.x + z.width as i32)
        .max()
        .unwrap_or(zone.x + zone.width as i32) as f32;
    let max_y = zones
        .iter()
        .map(|z| z.y + z.height as i32)
        .max()
        .unwrap_or(zone.y + zone.height as i32) as f32;
    let (zx, zy) = (zone.x as f32, zone.y as f32);
    let (zw, zh) = (zone.width as f32, zone.height as f32);
    // after the two saturating bursts the cursor sits in this desktop corner
    let (corner_x, corner_y) = match edge {
        Edge::Left => (min_x, min_y),
        Edge::Right => (max_x, min_y),
        Edge::Top => (min_x, min_y),
        Edge::Bottom => (min_x, max_y),
    };
    // start of the target edge
    let (start_x, start_y) = match edge {
        Edge::Left => (zx, zy),
        Edge::Right => (zx + zw, zy),
        Edge::Top => (zx, zy),
        Edge::Bottom => (zx, zy + zh),
    };
    let walk = pos.clamp(0.0, 1.0);
    let along = match edge {
        Edge::Left | Edge::Right => (0.0, walk * zh),
        Edge::Top | Edge::Bottom => (walk * zw, 0.0),
    };
    let mut events = entry_events(edge, 0.0);
    events.truncate(2);
    events.push(InputEvent::Motion {
        dx: start_x - corner_x,
        dy: start_y - corner_y,
    });
    events.push(InputEvent::Motion {
        dx: along.0,
        dy: along.1,
    });
    events
}
