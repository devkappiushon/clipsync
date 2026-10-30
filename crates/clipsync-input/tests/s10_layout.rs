#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_input::{
    Layout, LayoutDevice,
    layout_document::{LayoutDocument, Organization},
    layout_geometry::{InventoryView, OrganizationDraft, compile_organization},
    layout_storage::LayoutStorage,
};
use clipsync_proto::{Edge, PeerId, monitors::*};
fn key(peer: u8, uid: u8) -> MonitorKey {
    MonitorKey {
        peer: PeerId([peer; 32]),
        uid: MonitorUid([uid; 16]),
    }
}
fn fixture() -> (LayoutDocument, OrganizationDraft, Vec<InventoryView>) {
    let doc = LayoutDocument {
        legacy: Layout {
            rev: 7,
            devices: vec![
                LayoutDevice {
                    peer: PeerId([1; 32]),
                    name: "Desktop".into(),
                    x: 20,
                    y: 30,
                    hotkey: Some(1),
                },
                LayoutDevice {
                    peer: PeerId([2; 32]),
                    name: "Notebook".into(),
                    x: 40,
                    y: 50,
                    hotkey: Some(2),
                },
            ],
            ..Layout::default()
        },
        organization: None,
    };
    let inv = vec![
        InventoryView {
            peer: PeerId([1; 32]),
            inventory: MonitorInventory {
                rev: 1,
                space: CoordinateSpace::Physical,
                source: GeometrySource::Observed,
                capabilities: MonitorCapabilities {
                    internal_edges: true,
                    positioned_return: true,
                    route_receiver: false,
                },
                monitors: vec![
                    MonitorInfo {
                        uid: key(1, 1).uid,
                        label: "A".into(),
                        rect: NativeRect {
                            x: -1920,
                            y: 0,
                            width: 1920,
                            height: 1080,
                        },
                        scale: 1.5,
                    },
                    MonitorInfo {
                        uid: key(1, 2).uid,
                        label: "B".into(),
                        rect: NativeRect {
                            x: 0,
                            y: 0,
                            width: 1920,
                            height: 1080,
                        },
                        scale: 1.5,
                    },
                ],
            },
            aliases: vec![],
        },
        InventoryView {
            peer: PeerId([2; 32]),
            inventory: MonitorInventory {
                rev: 1,
                space: CoordinateSpace::Logical,
                source: GeometrySource::Observed,
                capabilities: MonitorCapabilities {
                    internal_edges: false,
                    positioned_return: false,
                    route_receiver: true,
                },
                monitors: vec![MonitorInfo {
                    uid: key(2, 3).uid,
                    label: "N".into(),
                    rect: NativeRect {
                        x: 0,
                        y: 0,
                        width: 1280,
                        height: 720,
                    },
                    scale: 1.0,
                }],
            },
            aliases: vec![],
        },
    ];
    let draft = OrganizationDraft {
        positions: vec![
            MonitorPosition {
                key: key(1, 1),
                x: 0.,
                y: 0.,
            },
            MonitorPosition {
                key: key(2, 3),
                x: 1280.,
                y: 0.,
            },
            MonitorPosition {
                key: key(1, 2),
                x: 2560.,
                y: 0.,
            },
        ],
        replace_link_ids: vec![],
    };
    (doc, draft, inv)
}
#[test]
fn opening_v1_is_read_only_and_legacy_edit_stays_v1() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("layout.toml");
    std::fs::write(&p, "# original\nversion=1\nrev=3\n").unwrap();
    let bytes = std::fs::read(&p).unwrap();
    let mut doc = LayoutDocument::read(&p).unwrap().unwrap();
    assert!(doc.organization.is_none());
    assert_eq!(bytes, std::fs::read(&p).unwrap());
    doc.legacy.rev = 4;
    LayoutStorage::new(d.path())
        .try_lock()
        .unwrap()
        .write(&doc)
        .unwrap();
    assert!(Layout::read(&p).unwrap().is_some());
    assert!(!d.path().join("layout.v1.backup.toml").exists());
}
#[test]
fn unknown_layout_version_never_truncates() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("layout.toml");
    std::fs::write(&p, "version=99\nfuture=true\n").unwrap();
    let bytes = std::fs::read(&p).unwrap();
    let g = LayoutStorage::new(d.path()).try_lock().unwrap();
    assert_eq!(g.read().unwrap_err().code(), "unsupported_version");
    assert!(g.write(&fixture().0).is_err());
    assert!(g.restore_v1(1).is_err());
    assert_eq!(bytes, std::fs::read(&p).unwrap());
}
#[test]
fn migration_preserves_hotkeys_options_and_manual_links() {
    let (doc, draft, inv) = fixture();
    let compiled = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(compiled.document.legacy.devices, doc.legacy.devices);
    assert_eq!(compiled.document.legacy.options, doc.legacy.options);
    let text = compiled.document.to_toml_string().unwrap();
    assert!(text.contains("monitor_uid"));
    assert_eq!(
        LayoutDocument::from_toml_str(&text).unwrap(),
        compiled.document
    );
    let empty = LayoutDocument {
        organization: Some(Organization::default()),
        ..doc
    };
    assert!(
        LayoutDocument::from_toml_str(&empty.to_toml_string().unwrap())
            .unwrap()
            .organization
            .is_some()
    );
}
#[test]
fn backup_failure_cancels_migration() {
    let d = tempfile::tempdir().unwrap();
    let (doc, draft, inv) = fixture();
    let g = LayoutStorage::new(d.path()).try_lock().unwrap();
    g.write(&doc).unwrap();
    let p = d.path().join("layout.toml");
    let before = std::fs::read(&p).unwrap();
    std::fs::write(d.path().join("layout.v1.backup.toml"), "different").unwrap();
    assert_eq!(
        g.write(&compile_organization(&doc, &draft, &inv).unwrap().document)
            .unwrap_err()
            .code(),
        "backup_conflict"
    );
    assert_eq!(before, std::fs::read(p).unwrap());
}
#[test]
fn restore_preserves_v2_and_does_not_touch_identity() {
    let d = tempfile::tempdir().unwrap();
    let (doc, draft, inv) = fixture();
    let g = LayoutStorage::new(d.path()).try_lock().unwrap();
    g.write(&doc).unwrap();
    let mut next = compile_organization(&doc, &draft, &inv).unwrap().document;
    next.legacy.rev = 8;
    g.write(&next).unwrap();
    std::fs::write(d.path().join("monitor-identities.toml"), "private marker").unwrap();
    let before = std::fs::read(d.path().join("layout.toml")).unwrap();
    assert_eq!(g.restore_v1(7).unwrap_err().code(), "stale_rev");
    assert_eq!(g.restore_v1(8).unwrap(), 9);
    assert_eq!(
        std::fs::read(d.path().join("layout.v2-before-recovery.8.toml")).unwrap(),
        before
    );
    assert!(g.read().unwrap().unwrap().organization.is_none());
    assert_eq!(
        std::fs::read_to_string(d.path().join("monitor-identities.toml")).unwrap(),
        "private marker"
    );
}
#[test]
fn two_windows_monitors_surround_notebook_with_aligned_dead_corners() {
    let (doc, draft, inv) = fixture();
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(c.effective_links.len(), 2);
    for l in &c.effective_links {
        assert_ne!(l.a.key.peer, l.b.key.peer);
        assert_eq!(l.a.edge, Edge::Right);
        assert_eq!(l.b.edge, Edge::Left);
        assert!((l.a.from - 8. / 720.).abs() < 1e-6);
        assert!((l.b.from - 8. / 720.).abs() < 1e-6);
        assert!((l.a.to - 712. / 720.).abs() < 1e-6);
    }
}
#[test]
fn overlap_nonfinite_bounds_and_estimates_are_rejected() {
    let (doc, draft, inv) = fixture();
    for x in [0., f64::NAN, f64::INFINITY, 1_000_001., -1_000_001.] {
        let mut bad = draft.clone();
        bad.positions[1].x = x;
        assert!(compile_organization(&doc, &bad, &inv).is_err());
    }
    let mut bad = inv.clone();
    bad[1].inventory.source = GeometrySource::Estimated;
    assert_eq!(
        compile_organization(&doc, &draft, &bad).unwrap_err().code(),
        "unsupported_topology"
    );
}
#[test]
fn advanced_conflicts_and_arbitrary_replacements_are_refused() {
    let (mut doc, mut draft, inv) = fixture();
    doc.legacy.links.push(clipsync_input::Link {
        id: "manual".into(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: "A".into(),
            edge: Edge::Right,
            from: 0.,
            to: 1.,
        },
        b: clipsync_input::Endpoint {
            peer: key(2, 3).peer,
            monitor: "N".into(),
            edge: Edge::Left,
            from: 0.,
            to: 1.,
        },
    });
    // A has two monitors: a confirmed binding, never its label or index, resolves it.
    let mut inv = inv;
    inv[0]
        .aliases
        .push(clipsync_input::layout_document::LegacyBinding {
            peer: key(1, 1).peer,
            alias: "A".into(),
            uid: key(1, 1).uid,
        });
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "advanced_link_conflict"
    );
    draft.replace_link_ids.push("manual".into());
    assert!(compile_organization(&doc, &draft, &inv).is_err());
}

