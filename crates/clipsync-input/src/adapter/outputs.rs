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

/// Zones estimated from DRM without any portal call: connected connectors with the first line
/// of `modes` (the preferred mode) as size, laid side by side from x = 0, scale 1. Exact for a
/// single monitor; a multi-monitor desktop gets the real geometry once a capture session exists.
#[cfg(target_os = "linux")]
pub(crate) fn sysfs_zones() -> Vec<super::Zone> {
    sysfs_zones_at(std::path::Path::new("/sys/class/drm"))
}

pub(crate) fn sysfs_zones_at(root: &std::path::Path) -> Vec<super::Zone> {
    let Ok(dir) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<(String, u32, u32)> = dir
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let status = std::fs::read_to_string(e.path().join("status")).ok()?;
            if status.trim() != "connected" {
                return None;
            }
            let (_, rest) = name.split_once('-')?;
            let modes = std::fs::read_to_string(e.path().join("modes")).ok()?;
            let (w, h) = modes.lines().next()?.trim().split_once('x')?;
            Some((rest.to_owned(), w.parse().ok()?, h.parse().ok()?))
        })
        .filter(|(_, w, h)| *w > 0 && *h > 0)
        .collect();
    found.sort();
    let mut x = 0i32;
    found
        .into_iter()
        .enumerate()
        .map(|(i, (name, width, height))| {
            let z = super::Zone {
                index: i as u32,
                name: Some(name),
                x,
                y: 0,
                width,
                height,
                scale: 1.0,
            };
            x = x.saturating_add(i32::try_from(width).unwrap_or(i32::MAX));
            z
        })
        .collect()
}
