//! Win32 calls behind a safe API (contract §13.3 and §13.5).
//!
//! The types compile on every system; the functions will live behind `cfg(windows)`. This crate
//! only calls the system and converts types: every decision (crossing, translation, coordinates,
//! shortcuts, what to block) belongs to `clipsync-input`.
//!
//! `Debug` of anything that names a key is redacted: no virtual-key, scan code or character ever
//! reaches a log (§13.2 rule 2e).

use std::fmt;

#[cfg(windows)]
mod ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// The keys that are safe to carry across a switch of machine (never a letter) and that the adapter
/// may ask the system about (Sentinel S7-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftWin,
    RightWin,
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
    Move {
        x: i32,
        y: i32,
    },
    Button {
        button: WinButton,
        down: bool,
    },
    /// In `WHEEL_DELTA` units (120).
    Wheel {
        delta: i32,
        horizontal: bool,
    },
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
    /// A callback panicked or the hooks stopped answering (the system removes a slow hook without
    /// saying so): the hook now passes everything and the session has to end. Sent once.
    HooksLost,
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

#[cfg(windows)]
mod api {
    use super::{Modifier, Monitor, Notice, RawEvent, Synth, Verdict, WinButton, WinError, ffi};

    /// The two low-level hooks, on a thread of their own with a hidden window (contract §13.5).
    /// Dropping it removes the hooks, closes the window and ends the thread.
    pub struct Hooks {
        _thread: ffi::ThreadHandle,
    }

    impl Hooks {
        /// `on_input` runs inside the hook callback: it must not wait for anything (§13.2 rule 2f).
        /// Injected events never reach it.
        pub fn start(
            on_input: Box<dyn FnMut(&RawEvent) -> Verdict + Send>,
            on_notice: Box<dyn FnMut(Notice) + Send>,
        ) -> Result<Hooks, WinError> {
            ffi::start_watcher(Some(on_input), on_notice).map(|_thread| Hooks { _thread })
        }
    }

    /// Like [`Hooks`] without any hook: only the notices (a machine that only receives).
    pub struct Watcher {
        _thread: ffi::ThreadHandle,
    }

    impl Watcher {
        pub fn start(on_notice: Box<dyn FnMut(Notice) + Send>) -> Result<Watcher, WinError> {
            ffi::start_watcher(None, on_notice).map(|_thread| Watcher { _thread })
        }
    }

    /// While it lives, the cursor is parked at `(x, y)` and hidden (a window of its own under the
    /// point, with an empty cursor).
    pub struct CursorPark {
        _thread: ffi::ThreadHandle,
    }

    impl CursorPark {
        pub fn new(x: i32, y: i32) -> Result<CursorPark, WinError> {
            ffi::start_park(x, y).map(|_thread| CursorPark { _thread })
        }
    }

    /// Win+Alt+P registered with the system while the module is paused (the hooks are gone then).
    /// The system only says that the combination was pressed; it never hands over a key.
    pub struct ResumeHotkey {
        _thread: ffi::ThreadHandle,
    }

    impl ResumeHotkey {
        /// `Err` means the system refused the combination: resuming is then only by the tray.
        pub fn register(on_press: Box<dyn FnMut() + Send>) -> Result<ResumeHotkey, WinError> {
            ffi::start_hotkey(ffi::RESUME_VK, Some(on_press))
                .map(|_thread| ResumeHotkey { _thread })
        }
    }

    /// Whether the system accepts Win+Alt+`vk` as a hotkey right now (registers it and lets it go).
    /// A probe for the CI and the diagnostic, not part of the module's behavior.
    pub fn hotkey_registrable(vk: u32) -> Result<(), WinError> {
        ffi::start_hotkey(vk, None).map(drop)
    }

    pub fn monitors() -> Result<Vec<Monitor>, WinError> {
        ffi::monitors()
    }

    /// `(x, y, width, height)` of the virtual desktop, in physical px.
    pub fn virtual_screen() -> (i32, i32, i32, i32) {
        ffi::virtual_screen()
    }

    pub fn cursor_pos() -> Result<(i32, i32), WinError> {
        ffi::cursor_pos()
    }

    /// `SendInput`. Returns how many inputs the system took.
    pub fn send(inputs: &[Synth]) -> Result<usize, WinError> {
        ffi::send(inputs)
    }

    /// Whether the desktop that receives input is the user's (`Default`). Any failure (the secure
    /// desktop refuses the call) is `false`.
    pub fn input_desktop_is_default() -> bool {
        ffi::input_desktop_is_default()
    }

    /// The KLID of the current keyboard layout (`00010416` is Português (Brasil) ABNT2), for the
    /// diagnostic only.
    pub fn keyboard_layout() -> Option<String> {
        ffi::keyboard_layout()
    }

    /// Whether this modifier is physically down right now (`GetAsyncKeyState`), to reconcile on
    /// activation. Closed on modifiers and mouse buttons: no other key can be read this way.
    pub fn modifier_down(modifier: Modifier) -> bool {
        ffi::modifier_down(modifier)
    }

    /// Whether this mouse button is physically down right now.
    pub fn button_down(button: WinButton) -> bool {
        ffi::button_down(button)
    }

    /// Handles open in this process, for the test that proves nothing leaks.
    pub fn process_handle_count() -> Result<u32, WinError> {
        ffi::process_handle_count()
    }
}

#[cfg(windows)]
pub use api::{
    CursorPark, Hooks, ResumeHotkey, Watcher, button_down, cursor_pos, hotkey_registrable,
    input_desktop_is_default, keyboard_layout, modifier_down, monitors, process_handle_count, send,
    virtual_screen,
};

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
        assert!(
            format!(
                "{:?}",
                RawEvent::Wheel {
                    delta: 120,
                    horizontal: false
                }
            )
            .contains("120")
        );
    }
}
