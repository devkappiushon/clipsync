//! Decisions of the Windows adapter that need no Windows (contract §13.6, "winmath"): zone names,
//! zones from monitors, barrier crossing, `MoveAbs` normalization, physical to logical px, motion
//! and shortcut recognition. Pure: it compiles and tests on every system, and `adapter/win/` only
//! calls it.
//!
//! Nothing here logs: key codes stay out of every message (§13.2 rule 2e).
use super::{Hotkey, Zone, barrier_segment, edge_position};
use clipsync_proto::{Edge, MAX_NAME_BYTES, validate_monitor_name};
use clipsync_win::{Monitor, WinKey};
use std::collections::HashSet;

/// Windows' reference DPI: scale 1.0.
pub const BASE_DPI: u32 = 96;

/// Normalized `MoveAbs` coordinates go from 0 to this value over the virtual desktop.
pub const ABS_MAX: i32 = 65535;
const ABS_RANGE: i64 = 65536;

/// The key the adapter taps to keep the Start menu and the Alt menu from opening after a shortcut:
/// `VK_OEM_CLEAR`-style unassigned code that no program binds.
pub const NEUTRAL_KEY: WinKey = WinKey::Vk(0xE8);

/// A rectangle in physical px of the virtual desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn of(monitor: &Monitor) -> Rect {
        Rect {
            x: monitor.x,
            y: monitor.y,
            width: monitor.width,
            height: monitor.height,
        }
    }

    fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }
}

// ---------------------------------------------------------------------------------------------
// Names and zones

const FALLBACK_NAME: &str = "Monitor";

