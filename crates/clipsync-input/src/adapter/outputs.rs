//! Output names for zones the portal does not name: DRM connectors reported as connected
//! (`/sys/class/drm/cardN-eDP-1/status`), sorted by connector order.
pub(crate) fn connected_outputs() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut out: Vec<String> = dir
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let status = std::fs::read_to_string(e.path().join("status")).ok()?;
            if status.trim() != "connected" {
                return None;
            }
            let (_, rest) = name.split_once('-')?;
            Some(rest.to_owned())
        })
        .collect();
    out.sort();
    out
}
