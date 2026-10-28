//! §13.6 "Adaptador Windows" item 1 (Flux): the decisions inside the hook callback, on every system.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::adapter::wincore::{
    ArmedBarrier, CaptureCore, Held, Out, Restart, RestartPolicy, resolve_barriers,
};
use clipsync_input::adapter::{BarrierSpec, Hotkey, Zone, ZoneRef};
use clipsync_proto::{Edge, InputEvent};
use clipsync_win::{RawEvent, Verdict, WinButton};
use std::sync::Arc;

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

/// Monitor 1920x1080 at the origin with its whole right edge linked.
fn core() -> CaptureCore {
    let mut c = CaptureCore::new();
    c.set_barriers(Arc::from(vec![Some(ArmedBarrier {
        zone: zone(0, 0, 1920, 1080),
        edge: Edge::Right,
        from: 0.0,
        to: 1.0,
    })]));
    c
}

fn feed(c: &mut CaptureCore, ev: RawEvent) -> (Verdict, Vec<Out>) {
    let mut out = Vec::new();
    let verdict = c.on_event(&ev, &mut |o| out.push(o));
    (verdict, out)
}

fn mv(x: i32, y: i32) -> RawEvent {
    RawEvent::Move { x, y }
}

fn key(scan: u32, extended: bool, up: bool) -> RawEvent {
    RawEvent::Key {
        vk: 0,
        scan,
        extended,
        up,
    }
}

const WIN: (u32, bool) = (0x5B, true); // KEY_LEFTMETA 125
const ALT: (u32, bool) = (0x38, false); // KEY_LEFTALT 56
const SHIFT: (u32, bool) = (0x2A, false); // KEY_LEFTSHIFT 42
const DIGIT_1: (u32, bool) = (0x02, false); // KEY_1
const LETTER_A: (u32, bool) = (0x1E, false); // KEY_A

fn press(c: &mut CaptureCore, k: (u32, bool)) -> (Verdict, Vec<Out>) {
    feed(c, key(k.0, k.1, false))
}
fn release(c: &mut CaptureCore, k: (u32, bool)) -> (Verdict, Vec<Out>) {
    feed(c, key(k.0, k.1, true))
}

/// Walks the cursor to the right edge: returns what the last move did.
fn cross(c: &mut CaptureCore) -> (Verdict, Vec<Out>) {
    assert_eq!(feed(c, mv(1000, 500)).0, Verdict::Pass);
    feed(c, mv(1919, 540))
}

fn activate(c: &mut CaptureCore) {
    let (verdict, out) = cross(c);
    assert_eq!(verdict, Verdict::Block);
    assert!(matches!(out.as_slice(), [Out::Activate { barrier: 0, .. }]));
    c.park_ready((960, 540), 96);
}

#[test]
fn idle_moves_pass_and_say_nothing() {
    let mut c = core();
    for (x, y) in [(10, 10), (500, 500), (1900, 500)] {
        assert_eq!(feed(&mut c, mv(x, y)), (Verdict::Pass, vec![]));
    }
    assert!(!c.capturing());
}

#[test]
fn reaching_the_last_pixel_activates_blocks_and_reports_the_position() {
    let mut c = core();
    let (verdict, out) = cross(&mut c);
    assert_eq!(verdict, Verdict::Block);
    match out.as_slice() {
        [
            Out::Activate {
                barrier: 0, pos, ..
            },
        ] => assert!((pos - 0.5).abs() < 1e-6),
        other => panic!("{other:?}"),
    }
    assert!(c.capturing());
}

#[test]
fn activation_carries_what_is_held_so_the_worker_can_carry_it_over() {
    let mut c = core();
    press(&mut c, SHIFT);
    press(&mut c, LETTER_A);
    feed(
        &mut c,
        RawEvent::Button {
            button: WinButton::Left,
            down: true,
        },
    );
    let (_, out) = cross(&mut c);
    let Some(Out::Activate { held, .. }) = out.first().copied() else {
        panic!("{out:?}")
    };
    assert!(held.key(42) && held.key(30));
    assert_eq!(held.modifiers().collect::<Vec<_>>(), [42]);
    assert_eq!(held.buttons().collect::<Vec<_>>(), [WinButton::Left]);
    assert_eq!(Held::from_words(held.to_words()), held);
}

