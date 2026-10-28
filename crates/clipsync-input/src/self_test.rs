//! `self_test()` (contract §12.6, L7): what the onboarding shows for keyboard and mouse. It only
//! asks the portals what they offer, which never opens a dialog (the same calls as `diagnose`:
//! build the proxy, read the version and the device types). It never creates a session, moves
//! the cursor or presses a key.
//!
//! Criterion, in this order:
//! 1. `Unavailable`: not Linux; or the InputCapture portal does not answer; or the RemoteDesktop
//!    portal does not answer or lacks the keyboard or the pointer device type; or the portals did
//!    not answer in 5 s. The module cannot run (it needs both capture and emulation).
//! 2. `Experimental`: both portals answer, and at least one of: no monitor detected, an Input
//!    Leap, Barrier or Synergy process is running, the desktop is not GNOME on Wayland (the only
//!    one the spikes covered), or typing with accents is not validated yet (`KEYBOARD_VALIDATED`).
//! 3. `Validated`: none of the above. Unreachable until `KEYBOARD_VALIDATED` turns true, which
//!    happens when the matrix promotes keyboard and mouse (README says "experimental" today).

use crate::msg;

/// Same shape as `clipsync_clipboard::SelfTestOutcome`. The `String` is the detail in Portuguese.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfTestOutcome {
    Validated(String),
    Experimental(String),
    Unavailable(String),
}

/// The matrix still says "typing not tested" (I2); flip when it is proven on a real machine.
#[cfg(any(target_os = "linux", test))]
const KEYBOARD_VALIDATED: bool = false;

#[cfg(target_os = "linux")]
const PORTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// What the portals and the system said, before judging.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone)]
struct Facts {
    /// InputCapture portal version, or why it did not answer.
    capture: Result<u32, String>,
    /// RemoteDesktop portal, or why it did not answer.
    remote: Result<Remote, String>,
    monitors: usize,
    conflicts: Vec<String>,
    gnome_wayland: bool,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, Copy)]
struct Remote {
    version: u32,
    keyboard: bool,
    pointer: bool,
}

#[cfg(any(target_os = "linux", test))]
fn classify(f: &Facts, keyboard_validated: bool) -> SelfTestOutcome {
    let capture = match &f.capture {
        Ok(v) => *v,
        Err(why) => return SelfTestOutcome::Unavailable(msg::self_test_no_capture(why)),
    };
    let remote = match &f.remote {
        Ok(r) => *r,
        Err(why) => return SelfTestOutcome::Unavailable(msg::self_test_no_remote(why)),
    };
    if !(remote.keyboard && remote.pointer) {
        return SelfTestOutcome::Unavailable(msg::SELF_TEST_NO_DEVICES.to_owned());
    }
    let mut caveats: Vec<String> = Vec::new();
    if f.monitors == 0 {
        caveats.push(msg::SELF_TEST_NO_MONITORS.to_owned());
    }
    if !f.conflicts.is_empty() {
        caveats.push(msg::self_test_conflicts(&f.conflicts));
    }
    if !f.gnome_wayland {
        caveats.push(msg::SELF_TEST_NOT_GNOME_WAYLAND.to_owned());
    }
    if !keyboard_validated {
        caveats.push(msg::SELF_TEST_KEYBOARD_UNPROVEN.to_owned());
    }
    let ready = msg::self_test_ready(capture, remote.version, f.monitors);
    if caveats.is_empty() {
        SelfTestOutcome::Validated(ready)
    } else {
        SelfTestOutcome::Experimental(format!("{ready}; {}", caveats.join("; ")))
    }
}

/// One line, no control characters: the text goes to the screen.
#[cfg(target_os = "linux")]
fn one_line(e: impl std::fmt::Display) -> String {
    let s: String = e
        .to_string()
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    s.chars().take(160).collect()
}

#[cfg(target_os = "linux")]
fn gnome_wayland() -> bool {
    let var = |k: &str| std::env::var(k).unwrap_or_default().to_ascii_lowercase();
    var("XDG_CURRENT_DESKTOP")
        .split(':')
        .any(|d| d == "gnome" || d == "ubuntu:gnome")
        && var("XDG_SESSION_TYPE") == "wayland"
}

#[cfg(target_os = "linux")]
async fn gather() -> Facts {
    use ashpd::desktop::input_capture::InputCapture;
    use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
    // Never `supported_capabilities()`: GNOME 50 returns an unknown bit and the parse fails.
    let capture = InputCapture::new()
        .await
        .map(|ic| ic.version())
        .map_err(one_line);
    let remote = match RemoteDesktop::new().await {
        Ok(rd) => rd
            .available_device_types()
            .await
            .map(|types| Remote {
                version: rd.version(),
                keyboard: types.contains(DeviceType::Keyboard),
                pointer: types.contains(DeviceType::Pointer),
            })
            .map_err(one_line),
        Err(e) => Err(one_line(e)),
    };
    Facts {
        capture,
        remote,
        monitors: crate::adapter::outputs::connected_outputs().len(),
        conflicts: crate::diag::running_conflicts(),
        gnome_wayland: gnome_wayland(),
    }
}

