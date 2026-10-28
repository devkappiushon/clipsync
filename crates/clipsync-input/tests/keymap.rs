//! §13.11 item 3 (Flux): keymap cases of §13.7 item 2. Key codes are asserted here, never logged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::keymap::{button_from_windows, button_to_windows, from_windows, to_windows};
use clipsync_win::{WinButton, WinKey};

// evdev codes, from linux/input-event-codes.h.
const KEY_ESC: u32 = 1;
const KEY_LEFTBRACE: u32 = 26;
const KEY_ENTER: u32 = 28;
const KEY_LEFTCTRL: u32 = 29;
const KEY_SEMICOLON: u32 = 39;
const KEY_APOSTROPHE: u32 = 40;
const KEY_GRAVE: u32 = 41;
const KEY_LEFTSHIFT: u32 = 42;
const KEY_BACKSLASH: u32 = 43;
const KEY_SLASH: u32 = 53;
const KEY_LEFTALT: u32 = 56;
const KEY_NUMLOCK: u32 = 69;
const KEY_KP0: u32 = 82;
const KEY_KPDOT: u32 = 83;
const KEY_102ND: u32 = 86;
const KEY_F12: u32 = 88;
const KEY_RO: u32 = 89;
const KEY_KPENTER: u32 = 96;
const KEY_RIGHTCTRL: u32 = 97;
const KEY_KPSLASH: u32 = 98;
const KEY_SYSRQ: u32 = 99;
const KEY_RIGHTALT: u32 = 100;
const KEY_PAUSE: u32 = 119;
const KEY_KPCOMMA: u32 = 121;
const KEY_LEFTMETA: u32 = 125;

const VK_PAUSE: u32 = 0x13;
const VK_SNAPSHOT: u32 = 0x2C;
const VK_NUMLOCK: u32 = 0x90;

/// `(scan code, extended, evdev code)` of every physical key of a US 104 keyboard and of the extra
/// keys of the ABNT2 (set 1). NumLock and Pause are by VK and have their own test.
const PHYSICAL: &[(u32, bool, u32)] = &[
    // main block: scan code == evdev code
    (0x01, false, 1),
    (0x02, false, 2),
    (0x03, false, 3),
    (0x04, false, 4),
    (0x05, false, 5),
    (0x06, false, 6),
    (0x07, false, 7),
    (0x08, false, 8),
    (0x09, false, 9),
    (0x0A, false, 10),
    (0x0B, false, 11),
    (0x0C, false, 12),
    (0x0D, false, 13),
    (0x0E, false, 14),
    (0x0F, false, 15),
    (0x10, false, 16),
    (0x11, false, 17),
    (0x12, false, 18),
    (0x13, false, 19),
    (0x14, false, 20),
    (0x15, false, 21),
    (0x16, false, 22),
    (0x17, false, 23),
    (0x18, false, 24),
    (0x19, false, 25),
    (0x1A, false, 26),
    (0x1B, false, 27),
    (0x1C, false, 28),
    (0x1D, false, 29),
    (0x1E, false, 30),
    (0x1F, false, 31),
    (0x20, false, 32),
    (0x21, false, 33),
    (0x22, false, 34),
    (0x23, false, 35),
    (0x24, false, 36),
    (0x25, false, 37),
    (0x26, false, 38),
    (0x27, false, 39),
    (0x28, false, 40),
    (0x29, false, 41),
    (0x2A, false, 42),
    (0x2B, false, 43),
    (0x2C, false, 44),
    (0x2D, false, 45),
    (0x2E, false, 46),
    (0x2F, false, 47),
    (0x30, false, 48),
    (0x31, false, 49),
    (0x32, false, 50),
    (0x33, false, 51),
    (0x34, false, 52),
    (0x35, false, 53),
    (0x36, false, 54),
    (0x37, false, 55),
    (0x38, false, 56),
    (0x39, false, 57),
    (0x3A, false, 58),
    (0x3B, false, 59),
    (0x3C, false, 60),
    (0x3D, false, 61),
    (0x3E, false, 62),
    (0x3F, false, 63),
    (0x40, false, 64),
    (0x41, false, 65),
    (0x42, false, 66),
    (0x43, false, 67),
    (0x44, false, 68),
    (0x46, false, 70),
    (0x47, false, 71),
    (0x48, false, 72),
    (0x49, false, 73),
    (0x4A, false, 74),
    (0x4B, false, 75),
    (0x4C, false, 76),
    (0x4D, false, 77),
    (0x4E, false, 78),
    (0x4F, false, 79),
    (0x50, false, 80),
    (0x51, false, 81),
    (0x52, false, 82),
    (0x53, false, 83),
    (0x57, false, 87),
    (0x58, false, 88),
    // ISO / ABNT2 extras
    (0x56, false, 86),
    (0x73, false, 89),
    (0x7E, false, 121),
    // with E0
    (0x1C, true, 96),
    (0x1D, true, 97),
    (0x35, true, 98),
    (0x37, true, 99),
    (0x38, true, 100),
    (0x47, true, 102),
    (0x48, true, 103),
    (0x49, true, 104),
    (0x4B, true, 105),
    (0x4D, true, 106),
    (0x4F, true, 107),
    (0x50, true, 108),
    (0x51, true, 109),
    (0x52, true, 110),
    (0x53, true, 111),
    (0x5B, true, 125),
    (0x5C, true, 126),
    (0x5D, true, 127),
];

