//! The decision core of the Windows capture (contract §13.6 "Adaptador Windows", item 1). It runs
//! inside the hook callback, so it is plain data and arithmetic: no lock, no allocation per event, no
//! I/O, no log. Everything the callback cannot do (park the cursor, call `SendInput`, wait for a
//! channel) comes out as an [`Out`] for the worker thread. Pure, so it tests on every system.
//!
//! Key and button codes never reach a log (§13.2 rule 2e): `Debug` of [`Out`] and [`Held`] is redacted.
use super::winmath::{HotkeyTracker, barrier_crossing, capture_delta};
use super::{Hotkey, Zone};
use crate::keymap;
use clipsync_proto::{Edge, InputEvent};
use clipsync_win::{RawEvent, Verdict, WinButton};
use std::fmt;
use std::sync::Arc;

/// A barrier the hook can trigger: the zone it sits on (physical px of the virtual desktop) and the
/// stretch of the edge that is linked.
#[derive(Debug, Clone)]
pub struct ArmedBarrier {
    pub zone: Zone,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}

/// What is physically held right now: keys by evdev code (the codes `keymap` makes, all below 256)
/// and mouse buttons. A fixed-size `Copy` value: no history, no counters (Sentinel R3).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Held {
    keys: [u64; 4],
    buttons: u8,
}

const KEY_LEFTCTRL: u32 = 29;
const KEY_LEFTSHIFT: u32 = 42;
const KEY_RIGHTSHIFT: u32 = 54;
const KEY_LEFTALT: u32 = 56;
const KEY_RIGHTCTRL: u32 = 97;
const KEY_RIGHTALT: u32 = 100;
const KEY_LEFTMETA: u32 = 125;
const KEY_RIGHTMETA: u32 = 126;

/// The keys that are safe to carry across: modifiers only, never a letter (Sentinel R3).
pub const MODIFIERS: [u32; 8] = [
    KEY_LEFTCTRL,
    KEY_LEFTSHIFT,
    KEY_RIGHTSHIFT,
    KEY_LEFTALT,
    KEY_RIGHTCTRL,
    KEY_RIGHTALT,
    KEY_LEFTMETA,
    KEY_RIGHTMETA,
];

const BUTTONS: [WinButton; 5] = [
    WinButton::Left,
    WinButton::Right,
    WinButton::Middle,
    WinButton::X1,
    WinButton::X2,
];

fn button_bit(button: WinButton) -> u8 {
    match button {
        WinButton::Left => 1,
        WinButton::Right => 2,
        WinButton::Middle => 4,
        WinButton::X1 => 8,
        WinButton::X2 => 16,
    }
}

impl Held {
    pub fn key(&self, code: u32) -> bool {
        code < 256 && self.keys[(code / 64) as usize] & (1 << (code % 64)) != 0
    }

    fn set_key(&mut self, code: u32, down: bool) {
        if code >= 256 {
            return;
        }
        let (word, bit) = ((code / 64) as usize, 1u64 << (code % 64));
        if down {
            self.keys[word] |= bit;
        } else {
            self.keys[word] &= !bit;
        }
    }

    pub fn button(&self, button: WinButton) -> bool {
        self.buttons & button_bit(button) != 0
    }

    fn set_button(&mut self, button: WinButton, down: bool) {
        if down {
            self.buttons |= button_bit(button);
        } else {
            self.buttons &= !button_bit(button);
        }
    }

    /// Adds a modifier or a button that the system says is down although the hook never saw it.
    pub fn add_key(&mut self, code: u32) {
        self.set_key(code, true);
    }

    pub fn add_button(&mut self, button: WinButton) {
        self.set_button(button, true);
    }

    /// Every key held, in evdev code order.
    pub fn keys(&self) -> impl Iterator<Item = u32> + '_ {
        (0..256u32).filter(|&c| self.key(c))
    }

    /// The modifiers held.
    pub fn modifiers(&self) -> impl Iterator<Item = u32> + '_ {
        MODIFIERS.into_iter().filter(|&c| self.key(c))
    }

    pub fn buttons(&self) -> impl Iterator<Item = WinButton> + '_ {
        BUTTONS.into_iter().filter(|&b| self.button(b))
    }

    pub fn is_empty(&self) -> bool {
        self.keys == [0; 4] && self.buttons == 0
    }

    /// The words to store in shared atomics, and back.
    pub fn to_words(self) -> [u64; 5] {
        [
            self.keys[0],
            self.keys[1],
            self.keys[2],
            self.keys[3],
            u64::from(self.buttons),
        ]
    }

    pub fn from_words(w: [u64; 5]) -> Held {
        Held {
            keys: [w[0], w[1], w[2], w[3]],
            buttons: (w[4] & 0xFF) as u8,
        }
    }
}

impl fmt::Debug for Held {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Held").finish_non_exhaustive()
    }
}

/// What the callback hands to the worker thread.
#[derive(Clone, Copy, PartialEq)]
pub enum Out {
    /// The cursor crossed barrier `barrier` at `pos` (0 to 1 along the whole edge). The worker parks
    /// the cursor, lets go locally of everything in `held`, and carries the modifiers and buttons over.
    Activate {
        barrier: usize,
        pos: f32,
        held: Held,
    },
    /// An event for the peer, in the order it happened.
    Input(InputEvent),
    Hotkey(Hotkey),
    /// Tap the neutral key and then release the key with this evdev code: the Win or Alt that was held
    /// for a shortcut must not open the Start menu or the Alt menu.
    TapThenRelease(u32),
}