#[test]
fn while_arming_moves_are_eaten_and_the_rest_already_goes_out() {
    let mut c = core();
    cross(&mut c);
    assert_eq!(feed(&mut c, mv(1919, 600)), (Verdict::Block, vec![]));
    let (verdict, out) = feed(
        &mut c,
        RawEvent::Button {
            button: WinButton::Right,
            down: true,
        },
    );
    assert_eq!(verdict, Verdict::Block);
    assert_eq!(
        out,
        [Out::Input(InputEvent::Button {
            code: 0x111,
            pressed: true
        })]
    );
}

#[test]
fn once_parked_every_move_is_a_delta_from_the_parked_point() {
    let mut c = core();
    activate(&mut c);
    let (verdict, out) = feed(&mut c, mv(970, 535));
    assert_eq!(verdict, Verdict::Block);
    assert_eq!(out, [Out::Input(InputEvent::Motion { dx: 10.0, dy: -5.0 })]);
    // no movement, no event
    assert_eq!(feed(&mut c, mv(960, 540)), (Verdict::Block, vec![]));
}

#[test]
fn the_delta_is_in_logical_px_of_the_parked_monitor() {
    let mut c = core();
    cross(&mut c);
    c.park_ready((960, 540), 144); // 150%
    let (_, out) = feed(&mut c, mv(990, 540));
    assert_eq!(out, [Out::Input(InputEvent::Motion { dx: 20.0, dy: 0.0 })]);
}

#[test]
fn buttons_wheel_and_keys_go_to_the_peer_in_evdev_terms() {
    let mut c = core();
    activate(&mut c);
    let (v, out) = feed(
        &mut c,
        RawEvent::Button {
            button: WinButton::X2,
            down: false,
        },
    );
    assert_eq!(
        (v, out),
        (
            Verdict::Block,
            vec![Out::Input(InputEvent::Button {
                code: 0x114,
                pressed: false
            })]
        )
    );
    // wheel: vertical goes with the sign swapped, horizontal as it is
    let (_, out) = feed(
        &mut c,
        RawEvent::Wheel {
            delta: 120,
            horizontal: false,
        },
    );
    assert_eq!(
        out,
        [Out::Input(InputEvent::ScrollDiscrete { dx: 0, dy: -120 })]
    );
    let (_, out) = feed(
        &mut c,
        RawEvent::Wheel {
            delta: -240,
            horizontal: true,
        },
    );
    assert_eq!(
        out,
        [Out::Input(InputEvent::ScrollDiscrete { dx: -240, dy: 0 })]
    );
    // ç of the ABNT2 is the physical position of KEY_SEMICOLON
    let (v, out) = press(&mut c, (0x27, false));
    assert_eq!(
        (v, out),
        (
            Verdict::Block,
            vec![Out::Input(InputEvent::Key {
                code: 39,
                pressed: true
            })]
        )
    );
    let (_, out) = release(&mut c, (0x27, false));
    assert_eq!(
        out,
        [Out::Input(InputEvent::Key {
            code: 39,
            pressed: false
        })]
    );
}

#[test]
fn a_held_key_goes_out_once_the_destination_repeats_it() {
    let mut c = core();
    activate(&mut c);
    assert_eq!(press(&mut c, LETTER_A).1.len(), 1);
    for _ in 0..5 {
        assert_eq!(press(&mut c, LETTER_A), (Verdict::Block, vec![]));
    }
    assert_eq!(release(&mut c, LETTER_A).1.len(), 1);
    assert_eq!(press(&mut c, LETTER_A).1.len(), 1);
}

#[test]
fn fake_and_unmapped_keys_are_dropped_in_both_modes() {
    let mut c = core();
    // the LCtrl that Windows sends before AltGr has scan code 0x21D
    assert_eq!(
        feed(&mut c, key(0x21D, false, false)),
        (Verdict::Pass, vec![])
    );
    activate(&mut c);
    assert_eq!(
        feed(&mut c, key(0x21D, false, false)),
        (Verdict::Block, vec![])
    );
    // AltGr itself is a real key: KEY_RIGHTALT
    let (_, out) = press(&mut c, (0x38, true));
    assert_eq!(
        out,
        [Out::Input(InputEvent::Key {
            code: 100,
            pressed: true
        })]
    );
}

