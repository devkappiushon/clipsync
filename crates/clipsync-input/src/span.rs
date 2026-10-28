//! The way back on the destination (contract §13.6 item 2). With `EnterSpan` the destination
//! follows the cursor in its own zone space and says `Returned` when it is pushed out through the
//! span `back_from..back_to` of the entry edge. Pure: the module feeds it the events.
use crate::adapter::Zone;
use crate::capture::RETURN_SLACK_PX;
use clipsync_proto::{Edge, InputEvent};

/// Where the cursor is, in the local zone space, from the entry point and the motion since.
#[derive(Debug, Clone)]
pub(crate) struct SpanTrack {
    edge: Edge,
    /// The monitor the cursor entered.
    zone: Zone,
    back: (f32, f32),
    x: f32,
    y: f32,
    /// Outward push accumulated while pinned on the span line.
    overshoot: f32,
}

/// Local px per logical px. Zones are logical on the GNOME and physical on Windows (§13.2 rule 3).
fn factor(z: &Zone) -> f32 {
    if cfg!(windows) { z.scale as f32 } else { 1.0 }
}

fn contains(z: &Zone, x: f32, y: f32) -> bool {
    x >= z.x as f32
        && x <= (z.x + z.width as i32 - 1) as f32
        && y >= z.y as f32
        && y <= (z.y + z.height as i32 - 1) as f32
}

fn clamp_into(z: &Zone, x: f32, y: f32) -> (f32, f32) {
    (
        x.clamp(z.x as f32, (z.x + z.width as i32 - 1) as f32),
        y.clamp(z.y as f32, (z.y + z.height as i32 - 1) as f32),
    )
}

impl SpanTrack {
    /// The cursor starts one pixel inside `zone`, at `pos` along `edge`.
    pub(crate) fn new(zone: Zone, edge: Edge, pos: f32, back: (f32, f32)) -> Self {
        let (zx, zy) = (zone.x as f32, zone.y as f32);
        let (zw, zh) = (zone.width as f32, zone.height as f32);
        let pos = pos.clamp(0.0, 1.0);
        let (x, y) = match edge {
            Edge::Left => (zx + 1.0, zy + pos * (zh - 1.0)),
            Edge::Right => (zx + zw - 2.0, zy + pos * (zh - 1.0)),
            Edge::Top => (zx + pos * (zw - 1.0), zy + 1.0),
            Edge::Bottom => (zx + pos * (zw - 1.0), zy + zh - 2.0),
        };
        Self {
            edge,
            zone,
            back,
            x,
            y,
            overshoot: 0.0,
        }
    }

    /// The platform says where the cursor really is: trust it over the estimate.
    pub(crate) fn sync(&mut self, x: i32, y: i32) {
        self.x = x as f32;
        self.y = y as f32;
    }

