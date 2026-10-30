#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_input::monitor_identities::{Evidence, IdentityError, IdentityStore, Observation};
use clipsync_proto::monitors::NativeRect;
fn obs(path: &str) -> Observation {
    Observation {
        evidence: Evidence::WindowsPath(path.into()),
        label: "Tela".into(),
        rect: NativeRect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: 1.5,
        legacy_aliases: vec![path.into()],
    }
}
#[test]
fn receiver_without_layout_keeps_layout_absent() {
    let d = tempfile::tempdir().unwrap();
    let s = IdentityStore::new(d.path()).reconcile(&[obs("a")]).unwrap();
    assert_eq!(s.rev, 1);
    assert!(!d.path().join("layout.toml").exists());
    assert_eq!(
        IdentityStore::new(d.path())
            .reconcile(&[obs("a")])
            .unwrap()
            .monitors,
        s.monitors
    );
}
#[test]
fn v1_receiver_does_not_migrate_on_identification() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("layout.toml");
    std::fs::write(&p, "# comentário\nversion = 1\nrev = 9\n").unwrap();
    let bytes = std::fs::read(&p).unwrap();
    IdentityStore::new(d.path()).reconcile(&[obs("a")]).unwrap();
    assert_eq!(std::fs::read(p).unwrap(), bytes);
}
#[test]
fn same_paths_reordered_keep_uids() {
    let d = tempfile::tempdir().unwrap();
    let st = IdentityStore::new(d.path());
    let a = st.reconcile(&[obs("a"), obs("b")]).unwrap();
    let bytes = std::fs::read(d.path().join("monitor-identities.toml")).unwrap();
    let b = st.reconcile(&[obs("b"), obs("a")]).unwrap();
    assert_eq!(a.rev, b.rev);
    assert_eq!(a.monitors, b.monitors);
    assert_eq!(
        bytes,
        std::fs::read(d.path().join("monitor-identities.toml")).unwrap()
    );
}
#[test]
fn duplicate_path_and_changed_edid_fail_closed() {
    let d = tempfile::tempdir().unwrap();
    let st = IdentityStore::new(d.path());
    assert!(matches!(
        st.reconcile(&[obs("a"), obs("a")]),
        Err(IdentityError::Ambiguous)
    ));
    let mut o = obs("a");
    o.evidence = Evidence::LinuxSingle {
        connector: "eDP-1".into(),
        edid_sha256: Some([1; 32]),
    };
    st.reconcile(&[o.clone()]).unwrap();
    let p = d.path().join("monitor-identities.toml");
    let bytes = std::fs::read(&p).unwrap();
    o.evidence = Evidence::LinuxSingle {
        connector: "eDP-1".into(),
        edid_sha256: Some([2; 32]),
    };
    assert!(matches!(st.reconcile(&[o]), Err(IdentityError::Ambiguous)));
    assert_eq!(bytes, std::fs::read(p).unwrap());
}
#[test]
fn future_registry_and_io_error_preserve_bytes() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("monitor-identities.toml");
    std::fs::write(&p, "version = 99\nfuture = true\n").unwrap();
    let bytes = std::fs::read(&p).unwrap();
    assert!(matches!(
        IdentityStore::new(d.path()).reconcile(&[obs("a")]),
        Err(IdentityError::UnsupportedVersion)
    ));
    assert_eq!(bytes, std::fs::read(&p).unwrap());
    std::fs::remove_file(&p).unwrap();
    std::fs::create_dir(&p).unwrap();
    assert!(matches!(
        IdentityStore::new(d.path()).reconcile(&[obs("a")]),
        Err(IdentityError::Io)
    ));
    assert!(p.is_dir());
}
#[test]
fn disconnect_reconnect_keeps_uid() {
    let d = tempfile::tempdir().unwrap();
    let st = IdentityStore::new(d.path());
    let a = st.reconcile(&[obs("a")]).unwrap();
    assert_eq!(st.reconcile(&[]).unwrap().rev, 2);
    assert!(matches!(
        st.reconcile(&[obs("changed")]),
        Err(IdentityError::Ambiguous)
    ));
    let b = st.reconcile(&[obs("a")]).unwrap();
    assert_eq!(b.rev, 3);
    assert_eq!(a.monitors, b.monitors);
    let mut changed = obs("a");
    changed.scale = 1.25;
    assert_eq!(st.reconcile(&[changed]).unwrap().rev, 4);
}
#[test]
fn two_writers_do_not_lose_identity() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("monitor-identities.lock");
    let lock = std::fs::File::create(p).unwrap();
    lock.try_lock().unwrap();
    assert!(matches!(
        IdentityStore::new(d.path()).reconcile(&[obs("a")]),
        Err(IdentityError::Busy)
    ));
    drop(lock);
    let a = IdentityStore::new(d.path()).reconcile(&[obs("a")]).unwrap();
    let b = IdentityStore::new(d.path())
        .reconcile(&[obs("a"), obs("b")])
        .unwrap();
    assert!(b.monitors.iter().any(|m| m.uid == a.monitors[0].uid));
    assert_eq!(b.monitors.len(), 2);
}

