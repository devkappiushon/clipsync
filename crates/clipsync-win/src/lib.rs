//! Win32 calls behind a safe API (contract §13.3 and §13.5).
//!
//! The types compile on every system; the functions will live behind `cfg(windows)`. This crate
//! only calls the system and converts types: every decision (crossing, translation, coordinates,
//! shortcuts, what to block) belongs to `clipsync-input`.
//!
//! `Debug` of anything that names a key is redacted: no virtual-key, scan code or character ever
//! reaches a log (§13.2 rule 2e).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// A key as the system names it. `Debug` is manual: the code is never printed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WinKey {
    Scan { code: u8, extended: bool },
    Vk(u16),
}

impl fmt::Debug for WinKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WinKey").field(&Redacted).finish()
    }
}

/// What the hook saw, already without injected events. `Debug` of `Key` and `Button` is redacted.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RawEvent {
    /// `MSLLHOOKSTRUCT.pt`, physical px of the virtual desktop.
    Move { x: i32, y: i32 },
    Button { button: WinButton, down: bool },
    /// In `WHEEL_DELTA` units (120).
    Wheel { delta: i32, horizontal: bool },
    /// `KBDLLHOOKSTRUCT`.
    Key {
        vk: u32,
        scan: u32,
        extended: bool,
        up: bool,
    },
}

impl fmt::Debug for RawEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawEvent::Move { x, y } => f.debug_struct("Move").field("x", x).field("y", y).finish(),
            RawEvent::Button { .. } => f.debug_tuple("Button").field(&Redacted).finish(),
            RawEvent::Wheel { delta, horizontal } => f
                .debug_struct("Wheel")
                .field("delta", delta)
                .field("horizontal", horizontal)
                .finish(),
            RawEvent::Key { .. } => f.debug_tuple("Key").field(&Redacted).finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    DisplayChanged,
    SessionLocked,
    SessionUnlocked,
    DesktopSwitched,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub friendly: String,
    pub device: String,
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub primary: bool,
}

/// One synthetic input for `SendInput`. `MoveAbs` is already normalized (0 to 65535 over the
/// virtual desktop) by `clipsync-input`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Synth {
    MoveAbs { nx: i32, ny: i32 },
    Button { button: WinButton, down: bool },
    Wheel { delta: i32, horizontal: bool },
    Key { key: WinKey, up: bool },
}

#[derive(thiserror::Error, Debug)]
#[error("win32 {call}: {code}")]
pub struct WinError {
    pub call: &'static str,
    pub code: u32,
}

struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_prints_key_or_button_codes() {
        let key = RawEvent::Key {
            vk: 0x41,
            scan: 0x1E,
            extended: false,
            up: false,
        };
        let text = format!(
            "{key:?} {:?} {:?} {:?}",
            WinKey::Scan {
                code: 0x1E,
                extended: true
            },
            WinKey::Vk(0x41),
            Synth::Key {
                key: WinKey::Vk(0x41),
                up: true
            }
        );
        assert!(!text.contains("65") && !text.contains("30") && !text.contains("0x"));
        assert!(text.contains("<redacted>"));
        let button = RawEvent::Button {
            button: WinButton::X2,
            down: true,
        };
        assert!(!format!("{button:?}").contains("X2"));
    }

    #[test]
    fn move_and_wheel_still_show_their_numbers() {
        assert!(format!("{:?}", RawEvent::Move { x: -7, y: 9 }).contains("-7"));
        assert!(format!("{:?}", RawEvent::Wheel { delta: 120, horizontal: false }).contains("120"));
    }
}