impl fmt::Debug for Out {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Out::Activate { barrier, pos, .. } => f
                .debug_struct("Activate")
                .field("barrier", barrier)
                .field("pos", pos)
                .finish_non_exhaustive(),
            Out::Input(ev) => f.debug_tuple("Input").field(ev).finish(),
            Out::Hotkey(h) => f.debug_tuple("Hotkey").field(h).finish(),
            Out::TapThenRelease(_) => f.write_str("TapThenRelease(<redacted>)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Idle,
    /// Crossed; the worker is still parking the cursor. Moves are eaten, the rest already goes out.
    Arming,
    /// Parked at `park` on a monitor of `dpi`: every move is a delta from that point.
    Active {
        park: (i32, i32),
        dpi: u32,
    },
}

#[derive(Debug)]
pub struct CaptureCore {
    barriers: Arc<[Option<ArmedBarrier>]>,
    locked: bool,
    mode: Mode,
    prev: Option<(i32, i32)>,
    tracker: HotkeyTracker,
    held: Held,
}

impl Default for CaptureCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureCore {
    pub fn new() -> CaptureCore {
        CaptureCore {
            barriers: Arc::from(Vec::new()),
            locked: false,
            mode: Mode::Idle,
            prev: None,
            tracker: HotkeyTracker::new(),
            held: Held::default(),
        }
    }

    /// `None` is a barrier whose zone is gone (§13.8 item 3): it stays in its place so the indexes
    /// the module knows do not shift.
    pub fn set_barriers(&mut self, barriers: Arc<[Option<ArmedBarrier>]>) {
        self.barriers = barriers;
    }

    /// With the screen locked no barrier activates.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn capturing(&self) -> bool {
        self.mode != Mode::Idle
    }

    /// The worker parked the cursor at `park`, on a monitor of `dpi`.
    pub fn park_ready(&mut self, park: (i32, i32), dpi: u32) {
        if self.mode == Mode::Arming {
            self.mode = Mode::Active { park, dpi };
        }
    }

    /// The capture ended (the cursor goes back to the local desktop).
    pub fn end_capture(&mut self) {
        self.mode = Mode::Idle;
        self.prev = None;
    }

    pub fn held(&self) -> Held {
        self.held
    }

    pub fn on_event(&mut self, ev: &RawEvent, out: &mut dyn FnMut(Out)) -> Verdict {
        match *ev {
            RawEvent::Move { x, y } => self.on_move((x, y), out),
            RawEvent::Button { button, down } => {
                self.held.set_button(button, down);
                self.forward(
                    InputEvent::Button {
                        code: keymap::button_from_windows(button),
                        pressed: down,
                    },
                    out,
                )
            }
            RawEvent::Wheel { delta, horizontal } => {
                let (dx, dy) = if horizontal { (delta, 0) } else { (0, -delta) };
                self.forward(InputEvent::ScrollDiscrete { dx, dy }, out)
            }
            RawEvent::Key {
                vk,
                scan,
                extended,
                up,
            } => self.on_key(vk, scan, extended, !up, out),
        }
    }

    /// During a capture the event goes to the peer and nowhere else; otherwise it passes.
    fn forward(&mut self, ev: InputEvent, out: &mut dyn FnMut(Out)) -> Verdict {
        if self.mode == Mode::Idle {
            return Verdict::Pass;
        }
        out(Out::Input(ev));
        Verdict::Block
    }

    fn on_move(&mut self, at: (i32, i32), out: &mut dyn FnMut(Out)) -> Verdict {
        match self.mode {
            Mode::Idle => {
                let prev = self.prev.replace(at);
                if self.locked {
                    return Verdict::Pass;
                }
                let from = prev.unwrap_or(at);
                for (index, slot) in self.barriers.iter().enumerate() {
                    let Some(b) = slot else { continue };
                    if let Some(pos) = barrier_crossing(&b.zone, b.edge, b.from, b.to, from, at) {
                        self.mode = Mode::Arming;
                        self.prev = None;
                        out(Out::Activate {
                            barrier: index,
                            pos,
                            held: self.held,
                        });
                        return Verdict::Block;
                    }
                }
                Verdict::Pass
            }
            Mode::Arming => Verdict::Block,
            Mode::Active { park, dpi } => {
                let (dx, dy) = capture_delta(at, park, dpi);
                if dx != 0.0 || dy != 0.0 {
                    out(Out::Input(InputEvent::Motion { dx, dy }));
                }
                Verdict::Block
            }
        }
    }

    fn on_key(
        &mut self,
        vk: u32,
        scan: u32,
        extended: bool,
        down: bool,
        out: &mut dyn FnMut(Out),
    ) -> Verdict {
        let Some(code) = keymap::from_windows(vk, scan, extended) else {
            // a fake modifier or a key with no counterpart
            return if self.mode == Mode::Idle {
                Verdict::Pass
            } else {
                Verdict::Block
            };
        };
        let repeat = down && self.held.key(code);
        self.held.set_key(code, down);

        let decision = self.tracker.on_key(code, down);
        if let Some(hotkey) = decision.shortcut {
            out(Out::Hotkey(hotkey));
        }
        if decision.swallow {
            return Verdict::Block;
        }
        if self.mode == Mode::Idle {
            if decision.neutral_tap {
                out(Out::TapThenRelease(code));
                return Verdict::Block;
            }
            return Verdict::Pass;
        }
        // The destination repeats a held key by itself: only the transitions go out.
        if !repeat {
            out(Out::Input(InputEvent::Key {
                code,
                pressed: down,
            }));
        }
        Verdict::Block
    }
}
