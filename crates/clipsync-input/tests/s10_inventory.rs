use clipsync_input::inventory::{linux_observation, windows_observation};
use clipsync_input::{CaptureAdapter, FakeCapture, fake_zone};
use clipsync_proto::GeometrySource;
use clipsync_win::Monitor;

fn monitor(path: &str, x: i32) -> Monitor {
    Monitor {
        path: path.into(),
        device: "display".into(),
        friendly: "Monitor".into(),
        x,
        y: 0,
        width: 1920,
        height: 1080,
        dpi: 144,
        primary: false,
    }
}
#[test]
fn windows_keeps_path_and_geometry_in_one_snapshot_and_refuses_ambiguity() {
    let observation =
        windows_observation(&[monitor("first", -1920), monitor("second", 0)]).unwrap();
    assert_eq!(observation.source, GeometrySource::Observed);
    assert_eq!(observation.observations[0].rect.x, -1920);
    assert_eq!(observation.observations[0].scale, 1.5);
    assert!(windows_observation(&[monitor("", 0)]).is_err());
    assert!(windows_observation(&[monitor("same", 0), monitor("same", 1920)]).is_err());
}
#[test]
fn linux_requires_exactly_one_connected_connector_and_one_observed_region() {
    let dir = tempfile::tempdir().unwrap();
    let connector = dir.path().join("card0-eDP-1");
    std::fs::create_dir(&connector).unwrap();
    std::fs::write(connector.join("status"), "connected\n").unwrap();
    std::fs::write(connector.join("edid"), b"private-edid").unwrap();
    let observation = linux_observation(dir.path(), &[fake_zone()]).unwrap();
    assert_eq!(observation.source, GeometrySource::Observed);
    assert!(!observation.capabilities.positioned_return);
    assert!(!format!("{observation:?}").contains("private-edid"));
    assert!(linux_observation(dir.path(), &[fake_zone(), fake_zone()]).is_err());
    let other = dir.path().join("card1-HDMI-A-1");
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("status"), "connected").unwrap();
    assert!(linux_observation(dir.path(), &[fake_zone()]).is_err());
}
#[tokio::test]
async fn fake_never_claims_observed_inventory_without_explicit_test_configuration() {
    let (mut cap, handle) = FakeCapture::new(vec![fake_zone()]);
    assert!(cap.known_monitor_observation().is_err());
    assert!(cap.prepare_monitor_observation().await.is_err());
    assert!(!handle.started());
    assert!(handle.barriers().is_empty());
}
#[test]
fn drm_modes_are_only_estimates_and_cannot_advertise_receiver_readiness() {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("card0-eDP-1");
    std::fs::create_dir(&c).unwrap();
    std::fs::write(c.join("status"), "connected").unwrap();
    std::fs::write(c.join("modes"), "1000x800\n").unwrap();
    let estimate = clipsync_input::inventory::linux_estimated_observation(dir.path()).unwrap();
    assert_eq!(estimate.source, GeometrySource::Estimated);
    assert!(!estimate.capabilities.route_receiver);
}