#[test]
fn idle_keys_and_buttons_pass_but_are_tracked() {
    let mut c = core();
    assert_eq!(press(&mut c, LETTER_A), (Verdict::Pass, vec![]));
    assert_eq!(
        feed(
            &mut c,
            RawEvent::Button {
                button: WinButton::Middle,
                down: true
            }
        ),
        (Verdict::Pass, vec![])
    );
    assert!(c.held().key(30) && c.held().button(WinButton::Middle));
    release(&mut c, LETTER_A);
    feed(
        &mut c,
        RawEvent::Button {
            button: WinButton::Middle,
            down: false,
        },
    );
    assert!(c.held().is_empty());
    assert_eq!(
        feed(
            &mut c,
            RawEvent::Wheel {
                delta: 120,
                horizontal: false
            }
        ),
        (Verdict::Pass, vec![])
    );
}

#[test]
fn win_alt_digit_is_a_shortcut_the_digit_goes_nowhere_and_the_release_gets_a_neutral_tap() {
    let mut c = core();
    assert_eq!(press(&mut c, WIN), (Verdict::Pass, vec![]));
    assert_eq!(press(&mut c, ALT), (Verdict::Pass, vec![]));
    let (v, out) = press(&mut c, DIGIT_1);
    assert_eq!(
        (v, out),
        (Verdict::Block, vec![Out::Hotkey(Hotkey::GoTo(1))])
    );
    assert_eq!(release(&mut c, DIGIT_1), (Verdict::Block, vec![]));
    // Win goes up first: the worker taps the neutral key and then lets it go
    let (v, out) = release(&mut c, WIN);
    assert_eq!((v, out), (Verdict::Block, vec![Out::TapThenRelease(125)]));
    // one tap covers both
    assert_eq!(release(&mut c, ALT), (Verdict::Pass, vec![]));
}

#[test]
fn a_shortcut_during_a_capture_is_reported_and_its_key_is_not_forwarded() {
    let mut c = core();
    activate(&mut c);
    press(&mut c, WIN);
    press(&mut c, ALT);
    let (v, out) = press(&mut c, (0x19, false)); // KEY_P
    assert_eq!(
        (v, out),
        (Verdict::Block, vec![Out::Hotkey(Hotkey::TogglePause)])
    );
    // the modifiers did go to the peer; their release goes too, with no local tap
    let (v, out) = release(&mut c, WIN);
    assert_eq!(
        (v, out),
        (
            Verdict::Block,
            vec![Out::Input(InputEvent::Key {
                code: 125,
                pressed: false
            })]
        )
    );
}

#[test]
fn a_locked_screen_keeps_the_barriers_quiet() {
    let mut c = core();
    c.set_locked(true);
    assert_eq!(cross(&mut c), (Verdict::Pass, vec![]));
    c.set_locked(false);
    feed(&mut c, mv(1000, 500));
    assert_eq!(feed(&mut c, mv(1919, 500)).0, Verdict::Block);
}

#[test]
fn ending_the_capture_gives_the_desktop_back_without_a_false_crossing() {
    let mut c = core();
    activate(&mut c);
    c.end_capture();
    assert!(!c.capturing());
    // the cursor came back two pixels inside the edge; the next real move is just a move
    assert_eq!(feed(&mut c, mv(1917, 540)), (Verdict::Pass, vec![]));
    assert_eq!(
        feed(
            &mut c,
            RawEvent::Wheel {
                delta: 120,
                horizontal: false
            }
        ),
        (Verdict::Pass, vec![])
    );
}

#[test]
fn a_barrier_whose_zone_is_gone_keeps_the_indexes_of_the_others() {
    let mut c = CaptureCore::new();
    c.set_barriers(Arc::from(vec![
        None,
        Some(ArmedBarrier {
            zone: zone(0, 0, 1920, 1080),
            edge: Edge::Bottom,
            from: 0.0,
            to: 1.0,
        }),
    ]));
    feed(&mut c, mv(500, 1000));
    let (verdict, out) = feed(&mut c, mv(500, 1079));
    assert_eq!(verdict, Verdict::Block);
    assert!(matches!(out.as_slice(), [Out::Activate { barrier: 1, .. }]));
}