/// What `from_windows` would get back for a key that `to_windows` produced.
fn back(key: WinKey) -> Option<u32> {
    match key {
        WinKey::Scan { code, extended } => from_windows(0, u32::from(code), extended),
        WinKey::Vk(vk) => from_windows(u32::from(vk), 0, false),
    }
}

#[test]
fn every_physical_key_goes_and_comes_back() {
    for &(scan, extended, evdev) in PHYSICAL {
        assert_eq!(
            from_windows(0, scan, extended),
            Some(evdev),
            "from scan {scan:#x} ext {extended}"
        );
        let key = to_windows(evdev).unwrap_or_else(|| panic!("no key for evdev {evdev}"));
        assert_eq!(back(key), Some(evdev), "round trip of evdev {evdev}");
    }
}

#[test]
fn the_vk_does_not_change_a_scan_code_key() {
    for vk in [0, 0x41, 0xFF, 0x1B] {
        assert_eq!(from_windows(vk, 0x1E, false), Some(30));
    }
}

#[test]
fn cedilla_accents_and_quotes_follow_the_physical_position() {
    // ABNT2: ç, ´ and `, ~ and ^, " and '. The destination's layout turns the position into text.
    assert_eq!(from_windows(0xC0, 0x27, false), Some(KEY_SEMICOLON));
    assert_eq!(from_windows(0xDB, 0x1A, false), Some(KEY_LEFTBRACE));
    assert_eq!(from_windows(0xDE, 0x28, false), Some(KEY_APOSTROPHE));
    assert_eq!(from_windows(0xDC, 0x29, false), Some(KEY_GRAVE));
    // US: the same positions are ; [ ' `
    assert_eq!(
        to_windows(KEY_SEMICOLON),
        Some(WinKey::Scan {
            code: 0x27,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_LEFTBRACE),
        Some(WinKey::Scan {
            code: 0x1A,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_APOSTROPHE),
        Some(WinKey::Scan {
            code: 0x28,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_GRAVE),
        Some(WinKey::Scan {
            code: 0x29,
            extended: false
        })
    );
    assert_eq!(from_windows(0xDC, 0x2B, false), Some(KEY_BACKSLASH));
}

#[test]
fn abnt2_extra_keys() {
    assert_eq!(from_windows(0xC1, 0x73, false), Some(KEY_RO));
    assert_eq!(from_windows(0xC2, 0x7E, false), Some(KEY_KPCOMMA));
    assert_eq!(from_windows(0xE2, 0x56, false), Some(KEY_102ND));
    assert_eq!(
        to_windows(KEY_RO),
        Some(WinKey::Scan {
            code: 0x73,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_KPCOMMA),
        Some(WinKey::Scan {
            code: 0x7E,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_102ND),
        Some(WinKey::Scan {
            code: 0x56,
            extended: false
        })
    );
    assert_eq!(from_windows(0xBF, 0x35, false), Some(KEY_SLASH));
}

#[test]
fn altgr_is_right_alt_and_the_fake_left_ctrl_is_dropped() {
    assert_eq!(from_windows(0xA5, 0x38, true), Some(KEY_RIGHTALT));
    // Windows sends a LCtrl with scan code 0x21D just before AltGr.
    assert_eq!(from_windows(0xA2, 0x21D, false), None);
    assert_eq!(from_windows(0xA2, 0x1D, false), Some(KEY_LEFTCTRL));
    // fake Shifts around the arrows and the numeric pad
    assert_eq!(from_windows(0xA0, 0x22A, true), None);
    assert_eq!(from_windows(0xA1, 0x236, true), None);
    assert_eq!(
        to_windows(KEY_RIGHTALT),
        Some(WinKey::Scan {
            code: 0x38,
            extended: true
        })
    );
    assert_eq!(from_windows(0xA4, 0x38, false), Some(KEY_LEFTALT));
}

#[test]
fn pause_numlock_and_print_screen_go_by_vk() {
    // Pause comes with scan code 0x45 and no E0, NumLock with 0x45 and sometimes E0.
    assert_eq!(from_windows(VK_PAUSE, 0x45, false), Some(KEY_PAUSE));
    assert_eq!(from_windows(VK_NUMLOCK, 0x45, true), Some(KEY_NUMLOCK));
    assert_eq!(from_windows(VK_NUMLOCK, 0x45, false), Some(KEY_NUMLOCK));
    assert_eq!(to_windows(KEY_PAUSE), Some(WinKey::Vk(0x13)));
    assert_eq!(to_windows(KEY_NUMLOCK), Some(WinKey::Vk(0x90)));
    assert_eq!(back(WinKey::Vk(0x13)), Some(KEY_PAUSE));
    assert_eq!(back(WinKey::Vk(0x90)), Some(KEY_NUMLOCK));
    // PrintScreen goes out as VK_SNAPSHOT and comes in by E0 37 or by the VK (Alt+PrintScreen is 0x54).
    assert_eq!(to_windows(KEY_SYSRQ), Some(WinKey::Vk(0x2C)));
    assert_eq!(from_windows(VK_SNAPSHOT, 0x37, true), Some(KEY_SYSRQ));
    assert_eq!(from_windows(VK_SNAPSHOT, 0x54, false), Some(KEY_SYSRQ));
    assert_eq!(from_windows(0, 0x37, true), Some(KEY_SYSRQ));
    // without the VK, 0x37 is the numeric pad asterisk
    assert_eq!(from_windows(0x6A, 0x37, false), Some(55));
}

#[test]
fn numeric_pad_and_its_extended_twins_stay_apart() {
    assert_eq!(from_windows(0, 0x1C, false), Some(KEY_ENTER));
    assert_eq!(from_windows(0, 0x1C, true), Some(KEY_KPENTER));
    assert_eq!(from_windows(0, 0x1D, true), Some(KEY_RIGHTCTRL));
    assert_eq!(from_windows(0, 0x35, true), Some(KEY_KPSLASH));
    assert_eq!(from_windows(0, 0x52, false), Some(KEY_KP0));
    assert_eq!(from_windows(0, 0x53, false), Some(KEY_KPDOT));
    assert_eq!(from_windows(0, 0x5B, true), Some(KEY_LEFTMETA));
}

#[test]
fn keys_without_a_counterpart_are_dropped() {
    // media keys, browser keys and the like come with E0 and a scan code that is not on the list
    for scan in [0x20, 0x2E, 0x30, 0x32, 0x65, 0x66, 0x6C, 0x6D] {
        assert_eq!(from_windows(0, scan, true), None, "scan {scan:#x} ext");
    }
    // no scan code at all, or beyond one byte
    assert_eq!(from_windows(0x41, 0, false), None);
    assert_eq!(from_windows(0x41, 0x100, false), None);
    // 0x54 and 0x55 have no evdev meaning
    assert_eq!(from_windows(0, 0x54, false), None);
    assert_eq!(from_windows(0, 0x55, false), None);
    // evdev codes with no Windows key: KEY_HENKAN, KEY_MICMUTE, 0 and beyond
    for code in [0, 84, 85, 92, 101, 248, 0x110, 0x1000, u32::MAX] {
        assert_eq!(to_windows(code), None, "evdev {code}");
    }
}

#[test]
fn the_edges_of_the_main_block() {
    assert_eq!(from_windows(0, 0x01, false), Some(KEY_ESC));
    assert_eq!(from_windows(0, 0x58, false), Some(KEY_F12));
    assert_eq!(from_windows(0, 0x59, false), None);
    assert_eq!(
        to_windows(KEY_F12),
        Some(WinKey::Scan {
            code: 0x58,
            extended: false
        })
    );
    assert_eq!(
        to_windows(KEY_LEFTSHIFT),
        Some(WinKey::Scan {
            code: 0x2A,
            extended: false
        })
    );
}

#[test]
fn mouse_buttons_go_and_come_back() {
    let all = [
        (WinButton::Left, 0x110),
        (WinButton::Right, 0x111),
        (WinButton::Middle, 0x112),
        (WinButton::X1, 0x113),
        (WinButton::X2, 0x114),
    ];
    for (button, code) in all {
        assert_eq!(button_from_windows(button), code);
        assert_eq!(button_to_windows(code), Some(button));
    }
    for code in [0, 0x10F, 0x115, 0x116, u32::MAX] {
        assert_eq!(button_to_windows(code), None);
    }
}