fn clean(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|c| validate_monitor_name(c.encode_utf8(&mut [0; 4])).is_ok())
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Cuts `text` to at most `max` bytes, never in the middle of a character.
fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn base_name(monitor: &Monitor) -> String {
    let friendly = clean(&monitor.friendly);
    if !friendly.is_empty() {
        return friendly;
    }
    let device = monitor
        .device
        .strip_prefix(r"\\.\")
        .unwrap_or(&monitor.device);
    let device = clean(device);
    if device.is_empty() {
        FALLBACK_NAME.to_owned()
    } else {
        device
    }
}

/// The stable name of each zone, in the order of `monitors`: the friendly name of
/// `QueryDisplayConfig`; if empty, the device without `\\.\`; a repeated name gets " 2", " 3" in the
/// order of `path`; at most 64 bytes, cut on a character boundary. Every name passes
/// `validate_monitor_name` and no two are equal.
pub fn zone_names(monitors: &[Monitor]) -> Vec<String> {
    let bases: Vec<String> = monitors.iter().map(base_name).collect();
    let mut order: Vec<usize> = (0..monitors.len()).collect();
    order.sort_by(|&a, &b| monitors[a].path.cmp(&monitors[b].path).then(a.cmp(&b)));

    let mut used: HashSet<String> = HashSet::new();
    let mut names = vec![String::new(); monitors.len()];
    for index in order {
        let mut candidate = cut(&bases[index], MAX_NAME_BYTES).to_owned();
        let mut n = 1;
        while used.contains(&candidate) {
            n += 1;
            let suffix = format!(" {n}");
            let base = cut(&bases[index], MAX_NAME_BYTES - suffix.len()).trim_end();
            candidate = format!("{base}{suffix}");
        }
        used.insert(candidate.clone());
        names[index] = candidate;
    }
    names
}

/// `scale` for a monitor DPI (`dpi / 96`); an unknown DPI (0) counts as 100%.
pub fn scale_of(dpi: u32) -> f64 {
    if dpi == 0 {
        1.0
    } else {
        f64::from(dpi) / f64::from(BASE_DPI)
    }
}

/// Zones in physical px of the virtual desktop, one per monitor, in the order of `monitors`.
pub fn zones(monitors: &[Monitor]) -> Vec<Zone> {
    zone_names(monitors)
        .into_iter()
        .zip(monitors)
        .enumerate()
        .map(|(index, (name, m))| Zone {
            index: index as u32,
            name: Some(name),
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
            scale: scale_of(m.dpi),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Barrier crossing

/// Whether the hook's move from `prev` to `new` (physical px) activates the barrier `from..to` on
/// `edge` of `zone`, and the position (0 to 1) along the whole edge where it happened.
///
/// It activates when the new point is on the last pixel inside the zone, within the segment, and
/// does not move back inward; or when the move crosses the line of the segment (an edge between two
/// local monitors). The hook may or may not clamp the point at the desktop's edge, so both count.
pub fn barrier_crossing(
    zone: &Zone,
    edge: Edge,
    from: f32,
    to: f32,
    prev: (i32, i32),
    new: (i32, i32),
) -> Option<f32> {
    let seg = barrier_segment(zone, edge, from, to);
    // (normal axis of prev and new, tangent of new, line, tangent range, outward sign)
    let (n_prev, n_new, t_new, line, t1, t2, sign) = match edge {
        Edge::Right => (prev.0, new.0, new.1, seg.x1, seg.y1, seg.y2, 1i64),
        Edge::Left => (prev.0, new.0, new.1, seg.x1, seg.y1, seg.y2, -1),
        Edge::Bottom => (prev.1, new.1, new.0, seg.y1, seg.x1, seg.x2, 1),
        Edge::Top => (prev.1, new.1, new.0, seg.y1, seg.x1, seg.x2, -1),
    };
    if t_new < t1 || t_new > t2 {
        return None;
    }
    let last_inside = if sign > 0 { line - 1 } else { line };
    let out = |n: i32| sign * i64::from(n);
    let (o_last, o_prev, o_new) = (out(last_inside), out(n_prev), out(n_new));
    let crossed = o_new > o_last && o_prev <= o_last;
    let pushing = o_new == o_last && o_new >= o_prev;
    (crossed || pushing).then(|| edge_position(zone, edge, new.0 as f32, new.1 as f32))
}

// ---------------------------------------------------------------------------------------------
// Coordinates

/// The `MoveAbs` pair (0 to 65535 over the virtual desktop `rect`) that lands on pixel `(x, y)`.
/// The point is clamped into `rect` first. Windows maps `n` to the pixel `floor(n * width / 65536)`,
/// so the smallest `n` that lands on a pixel is `ceil(pixel * 65536 / width)`; the origin may be
/// negative.
pub fn normalize_abs(rect: Rect, x: i32, y: i32) -> (i32, i32) {
    (
        normalize_axis(rect.x, rect.width, x),
        normalize_axis(rect.y, rect.height, y),
    )
}

fn normalize_axis(origin: i32, len: u32, value: i32) -> i32 {
    if len == 0 {
        return 0;
    }
    let len = i64::from(len);
    let rel = (i64::from(value) - i64::from(origin)).clamp(0, len - 1);
    let n = (rel * ABS_RANGE + len - 1) / len;
    n.min(i64::from(ABS_MAX)) as i32
}

/// The pixel a `MoveAbs` pair lands on (the inverse of [`normalize_abs`]).
pub fn denormalize_abs(rect: Rect, nx: i32, ny: i32) -> (i32, i32) {
    (
        denormalize_axis(rect.x, rect.width, nx),
        denormalize_axis(rect.y, rect.height, ny),
    )
}

fn denormalize_axis(origin: i32, len: u32, n: i32) -> i32 {
    let n = i64::from(n.clamp(0, ABS_MAX));
    let rel = (n * i64::from(len) / ABS_RANGE).min(i64::from(len.saturating_sub(1)));
    (i64::from(origin) + rel) as i32
}

/// Physical px to logical px for a monitor of `dpi`.
pub fn physical_to_logical(px: f32, dpi: u32) -> f32 {
    (f64::from(px) / scale_of(dpi)) as f32
}

/// Logical px to physical px for a monitor of `dpi`.
pub fn logical_to_physical(px: f32, dpi: u32) -> f32 {
    (f64::from(px) * scale_of(dpi)) as f32
}

/// Motion during an active capture: the hook point minus the parked cursor, in logical px by the
/// scale of the monitor where the cursor is parked.
pub fn capture_delta(point: (i32, i32), parked: (i32, i32), dpi: u32) -> (f32, f32) {
    (
        physical_to_logical((point.0 - parked.0) as f32, dpi),
        physical_to_logical((point.1 - parked.1) as f32, dpi),
    )
}

/// The point of the union of `rects` closest to `point`. With no rect, `point` itself.
pub fn clamp_to_union(point: (i32, i32), rects: &[Rect]) -> (i32, i32) {
    let mut best: Option<((i64, i64), i64)> = None;
    for r in rects.iter().filter(|r| r.width > 0 && r.height > 0) {
        let x = i64::from(point.0).clamp(i64::from(r.x), r.right() - 1);
        let y = i64::from(point.1).clamp(i64::from(r.y), r.bottom() - 1);
        let (dx, dy) = (x - i64::from(point.0), y - i64::from(point.1));
        let dist = dx * dx + dy * dy;
        if best.is_none_or(|(_, d)| dist < d) {
            best = Some(((x, y), dist));
        }
    }
    best.map_or(point, |((x, y), _)| (x as i32, y as i32))
}

/// Emulated motion (§13.6 item 2): new position = current cursor plus delta times the scale of the
/// monitor the cursor is on, held inside the union of the monitors. `rest` is the fraction that did
/// not fit in a whole pixel; give back what this returns on the next call. The move goes out as
/// `MoveAbs`, not as relative motion, so the system's acceleration does not act twice.
pub fn emulate_motion(
    cursor: (i32, i32),
    rest: (f32, f32),
    delta: (f32, f32),
    scale: f64,
    rects: &[Rect],
) -> ((i32, i32), (f32, f32)) {
    let fx = f64::from(rest.0) + f64::from(delta.0) * scale;
    let fy = f64::from(rest.1) + f64::from(delta.1) * scale;
    let (sx, sy) = (fx.round(), fy.round());
    let target = (
        (i64::from(cursor.0) + sx as i64).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        (i64::from(cursor.1) + sy as i64).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
    );
    let placed = clamp_to_union(target, rects);
    // Pressing against a wall must not store motion to be paid later.
    let rest = (
        if placed.0 == target.0 {
            (fx - sx) as f32
        } else {
            0.0
        },
        if placed.1 == target.1 {
            (fy - sy) as f32
        } else {
            0.0
        },
    );
    (placed, rest)
}

// ---------------------------------------------------------------------------------------------
// Shortcuts

const KEY_P: u32 = 25;
const KEY_L: u32 = 38;
const KEY_1: u32 = 2;
const KEY_9: u32 = 10;
const KEY_LEFTALT: u32 = 56;
const KEY_RIGHTALT: u32 = 100;
const KEY_LEFTMETA: u32 = 125;
const KEY_RIGHTMETA: u32 = 126;

fn is_win(code: u32) -> bool {
    code == KEY_LEFTMETA || code == KEY_RIGHTMETA
}

fn is_alt(code: u32) -> bool {
    code == KEY_LEFTALT || code == KEY_RIGHTALT
}

fn shortcut_of(code: u32) -> Option<Hotkey> {
    match code {
        KEY_1..=KEY_9 => Some(Hotkey::GoTo((code - KEY_1 + 1) as u8)),
        KEY_L => Some(Hotkey::ToggleLock),
        KEY_P => Some(Hotkey::TogglePause),
        _ => None,
    }
}

/// What to do with one key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyDecision {
    /// Eat the event: it goes neither to the system nor to the peer.
    pub swallow: bool,
    /// The press that completed Win+Alt+1 to 9, L or P.
    pub shortcut: Option<Hotkey>,
    /// Tap [`NEUTRAL_KEY`] (press and release) before letting this release through, so the Win or
    /// Alt that was held for a shortcut does not open the Start menu or the Alt menu.
    pub neutral_tap: bool,
}

/// How many shortcut keys can be held at once. More than that cannot happen with two hands on
/// Win+Alt+key; a fifth is simply not recognized.
const SWALLOW_SLOTS: usize = 4;

/// Recognizes Win+Alt+1 to 9, Win+Alt+L and Win+Alt+P in the stream of evdev key events. It is a
/// function of what is held right now plus the key that just arrived: `Copy`, fixed size, no
/// allocation, no history, no counters, no time window (Sentinel R3).
#[derive(Clone, Copy, Default)]
pub struct HotkeyTracker {
    wins: u8,
    alts: u8,
    swallowed: [u32; SWALLOW_SLOTS],
    fired: bool,
}

impl std::fmt::Debug for HotkeyTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotkeyTracker").finish_non_exhaustive()
    }
}

impl HotkeyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget everything held (session or lock change).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn bit(code: u32) -> u8 {
        if code == KEY_LEFTMETA || code == KEY_LEFTALT {
            1
        } else {
            2
        }
    }

    pub fn on_key(&mut self, code: u32, pressed: bool) -> KeyDecision {
        let mut decision = KeyDecision::default();
        if is_win(code) || is_alt(code) {
            let held = if is_win(code) {
                &mut self.wins
            } else {
                &mut self.alts
            };
            if pressed {
                *held |= Self::bit(code);
            } else {
                *held &= !Self::bit(code);
                decision.neutral_tap = self.fired;
                if self.wins == 0 && self.alts == 0 {
                    self.fired = false;
                } else if decision.neutral_tap {
                    // one tap between the press and the release covers both modifiers
                    self.fired = false;
                }
            }
            return decision;
        }

        if let Some(slot) = self.swallowed.iter_mut().find(|c| **c == code) {
            // auto-repeat of a swallowed key, or its release (possibly after Win or Alt went up)
            if !pressed {
                *slot = 0;
            }
            decision.swallow = true;
            return decision;
        }
        if pressed
            && self.wins != 0
            && self.alts != 0
            && let Some(hotkey) = shortcut_of(code)
            && let Some(slot) = self.swallowed.iter_mut().find(|c| **c == 0)
        {
            *slot = code;
            self.fired = true;
            decision.swallow = true;
            decision.shortcut = Some(hotkey);
        }
        decision
    }
}
