//! §13.11 item 3 (Flux): the pure decisions of the Windows adapter, on every system.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::adapter::winmath::{
    HotkeyTracker, KeyDecision, NEUTRAL_KEY, Rect, barrier_crossing, capture_delta, clamp_to_union,
    denormalize_abs, emulate_motion, logical_to_physical, normalize_abs, physical_to_logical,
    scale_of, zone_names, zones,
};
use clipsync_input::adapter::{Hotkey, Zone};
use clipsync_proto::{Edge, validate_monitor_name};
use clipsync_win::{Monitor, WinKey};

fn monitor(
    friendly: &str,
    device: &str,
    path: &str,
    rect: (i32, i32, u32, u32),
    dpi: u32,
) -> Monitor {
    let (x, y, w, h) = rect;
    Monitor {
        friendly: friendly.into(),
        device: device.into(),
        path: path.into(),
        x,
        y,
        width: w,
        height: h,
        dpi,
        primary: x == 0 && y == 0,
    }
}

// ----- names

#[test]
fn name_is_the_friendly_name_then_the_device_without_the_prefix() {
    let ms = [
        monitor("DELL U2720Q", r"\\.\DISPLAY1", "p1", (0, 0, 2560, 1440), 96),
        monitor("", r"\\.\DISPLAY2", "p2", (2560, 0, 1920, 1080), 96),
        monitor("  ", "", "p3", (4480, 0, 1920, 1080), 96),
    ];
    assert_eq!(zone_names(&ms), ["DELL U2720Q", "DISPLAY2", "Monitor"]);
}

#[test]
fn repeated_names_get_a_number_in_path_order_whatever_the_enumeration_order() {
    // enumeration order is b, a, c; path order is a, b, c
    let ms = [
        monitor("DELL", "", r"\\?\DISPLAY#B", (0, 0, 1920, 1080), 96),
        monitor("DELL", "", r"\\?\DISPLAY#A", (1920, 0, 1920, 1080), 96),
        monitor("DELL", "", r"\\?\DISPLAY#C", (3840, 0, 1920, 1080), 96),
    ];
    assert_eq!(zone_names(&ms), ["DELL 2", "DELL", "DELL 3"]);
    // stable: the same monitors in another enumeration order keep their names
    let swapped = [ms[1].clone(), ms[2].clone(), ms[0].clone()];
    assert_eq!(zone_names(&swapped), ["DELL", "DELL 3", "DELL 2"]);
}

#[test]
fn a_new_monitor_with_another_name_does_not_rename_the_others() {
    let a = monitor("DELL", "", "p1", (0, 0, 1920, 1080), 96);
    let b = monitor("LG", "", "p2", (1920, 0, 1920, 1080), 96);
    let before = zone_names(std::slice::from_ref(&a));
    let after = zone_names(&[a, b]);
    assert_eq!(before[0], after[0]);
}

#[test]
fn long_names_are_cut_on_a_character_boundary_at_64_bytes() {
    let long = "Monitor ultrawide de escritório com acentuação ção ção ção ção ção ção";
    assert!(long.len() > 64);
    let name = &zone_names(&[monitor(long, "", "p", (0, 0, 1, 1), 96)])[0];
    assert!(name.len() <= 64, "{} bytes", name.len());
    assert!(long.starts_with(name.as_str()));
    // 63 ASCII bytes and a 2-byte character: the cut has to drop the whole character
    let tricky = format!("{}ç", "a".repeat(63));
    let name = &zone_names(&[monitor(&tricky, "", "p", (0, 0, 1, 1), 96)])[0];
    assert_eq!(name, &"a".repeat(63));
}

#[test]
fn the_number_still_fits_when_the_name_is_cut() {
    let long = "x".repeat(80);
    let ms = [
        monitor(&long, "", "p1", (0, 0, 1, 1), 96),
        monitor(&long, "", "p2", (1, 0, 1, 1), 96),
    ];
    let names = zone_names(&ms);
    assert_eq!(names[0], "x".repeat(64));
    assert_eq!(names[1], format!("{} 2", "x".repeat(62)));
    assert!(names.iter().all(|n| n.len() <= 64));
}