#[test]
fn invalid_geometry_and_revision_overflow_do_not_publish_changes() {
    let d = tempfile::tempdir().unwrap();
    let st = IdentityStore::new(d.path());
    st.reconcile(&[obs("a")]).unwrap();
    let path = d.path().join("monitor-identities.toml");
    let original = std::fs::read(&path).unwrap();
    for scale in [f64::NAN, f64::INFINITY, 0., -1.] {
        let mut o = obs("a");
        o.scale = scale;
        assert!(matches!(
            st.reconcile(&[o]),
            Err(IdentityError::Unavailable)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    let text =
        String::from_utf8(original)
            .unwrap()
            .replacen("rev = 1", "rev = 18446744073709551615", 1);
    std::fs::write(&path, &text).unwrap();
    let mut o = obs("a");
    o.scale = 1.25;
    assert!(st.reconcile(&[o]).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
}
#[test]
fn ambiguous_aliases_are_not_exported_and_evidence_debug_is_redacted() {
    let d = tempfile::tempdir().unwrap();
    let a = obs("private-a");
    let mut b = obs("private-b");
    b.legacy_aliases = a.legacy_aliases.clone();
    let snapshot = IdentityStore::new(d.path())
        .reconcile(&[a.clone(), b])
        .unwrap();
    assert!(snapshot.aliases.is_empty());
    assert!(!format!("{:?}", a.evidence).contains("private-a"));
}
#[cfg(unix)]
#[test]
fn private_registry_and_lock_reject_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let d = tempfile::tempdir().unwrap();
    let victim = d.path().join("victim");
    std::fs::write(&victim, "do not touch").unwrap();
    symlink(&victim, d.path().join("monitor-identities.lock")).unwrap();
    assert!(matches!(
        IdentityStore::new(d.path()).reconcile(&[obs("a")]),
        Err(IdentityError::Io)
    ));
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "do not touch");
    std::fs::remove_file(d.path().join("monitor-identities.lock")).unwrap();
    IdentityStore::new(d.path()).reconcile(&[obs("a")]).unwrap();
    for file in ["monitor-identities.toml", "monitor-identities.lock"] {
        assert_eq!(
            std::fs::metadata(d.path().join(file))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn fix1_debug_redacts_aliases_through_public_containers() {
    use clipsync_input::{
        Endpoint, Layout, LayoutDevice, Link,
        layout_document::{LayoutDocument, LegacyBinding, Organization},
        layout_geometry::CompiledOrganization,
    };
    use clipsync_proto::{Edge, PeerId};
    let secret = "sensitive-device-path-and-alias";
    let observation = obs(secret);
    assert!(!format!("{observation:?}").contains(secret));
    let dir = tempfile::tempdir().unwrap();
    let snapshot = IdentityStore::new(dir.path())
        .reconcile(&[observation])
        .unwrap();
    assert_eq!(snapshot.aliases[0].alias, secret);
    assert!(!format!("{:?}", snapshot.aliases[0]).contains(secret));
    assert!(!format!("{snapshot:?}").contains(secret));
    let binding = LegacyBinding {
        peer: PeerId([1; 32]),
        alias: secret.into(),
        uid: snapshot.monitors[0].uid,
    };
    assert!(!format!("{binding:?}").contains(secret));
    let organization = Organization {
        legacy_bindings: vec![binding],
        ..Organization::default()
    };
    assert!(!format!("{organization:?}").contains(secret));
    let mut legacy = Layout::default();
    for peer in [1, 2] {
        legacy.devices.push(LayoutDevice {
            peer: PeerId([peer; 32]),
            name: "Máquina".into(),
            x: 0,
            y: 0,
            hotkey: None,
        });
    }
    legacy.links.push(Link {
        id: "public-link".into(),
        a: Endpoint {
            peer: PeerId([1; 32]),
            monitor: secret.into(),
            edge: Edge::Right,
            from: 0.,
            to: 1.,
        },
        b: Endpoint {
            peer: PeerId([2; 32]),
            monitor: secret.into(),
            edge: Edge::Left,
            from: 0.,
            to: 1.,
        },
    });
    let document = LayoutDocument {
        legacy,
        organization: Some(organization),
    };
    assert!(!format!("{document:?}").contains(secret));
    assert!(document.to_toml_string().unwrap().contains(secret));
    let compiled = CompiledOrganization {
        document,
        effective_links: vec![],
        preserved_link_ids: vec![],
        replaced_link_ids: vec![],
        unresolved_link_ids: vec![],
    };
    assert!(!format!("{compiled:?}").contains(secret));
}
