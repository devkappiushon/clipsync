//! Keys and buttons between Windows and evdev (contract §13.7). Pure: it compiles and tests on
//! every system.
//!
//! The wire carries the physical position of the key (evdev code, §4); whoever receives applies its
//! own layout. So the main block is a scan code (set 1) and nothing else: ç, dead keys and AltGr
//! come out of the destination's layout. Codes are never logged (§13.2 rule 2e).
use clipsync_win::{WinButton, WinKey};

const VK_PAUSE: u16 = 0x13;
const VK_SNAPSHOT: u16 = 0x2C;
const VK_NUMLOCK: u16 = 0x90;

const KEY_NUMLOCK: u32 = 69;
const KEY_SYSRQ: u32 = 99;
const KEY_PAUSE: u32 = 119;
const KEY_RO: u32 = 89;
const KEY_KPCOMMA: u32 = 121;

const SCAN_RO: u8 = 0x73;
const SCAN_KPCOMMA: u8 = 0x7E;

/// Scan codes above one byte are not keys: the 0x200 bit marks the fake LCtrl that Windows sends
/// with AltGr (0x21D) and the fake Shifts around the arrows (0x22A, 0x236).
const SCAN_ONE_BYTE: u32 = 0xFF;

/// Main block, set 1 without E0: the scan code is the evdev code, from 0x01 `KEY_ESC` to 0x58
/// `KEY_F12`. 0x54 (Alt+PrintScreen, comes by VK) and 0x55 (no key on these keyboards) have no
/// evdev meaning, so they are out; 0x56 is the `KEY_102ND` of the ABNT2 and the ISO layouts.
fn main_block(scan: u8) -> bool {
    matches!(scan, 0x01..=0x53 | 0x56..=0x58)
}

/// With E0: `(scan code, evdev code)`. `KEY_SYSRQ` is here for the way in; going out it is a VK.
const EXTENDED: [(u8, u32); 18] = [
    (0x1C, 96),  // KEY_KPENTER
    (0x1D, 97),  // KEY_RIGHTCTRL
    (0x35, 98),  // KEY_KPSLASH
    (0x37, 99),  // KEY_SYSRQ
    (0x38, 100), // KEY_RIGHTALT (AltGr)
    (0x47, 102), // KEY_HOME
    (0x48, 103), // KEY_UP
    (0x49, 104), // KEY_PAGEUP
    (0x4B, 105), // KEY_LEFT
    (0x4D, 106), // KEY_RIGHT
    (0x4F, 107), // KEY_END
    (0x50, 108), // KEY_DOWN
    (0x51, 109), // KEY_PAGEDOWN
    (0x52, 110), // KEY_INSERT
    (0x53, 111), // KEY_DELETE
    (0x5B, 125), // KEY_LEFTMETA
    (0x5C, 126), // KEY_RIGHTMETA
    (0x5D, 127), // KEY_COMPOSE
];

/// What the hook saw to an evdev `KEY_*` code. `None` means drop the event: a fake modifier or a
/// key with no counterpart.
pub fn from_windows(vk: u32, scan: u32, extended: bool) -> Option<u32> {
    // Pause and NumLock share the scan code 0x45 (Pause comes without E0, NumLock sometimes with),
    // and Alt+PrintScreen comes as 0x54: these three are told apart by the VK.
    match u16::try_from(vk) {
        Ok(VK_PAUSE) => return Some(KEY_PAUSE),
        Ok(VK_NUMLOCK) => return Some(KEY_NUMLOCK),
        Ok(VK_SNAPSHOT) => return Some(KEY_SYSRQ),
        _ => {}
    }
    if scan > SCAN_ONE_BYTE {
        return None;
    }
    let scan = scan as u8;
    if extended {
        return EXTENDED
            .iter()
            .find(|(s, _)| *s == scan)
            .map(|(_, code)| *code);
    }
    match scan {
        SCAN_RO => Some(KEY_RO),
        SCAN_KPCOMMA => Some(KEY_KPCOMMA),
        s if main_block(s) => Some(u32::from(s)),
        _ => None,
    }
}

/// An evdev `KEY_*` code to what `SendInput` takes. `None` means no key on Windows.
pub fn to_windows(code: u32) -> Option<WinKey> {
    let scan = |code: u8, extended: bool| Some(WinKey::Scan { code, extended });
    match code {
        KEY_PAUSE => Some(WinKey::Vk(VK_PAUSE)),
        KEY_NUMLOCK => Some(WinKey::Vk(VK_NUMLOCK)),
        KEY_SYSRQ => Some(WinKey::Vk(VK_SNAPSHOT)),
        KEY_RO => scan(SCAN_RO, false),
        KEY_KPCOMMA => scan(SCAN_KPCOMMA, false),
        _ => {
            if let Ok(byte) = u8::try_from(code)
                && main_block(byte)
            {
                return scan(byte, false);
            }
            EXTENDED
                .iter()
                .find(|(_, evdev)| *evdev == code)
                .and_then(|(s, _)| scan(*s, true))
        }
    }
}

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;

pub fn button_from_windows(button: WinButton) -> u32 {
    match button {
        WinButton::Left => BTN_LEFT,
        WinButton::Right => BTN_RIGHT,
        WinButton::Middle => BTN_MIDDLE,
        WinButton::X1 => BTN_SIDE,
        WinButton::X2 => BTN_EXTRA,
    }
}

pub fn button_to_windows(code: u32) -> Option<WinButton> {
    match code {
        BTN_LEFT => Some(WinButton::Left),
        BTN_RIGHT => Some(WinButton::Right),
        BTN_MIDDLE => Some(WinButton::Middle),
        BTN_SIDE => Some(WinButton::X1),
        BTN_EXTRA => Some(WinButton::X2),
        _ => None,
    }
}
