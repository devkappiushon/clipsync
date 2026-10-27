use clipsync_proto::*;

#[test]
fn peer_id_hex_roundtrip_and_short_form() {
    let id = PeerId::from_spki_der(b"spki");
    let hex = id.to_hex();
    assert_eq!(hex.len(), 64);
    assert_eq!(PeerId::from_hex(&hex), Ok(id));
    let short = id.short();
    assert_eq!(short.len(), 19);
    assert_eq!(short.replace('-', ""), hex[..16]);
    assert_eq!(short.matches('-').count(), 3);
    assert_eq!(id.to_string(), short);
    assert_eq!(format!("{id:?}"), short);
}

#[test]
fn peer_id_from_hex_errors() {
    assert_eq!(PeerId::from_hex("abc"), Err(IdError::Length));
    assert_eq!(PeerId::from_hex(&"zz".repeat(32)), Err(IdError::Hex));
}

#[test]
fn peer_id_is_sha256_of_spki() {
    // SHA-256("abc")
    let id = PeerId::from_spki_der(b"abc");
    assert_eq!(
        id.to_hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn event_ids_are_random() {
    let a = EventId::random();
    let b = EventId::random();
    assert_ne!(a, b);
    assert_eq!(a.short().len(), 8);
}

#[test]
fn module_set() {
    let s = ModuleSet {
        clipboard: true,
        input: false,
    };
    assert!(s.contains(ModuleKind::Clipboard));
    assert!(!s.contains(ModuleKind::Input));
    assert!(!s.is_empty());
    assert!(ModuleSet::default().is_empty());
}

#[test]
fn mono_us_is_monotonic() {
    let a = mono_us();
    let b = mono_us();
    assert!(a > 0 && b >= a);
}

#[test]
fn reject_reasons_have_portuguese_text() {
    assert_eq!(RejectReason::Paused.pt(), "pausado");
    assert_eq!(RejectReason::ModuleDisabled.pt(), "módulo desligado");
    assert!(!clipsync_proto::msg::MODULE_DISABLED.is_empty());
}
