//! Barrier geometry, proven in spike input 2: the right edge sits at `x = zone.x + zone.width`
//! (`x = zone.x + zone.width - 1` is refused by the portal).
use super::Zone;
use clipsync_proto::Edge;

/// Inclusive segment `(x1, y1)..=(x2, y2)` in the zones' coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

fn span(offset: i32, len: u32, from: f32, to: f32) -> (i32, i32) {
    let l = len as f32;
    let a = offset + (from.clamp(0.0, 1.0) * l).round() as i32;
    let b = offset + (to.clamp(0.0, 1.0) * l).round() as i32 - 1;
    (a, b.max(a))
}

/// Partial segment `from..to` (0..=1) along `edge` of `zone`:
/// `offset + round(from*len) ..= offset + round(to*len) - 1`.
pub fn barrier_segment(zone: &Zone, edge: Edge, from: f32, to: f32) -> Segment {
    match edge {
        Edge::Right | Edge::Left => {
            let x = if edge == Edge::Right {
                zone.x + zone.width as i32
            } else {
                zone.x
            };
            let (y1, y2) = span(zone.y, zone.height, from, to);
            Segment {
                x1: x,
                y1,
                x2: x,
                y2,
            }
        }
        Edge::Top | Edge::Bottom => {
            let y = if edge == Edge::Bottom {
                zone.y + zone.height as i32
            } else {
                zone.y
            };
            let (x1, x2) = span(zone.x, zone.width, from, to);
            Segment {
                x1,
                y1: y,
                x2,
                y2: y,
            }
        }
    }
}

/// Normalized position (0..=1) of the cursor along `edge`, from absolute zone coordinates.
pub fn edge_position(zone: &Zone, edge: Edge, cx: f32, cy: f32) -> f32 {
    let (rel, len) = match edge {
        Edge::Left | Edge::Right => (cy - zone.y as f32, zone.height as f32),
        Edge::Top | Edge::Bottom => (cx - zone.x as f32, zone.width as f32),
    };
    if len <= 0.0 {
        return 0.0;
    }
    (rel / len).clamp(0.0, 1.0)
}
