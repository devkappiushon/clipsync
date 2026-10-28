//! Runs only on Windows (the CI Windows runner). Nothing here reads a key: the hooks are started with
//! a callback that would flag any event it sees, and the only input is the one this test injects.
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_win::{
    Hooks, Modifier, Notice, RawEvent, Synth, Verdict, Watcher, WinButton, WinKey, button_down,
    cursor_pos, hotkey_registrable, input_desktop_is_default, modifier_down, monitors,
    process_handle_count, send, virtual_screen,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// Sentinel C4: opening the input desktop on every batch must not leak a handle.
#[test]
fn ten_thousand_desktop_checks_do_not_leak_handles() {
    for _ in 0..200 {
        let _ = input_desktop_is_default();
    }
    let before = process_handle_count().unwrap();
    for _ in 0..10_000 {
        let _ = input_desktop_is_default();
    }
    let after = process_handle_count().unwrap();
    assert!(after <= before + 2, "handles went from {before} to {after}");
}

#[test]
fn hooks_and_watcher_start_and_stop_without_leaking() {
    let before = process_handle_count().unwrap();
    for _ in 0..20 {
        let hooks = Hooks::start(Box::new(|_| Verdict::Pass), Box::new(|_| {})).unwrap();
        drop(hooks);
        let watcher = Watcher::start(Box::new(|_| {})).unwrap();
        drop(watcher);
    }
    let after = process_handle_count().unwrap();
    assert!(after <= before + 4, "handles went from {before} to {after}");
}

/// I-S7-1: what the system marks as injected passes straight through and is never read.
#[test]
fn injected_input_never_reaches_the_callback() {
    let saw_any = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&saw_any);
    let hooks = Hooks::start(
        Box::new(move |event: &RawEvent| {
            if matches!(event, RawEvent::Key { .. } | RawEvent::Button { .. }) {
                flag.store(true, Ordering::SeqCst);
            }
            Verdict::Pass
        }),
        Box::new(|_| {}),
    )
    .unwrap();
    // VK 0xE8 is unassigned: pressing and releasing it changes nothing on the machine.
    let sent = send(&[
        Synth::Key {
            key: WinKey::Vk(0xE8),
            up: false,
        },
        Synth::Key {
            key: WinKey::Vk(0xE8),
            up: true,
        },
    ])
    .unwrap();
    assert_eq!(sent, 2);
    std::thread::sleep(Duration::from_millis(300));
    assert!(!saw_any.load(Ordering::SeqCst));
    drop(hooks);
}

#[test]
fn the_desktop_and_the_monitors_can_be_read() {
    let list = monitors().unwrap();
    for monitor in &list {
        assert!(monitor.width > 0 && monitor.height > 0);
        assert!(monitor.dpi >= 96);
        assert!(!monitor.device.is_empty() && !monitor.path.is_empty());
    }
    let (_, _, width, height) = virtual_screen();
    assert!(width > 0 && height > 0);
    assert!(cursor_pos().is_ok());
    // the physical state is whatever it is; the calls just have to work
    let _ = (
        modifier_down(Modifier::LeftShift),
        button_down(WinButton::Left),
    );
}

/// I-S7-2: does the system accept Win+Alt+digit and Win+Alt+P as hotkeys? This records the answer in
/// the CI log (`--nocapture` is not needed: a failed assert prints it) and asserts only on P, which
/// the pause design needs; the digits are informational.
#[test]
fn which_win_alt_hotkeys_the_system_accepts() {
    let mut refused = Vec::new();
    for vk in (0x31..=0x39).chain([0x4C, 0x50]) {
        if let Err(e) = hotkey_registrable(vk) {
            refused.push((vk, e.code));
        }
    }
    eprintln!("RegisterHotKey refused (vk, code): {refused:?}");
    assert!(
        !refused.iter().any(|(vk, _)| *vk == 0x50),
        "Win+Alt+P refused: {refused:?}"
    );
}

#[test]
fn a_notice_clock_runs_and_stops() {
    let ticks = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&ticks);
    let watcher = Watcher::start(Box::new(move |notice| {
        if matches!(notice, Notice::DesktopSwitched) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }))
    .unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    drop(watcher);
    // the desktop did not change during the test
    assert_eq!(ticks.load(Ordering::SeqCst), 0);
}