#[test]
fn debug_never_shows_a_key_a_button_or_what_is_held() {
    let mut c = core();
    press(&mut c, LETTER_A);
    let text = format!("{:?} {:?} {:?}", c.held(), Out::TapThenRelease(125), c);
    assert!(!text.contains("125"), "{text}");
    let activate = Out::Activate {
        barrier: 0,
        pos: 0.5,
        held: c.held(),
    };
    assert!(!format!("{activate:?}").contains("30"));
}

#[test]
fn barriers_are_resolved_by_name_or_index_and_a_missing_zone_keeps_its_place() {
    let zones = [
        Zone {
            index: 0,
            name: Some("DELL".into()),
            ..zone(0, 0, 1920, 1080)
        },
        Zone {
            index: 1,
            name: Some("LG".into()),
            ..zone(1920, 0, 1920, 1080)
        },
    ];
    let spec = |zone, edge| BarrierSpec {
        zone,
        edge,
        from: 0.0,
        to: 1.0,
    };
    let resolved = resolve_barriers(
        &[
            spec(ZoneRef::Name("LG".into()), Edge::Right),
            spec(ZoneRef::Name("gone".into()), Edge::Left),
            spec(ZoneRef::Index(0), Edge::Bottom),
        ],
        &zones,
    );
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0].as_ref().map(|b| b.zone.index), Some(1));
    assert!(resolved[1].is_none());
    assert_eq!(
        resolved[2].as_ref().map(|b| (b.zone.index, b.edge)),
        Some((0, Edge::Bottom))
    );
}

#[test]
fn a_switch_starts_the_capture_without_a_crossing() {
    let mut c = core();
    c.begin();
    assert!(c.capturing());
    // still arming: moves are eaten, a key already goes out
    assert_eq!(feed(&mut c, mv(100, 100)), (Verdict::Block, vec![]));
    let (v, out) = press(&mut c, LETTER_A);
    assert_eq!(
        (v, out),
        (
            Verdict::Block,
            vec![Out::Input(InputEvent::Key {
                code: 30,
                pressed: true
            })]
        )
    );
    c.park_ready((960, 540), 96);
    let (_, out) = feed(&mut c, mv(961, 540));
    assert_eq!(out, [Out::Input(InputEvent::Motion { dx: 1.0, dy: 0.0 })]);
    // a second begin while capturing changes nothing
    c.begin();
    assert_eq!(feed(&mut c, mv(962, 540)).0, Verdict::Block);
}

#[test]
fn three_losses_in_ten_seconds_are_recreated_and_the_fourth_gives_up_for_good() {
    let mut p = RestartPolicy::new();
    assert_eq!(p.on_lost(0), Restart::Recreate);
    assert_eq!(p.on_lost(2_000), Restart::Recreate);
    assert_eq!(p.on_lost(9_999), Restart::Recreate);
    assert_eq!(p.on_lost(9_999), Restart::GiveUp);
    assert!(p.gave_up());
    // it does not come back by itself, not even after the window
    assert_eq!(p.on_lost(60_000), Restart::GiveUp);
}

#[test]
fn losses_spread_over_time_never_give_up() {
    let mut p = RestartPolicy::new();
    for i in 0..50u64 {
        assert_eq!(p.on_lost(i * 6_000), Restart::Recreate, "loss {i}");
    }
    assert!(!p.gave_up());
    // the old ones leave the window: three, then a pause, then three again
    let mut p = RestartPolicy::new();
    for t in [0, 1, 2] {
        assert_eq!(p.on_lost(t * 1_000), Restart::Recreate);
    }
    for t in [12_000, 13_000, 14_000] {
        assert_eq!(p.on_lost(t), Restart::Recreate);
    }
    assert_eq!(p.on_lost(14_500), Restart::GiveUp);
}

#[test]
fn the_policy_is_copy_and_fixed_size() {
    fn is_copy<T: Copy>() {}
    is_copy::<RestartPolicy>();
    assert!(std::mem::size_of::<RestartPolicy>() <= 48);
}