/// Asks the portals what they offer. No dialog, no session, nothing is sent to the system.
#[cfg(target_os = "linux")]
pub async fn self_test() -> SelfTestOutcome {
    match tokio::time::timeout(PORTAL_TIMEOUT, gather()).await {
        Ok(facts) => classify(&facts, KEYBOARD_VALIDATED),
        Err(_) => SelfTestOutcome::Unavailable(msg::SELF_TEST_TIMEOUT.to_owned()),
    }
}

/// There is no adapter outside Linux in this version.
#[cfg(not(target_os = "linux"))]
pub async fn self_test() -> SelfTestOutcome {
    SelfTestOutcome::Unavailable(msg::SELF_TEST_NO_PLATFORM.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Facts {
        Facts {
            capture: Ok(1),
            remote: Ok(Remote {
                version: 2,
                keyboard: true,
                pointer: true,
            }),
            monitors: 2,
            conflicts: Vec::new(),
            gnome_wayland: true,
        }
    }

    fn detail(o: &SelfTestOutcome) -> &str {
        match o {
            SelfTestOutcome::Validated(d)
            | SelfTestOutcome::Experimental(d)
            | SelfTestOutcome::Unavailable(d) => d,
        }
    }

    #[test]
    fn everything_in_place_is_validated_only_once_typing_is_proven() {
        let o = classify(&good(), true);
        assert!(matches!(o, SelfTestOutcome::Validated(_)), "{o:?}");
        assert!(detail(&o).contains("captura (v1)") && detail(&o).contains("2 monitor(es)"));
        // As shipped, a ready machine gets what the constant says.
        let o = classify(&good(), KEYBOARD_VALIDATED);
        assert_eq!(
            matches!(o, SelfTestOutcome::Validated(_)),
            KEYBOARD_VALIDATED,
            "{o:?}"
        );
        // Without the proof the best is experimental, and the detail names what is missing.
        let unproven = classify(&good(), false);
        assert!(
            matches!(unproven, SelfTestOutcome::Experimental(_)),
            "{unproven:?}"
        );
        assert!(
            detail(&unproven).contains("digitação com acentos"),
            "{}",
            detail(&unproven)
        );
    }

    #[test]
    fn a_missing_portal_or_device_type_is_unavailable() {
        let mut f = good();
        f.capture = Err("sem serviço".into());
        let o = classify(&f, true);
        assert!(matches!(o, SelfTestOutcome::Unavailable(_)));
        assert!(detail(&o).contains("captura") && detail(&o).contains("sem serviço"));

        let mut f = good();
        f.remote = Err("sem serviço".into());
        let o = classify(&f, true);
        assert!(matches!(o, SelfTestOutcome::Unavailable(_)));
        assert!(detail(&o).contains("emulação"));

        for (keyboard, pointer) in [(false, true), (true, false), (false, false)] {
            let mut f = good();
            f.remote = Ok(Remote {
                version: 2,
                keyboard,
                pointer,
            });
            let o = classify(&f, true);
            assert!(
                matches!(o, SelfTestOutcome::Unavailable(_)),
                "{keyboard} {pointer}"
            );
            assert_eq!(detail(&o), msg::SELF_TEST_NO_DEVICES);
        }
    }

    #[test]
    fn each_caveat_makes_it_experimental_and_is_named() {
        let mut f = good();
        f.monitors = 0;
        let o = classify(&f, true);
        assert!(matches!(o, SelfTestOutcome::Experimental(_)));
        assert!(detail(&o).contains("nenhum monitor"));

        let mut f = good();
        f.conflicts = vec!["barriers".into(), "synergys".into()];
        let o = classify(&f, true);
        assert!(matches!(o, SelfTestOutcome::Experimental(_)));
        assert!(detail(&o).contains("barriers, synergys"));

        let mut f = good();
        f.gnome_wayland = false;
        let o = classify(&f, true);
        assert!(matches!(o, SelfTestOutcome::Experimental(_)));
        assert!(detail(&o).contains("GNOME no Wayland"));

        // Several at once are all listed.
        let mut f = good();
        f.monitors = 0;
        f.gnome_wayland = false;
        let d = detail(&classify(&f, true)).to_owned();
        assert!(
            d.contains("nenhum monitor") && d.contains("GNOME no Wayland"),
            "{d}"
        );
    }

    #[test]
    fn an_unavailable_portal_outranks_every_caveat() {
        let mut f = good();
        f.monitors = 0;
        f.gnome_wayland = false;
        f.conflicts = vec!["barriers".into()];
        f.capture = Err("x".into());
        assert!(matches!(
            classify(&f, false),
            SelfTestOutcome::Unavailable(_)
        ));
    }

    #[test]
    fn details_have_no_dash_and_no_line_break() {
        let mut f = good();
        f.conflicts = vec!["barriers".into()];
        f.monitors = 0;
        f.gnome_wayland = false;
        for o in [classify(&f, false), classify(&good(), true)] {
            let d = detail(&o);
            assert!(!d.contains('\u{2014}') && !d.contains('\n'), "{d}");
        }
    }
}