#[test]
fn every_name_passes_the_wire_validation_and_none_repeats() {
    let ms = [
        monitor("Dell\u{200B}\u{202E}Hidden\n", "", "p1", (0, 0, 1, 1), 96),
        monitor("Dell\u{200B}\u{202E}Hidden\n", "", "p2", (1, 0, 1, 1), 96),
        monitor("DellHidden 2", "", "p0", (2, 0, 1, 1), 96),
        monitor("\u{0007}\u{200B}", r"\\.\DISPLAY9", "p4", (3, 0, 1, 1), 96),
    ];
    let names = zone_names(&ms);
    for n in &names {
        validate_monitor_name(n).unwrap_or_else(|e| panic!("{n:?}: {e}"));
    }
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "{names:?}");
    assert_eq!(names[3], "DISPLAY9");
}

#[test]
fn zones_follow_the_monitors_with_the_scale_from_the_dpi() {
    let ms = [
        monitor("Principal", "", "p1", (0, 0, 1920, 1080), 144),
        monitor("Esquerda", "", "p2", (-2560, -200, 2560, 1440), 96),
    ];
    let z = zones(&ms);
    assert_eq!(z.len(), 2);
    assert_eq!(
        (
            z[0].index,
            z[0].name.as_deref(),
            z[0].x,
            z[0].width,
            z[0].scale
        ),
        (0, Some("Principal"), 0, 1920, 1.5)
    );
    assert_eq!(
        (z[1].index, z[1].x, z[1].y, z[1].height, z[1].scale),
        (1, -2560, -200, 1440, 1.0)
    );
    assert_eq!(scale_of(0), 1.0);
}

// ----- coordinates

fn desktop() -> Rect {
    // two monitors side by side, the left one at a negative origin and higher than the other
    Rect {
        x: -1920,
        y: -200,
        width: 3840,
        height: 1280,
    }
}

#[test]
fn move_abs_at_the_four_corners_of_a_desktop_with_a_negative_origin() {
    let d = desktop();
    assert_eq!(normalize_abs(d, -1920, -200), (0, 0));
    for (x, y) in [(-1920, -200), (1919, -200), (-1920, 1079), (1919, 1079)] {
        let (nx, ny) = normalize_abs(d, x, y);
        assert!((0..=65535).contains(&nx) && (0..=65535).contains(&ny));
        assert_eq!(denormalize_abs(d, nx, ny), (x, y), "corner {x},{y}");
    }
    // the last pixel is near the end of the range, not at 0
    assert!(normalize_abs(d, 1919, 1079).0 > 65000);
    // outside the desktop is held at the nearest pixel
    assert_eq!(normalize_abs(d, -5000, 9000), normalize_abs(d, -1920, 1079));
}

#[test]
fn move_abs_lands_on_every_pixel_it_was_asked_for() {
    for (origin, len) in [(0, 1920), (-1920, 3840), (-1280, 5120), (0, 1366), (100, 7)] {
        let r = Rect {
            x: origin,
            y: 0,
            width: len,
            height: 1,
        };
        for x in origin..origin + len as i32 {
            let (nx, _) = normalize_abs(r, x, 0);
            assert_eq!(denormalize_abs(r, nx, 0).0, x, "width {len} pixel {x}");
        }
    }
}

#[test]
fn physical_and_logical_px_at_96_120_144_and_192_dpi() {
    for (dpi, logical) in [(96, 100.0), (120, 80.0), (144, 100.0 / 1.5), (192, 50.0)] {
        assert!(
            (physical_to_logical(100.0, dpi) - logical).abs() < 1e-4,
            "dpi {dpi}"
        );
        assert!((logical_to_physical(physical_to_logical(100.0, dpi), dpi) - 100.0).abs() < 1e-3);
    }
    assert_eq!(physical_to_logical(50.0, 0), 50.0);
    // capture: parked at (960, 540); the hook says (990, 510); the monitor is at 150%
    assert_eq!(capture_delta((990, 510), (960, 540), 144), (20.0, -20.0));
    assert_eq!(capture_delta((960, 540), (960, 540), 192), (0.0, 0.0));
}

