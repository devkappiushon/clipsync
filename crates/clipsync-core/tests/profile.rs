#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_core::{CoreError, InitOptions, PeerEntry, Peers, Profile, State};
use clipsync_proto::{ModuleSet, PeerId};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn opts(clipboard: bool, fake: bool) -> InitOptions {
    InitOptions {
        device_name: "notebook-a".into(),
        modules: ModuleSet {
            clipboard,
            input: true,
        },
        listen: "127.0.0.1:47800".parse().unwrap(),
        fake_adapters: fake,
    }
}

#[cfg(unix)]
fn mode(p: &std::path::Path) -> u32 {
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

#[test]
fn profile_init_creates_private_profile_and_reloads_same_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    let id = p.init(opts(false, false)).unwrap();
    #[cfg(unix)]
    {
        assert_eq!(mode(&p.dir), 0o700);
        assert_eq!(mode(&p.dir.join("identity.key.pem")), 0o600);
        assert_eq!(mode(&p.dir.join("config.toml")), 0o600);
    }
    assert_eq!(p.load_identity().unwrap().id(), id.id());
    let cfg = p.load_config().unwrap();
    assert_eq!((cfg.device_name.as_str(), cfg.version), ("notebook-a", 1));
    assert!(cfg.modules.input && !cfg.modules.clipboard);
    assert!(cfg.clipboard.is_none() && cfg.input.is_some());
    assert!(p.load_peers().unwrap().peer.is_empty());
    assert!(!format!("{id:?}").contains("PRIVATE"));
}

#[test]
fn profile_init_with_writes_the_identity_it_was_given_and_nothing_on_a_bad_name() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    let made = clipsync_core::Identity::generate().unwrap();
    let mut bad = opts(false, false);
    bad.device_name = "a\u{1b}b".into();
    assert!(matches!(
        p.init_with(bad, made.clone()),
        Err(CoreError::Config(_))
    ));
    assert!(!p.is_initialized(), "a refused init writes nothing");
    let saved = p.init_with(opts(false, false), made.clone()).unwrap();
    assert_eq!(saved.id(), made.id());
    assert_eq!(p.load_identity().unwrap().id(), made.id());
}

#[test]
fn profile_fake_adapters_are_written_to_the_module_tables() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(true, true)).unwrap();
    let cfg = p.load_config().unwrap();
    assert_eq!(cfg.clipboard.unwrap()["adapter"].as_str(), Some("fake"));
    let input = cfg.input.unwrap();
    assert_eq!(
        (input["capture"].as_str(), input["emulate"].as_str()),
        (Some("fake"), Some("fake"))
    );
}

#[test]
fn profile_both_modules_off_and_bad_names_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    let mut o = opts(false, false);
    o.modules = ModuleSet::default();
    assert!(matches!(p.init(o), Err(CoreError::Config(_))));
    let mut o = opts(true, false);
    o.device_name = "x".repeat(65);
    assert!(matches!(p.init(o), Err(CoreError::Config(_))));
    let mut o = opts(true, false);
    o.device_name = String::new();
    assert!(p.init(o).is_err());
}

#[test]
fn profile_config_with_unknown_key_or_wrong_version_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(false, false)).unwrap();
    let good = std::fs::read_to_string(p.config_path()).unwrap();
    std::fs::write(p.config_path(), good.replace("version = 1", "version = 2")).unwrap();
    let e = p.load_config().unwrap_err().to_string();
    assert!(e.contains("versão 2"), "{e}");
    std::fs::write(p.config_path(), format!("surprise = 1\n{good}")).unwrap();
    assert!(p.load_config().is_err());
    std::fs::write(
        p.config_path(),
        good.replace("input = true", "input = false"),
    )
    .unwrap();
    assert!(p.load_config().is_err(), "no module on");
}

#[test]
fn profile_key_with_loose_mode_is_refused_and_missing_profile_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    assert!(p.load_config().is_err() && !p.is_initialized());
    p.init(opts(false, false)).unwrap();
    #[cfg(unix)]
    {
        std::fs::set_permissions(
            p.dir.join("identity.key.pem"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(matches!(p.load_identity(), Err(CoreError::Identity(_))));
    }
}

#[test]
fn profile_reinit_gets_a_new_identity_but_keeps_peers() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    let first = p.init(opts(false, false)).unwrap();
    let mut peers = Peers::default();
    peers.upsert(PeerEntry {
        id: PeerId([7; 32]),
        name: "notebook-b".into(),
        addr: Some("127.0.0.1:47801".parse().unwrap()),
        paired_at: "2026-10-27T14:00:00Z".into(),
        perms: clipsync_proto::PeerPerms {
            clipboard_in: true,
            clipboard_out: false,
            input_in: false,
        },
    });
    p.save_peers(&peers).unwrap();
    let second = p.init(opts(false, false)).unwrap();
    assert_ne!(first.id(), second.id());
    assert_eq!(p.load_peers().unwrap(), peers);
    #[cfg(unix)]
    assert_eq!(mode(&p.peers_path()), 0o600);
}