    /// The estimate, for tests.
    #[cfg(test)]
    pub(crate) fn at(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Fraction of the entry edge where the cursor is (0 to 1).
    fn along(&self) -> f32 {
        let z = &self.zone;
        let f = match self.edge {
            Edge::Left | Edge::Right => (self.y - z.y as f32) / (z.height as f32 - 1.0).max(1.0),
            Edge::Top | Edge::Bottom => (self.x - z.x as f32) / (z.width as f32 - 1.0).max(1.0),
        };
        f.clamp(0.0, 1.0)
    }

    /// Coordinate of the entry edge line on its normal axis, and the outward sign.
    fn line(&self) -> (f32, f32) {
        let z = &self.zone;
        match self.edge {
            Edge::Left => (z.x as f32, -1.0),
            Edge::Right => ((z.x + z.width as i32 - 1) as f32, 1.0),
            Edge::Top => (z.y as f32, -1.0),
            Edge::Bottom => ((z.y + z.height as i32 - 1) as f32, 1.0),
        }
    }

    fn normal_is_x(&self) -> bool {
        matches!(self.edge, Edge::Left | Edge::Right)
    }

    fn current_zone<'a>(&'a self, zones: &'a [Zone]) -> &'a Zone {
        zones
            .iter()
            .find(|z| contains(z, self.x, self.y))
            .unwrap_or(&self.zone)
    }

    /// Follows the motion in `events`. While the cursor is on the span line, the push outward is
    /// taken out of the event (the line is a wall for the emulator) and counted; past
    /// `RETURN_SLACK_PX` it is a return: the events after it are dropped and the position along
    /// the entry edge comes back. Everywhere else the motion passes and the position is kept
    /// inside the union of `zones`.
    pub(crate) fn step(&mut self, zones: &[Zone], events: &mut Vec<InputEvent>) -> Option<f32> {
        let (line, sign) = self.line();
        for i in 0..events.len() {
            let InputEvent::Motion { dx, dy } = events[i] else {
                continue;
            };
            let f = factor(self.current_zone(zones));
            let (ddx, ddy) = (dx * f, dy * f);
            let (c, d) = if self.normal_is_x() {
                (self.x, ddx)
            } else {
                (self.y, ddy)
            };
            let in_span = {
                let a = self.along();
                a >= self.back.0 && a <= self.back.1
            };
            let next = c + d;
            if in_span && (next - line) * sign > 0.0 {
                self.overshoot += (next - line) * sign;
                // The cursor stays on the line; the emulator is told only the way there.
                let to_line = (line - c) / f;
                events[i] = if self.normal_is_x() {
                    InputEvent::Motion { dx: to_line, dy }
                } else {
                    InputEvent::Motion { dx, dy: to_line }
                };
                if self.normal_is_x() {
                    self.x = line;
                    self.y += ddy;
                } else {
                    self.y = line;
                    self.x += ddx;
                }
                self.keep_inside(zones);
                if self.overshoot > RETURN_SLACK_PX {
                    events.truncate(i + 1);
                    return Some(self.along());
                }
            } else {
                self.overshoot = 0.0;
                self.x += ddx;
                self.y += ddy;
                self.keep_inside(zones);
            }
        }
        None
    }

    /// Keeps the estimate inside the union of the zones: a point in a gap goes to the nearest zone.
    fn keep_inside(&mut self, zones: &[Zone]) {
        let all = if zones.is_empty() {
            std::slice::from_ref(&self.zone)
        } else {
            zones
        };
        if all.iter().any(|z| contains(z, self.x, self.y)) {
            return;
        }
        let best = all
            .iter()
            .map(|z| {
                let (cx, cy) = clamp_into(z, self.x, self.y);
                let dist = (cx - self.x).powi(2) + (cy - self.y).powi(2);
                (dist, cx, cy)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, cx, cy)) = best {
            self.x = cx;
            self.y = cy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(index: u32, x: i32, y: i32, w: u32, h: u32) -> Zone {
        Zone {
            index,
            name: Some(format!("m{index}")),
            x,
            y,
            width: w,
            height: h,
            scale: 1.0,
        }
    }

    fn motion(dx: f32, dy: f32) -> Vec<InputEvent> {
        vec![InputEvent::Motion { dx, dy }]
    }

    #[test]
    fn a_push_out_through_the_span_returns_with_the_position_along_the_edge() {
        let z = zone(0, 0, 0, 1000, 1000);
        // entered at the top edge, way back over the left half
        let mut t = SpanTrack::new(z.clone(), Edge::Top, 0.25, (0.0, 0.5));
        let mut ev = motion(0.0, 3.0);
        assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        // 3 px in, then 4 px out: back at the line with 0 px of push
        let mut ev = motion(0.0, -4.0);
        assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        let mut ev = motion(0.0, -6.0);
        assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        let mut ev = motion(0.0, -6.0);
        let pos = t.step(std::slice::from_ref(&z), &mut ev);
        let pos = pos.expect("12 px of push is past the slack");
        assert!((pos - 0.25).abs() < 0.01, "{pos}");
    }

    #[test]
    fn the_push_is_taken_out_of_the_emitted_motion() {
        let z = zone(0, 0, 0, 1000, 1000);
        let mut t = SpanTrack::new(z.clone(), Edge::Left, 0.5, (0.0, 1.0));
        let mut ev = motion(-5.0, 2.0);
        t.step(std::slice::from_ref(&z), &mut ev);
        // the cursor sat 1 px inside the line: only that 1 px goes to the emulator
        assert_eq!(ev, vec![InputEvent::Motion { dx: -1.0, dy: 2.0 }]);
    }

    #[test]
    fn outside_the_span_the_edge_is_a_wall_that_never_returns() {
        let z = zone(0, 0, 0, 1000, 1000);
        // way back only over the top fifth; the cursor enters at 0.9
        let mut t = SpanTrack::new(z.clone(), Edge::Left, 0.9, (0.0, 0.2));
        for _ in 0..10 {
            let mut ev = motion(-50.0, 0.0);
            assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        }
        assert!(
            (t.at().0 - 0.0).abs() < 0.5,
            "pinned at the zone edge: {:?}",
            t.at()
        );
    }

    #[test]
    fn another_edge_does_not_return() {
        let z = zone(0, 0, 0, 1000, 1000);
        let mut t = SpanTrack::new(z.clone(), Edge::Left, 0.5, (0.0, 1.0));
        for _ in 0..10 {
            let mut ev = motion(0.0, 80.0);
            assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        }
        for _ in 0..10 {
            let mut ev = motion(80.0, 0.0);
            assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
        }
    }

    #[test]
    fn a_neighbour_monitor_at_the_line_does_not_stop_the_return() {
        // the entry monitor has another one on its left: the span line still counts as the edge
        let near = zone(0, 0, 0, 1000, 1000);
        let left = zone(1, -1000, 0, 1000, 1000);
        let zones = [near.clone(), left];
        let mut t = SpanTrack::new(near, Edge::Left, 0.5, (0.0, 1.0));
        let mut out = None;
        for _ in 0..4 {
            let mut ev = motion(-6.0, 0.0);
            out = out.or(t.step(&zones, &mut ev));
        }
        assert!(out.is_some());
    }

    #[test]
    fn jitter_below_the_slack_does_not_return() {
        let z = zone(0, 0, 0, 1000, 1000);
        let mut t = SpanTrack::new(z.clone(), Edge::Left, 0.5, (0.0, 1.0));
        for _ in 0..20 {
            let mut ev = motion(-3.0, 0.0);
            assert!(t.step(std::slice::from_ref(&z), &mut ev).is_none());
            let mut back = motion(4.0, 0.0);
            assert!(t.step(std::slice::from_ref(&z), &mut back).is_none());
        }
    }

    #[test]
    fn the_estimate_stays_inside_the_union_of_the_zones() {
        let a = zone(0, 0, 0, 100, 100);
        let b = zone(1, 200, 0, 100, 100);
        let zones = [a.clone(), b];
        let mut t = SpanTrack::new(a, Edge::Left, 0.5, (0.0, 1.0));
        let mut ev = motion(120.0, 0.0);
        t.step(&zones, &mut ev);
        let (x, y) = t.at();
        assert!(zones.iter().any(|z| contains(z, x, y)), "{x},{y}");
    }
}
