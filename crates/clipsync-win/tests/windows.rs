//! Runs only on Windows (the CI Windows runner). Nothing here reads a key: the hooks are started with
//! a callback that would flag any event it sees, and the only input is the one this test injects.
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_win::{
    Hooks, Modifier, Notice, RawEvent, Synth, Verdict, Watcher, WinButton, WinKey, button_down,
    cursor_pos, hotkey_registrable, input_desktop_is_default, modifier_down, monitors,
    process_handle_count, send, virtual_screen,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Every test in a file shares one process, and the handle count is a process-wide number: the tests
/// run one at a time, or the threads and hooks of one show up as a "leak" in another.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Handle count after each of `cycles` runs of `cycle`, after `warmup` runs that are not counted: the
/// first use of the window, hook and session APIs opens handles that stay for the whole process.
fn handles_after_each(warmup: usize, cycles: usize, mut cycle: impl FnMut()) -> Vec<u32> {
    for _ in 0..warmup {
        cycle();
    }
    let mut series = vec![process_handle_count().unwrap()];
    for _ in 0..cycles {
        cycle();
        series.push(process_handle_count().unwrap());
    }
    series
}

/// A real leak grows with every cycle; noise does not. Fails only past a small slack for the whole run.
fn assert_no_growth(name: &str, series: &[u32]) {
    let (first, last) = (series[0], series[series.len() - 1]);
    assert!(
        last <= first + 4,
        "{name}: handles went from {first} to {last}, per cycle: {series:?}"
    );
}

/// Sentinel C4: opening the input desktop on every batch must not leak a handle.
#[test]
fn ten_thousand_desktop_checks_do_not_leak_handles() {
    let _one_at_a_time = serial();
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
    let _one_at_a_time = serial();
    let watcher = handles_after_each(2, 20, || {
        drop(Watcher::start(Box::new(|_| {})).unwrap());
    });
    assert_no_growth("watcher", &watcher);
    let hooks = handles_after_each(2, 20, || {
        drop(Hooks::start(Box::new(|_| Verdict::Pass), Box::new(|_| {})).unwrap());
    });
    assert_no_growth("hooks", &hooks);
    let park = handles_after_each(2, 20, || {
        drop(clipsync_win::CursorPark::new(10, 10).unwrap());
    });
    assert_no_growth("cursor park", &park);
}

/// I-S7-1: what the system marks as injected passes straight through and is never read.
#[test]
fn injected_input_never_reaches_the_callback() {
    let _one_at_a_time = serial();
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
    let _one_at_a_time = serial();
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
    let _one_at_a_time = serial();
    let mut refused = Vec::new();
    for vk in (0x31..=0x39).chain([0x4C, 0x50]) {
        if let Err(e) = hotkey_registrable(vk) {
            refused.push((vk, e.code));
        }
    }
    let report = format!("RegisterHotKey refused (vk, code): {refused:?}");
    eprintln!("{report}");
    // The CI log hides the output of a passing test: the job summary does not.
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY")
        && let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{report}");
    }
    assert!(
        !refused.iter().any(|(vk, _)| *vk == 0x50),
        "Win+Alt+P refused: {report}"
    );
}

#[test]
fn a_notice_clock_runs_and_stops() {
    let _one_at_a_time = serial();
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