#[test]
fn profile_peers_upsert_replaces_by_id_or_name_and_rejects_duplicates_on_load() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(false, false)).unwrap();
    let entry = |id: u8, name: &str| PeerEntry {
        id: PeerId([id; 32]),
        name: name.into(),
        addr: Some("127.0.0.1:1".parse().unwrap()),
        paired_at: "2026-10-27T14:00:00Z".into(),
        perms: clipsync_proto::PeerPerms {
            clipboard_in: false,
            clipboard_out: false,
            input_in: false,
        },
    };
    let mut peers = Peers::default();
    peers.upsert(entry(1, "b"));
    peers.upsert(entry(2, "b")); // same name, re-keyed device
    assert_eq!(peers.peer.len(), 1);
    assert_eq!(peers.find("b").unwrap().id, PeerId([2; 32]));
    assert!(peers.find(&PeerId([2; 32]).short()).is_some());
    peers.peer.push(entry(2, "c")); // duplicate id sneaked in by hand
    p.save_peers(&peers).unwrap();
    assert!(matches!(p.load_peers(), Err(CoreError::Profile(_))));
}

#[test]
fn profile_state_defaults_to_running_and_roundtrips_pause() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(false, false)).unwrap();
    assert!(!p.load_state().unwrap().paused);
    p.save_state(&State {
        version: 1,
        paused: true,
    })
    .unwrap();
    assert!(p.load_state().unwrap().paused);
}

#[test]
fn profile_identity_id_is_the_single_pin_function() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    let id = p.init(opts(false, false)).unwrap();
    assert_eq!(id.id(), clipsync_core::tls::pin_of(&id.cert_der()).unwrap());
}

#[test]
fn profile_peers_file_has_64_hex_ids_and_flat_permissions() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(false, false)).unwrap();
    let mut peers = Peers::default();
    peers.upsert(PeerEntry {
        id: PeerId([0xAB; 32]),
        name: "b".into(),
        addr: None,
        paired_at: "2026-10-27T14:00:00Z".into(),
        perms: clipsync_proto::PeerPerms {
            clipboard_in: true,
            clipboard_out: true,
            input_in: false,
        },
    });
    p.save_peers(&peers).unwrap();
    let text = std::fs::read_to_string(p.peers_path()).unwrap();
    assert!(
        text.contains(&format!("id = \"{}\"", "ab".repeat(32))),
        "{text}"
    );
    assert!(
        text.contains("clipboard_in = true") && text.contains("input_in = false"),
        "{text}"
    );
    assert_eq!(p.load_peers().unwrap(), peers);
}

#[test]
fn profile_peer_with_a_hostile_name_is_quarantined_and_the_rest_loads() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(false, false)).unwrap();
    let entry = |id: u8, name: &str| {
        format!(
            "[[peer]]\nid = \"{}\"\nname = \"{}\"\naddr = \"127.0.0.1:1\"\npaired_at = \"2026-10-27T14:00:00Z\"\nclipboard_in = false\nclipboard_out = false\ninput_in = false\n",
            format!("{id:02x}").repeat(32),
            // TOML basic strings take \uXXXX escapes, not Rust's \u{..}
            name.chars()
                .map(|c| if c.is_ascii() && !c.is_control() {
                    c.to_string()
                } else {
                    format!("\\u{:04X}", c as u32)
                })
                .collect::<String>()
        )
    };
    let text = format!(
        "version = 1\n{}{}{}{}",
        entry(1, "bom"),
        entry(2, "ev\u{1b}[2Jil"),
        entry(3, "csi\u{9b}8-bit"),
        entry(4, "rtl\u{202e}txt")
    );
    std::fs::write(p.peers_path(), text).unwrap();
    let peers = p.load_peers().unwrap();
    let names: Vec<&str> = peers.peer.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        ["bom"],
        "controls, C1 and bidi overrides are quarantined"
    );
}

#[test]
fn profile_device_name_with_formatting_characters_is_refused_at_init() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    for bad in ["a\u{202e}b", "a\u{200b}b", "a\u{feff}b", "a\u{9b}b", "a\nb"] {
        let mut o = opts(true, false);
        o.device_name = bad.into();
        assert!(matches!(p.init(o), Err(CoreError::Config(_))), "{bad:?}");
    }
}

/// §12.6 core item 5: `[clipboard] mode` is rewritten in place; every other key survives.
#[test]
fn profile_set_clipboard_mode_keeps_the_rest_of_the_config() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().join("a"))).unwrap();
    p.init(opts(true, true)).unwrap();
    let before = p.load_config().unwrap();
    p.set_clipboard_mode("manual").unwrap();
    let after = p.load_config().unwrap();
    let clip = after.clipboard.as_ref().unwrap();
    assert_eq!(clip.get("mode").and_then(|v| v.as_str()), Some("manual"));
    assert_eq!(clip.get("adapter").and_then(|v| v.as_str()), Some("fake"));
    assert_eq!(
        (after.device_name, after.listen, after.modules, after.input),
        (
            before.device_name,
            before.listen,
            before.modules,
            before.input
        )
    );
    #[cfg(unix)]
    assert_eq!(mode(&p.config_path()), 0o600);
    assert!(matches!(
        p.set_clipboard_mode("confirmar"),
        Err(CoreError::Config(_))
    ));
    let q = Profile::resolve(Some(tmp.path().join("vazio"))).unwrap();
    assert!(q.set_clipboard_mode("auto").is_err());
}