#[test]
fn the_cursor_is_held_inside_the_union_of_the_monitors() {
    // an L: 1920x1080 at the origin and 1280x1024 below it, to the right
    let l = [
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        Rect {
            x: 1920,
            y: 500,
            width: 1280,
            height: 1024,
        },
    ];
    assert_eq!(clamp_to_union((100, 100), &l), (100, 100));
    assert_eq!(clamp_to_union((2000, 200), &l), (1919, 200)); // void above the second monitor
    assert_eq!(clamp_to_union((-50, -50), &l), (0, 0));
    assert_eq!(clamp_to_union((9000, 9000), &l), (3199, 1523));
    assert_eq!(clamp_to_union((5, 5), &[]), (5, 5));
}

#[test]
fn emulated_motion_scales_accumulates_the_rest_and_stops_at_the_wall() {
    let one = [Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    }];
    // 150%: 10 logical px are 15 physical
    assert_eq!(
        emulate_motion((100, 100), (0.0, 0.0), (10.0, -10.0), 1.5, &one),
        ((115, 85), (0.0, 0.0))
    );
    // 0.4 px a time: the rest is paid when it adds up to a pixel
    let (mut at, mut rest) = ((100, 100), (0.0, 0.0));
    for _ in 0..5 {
        (at, rest) = emulate_motion(at, rest, (0.4, 0.0), 1.0, &one);
    }
    assert_eq!(at, (102, 100));
    // against the wall, nothing is stored for later
    let (at, rest) = emulate_motion((1919, 500), (0.0, 0.0), (7.3, 0.0), 1.0, &one);
    assert_eq!((at, rest.0), ((1919, 500), 0.0));
}

// ----- barrier crossing

fn zone(x: i32, y: i32, w: u32, h: u32) -> Zone {
    Zone {
        index: 0,
        name: Some("z".into()),
        x,
        y,
        width: w,
        height: h,
        scale: 1.0,
    }
}

#[test]
fn crossing_on_the_last_pixel_of_the_segment() {
    let z = zone(0, 0, 1920, 1080);
    // right edge, the whole edge: last pixel is x = 1919
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.0, 1.0, (1900, 540), (1919, 540)),
        Some(0.5)
    );
    // staying there (the hook clamps the point at the desktop's edge) is a push
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.0, 1.0, (1919, 540), (1919, 540)),
        Some(0.5)
    );
    // coming back inward is not
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.0, 1.0, (1919, 540), (1900, 540)),
        None
    );
    // one pixel short
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.0, 1.0, (1800, 540), (1918, 540)),
        None
    );
    // an unclamped point beyond the line, coming from inside
    assert!(barrier_crossing(&z, Edge::Right, 0.0, 1.0, (1915, 540), (1925, 540)).is_some());
}

#[test]
fn crossing_is_limited_to_the_linked_stretch() {
    let z = zone(0, 0, 1920, 1080);
    // right edge, stretch 0.25..0.75 = y 270..=809
    assert!(barrier_crossing(&z, Edge::Right, 0.25, 0.75, (1900, 300), (1919, 300)).is_some());
    assert!(barrier_crossing(&z, Edge::Right, 0.25, 0.75, (1900, 809), (1919, 809)).is_some());
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.25, 0.75, (1900, 100), (1919, 100)),
        None
    );
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.25, 0.75, (1900, 810), (1919, 810)),
        None
    );
    // the corner pixel is outside a stretch that stops short of it
    assert_eq!(
        barrier_crossing(&z, Edge::Right, 0.0, 0.5, (1900, 1079), (1919, 1079)),
        None
    );
}

#[test]
fn crossing_on_each_of_the_four_edges_of_a_monitor_at_a_negative_origin() {
    let z = zone(-1920, -200, 1920, 1080);
    // left: last pixel is x = -1920
    assert!(barrier_crossing(&z, Edge::Left, 0.0, 1.0, (-1900, 100), (-1920, 100)).is_some());
    assert!(barrier_crossing(&z, Edge::Left, 0.0, 1.0, (-1900, 100), (-1919, 100)).is_none());
    // top: last pixel is y = -200
    assert!(barrier_crossing(&z, Edge::Top, 0.0, 1.0, (-1000, -150), (-1000, -200)).is_some());
    // bottom: last pixel is y = 879
    assert!(barrier_crossing(&z, Edge::Bottom, 0.0, 1.0, (-1000, 800), (-1000, 879)).is_some());
    assert!(barrier_crossing(&z, Edge::Bottom, 0.0, 1.0, (-1000, 800), (-1000, 878)).is_none());
    // right: last pixel is x = -1
    assert!(barrier_crossing(&z, Edge::Right, 0.0, 1.0, (-100, 0), (-1, 0)).is_some());
}