#[test]
fn exact_replacement_whitelist_is_required() {
    let (doc, mut draft, inv) = fixture();
    let first = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(
        compile_organization(&first.document, &draft, &inv)
            .unwrap_err()
            .code(),
        "advanced_link_conflict"
    );
    draft.replace_link_ids = first.effective_links.iter().map(|l| l.id.clone()).collect();
    let second = compile_organization(&first.document, &draft, &inv).unwrap();
    assert_eq!(second.document, first.document);
    draft
        .replace_link_ids
        .push(draft.replace_link_ids[0].clone());
    assert!(compile_organization(&first.document, &draft, &inv).is_err());
}
#[test]
fn different_dpi_and_partial_contact_keep_native_origin_out_of_canvas() {
    let (doc, mut draft, mut inv) = fixture();
    draft.positions.pop();
    inv[0].inventory.monitors[0].scale = 1.25;
    draft.positions[1].x = 1536.;
    draft.positions[1].y = 100.;
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    let l = &c.effective_links[0];
    assert!((l.a.from - 0.125).abs() < 1e-6); // (100+8)/864, never native x=-1920 / DPI
    assert!((l.a.to - 812. / 864.).abs() < 1e-6);
    assert!((l.b.from - 8. / 720.).abs() < 1e-6);
    assert!((l.b.to - 712. / 720.).abs() < 1e-6);
    assert_eq!(
        c.document
            .organization
            .unwrap()
            .positions
            .iter()
            .find(|p| p.key == key(1, 1))
            .unwrap()
            .x,
        0.
    );
}
#[test]
fn native_overflow_zero_dimensions_scales_and_duplicate_inventory_fail() {
    let (doc, draft, inv) = fixture();
    for (width, scale, x) in [
        (0, 1., 0),
        (1920, 0., 0),
        (1920, f64::NAN, 0),
        (1920, f64::INFINITY, 0),
        (1920, 1., i32::MAX),
        (1920, f64::MIN_POSITIVE, 0),
    ] {
        let mut bad = inv.clone();
        let m = &mut bad[0].inventory.monitors[0];
        m.rect.width = width;
        m.rect.x = x;
        m.scale = scale;
        assert_eq!(
            compile_organization(&doc, &draft, &bad).unwrap_err().code(),
            "invalid_geometry"
        );
    }
    let mut bad = inv.clone();
    bad[0].inventory.monitors[1].uid = key(1, 1).uid;
    assert_eq!(
        compile_organization(&doc, &draft, &bad).unwrap_err().code(),
        "identity_ambiguous"
    );
    let mut bad = inv;
    bad[0].inventory.monitors.remove(0);
    assert_eq!(
        compile_organization(&doc, &draft, &bad).unwrap_err().code(),
        "monitor_missing"
    );
}
#[test]
fn empty_after_dead_corner_and_f32_rounding_are_rejected() {
    let (mut doc, mut draft, inv) = fixture();
    draft.positions.pop();
    draft.positions[1].y = 719.;
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "empty_segment"
    );
    doc.legacy.options.dead_corner_px = 0;
    draft.positions[1].y = 720. - 0.000001;
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "empty_segment"
    );
}
#[test]
fn bottom_top_contact_and_gap_do_not_invent_barriers() {
    let (doc, mut draft, inv) = fixture();
    draft.positions.pop();
    draft.positions[1].x = 0.;
    draft.positions[1].y = 720.;
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(c.effective_links[0].a.edge, Edge::Bottom);
    assert_eq!(c.effective_links[0].b.edge, Edge::Top);
    draft.positions[1].y = 721.;
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "unsupported_topology"
    );
}
#[test]
fn unresolved_references_and_other_peers_are_preserved() {
    let (mut doc, draft, inv) = fixture();
    doc.legacy.devices.push(LayoutDevice {
        peer: PeerId([3; 32]),
        name: "Outro".into(),
        x: 99,
        y: 99,
        hotkey: Some(3),
    });
    doc.legacy.links.push(clipsync_input::Link {
        id: "unresolved".into(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: "#0".into(),
            edge: Edge::Right,
            from: 0.,
            to: 1.,
        },
        b: clipsync_input::Endpoint {
            peer: PeerId([3; 32]),
            monitor: "other".into(),
            edge: Edge::Left,
            from: 0.,
            to: 1.,
        },
    });
    doc.organization = Some(Organization {
        positions: vec![MonitorPosition {
            key: key(3, 7),
            x: 99.,
            y: 123.,
        }],
        ..Organization::default()
    });
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(c.document.legacy, doc.legacy);
    assert_eq!(c.preserved_link_ids, vec!["unresolved"]);
    assert!(
        c.document
            .organization
            .unwrap()
            .positions
            .iter()
            .any(|p| p.key == key(3, 7) && p.y == 123.)
    );
}
#[test]
fn identical_legacy_is_adopted_only_with_confirmed_identity_and_reported() {
    let (mut doc, draft, mut inv) = fixture();
    doc.legacy.links.push(clipsync_input::Link {
        id: "manual-equal".into(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: "A".into(),
            edge: Edge::Right,
            from: 8. / 720.,
            to: 712. / 720.,
        },
        b: clipsync_input::Endpoint {
            peer: key(2, 3).peer,
            monitor: "N".into(),
            edge: Edge::Left,
            from: 8. / 720.,
            to: 712. / 720.,
        },
    });
    // An unresolved endpoint preserves the whole legacy link as inactive.
    let unresolved = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(unresolved.unresolved_link_ids, vec!["manual-equal"]);
    assert_eq!(unresolved.document.legacy.links, doc.legacy.links);
    inv[0]
        .aliases
        .push(clipsync_input::layout_document::LegacyBinding {
            peer: key(1, 1).peer,
            alias: "A".into(),
            uid: key(1, 1).uid,
        });
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    assert!(c.document.legacy.links.is_empty());
    assert_eq!(c.replaced_link_ids, vec!["manual-equal"]);
    assert!(c.effective_links.iter().any(|l| l.id == "manual-equal"));
}
#[test]
fn backup_retry_requires_exact_original_and_preserves_exclusive_recovery() {
    let d = tempfile::tempdir().unwrap();
    let (doc, draft, inv) = fixture();
    let storage = LayoutStorage::new(d.path());
    let guard = storage.try_lock().unwrap();
    assert_eq!(storage.try_lock().err().unwrap().code(), "busy");
    guard.write(&doc).unwrap();
    let bytes = std::fs::read(d.path().join("layout.toml")).unwrap();
    std::fs::write(d.path().join("layout.v1.backup.toml"), &bytes).unwrap();
    let mut next = compile_organization(&doc, &draft, &inv).unwrap().document;
    next.legacy.rev = 8;
    guard.write(&next).unwrap();
    std::fs::write(
        d.path().join("layout.v2-before-recovery.8.toml"),
        "existing",
    )
    .unwrap();
    assert!(guard.restore_v1(8).is_err());
    assert_eq!(guard.read().unwrap(), Some(next));
}
#[cfg(unix)]
#[test]
fn all_persisted_files_are_private_including_reused_backup() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let (doc, draft, inv) = fixture();
    let g = LayoutStorage::new(d.path()).try_lock().unwrap();
    g.write(&doc).unwrap();
    let backup = d.path().join("layout.v1.backup.toml");
    std::fs::copy(d.path().join("layout.toml"), &backup).unwrap();
    std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o644)).unwrap();
    g.write(&compile_organization(&doc, &draft, &inv).unwrap().document)
        .unwrap();
    for file in ["layout.toml", "layout.lock", "layout.v1.backup.toml"] {
        assert_eq!(
            std::fs::metadata(d.path().join(file))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(
        std::fs::metadata(d.path()).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[test]
fn generated_ids_are_compact_stable_and_never_overwrite_preserved_ids() {
    let (doc, mut draft, inv) = fixture();
    let original = compile_organization(&doc, &draft, &inv).unwrap();
    assert!(original.effective_links.iter().all(|l| l.id.len() == 64));
    draft.positions.reverse();
    let reversed = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(original.effective_links, reversed.effective_links);
    let mut doc = doc;
    doc.legacy.links.push(clipsync_input::Link {
        id: original.effective_links[0].id.clone(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: "#0".into(),
            edge: Edge::Top,
            from: 0.,
            to: 1.,
        },
        b: clipsync_input::Endpoint {
            peer: key(2, 3).peer,
            monitor: "#0".into(),
            edge: Edge::Bottom,
            from: 0.,
            to: 1.,
        },
    });
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "advanced_link_conflict"
    );
}

#[test]
fn resolved_advanced_link_on_another_edge_is_preserved() {
    let (mut doc, draft, mut inv) = fixture();
    inv[0]
        .aliases
        .push(clipsync_input::layout_document::LegacyBinding {
            peer: key(1, 1).peer,
            alias: "A".into(),
            uid: key(1, 1).uid,
        });
    doc.legacy.links.push(clipsync_input::Link {
        id: "advanced-top".into(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: "A".into(),
            edge: Edge::Top,
            from: 0.25,
            to: 0.75,
        },
        b: clipsync_input::Endpoint {
            peer: key(2, 3).peer,
            monitor: "N".into(),
            edge: Edge::Bottom,
            from: 0.25,
            to: 0.75,
        },
    });
    let c = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(c.document.legacy, doc.legacy);
    assert_eq!(c.preserved_link_ids, vec!["advanced-top"]);
    assert!(c.unresolved_link_ids.is_empty());
}
#[test]
fn restoration_overflow_and_backup_directory_preserve_active_layout() {
    let d = tempfile::tempdir().unwrap();
    let (doc, draft, inv) = fixture();
    let g = LayoutStorage::new(d.path()).try_lock().unwrap();
    g.write(&doc).unwrap();
    let before = std::fs::read(d.path().join("layout.toml")).unwrap();
    std::fs::create_dir(d.path().join("layout.v1.backup.toml")).unwrap();
    assert!(
        g.write(&compile_organization(&doc, &draft, &inv).unwrap().document)
            .is_err()
    );
    assert_eq!(std::fs::read(d.path().join("layout.toml")).unwrap(), before);
    std::fs::remove_dir(d.path().join("layout.v1.backup.toml")).unwrap();
    let mut next = compile_organization(&doc, &draft, &inv).unwrap().document;
    next.legacy.rev = u64::MAX;
    g.write(&next).unwrap();
    assert_eq!(g.restore_v1(u64::MAX).unwrap_err().code(), "invalid_field");
    assert_eq!(g.read().unwrap(), Some(next));
}

fn legacy_pair(id: &str, a: &str, b: &str, a_edge: Edge, b_edge: Edge) -> clipsync_input::Link {
    clipsync_input::Link {
        id: id.into(),
        a: clipsync_input::Endpoint {
            peer: key(1, 1).peer,
            monitor: a.into(),
            edge: a_edge,
            from: 0.,
            to: 1.,
        },
        b: clipsync_input::Endpoint {
            peer: key(2, 3).peer,
            monitor: b.into(),
            edge: b_edge,
            from: 0.,
            to: 1.,
        },
    }
}
fn bind_alias(inv: &mut [InventoryView], index: usize, alias: &str, uid: u8) {
    inv[index]
        .aliases
        .push(clipsync_input::layout_document::LegacyBinding {
            peer: inv[index].peer,
            alias: alias.into(),
            uid: MonitorUid([uid; 16]),
        });
}
#[test]
fn fix1_legacy_raw_full_edge_adopts_only_equal_effective_geometry() {
    let (mut doc, draft, mut inv) = fixture();
    for m in &mut inv[0].inventory.monitors {
        m.rect.width = 1280;
        m.rect.height = 720;
        m.scale = 1.;
    }
    bind_alias(&mut inv, 0, "A", 1);
    doc.legacy
        .links
        .push(legacy_pair("kept-id", "A", "N", Edge::Right, Edge::Left));
    let compiled = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(compiled.replaced_link_ids, vec!["kept-id"]);
    assert!(compiled.document.legacy.links.is_empty());
    let adopted = compiled
        .effective_links
        .iter()
        .find(|l| l.id == "kept-id")
        .unwrap();
    assert_eq!((adopted.a.from, adopted.a.to), (8. / 720., 712. / 720.));
    assert_eq!((adopted.b.from, adopted.b.to), (8. / 720., 712. / 720.));
    assert_eq!(doc.legacy.links[0].a.from, 0.);
    // The former native height has a smaller dead corner fraction: not equivalent.
    inv[0].inventory.monitors[0].rect.width = 1920;
    inv[0].inventory.monitors[0].rect.height = 1080;
    inv[0].inventory.monitors[0].scale = 1.5;
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "advanced_link_conflict"
    );
}

#[test]
fn fix1_alias_overlap_is_refused_before_adoption_or_preservation_in_either_order() {
    for (a_edge, b_edge) in [(Edge::Right, Edge::Left), (Edge::Top, Edge::Bottom)] {
        for reverse in [false, true] {
            let (mut doc, draft, mut inv) = fixture();
            for m in &mut inv[0].inventory.monitors {
                m.rect.width = 1280;
                m.rect.height = 720;
                m.scale = 1.;
            }
            for (index, alias, uid) in [(0, "A", 1), (0, "#0", 1), (1, "N", 3), (1, "#0", 3)] {
                bind_alias(&mut inv, index, alias, uid);
            }
            doc.legacy.links = vec![
                legacy_pair("first", "A", "N", a_edge, b_edge),
                legacy_pair("second", "#0", "#0", a_edge, b_edge),
            ];
            if reverse {
                doc.legacy.links.reverse();
            }
            doc.validate().unwrap();
            let before = doc.clone();
            assert_eq!(
                compile_organization(&doc, &draft, &inv).unwrap_err().code(),
                "segment_overlap"
            );
            assert_eq!(doc, before);
        }
    }
}
#[test]
fn fix1_resolved_legacy_overlap_with_preserved_managed_link_is_refused() {
    let (mut doc, draft, mut inv) = fixture();
    bind_alias(&mut inv, 0, "A", 1);
    doc.legacy.devices.push(LayoutDevice {
        peer: key(3, 4).peer,
        name: "Outro".into(),
        x: 0,
        y: 0,
        hotkey: None,
    });
    doc.organization = Some(Organization {
        links: vec![MonitorLink {
            id: "preserved".into(),
            a: MonitorEndpoint {
                key: key(1, 1),
                edge: Edge::Top,
                from: 0.25,
                to: 0.75,
            },
            b: MonitorEndpoint {
                key: key(3, 4),
                edge: Edge::Bottom,
                from: 0.25,
                to: 0.75,
            },
        }],
        ..Organization::default()
    });
    doc.legacy
        .links
        .push(legacy_pair("legacy-top", "A", "N", Edge::Top, Edge::Bottom));
    assert_eq!(
        compile_organization(&doc, &draft, &inv).unwrap_err().code(),
        "segment_overlap"
    );
}
#[test]
fn fix1_distinct_effective_segments_keep_both_alias_ids() {
    let (mut doc, draft, mut inv) = fixture();
    for (index, alias, uid) in [(0, "A", 1), (0, "#0", 1), (1, "N", 3), (1, "#0", 3)] {
        bind_alias(&mut inv, index, alias, uid);
    }
    let mut first = legacy_pair("first", "A", "N", Edge::Top, Edge::Bottom);
    first.a.to = 0.4;
    first.b.to = 0.4;
    let mut second = legacy_pair("second", "#0", "#0", Edge::Top, Edge::Bottom);
    second.a.from = 0.6;
    second.b.from = 0.6;
    doc.legacy.links = vec![first, second];
    let compiled = compile_organization(&doc, &draft, &inv).unwrap();
    assert_eq!(compiled.preserved_link_ids, vec!["first", "second"]);
    assert_eq!(compiled.document.legacy.links, doc.legacy.links);
}

#[test]
fn fix2_unresolved_link_preserves_even_an_empty_known_segment_in_both_orders() {
    for reverse in [false, true] {
        let (mut doc, draft, mut inv) = fixture();
        bind_alias(&mut inv, 0, "A", 1);
        let mut old = legacy_pair("inactive-legacy", "A", "#missing", Edge::Top, Edge::Bottom);
        old.a.to = 0.001; // Entirely inside the 8-pixel corner on a 1920-pixel edge.
        if reverse {
            std::mem::swap(&mut old.a, &mut old.b);
        }
        doc.legacy.links.push(old);
        doc.validate().unwrap();
        let compiled = compile_organization(&doc, &draft, &inv).unwrap();
        assert_eq!(compiled.document.legacy, doc.legacy);
        assert_eq!(compiled.preserved_link_ids, vec!["inactive-legacy"]);
        assert_eq!(compiled.unresolved_link_ids, vec!["inactive-legacy"]);
        assert!(compiled.replaced_link_ids.is_empty());
        assert_eq!(compiled.effective_links.len(), 2);
        assert!(
            compiled
                .effective_links
                .iter()
                .all(|l| l.id != "inactive-legacy")
        );
        // An inactive legacy link must not hide an invalid newly generated segment.
        let mut invalid = draft;
        invalid.positions[1].y = 719.;
        assert_eq!(
            compile_organization(&doc, &invalid, &inv)
                .unwrap_err()
                .code(),
            "empty_segment"
        );
    }
}
#[test]
fn fix2_fully_resolved_empty_segment_still_fails_in_both_orders() {
    for reverse in [false, true] {
        let (mut doc, draft, mut inv) = fixture();
        bind_alias(&mut inv, 0, "A", 1);
        let mut old = legacy_pair("known-empty", "A", "N", Edge::Top, Edge::Bottom);
        old.a.to = 0.001;
        if reverse {
            std::mem::swap(&mut old.a, &mut old.b);
        }
        doc.legacy.links.push(old);
        assert_eq!(
            compile_organization(&doc, &draft, &inv).unwrap_err().code(),
            "empty_segment"
        );
    }
}
#[test]
fn fix2_fully_resolved_valid_segment_remains_preserved_and_resolved() {
    for reverse in [false, true] {
        let (mut doc, draft, mut inv) = fixture();
        bind_alias(&mut inv, 0, "A", 1);
        let mut old = legacy_pair("known-valid", "A", "N", Edge::Top, Edge::Bottom);
        old.a.from = 0.25;
        old.a.to = 0.75;
        if reverse {
            std::mem::swap(&mut old.a, &mut old.b);
        }
        doc.legacy.links.push(old);
        let compiled = compile_organization(&doc, &draft, &inv).unwrap();
        assert_eq!(compiled.document.legacy, doc.legacy);
        assert_eq!(compiled.preserved_link_ids, vec!["known-valid"]);
        assert!(compiled.unresolved_link_ids.is_empty());
        assert!(compiled.replaced_link_ids.is_empty());
        assert_eq!(compiled.effective_links.len(), 2);
    }
}
