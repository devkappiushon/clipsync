//! §9 S3 item 1 (f, g, h) and config parsing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use clipsync_input::adapter::{Segment, barrier_segment};
use clipsync_input::{AdapterChoice, CaptureEvent, InputConfig, TokenStore, Zone};
use clipsync_proto::{Edge, InputDatagram, InputEvent};

fn zone() -> Zone {
    Zone {
        index: 0,
        name: Some("eDP-1".into()),
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale: 1.0,
    }
}

#[test]
fn debug_hides_key_and_button_codes() {
    let key = InputEvent::Key {
        code: 4660,
        pressed: true,
    };
    let button = InputEvent::Button {
        code: 4661,
        pressed: false,
    };
    let cap = CaptureEvent::Input {
        t_mono_us: 1,
        ev: key,
    };
    let dg = InputDatagram {
        session: 1,
        seq: 2,
        t_mono_us: 3,
        events: vec![key, button],
    };
    for text in [
        format!("{key:?}"),
        format!("{button:?}"),
        format!("{cap:?}"),
        format!("{dg:?}"),
    ] {
        assert!(!text.contains("4660") && !text.contains("4661"), "{text}");
    }
    assert!(format!("{key:?}").contains("redacted"));
}

#[test]
fn upper_half_of_the_right_edge_is_x_1920_y_0_to_539() {
    let s = barrier_segment(&zone(), Edge::Right, 0.0, 0.5);
    assert_eq!(
        s,
        Segment {
            x1: 1920,
            y1: 0,
            x2: 1920,
            y2: 539
        }
    );
    assert_ne!(s.x1, 1919, "the portal refuses 1919");
}

#[test]
fn other_edges_and_offsets() {
    let mut z = zone();
    z.x = 1920;
    z.y = 100;
    assert_eq!(
        barrier_segment(&z, Edge::Right, 0.0, 1.0),
        Segment {
            x1: 3840,
            y1: 100,
            x2: 3840,
            y2: 1179
        }
    );
    assert_eq!(
        barrier_segment(&z, Edge::Left, 0.5, 1.0),
        Segment {
            x1: 1920,
            y1: 640,
            x2: 1920,
            y2: 1179
        }
    );
    assert_eq!(
        barrier_segment(&z, Edge::Top, 0.0, 0.25),
        Segment {
            x1: 1920,
            y1: 100,
            x2: 2399,
            y2: 100
        }
    );
    assert_eq!(
        barrier_segment(&z, Edge::Bottom, 0.0, 1.0),
        Segment {
            x1: 1920,
            y1: 1180,
            x2: 3839,
            y2: 1180
        }
    );
}

#[test]
fn token_file_roundtrips_and_is_0600_on_unix() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::new(dir.path());
    assert_eq!(store.load(), None);
    store.save("tok-AbC_123").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    assert_eq!(store.load().as_deref(), Some("tok-AbC_123"));
    let text = std::fs::read_to_string(store.path()).unwrap();
    assert!(text.contains("version = 1") && text.contains("remote_desktop"));
}

#[test]
fn invalid_token_is_discarded() {
    let dir = tempfile::tempdir().unwrap();
    let store = TokenStore::new(dir.path());
    for bad in [
        "remote_desktop = \"has space\"\n",
        "remote_desktop = \"\"\n",
        "remote_desktop = 5\n",
        "not toml at all [[[",
    ] {
        std::fs::write(store.path(), bad).unwrap();
        assert_eq!(store.load(), None, "{bad}");
        assert!(!store.path().exists(), "discarded: {bad}");
    }
    assert!(store.save("with space").is_err());
    let long = "a".repeat(300);
    assert!(store.save(&long).is_err());
}

#[test]
fn config_defaults_and_barriers() {
    assert_eq!(
        InputConfig::from_table(None).unwrap(),
        InputConfig::default()
    );
    let t: toml::Table = toml::from_str(
        r#"
        capture = "portal"
        emulate = "fake"
        [[barrier]]
        monitor = "eDP-1"
        edge = "right"
        from = 0.0
        to = 0.5
        peer = "notebook-b"
        peer_edge = "left"
        "#,
    )
    .unwrap();
    let c = InputConfig::from_table(Some(&t)).unwrap();
    assert_eq!(c.capture, AdapterChoice::Portal);
    assert_eq!(c.emulate, AdapterChoice::Fake);
    assert_eq!(c.barriers.len(), 1);
    assert_eq!(c.barriers[0].edge, Edge::Right);
    assert_eq!(c.barriers[0].peer_edge, Edge::Left);
}

#[test]
fn config_rejects_bad_input() {
    let bad = [
        "unknown = 1",
        "capture = \"x11\"",
        "[[barrier]]\nmonitor=\"a\"\nedge=\"right\"\nfrom=0.6\nto=0.5\npeer=\"b\"\npeer_edge=\"left\"",
        "[[barrier]]\nmonitor=\"a\"\nedge=\"diagonal\"\npeer=\"b\"\npeer_edge=\"left\"",
    ];
    for b in bad {
        let t: toml::Table = toml::from_str(b).unwrap();
        assert!(InputConfig::from_table(Some(&t)).is_err(), "{b}");
    }
}

#[test]
fn hash_digits_is_a_zone_index_everything_else_a_name() {
    use clipsync_input::{ZoneRef, zone_ref};
    assert!(matches!(zone_ref("#0"), ZoneRef::Index(0)));
    assert!(matches!(zone_ref("#12"), ZoneRef::Index(12)));
    for name in ["eDP-1", "HDMI-A-1", "#", "#x", "#1a", "0", ""] {
        assert!(
            matches!(zone_ref(name), ZoneRef::Name(ref n) if n == name),
            "{name}"
        );
    }
}