#[test]
fn crossing_the_line_between_two_local_monitors() {
    // monitor 1 at 0..1920, monitor 2 at 1920..3840; the link is on the right edge of monitor 1
    let z1 = zone(0, 0, 1920, 1080);
    // the cursor crosses from monitor 1 to monitor 2 in one move
    assert!(barrier_crossing(&z1, Edge::Right, 0.0, 1.0, (1910, 500), (1935, 500)).is_some());
    // already on monitor 2: nothing to activate
    assert_eq!(
        barrier_crossing(&z1, Edge::Right, 0.0, 1.0, (1930, 500), (1940, 500)),
        None
    );
    // monitor 2's own left edge, going left from monitor 2 into monitor 1
    let z2 = zone(1920, 0, 1920, 1080);
    assert!(barrier_crossing(&z2, Edge::Left, 0.0, 1.0, (1930, 500), (1915, 500)).is_some());
    assert_eq!(
        barrier_crossing(&z2, Edge::Left, 0.0, 1.0, (1915, 500), (1900, 500)),
        None
    );
}

#[test]
fn the_position_is_along_the_whole_edge_even_for_a_stretch() {
    let z = zone(0, 0, 1920, 1080);
    let pos = barrier_crossing(&z, Edge::Right, 0.25, 0.75, (1900, 540), (1919, 540)).unwrap();
    assert!((pos - 0.5).abs() < 1e-6);
    let pos = barrier_crossing(&z, Edge::Bottom, 0.0, 1.0, (480, 1000), (480, 1079)).unwrap();
    assert!((pos - 0.25).abs() < 1e-6);
}

// ----- shortcuts

const KEY_1: u32 = 2;
const KEY_9: u32 = 10;
const KEY_P: u32 = 25;
const KEY_L: u32 = 38;
const KEY_A: u32 = 30;
const KEY_LEFTALT: u32 = 56;
const KEY_RIGHTALT: u32 = 100;
const KEY_LEFTMETA: u32 = 125;
const KEY_RIGHTMETA: u32 = 126;

fn press(t: &mut HotkeyTracker, code: u32) -> KeyDecision {
    t.on_key(code, true)
}
fn release(t: &mut HotkeyTracker, code: u32) -> KeyDecision {
    t.on_key(code, false)
}

#[test]
fn win_alt_digit_l_and_p_are_recognized_and_swallowed() {
    for (code, expected) in [
        (KEY_1, Hotkey::GoTo(1)),
        (KEY_1 + 2, Hotkey::GoTo(3)),
        (KEY_9, Hotkey::GoTo(9)),
        (KEY_L, Hotkey::ToggleLock),
        (KEY_P, Hotkey::TogglePause),
    ] {
        let mut t = HotkeyTracker::new();
        assert_eq!(press(&mut t, KEY_LEFTMETA), KeyDecision::default());
        assert_eq!(press(&mut t, KEY_LEFTALT), KeyDecision::default());
        let d = press(&mut t, code);
        assert_eq!((d.swallow, d.shortcut), (true, Some(expected)));
        // its release goes nowhere either
        let d = release(&mut t, code);
        assert_eq!((d.swallow, d.shortcut), (true, None));
    }
}

#[test]
fn the_order_of_win_and_alt_and_the_right_hand_keys_do_not_matter() {
    let mut t = HotkeyTracker::new();
    press(&mut t, KEY_RIGHTALT);
    press(&mut t, KEY_RIGHTMETA);
    assert_eq!(press(&mut t, KEY_P).shortcut, Some(Hotkey::TogglePause));
}

