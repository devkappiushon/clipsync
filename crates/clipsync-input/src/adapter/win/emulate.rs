//! `WinEmulate` (name `"win-sendinput"`): `SendInput` behind the checks of contract §13.6,
//! "Adaptador Windows" item 2. Motion is absolute, computed from the delta, so the system's pointer
//! acceleration does not act twice. Nothing here logs a key (§13.2 rule 2e).
use super::super::winmath::{self, Rect};
use super::super::{EmulateAdapter, InputError, Zone};
use crate::keymap;
use async_trait::async_trait;
use clipsync_proto::{Edge, InputEvent};
use clipsync_win::{Monitor, Notice, Synth, Watcher, WinError};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Wheel units per logical pixel of smooth scroll: 120 for every 15 px (to tune at the roteiro).
const WHEEL_PER_PX: f32 = 120.0 / 15.0;

#[derive(Default)]
struct State {
    session_locked: AtomicBool,
    monitors_changed: AtomicBool,
}

pub struct WinEmulate {
    watcher: Option<Watcher>,
    state: Arc<State>,
    monitors: Vec<Monitor>,
    motion_rest: (f32, f32),
    scroll_rest: (f32, f32),
    keys: HashSet<u32>,
    buttons: HashSet<u32>,
}

impl Default for WinEmulate {
    fn default() -> Self {
        Self::new()
    }
}

fn io_err(call: &str, e: &WinError) -> InputError {
    InputError::Other(format!("{call}: {e}"))
}

fn virtual_rect() -> Rect {
    let (x, y, w, h) = clipsync_win::virtual_screen();
    Rect {
        x,
        y,
        width: w.max(0) as u32,
        height: h.max(0) as u32,
    }
}

fn move_abs(x: i32, y: i32) -> Synth {
    let (nx, ny) = winmath::normalize_abs(virtual_rect(), x, y);
    Synth::MoveAbs { nx, ny }
}

impl WinEmulate {
    pub fn new() -> WinEmulate {
        WinEmulate {
            watcher: None,
            state: Arc::new(State::default()),
            monitors: Vec::new(),
            motion_rest: (0.0, 0.0),
            scroll_rest: (0.0, 0.0),
            keys: HashSet::new(),
            buttons: HashSet::new(),
        }
    }

    /// The KLID of the keyboard layout of this machine, for the diagnostic only.
    pub fn keyboard_layout(&self) -> Option<String> {
        clipsync_win::keyboard_layout()
    }

    /// Before every batch: with the screen locked or the secure desktop up, nothing is injected.
    fn ensure_unlocked(&self) -> Result<(), InputError> {
        if self.state.session_locked.load(Ordering::Relaxed)
            || !clipsync_win::input_desktop_is_default()
        {
            return Err(InputError::Locked);
        }
        Ok(())
    }

    fn refresh_monitors(&mut self) {
        if self.monitors.is_empty() || self.state.monitors_changed.swap(false, Ordering::AcqRel) {
            self.monitors = clipsync_win::monitors().unwrap_or_default();
        }
    }

    fn rects(&self) -> Vec<Rect> {
        self.monitors.iter().map(Rect::of).collect()
    }

    /// Scale of the monitor under `at`; 100% when none.
    fn scale_at(&self, at: (i32, i32)) -> f64 {
        let dpi = self
            .monitors
            .iter()
            .find(|m| {
                let r = Rect::of(m);
                let (x, y) = (i64::from(at.0), i64::from(at.1));
                x >= i64::from(r.x)
                    && x < i64::from(r.x) + i64::from(r.width)
                    && y >= i64::from(r.y)
                    && y < i64::from(r.y) + i64::from(r.height)
            })
            .map_or(0, |m| m.dpi);
        winmath::scale_of(dpi)
    }

    fn send(batch: &[Synth]) -> Result<(), InputError> {
        if batch.is_empty() {
            return Ok(());
        }
        clipsync_win::send(batch)
            .map(drop)
            .map_err(|e| io_err("SendInput", &e))
    }

    /// Smooth scroll in logical px to wheel units, keeping what does not fill a whole unit.
    fn wheel_units(rest: &mut f32, px: f32) -> i32 {
        let total = *rest + px * WHEEL_PER_PX;
        let whole = total.trunc();
        *rest = total - whole;
        whole as i32
    }
}

#[async_trait]
impl EmulateAdapter for WinEmulate {
    fn name(&self) -> &'static str {
        "win-sendinput"
    }

    async fn start(&mut self) -> Result<(), InputError> {
        let state = Arc::clone(&self.state);
        let watcher = Watcher::start(Box::new(move |notice| match notice {
            Notice::SessionLocked => state.session_locked.store(true, Ordering::Relaxed),
            Notice::SessionUnlocked => state.session_locked.store(false, Ordering::Relaxed),
            Notice::DisplayChanged => state.monitors_changed.store(true, Ordering::Release),
            // The secure desktop is checked before every batch; a lost hook is not this adapter's.
            Notice::DesktopSwitched | Notice::HooksLost => {}
        }))
        .map_err(|e| io_err("Watcher", &e))?;
        self.watcher = Some(watcher);
        self.refresh_monitors();
        Ok(())
    }

    async fn emit(&mut self, events: &[InputEvent]) -> Result<(), InputError> {
        self.ensure_unlocked()?;
        self.refresh_monitors();
        let rects = self.rects();
        let mut cursor = clipsync_win::cursor_pos().map_err(|e| io_err("GetCursorPos", &e))?;
        let mut batch = Vec::with_capacity(events.len());
        for event in events {
            match *event {
                InputEvent::Motion { dx, dy } => {
                    let scale = self.scale_at(cursor);
                    let (at, rest) =
                        winmath::emulate_motion(cursor, self.motion_rest, (dx, dy), scale, &rects);
                    self.motion_rest = rest;
                    if at != cursor {
                        cursor = at;
                        batch.push(move_abs(at.0, at.1));
                    }
                }
                InputEvent::Button { code, pressed } => {
                    if let Some(button) = keymap::button_to_windows(code) {
                        if pressed {
                            self.buttons.insert(code);
                        } else {
                            self.buttons.remove(&code);
                        }
                        batch.push(Synth::Button {
                            button,
                            down: pressed,
                        });
                    }
                }
                InputEvent::Scroll { dx, dy } => {
                    // down is positive in the protocol, up is positive for the wheel
                    let vertical = -Self::wheel_units(&mut self.scroll_rest.1, dy);
                    let horizontal = Self::wheel_units(&mut self.scroll_rest.0, dx);
                    if vertical != 0 {
                        batch.push(Synth::Wheel {
                            delta: vertical,
                            horizontal: false,
                        });
                    }
                    if horizontal != 0 {
                        batch.push(Synth::Wheel {
                            delta: horizontal,
                            horizontal: true,
                        });
                    }
                }
                InputEvent::ScrollDiscrete { dx, dy } => {
                    if dy != 0 {
                        batch.push(Synth::Wheel {
                            delta: -dy,
                            horizontal: false,
                        });
                    }
                    if dx != 0 {
                        batch.push(Synth::Wheel {
                            delta: dx,
                            horizontal: true,
                        });
                    }
                }
                InputEvent::Key { code, pressed } => {
                    // a key with no counterpart on Windows is dropped
                    if let Some(key) = keymap::to_windows(code) {
                        if pressed {
                            self.keys.insert(code);
                        } else {
                            self.keys.remove(&code);
                        }
                        batch.push(Synth::Key { key, up: !pressed });
                    }
                }
            }
        }
        Self::send(&batch)
    }

    async fn release_all(&mut self) -> Result<(), InputError> {
        let mut batch = Vec::new();
        for code in self.keys.drain() {
            if let Some(key) = keymap::to_windows(code) {
                batch.push(Synth::Key { key, up: true });
            }
        }
        for code in self.buttons.drain() {
            if let Some(button) = keymap::button_to_windows(code) {
                batch.push(Synth::Button {
                    button,
                    down: false,
                });
            }
        }
        self.scroll_rest = (0.0, 0.0);
        self.motion_rest = (0.0, 0.0);
        // Letting go is always allowed, even on a locked screen: it injects nothing that acts.
        Self::send(&batch)
    }

    async fn enter(
        &mut self,
        zone: &Zone,
        _zones: &[Zone],
        edge: Edge,
        pos: f32,
    ) -> Result<(), InputError> {
        self.ensure_unlocked()?;
        let (x, y) = winmath::inside_point(zone, edge, pos);
        self.motion_rest = (0.0, 0.0);
        Self::send(&[move_abs(x, y)])
    }

    fn cursor(&self) -> Option<(i32, i32)> {
        clipsync_win::cursor_pos().ok()
    }

    async fn stop(&mut self) -> Result<(), InputError> {
        let released = self.release_all().await;
        self.watcher = None;
        released
    }
}