#[test]
fn without_both_modifiers_nothing_is_recognized_and_nothing_is_eaten() {
    let mut t = HotkeyTracker::new();
    assert_eq!(press(&mut t, KEY_1), KeyDecision::default());
    release(&mut t, KEY_1);
    press(&mut t, KEY_LEFTMETA);
    assert_eq!(press(&mut t, KEY_L), KeyDecision::default());
    release(&mut t, KEY_L);
    release(&mut t, KEY_LEFTMETA);
    press(&mut t, KEY_LEFTALT);
    assert_eq!(press(&mut t, KEY_P), KeyDecision::default());
    // Win+Alt with a key that is not a shortcut
    press(&mut t, KEY_LEFTMETA);
    assert_eq!(press(&mut t, KEY_A), KeyDecision::default());
}

#[test]
fn auto_repeat_fires_once_and_a_late_release_is_still_swallowed() {
    let mut t = HotkeyTracker::new();
    press(&mut t, KEY_LEFTMETA);
    press(&mut t, KEY_LEFTALT);
    assert!(press(&mut t, KEY_L).shortcut.is_some());
    for _ in 0..3 {
        let d = press(&mut t, KEY_L);
        assert_eq!((d.swallow, d.shortcut), (true, None));
    }
    // Win goes up before the key
    release(&mut t, KEY_LEFTMETA);
    release(&mut t, KEY_LEFTALT);
    let d = release(&mut t, KEY_L);
    assert_eq!((d.swallow, d.shortcut), (true, None));
    // and after that, L is a normal key again
    assert_eq!(press(&mut t, KEY_L), KeyDecision::default());
}

#[test]
fn a_neutral_tap_comes_with_the_first_release_after_a_shortcut_and_only_then() {
    let mut t = HotkeyTracker::new();
    press(&mut t, KEY_LEFTMETA);
    press(&mut t, KEY_LEFTALT);
    press(&mut t, KEY_1);
    release(&mut t, KEY_1);
    // Win up first: tap before it, so the Start menu does not open
    let d = release(&mut t, KEY_LEFTMETA);
    assert!(d.neutral_tap && !d.swallow);
    // one tap covers both: Alt's release has none
    assert!(!release(&mut t, KEY_LEFTALT).neutral_tap);
    // a plain Win press and release is untouched: the Start menu may open as usual
    press(&mut t, KEY_LEFTMETA);
    assert!(!release(&mut t, KEY_LEFTMETA).neutral_tap);
}

#[test]
fn reset_forgets_what_was_held() {
    let mut t = HotkeyTracker::new();
    press(&mut t, KEY_LEFTMETA);
    press(&mut t, KEY_LEFTALT);
    press(&mut t, KEY_P);
    t.reset();
    assert_eq!(release(&mut t, KEY_P), KeyDecision::default());
    assert_eq!(press(&mut t, KEY_P), KeyDecision::default());
}

#[test]
fn the_neutral_key_is_a_vk_no_program_binds() {
    assert_eq!(NEUTRAL_KEY, WinKey::Vk(0xE8));
}

#[test]
fn debug_of_the_tracker_shows_nothing_held() {
    let mut t = HotkeyTracker::new();
    press(&mut t, KEY_LEFTMETA);
    press(&mut t, KEY_LEFTALT);
    press(&mut t, KEY_P);
    let text = format!("{t:?}");
    assert!(!text.contains("25") && !text.contains("125"), "{text}");
}

#[test]
fn the_tracker_is_copy_and_holds_no_allocation() {
    // Sentinel R3: a function of what is held now plus the key that arrived.
    fn is_copy<T: Copy>() {}
    is_copy::<HotkeyTracker>();
    assert!(std::mem::size_of::<HotkeyTracker>() <= 32);
    // a key that is not a shortcut leaves no trace: the same events give the same answers
    let mut a = HotkeyTracker::new();
    let before = a;
    for _ in 0..10_000 {
        press(&mut a, KEY_A);
        release(&mut a, KEY_A);
    }
    assert_eq!(format!("{a:?}"), format!("{before:?}"));
    assert_eq!(press(&mut a, KEY_LEFTMETA), KeyDecision::default());
}
