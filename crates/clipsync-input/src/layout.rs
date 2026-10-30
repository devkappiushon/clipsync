//! `layout.toml`: the single source of the screen layout (S6). Devices with their position on the
//! canvas, links between monitor-edge segments, and options. Pure data and validation: the
//! derivation of barriers and the Tauri commands live elsewhere.
use crate::config::{EdgeRaw, InputConfig};
use crate::msg;
use clipsync_proto::{Edge, MAX_NAME_BYTES, PeerId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub const LAYOUT_VERSION: u32 = 1;
pub const DEFAULT_DEAD_CORNER_PX: u32 = 8;
pub const MAX_DEAD_CORNER_PX: u32 = 32;
pub const MAX_HOTKEY: u8 = 9;
/// How far from the origin a device may sit on the canvas, on each axis.
pub const MAX_CANVAS_PX: i32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutOptions {
    pub dead_corner_px: u32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            dead_corner_px: DEFAULT_DEAD_CORNER_PX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutDevice {
    pub peer: PeerId,
    pub name: String,
    /// Canvas position in logical px. Visual only: it never changes a barrier.
    pub x: i32,
    pub y: i32,
    /// `Win+Alt+<n>` jumps to this device (1..=9).
    pub hotkey: Option<u8>,
}

/// One side of a link: a segment `from..to` (0..1) of one edge of one monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub peer: PeerId,
    /// A connector name or `"#idx"` (see `config::zone_ref`).
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
}

/// Undirected: it produces one barrier on each machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub id: String,
    pub a: Endpoint,
    pub b: Endpoint,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    pub rev: u64,
    pub options: LayoutOptions,
    pub devices: Vec<LayoutDevice>,
    pub links: Vec<Link>,
}

/// The barrier one machine installs for one link, with the destination side it leads to
/// (contract §12.6 items 2 and 4). The other machine derives the mirror from the same link.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedBarrier {
    pub link: String,
    /// Local side.
    pub monitor: String,
    pub edge: Edge,
    pub from: f32,
    pub to: f32,
    /// Destination side.
    pub peer: PeerId,
    pub peer_monitor: String,
    pub peer_edge: Edge,
    pub peer_from: f32,
    pub peer_to: f32,
}

impl DerivedBarrier {
    fn new(link: &str, local: &Endpoint, remote: &Endpoint) -> Self {
        Self {
            link: link.to_owned(),
            monitor: local.monitor.clone(),
            edge: local.edge,
            from: local.from,
            to: local.to,
            peer: remote.peer,
            peer_monitor: remote.monitor.clone(),
            peer_edge: remote.edge,
            peer_from: remote.from,
            peer_to: remote.to,
        }
    }

    /// Position on the destination edge for a cursor at `pos` on the local edge.
    pub fn map_pos(&self, pos: f32) -> f32 {
        crate::config::map_pos((self.from, self.to), (self.peer_from, self.peer_to), pos)
    }

    /// Middle of the destination segment: where a hotkey switch lands.
    pub fn peer_middle(&self) -> f32 {
        (self.peer_from + self.peer_to) / 2.0
    }
}

/// Dead corners (`options.dead_corner_px`): the segment `from..to` of an edge `len_px` long
/// loses `dead_px` at each end of the EDGE (not of the segment). `None` when nothing is left.
pub fn trim_dead_corner(from: f32, to: f32, len_px: u32, dead_px: u32) -> Option<(f32, f32)> {
    if len_px == 0 {
        return None;
    }
    let d = dead_px as f32 / len_px as f32;
    let (a, b) = (from.max(d), to.min(1.0 - d));
    (a < b).then_some((a, b))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    Parse(String),
    Io(String),
    UnsupportedVersion(u32),
    InvalidField(String),
    DuplicateId(String),
    UnknownDevice {
        link: String,
    },
    SameDevice {
        link: String,
    },
    EmptySegment {
        link: String,
    },
    SegmentOutOfRange {
        link: String,
    },
    SegmentOverlap {
        link: String,
        other: String,
    },
    ManualBarrierConflict,
    Busy,
    BackupConflict,
    StaleRev,
    /// The new revision is visible, but the directory sync failed after rename.
    PublishedUnconfirmed {
        rev: u64,
    },
}

impl LayoutError {
    /// Stable code for the UI (`details` carries the ids).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse(_) => "parse",
            Self::Io(_) => "io",
            Self::UnsupportedVersion(_) => "unsupported_version",
            Self::InvalidField(_) => "invalid_field",
            Self::DuplicateId(_) => "duplicate_id",
            Self::UnknownDevice { .. } => "unknown_device",
            Self::SameDevice { .. } => "same_device",
            Self::EmptySegment { .. } => "empty_segment",
            Self::SegmentOutOfRange { .. } => "segment_out_of_range",
            Self::SegmentOverlap { .. } => "segment_overlap",
            Self::ManualBarrierConflict => "manual_barrier_conflict",
            Self::Busy => "busy",
            Self::BackupConflict => "backup_conflict",
            Self::StaleRev => "stale_rev",
            Self::PublishedUnconfirmed { .. } => "published_unconfirmed",
        }
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Parse(d) => msg::layout_parse(d),
            Self::Io(d) => msg::layout_io(d),
            Self::UnsupportedVersion(v) => msg::layout_unsupported_version(*v),
            Self::InvalidField(d) => msg::layout_invalid_field(d),
            Self::DuplicateId(id) => msg::layout_duplicate_id(id),
            Self::UnknownDevice { link } => msg::layout_unknown_device(link),
            Self::SameDevice { link } => msg::layout_same_device(link),
            Self::EmptySegment { link } => msg::layout_empty_segment(link),
            Self::SegmentOutOfRange { link } => msg::layout_out_of_range(link),
            Self::SegmentOverlap { link, other } => msg::layout_overlap(link, other),
            Self::ManualBarrierConflict => msg::LAYOUT_MANUAL_CONFLICT.to_owned(),
            Self::Busy => "layout em uso".into(),
            Self::BackupConflict => "backup anterior difere do layout atual".into(),
            Self::StaleRev => "revisão do layout mudou".into(),
            Self::PublishedUnconfirmed { rev } => {
                format!("revisão {rev} publicada; durabilidade não confirmada")
            }
        };
        f.write_str(&s)
    }
}

impl std::error::Error for LayoutError {}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawLayout {
    version: u32,
    #[serde(default)]
    rev: u64,
    #[serde(default)]
    options: RawOptions,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    device: Vec<RawDevice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    link: Vec<RawLink>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawOptions {
    #[serde(default = "default_dead_corner")]
    dead_corner_px: u32,
}

fn default_dead_corner() -> u32 {
    DEFAULT_DEAD_CORNER_PX
}

impl Default for RawOptions {
    fn default() -> Self {
        Self {
            dead_corner_px: DEFAULT_DEAD_CORNER_PX,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawDevice {
    peer: String,
    name: String,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hotkey: Option<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawLink {
    id: String,
    a: RawEndpoint,
    b: RawEndpoint,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawEndpoint {
    peer: String,
    monitor: String,
    edge: EdgeRaw,
    from: f64,
    to: f64,
}

fn invalid(detail: String) -> LayoutError {
    LayoutError::InvalidField(detail)
}

/// `0.1_f32` is written as `0.1`, not as `0.10000000149011612`.
fn clean(v: f32) -> f64 {
    format!("{v}").parse().unwrap_or(f64::from(v))
}

fn peer_of(s: &str) -> Result<PeerId, LayoutError> {
    PeerId::from_hex(s).map_err(|_| invalid(msg::layout_bad_peer(s)))
}

impl RawEndpoint {
    fn parse(self) -> Result<Endpoint, LayoutError> {
        Ok(Endpoint {
            peer: peer_of(&self.peer)?,
            monitor: self.monitor,
            edge: self.edge.into(),
            from: self.from as f32,
            to: self.to as f32,
        })
    }
}

impl From<&Endpoint> for RawEndpoint {
    fn from(e: &Endpoint) -> Self {
        Self {
            peer: e.peer.to_hex(),
            monitor: e.monitor.clone(),
            edge: e.edge.into(),
            from: clean(e.from),
            to: clean(e.to),
        }
    }
}

impl Endpoint {
    fn overlaps(&self, o: &Endpoint) -> bool {
        self.peer == o.peer
            && self.monitor == o.monitor
            && self.edge == o.edge
            && self.from < o.to
            && o.from < self.to
    }

    fn in_range(&self) -> bool {
        let ok = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        ok(self.from) && ok(self.to)
    }
}

impl Layout {
    /// Parses and validates. Unknown keys are an error.
    pub fn from_toml_str(s: &str) -> Result<Self, LayoutError> {
        let raw: RawLayout = toml::from_str(s).map_err(|e| LayoutError::Parse(e.to_string()))?;
        if raw.version != LAYOUT_VERSION {
            return Err(LayoutError::UnsupportedVersion(raw.version));
        }
        let mut devices = Vec::new();
        for d in raw.device {
            devices.push(LayoutDevice {
                peer: peer_of(&d.peer)?,
                name: d.name,
                x: d.x,
                y: d.y,
                hotkey: d.hotkey,
            });
        }
        let mut links = Vec::new();
        for l in raw.link {
            links.push(Link {
                id: l.id,
                a: l.a.parse()?,
                b: l.b.parse()?,
            });
        }
        let layout = Self {
            rev: raw.rev,
            options: LayoutOptions {
                dead_corner_px: raw.options.dead_corner_px,
            },
            devices,
            links,
        };
        layout.validate()?;
        Ok(layout)
    }

    pub fn to_toml_string(&self) -> Result<String, LayoutError> {
        let raw = RawLayout {
            version: LAYOUT_VERSION,
            rev: self.rev,
            options: RawOptions {
                dead_corner_px: self.options.dead_corner_px,
            },
            device: self
                .devices
                .iter()
                .map(|d| RawDevice {
                    peer: d.peer.to_hex(),
                    name: d.name.clone(),
                    x: d.x,
                    y: d.y,
                    hotkey: d.hotkey,
                })
                .collect(),
            link: self
                .links
                .iter()
                .map(|l| RawLink {
                    id: l.id.clone(),
                    a: (&l.a).into(),
                    b: (&l.b).into(),
                })
                .collect(),
        };
        toml::to_string(&raw).map_err(|e| LayoutError::Parse(e.to_string()))
    }

    /// Structural rules; also run by `from_toml_str`, and by the commands before they write.
    pub fn validate(&self) -> Result<(), LayoutError> {
        if self.options.dead_corner_px > MAX_DEAD_CORNER_PX {
            return Err(invalid(msg::layout_bad_dead_corner(MAX_DEAD_CORNER_PX)));
        }
        for (i, d) in self.devices.iter().enumerate() {
            let short = d.peer.short();
            if d.name.is_empty() || d.name.len() > MAX_NAME_BYTES {
                return Err(invalid(msg::layout_bad_name(&short)));
            }
            if self.devices[..i].iter().any(|o| o.peer == d.peer) {
                return Err(invalid(msg::layout_dup_device(&short)));
            }
            if d.x.unsigned_abs() > MAX_CANVAS_PX.unsigned_abs()
                || d.y.unsigned_abs() > MAX_CANVAS_PX.unsigned_abs()
            {
                return Err(invalid(msg::layout_bad_position(&short, MAX_CANVAS_PX)));
            }
            if let Some(n) = d.hotkey {
                if !(1..=MAX_HOTKEY).contains(&n) {
                    return Err(invalid(msg::layout_bad_hotkey(&short)));
                }
                if self.devices[..i].iter().any(|o| o.hotkey == Some(n)) {
                    return Err(invalid(msg::layout_dup_hotkey(n)));
                }
            }
        }
        let known = |p: &PeerId| self.devices.iter().any(|d| d.peer == *p);
        for (i, l) in self.links.iter().enumerate() {
            let id = l.id.as_str();
            if id.is_empty() || l.a.monitor.is_empty() || l.b.monitor.is_empty() {
                return Err(invalid(msg::layout_bad_link_field(id)));
            }
            if self.links[..i].iter().any(|o| o.id == l.id) {
                return Err(LayoutError::DuplicateId(l.id.clone()));
            }
            if !known(&l.a.peer) || !known(&l.b.peer) {
                return Err(LayoutError::UnknownDevice { link: l.id.clone() });
            }
            if l.a.peer == l.b.peer {
                return Err(LayoutError::SameDevice { link: l.id.clone() });
            }
            if !l.a.in_range() || !l.b.in_range() {
                return Err(LayoutError::SegmentOutOfRange { link: l.id.clone() });
            }
            if l.a.from >= l.a.to || l.b.from >= l.b.to {
                return Err(LayoutError::EmptySegment { link: l.id.clone() });
            }
            for o in &self.links[..i] {
                let hit = [&l.a, &l.b]
                    .iter()
                    .any(|e| e.overlaps(&o.a) || e.overlaps(&o.b));
                if hit {
                    return Err(LayoutError::SegmentOverlap {
                        link: l.id.clone(),
                        other: o.id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Links in `layout.toml` and manual `[[input.barrier]]` together are ambiguous.
    pub fn ensure_no_manual_barriers(&self, cfg: &InputConfig) -> Result<(), LayoutError> {
        if self.links.is_empty() || cfg.barriers.is_empty() {
            Ok(())
        } else {
            Err(LayoutError::ManualBarrierConflict)
        }
    }

    /// The barriers `me` installs: one per link that has `me` on either side, in link order.
    /// Both machines run this over the same file and get mirrored barriers.
    pub fn barriers_for(&self, me: PeerId) -> Vec<DerivedBarrier> {
        self.links
            .iter()
            .filter_map(|l| {
                if l.a.peer == me {
                    Some(DerivedBarrier::new(&l.id, &l.a, &l.b))
                } else if l.b.peer == me {
                    Some(DerivedBarrier::new(&l.id, &l.b, &l.a))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn device(&self, peer: PeerId) -> Option<&LayoutDevice> {
        self.devices.iter().find(|d| d.peer == peer)
    }

    pub fn link(&self, id: &str) -> Option<&Link> {
        self.links.iter().find(|l| l.id == id)
    }

    /// `Ok(None)` when the file does not exist.
    pub fn read(path: &Path) -> Result<Option<Self>, LayoutError> {
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_toml_str(&s).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(LayoutError::Io(e.to_string())),
        }
    }

    /// Validates, then writes a private temp file next to `path` and renames it over `path`.
    pub fn write(&self, path: &Path) -> Result<(), LayoutError> {
        self.validate()?;
        let text = self.to_toml_string()?;
        write_private(path, text.as_bytes()).map_err(|e| LayoutError::Io(e.to_string()))
    }
}

/// Same shape as the core's `write_private`: a temp name unique per write, `create_new` (never
/// opens an existing file or follows a link planted at that name), mode 0600 on Unix, `fsync`,
/// then an atomic rename. A symlink at `path` itself is replaced, not followed.
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    static NEXT_TMP: AtomicU64 = AtomicU64::new(0);
    let n = NEXT_TMP.fetch_add(1, Ordering::Relaxed);
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".tmp-{}-{n}", std::process::id()));
    let tmp = std::path::PathBuf::from(name);
    let res = (|| {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

impl Link {
    /// `(local, remote)` when `me` is on one side of this link.
    pub fn sides_for(&self, me: PeerId) -> Option<(&Endpoint, &Endpoint)> {
        if self.a.peer == me {
            Some((&self.a, &self.b))
        } else if self.b.peer == me {
            Some((&self.b, &self.a))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BarrierConfig;

    fn id(n: u8) -> PeerId {
        PeerId([n; 32])
    }

    fn ep(n: u8, monitor: &str, edge: Edge, from: f32, to: f32) -> Endpoint {
        Endpoint {
            peer: id(n),
            monitor: monitor.into(),
            edge,
            from,
            to,
        }
    }

    fn device(n: u8, name: &str) -> LayoutDevice {
        LayoutDevice {
            peer: id(n),
            name: name.into(),
            x: 0,
            y: 0,
            hotkey: None,
        }
    }

    fn sample() -> Layout {
        Layout {
            rev: 7,
            options: LayoutOptions { dead_corner_px: 12 },
            devices: vec![
                LayoutDevice {
                    x: 160,
                    y: -32,
                    hotkey: Some(2),
                    ..device(1, "pc-pessoal")
                },
                device(2, "notebook-a"),
            ],
            links: vec![Link {
                id: "l1".into(),
                a: ep(1, "HDMI-1", Edge::Right, 0.5, 1.0),
                b: ep(2, "eDP-1", Edge::Left, 0.0, 0.1),
            }],
        }
    }

    fn with_links(links: Vec<Link>) -> Layout {
        Layout { links, ..sample() }
    }

    fn link(id: &str, a: Endpoint, b: Endpoint) -> Link {
        Link {
            id: id.into(),
            a,
            b,
        }
    }

    #[test]
    fn roundtrip_keeps_every_field() {
        let l = sample();
        let back = Layout::from_toml_str(&l.to_toml_string().unwrap()).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn written_fractions_are_clean_decimals() {
        let s = sample().to_toml_string().unwrap();
        assert!(s.contains("to = 0.1\n") || s.contains("to = 0.1 "), "{s}");
        assert!(!s.contains("0.1000"), "{s}");
    }

    #[test]
    fn peer_is_written_as_64_hex() {
        let s = sample().to_toml_string().unwrap();
        assert!(s.contains(&"01".repeat(32)), "{s}");
    }

    #[test]
    fn empty_file_with_version_is_an_empty_layout() {
        let l = Layout::from_toml_str("version = 1\n").unwrap();
        assert_eq!(l, Layout::default());
        assert_eq!(l.options.dead_corner_px, 8);
    }

    #[test]
    fn wrong_version_is_refused_with_the_reason() {
        let e = Layout::from_toml_str("version = 2\n").unwrap_err();
        assert_eq!(e, LayoutError::UnsupportedVersion(2));
        assert!(e.to_string().contains("versão 2"));
    }

    #[test]
    fn missing_version_is_a_parse_error() {
        let e = Layout::from_toml_str("rev = 1\n").unwrap_err();
        assert_eq!(e.code(), "parse");
    }

    #[test]
    fn unknown_key_is_refused() {
        let e = Layout::from_toml_str("version = 1\nfoo = 1\n").unwrap_err();
        assert_eq!(e.code(), "parse");
    }

    #[test]
    fn bad_peer_hex_is_refused() {
        let toml = "version = 1\n[[device]]\npeer = \"zz\"\nname = \"a\"\n";
        let e = Layout::from_toml_str(toml).unwrap_err();
        assert_eq!(e.code(), "invalid_field");
    }

    #[test]
    fn link_inside_one_device_is_refused() {
        let l = with_links(vec![link(
            "l1",
            ep(1, "DP-1", Edge::Right, 0.0, 1.0),
            ep(1, "HDMI-1", Edge::Left, 0.0, 1.0),
        )]);
        let e = l.validate().unwrap_err();
        assert_eq!(e, LayoutError::SameDevice { link: "l1".into() });
        assert!(e.to_string().contains("mesmo dispositivo"));
    }

    #[test]
    fn empty_segment_is_refused_on_either_side() {
        for (a, b) in [(0.5, 0.5), (0.8, 0.2)] {
            let on_a = with_links(vec![link(
                "l1",
                ep(1, "DP-1", Edge::Right, a, b),
                ep(2, "eDP-1", Edge::Left, 0.0, 1.0),
            )]);
            let on_b = with_links(vec![link(
                "l1",
                ep(1, "DP-1", Edge::Right, 0.0, 1.0),
                ep(2, "eDP-1", Edge::Left, a, b),
            )]);
            for l in [on_a, on_b] {
                assert_eq!(l.validate().unwrap_err().code(), "empty_segment");
            }
        }
    }

    #[test]
    fn segment_outside_zero_one_or_not_finite_is_refused() {
        for (from, to) in [
            (-0.1, 0.5),
            (0.0, 1.5),
            (f32::NAN, 0.5),
            (0.0, f32::INFINITY),
        ] {
            let l = with_links(vec![link(
                "l1",
                ep(1, "DP-1", Edge::Right, from, to),
                ep(2, "eDP-1", Edge::Left, 0.0, 1.0),
            )]);
            assert_eq!(l.validate().unwrap_err().code(), "segment_out_of_range");
        }
    }

    #[test]
    fn link_to_a_device_missing_from_the_list_is_refused() {
        let l = with_links(vec![link(
            "l1",
            ep(1, "DP-1", Edge::Right, 0.0, 1.0),
            ep(9, "eDP-1", Edge::Left, 0.0, 1.0),
        )]);
        assert_eq!(l.validate().unwrap_err().code(), "unknown_device");
    }

    #[test]
    fn repeated_link_id_is_refused() {
        let l = with_links(vec![
            link(
                "l1",
                ep(1, "DP-1", Edge::Right, 0.0, 0.5),
                ep(2, "eDP-1", Edge::Left, 0.0, 0.5),
            ),
            link(
                "l1",
                ep(1, "DP-1", Edge::Right, 0.5, 1.0),
                ep(2, "eDP-1", Edge::Left, 0.5, 1.0),
            ),
        ]);
        assert_eq!(l.validate().unwrap_err().code(), "duplicate_id");
    }

    #[test]
    fn overlap_on_the_same_edge_is_refused_even_on_the_b_side() {
        let l = with_links(vec![
            link(
                "l1",
                ep(1, "DP-1", Edge::Right, 0.0, 0.6),
                ep(2, "eDP-1", Edge::Left, 0.0, 0.5),
            ),
            link(
                "l2",
                ep(1, "DP-1", Edge::Right, 0.6, 1.0),
                ep(2, "eDP-1", Edge::Left, 0.4, 1.0),
            ),
        ]);
        assert_eq!(
            l.validate().unwrap_err(),
            LayoutError::SegmentOverlap {
                link: "l2".into(),
                other: "l1".into()
            }
        );
    }

    #[test]
    fn segments_that_only_touch_or_sit_on_other_edges_are_fine() {
        let l = with_links(vec![
            link(
                "l1",
                ep(1, "DP-1", Edge::Right, 0.0, 0.5),
                ep(2, "eDP-1", Edge::Left, 0.0, 0.5),
            ),
            link(
                "l2",
                ep(1, "DP-1", Edge::Right, 0.5, 1.0),
                ep(2, "eDP-1", Edge::Left, 0.5, 1.0),
            ),
            link(
                "l3",
                ep(1, "DP-1", Edge::Bottom, 0.0, 1.0),
                ep(2, "eDP-1", Edge::Top, 0.0, 1.0),
            ),
            link(
                "l4",
                ep(1, "HDMI-1", Edge::Right, 0.0, 1.0),
                ep(2, "eDP-1", Edge::Right, 0.0, 1.0),
            ),
        ]);
        l.validate().unwrap();
    }

    #[test]
    fn hotkey_out_of_1_to_9_or_repeated_is_refused() {
        for bad in [0u8, 10] {
            let mut l = sample();
            l.devices[1].hotkey = Some(bad);
            assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
        }
        let mut l = sample();
        l.devices[1].hotkey = Some(2);
        assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
    }

    #[test]
    fn dead_corner_above_32_is_refused() {
        let mut l = sample();
        l.options.dead_corner_px = 33;
        assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
        l.options.dead_corner_px = 32;
        l.validate().unwrap();
    }

    #[test]
    fn a_device_far_off_the_canvas_is_refused_by_validate_write_and_read() {
        for (x, y) in [
            (MAX_CANVAS_PX + 1, 0),
            (0, MAX_CANVAS_PX + 1),
            (-MAX_CANVAS_PX - 1, 0),
            (0, i32::MIN),
            (i32::MAX, i32::MAX),
        ] {
            let mut l = sample();
            l.devices[1].x = x;
            l.devices[1].y = y;
            let e = l.validate().unwrap_err();
            assert_eq!(e.code(), "invalid_field", "{x},{y}");
            assert!(e.to_string().contains("posição"), "{e}");
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("layout.toml");
            assert_eq!(l.write(&p).unwrap_err().code(), "invalid_field");
            assert!(!p.exists(), "nothing written");
        }
        // The limit itself is fine, on both signs.
        let mut l = sample();
        l.devices[1].x = MAX_CANVAS_PX;
        l.devices[1].y = -MAX_CANVAS_PX;
        l.validate().unwrap();
        // A hand-edited file is held to the same rule.
        let toml = sample()
            .to_toml_string()
            .unwrap()
            .replace("x = 0", "x = 2000000");
        assert_eq!(
            Layout::from_toml_str(&toml).unwrap_err().code(),
            "invalid_field"
        );
    }

    #[test]
    fn device_name_must_be_1_to_64_bytes_and_ids_unique() {
        let mut l = sample();
        l.devices[0].name = String::new();
        assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
        l.devices[0].name = "x".repeat(MAX_NAME_BYTES + 1);
        assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
        let mut l = sample();
        l.devices[1].peer = l.devices[0].peer;
        assert_eq!(l.validate().unwrap_err().code(), "invalid_field");
    }

    #[test]
    fn manual_barriers_with_links_conflict() {
        let cfg = InputConfig {
            barriers: vec![BarrierConfig {
                monitor: "eDP-1".into(),
                edge: Edge::Right,
                from: 0.0,
                to: 0.5,
                peer: "notebook-b".into(),
                peer_edge: Edge::Left,
                peer_monitor: None,
                peer_from: None,
                peer_to: None,
            }],
            ..InputConfig::default()
        };
        let e = sample().ensure_no_manual_barriers(&cfg).unwrap_err();
        assert_eq!(e, LayoutError::ManualBarrierConflict);
        assert!(e.to_string().contains("[[input.barrier]]"));
    }

    #[test]
    fn no_conflict_without_links_or_without_manual_barriers() {
        let manual = InputConfig {
            barriers: vec![BarrierConfig {
                monitor: "eDP-1".into(),
                edge: Edge::Right,
                from: 0.0,
                to: 0.5,
                peer: "notebook-b".into(),
                peer_edge: Edge::Left,
                peer_monitor: None,
                peer_from: None,
                peer_to: None,
            }],
            ..InputConfig::default()
        };
        Layout::default()
            .ensure_no_manual_barriers(&manual)
            .unwrap();
        sample()
            .ensure_no_manual_barriers(&InputConfig::default())
            .unwrap();
    }

    #[test]
    fn read_of_a_missing_file_is_none() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(Layout::read(&d.path().join("layout.toml")).unwrap(), None);
    }

    #[test]
    fn write_then_read_returns_the_same_layout_and_leaves_no_tmp() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        sample().write(&p).unwrap();
        assert_eq!(Layout::read(&p).unwrap(), Some(sample()));
        let names: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("layout.toml")]);
    }

    #[test]
    fn write_replaces_the_old_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        sample().write(&p).unwrap();
        let mut l2 = sample();
        l2.rev = 8;
        l2.write(&p).unwrap();
        assert_eq!(Layout::read(&p).unwrap().unwrap().rev, 8);
    }

    #[test]
    fn invalid_layout_is_not_written() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        sample().write(&p).unwrap();
        let bad = with_links(vec![link(
            "l1",
            ep(1, "DP-1", Edge::Right, 0.0, 1.0),
            ep(1, "HDMI-1", Edge::Left, 0.0, 1.0),
        )]);
        assert_eq!(bad.write(&p).unwrap_err().code(), "same_device");
        assert_eq!(Layout::read(&p).unwrap(), Some(sample()));
    }

    #[cfg(unix)]
    #[test]
    fn a_planted_symlink_is_never_followed_and_the_file_is_private() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let d = tempfile::tempdir().unwrap();
        let victim = d.path().join("victim.txt");
        std::fs::write(&victim, "intacto").unwrap();
        let p = d.path().join("layout.toml");
        // The old fixed temp name, and the target itself, both pointing at the victim.
        symlink(&victim, d.path().join("layout.toml.tmp")).unwrap();
        symlink(&victim, &p).unwrap();
        sample().write(&p).unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "intacto");
        assert!(
            !std::fs::symlink_metadata(&p)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(Layout::read(&p).unwrap(), Some(sample()));
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn concurrent_writes_do_not_share_a_temp_file_and_leave_none_behind() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        std::thread::scope(|s| {
            for rev in 1..=8u64 {
                let p = &p;
                s.spawn(move || {
                    let mut l = sample();
                    l.rev = rev;
                    l.write(p).unwrap();
                });
            }
        });
        let back = Layout::read(&p).unwrap().unwrap();
        assert!((1..=8).contains(&back.rev));
        let names: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("layout.toml")]);
    }

    #[test]
    fn read_of_a_broken_file_names_the_problem() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        std::fs::write(&p, "version = 1\n[[link]]\nid = \"l1\"\n").unwrap();
        let e = Layout::read(&p).unwrap_err();
        assert_eq!(e.code(), "parse");
        assert!(e.to_string().starts_with("layout.toml inválido"));
    }

    #[test]
    fn barriers_are_derived_on_both_sides_of_a_link() {
        let l = sample();
        let on_1 = l.barriers_for(id(1));
        let on_2 = l.barriers_for(id(2));
        assert_eq!(on_1.len(), 1);
        assert_eq!(on_2.len(), 1);
        let (a, b) = (&on_1[0], &on_2[0]);
        assert_eq!(
            (a.monitor.as_str(), a.edge, a.from, a.to),
            ("HDMI-1", Edge::Right, 0.5, 1.0)
        );
        assert_eq!(
            (a.peer, a.peer_monitor.as_str(), a.peer_edge),
            (id(2), "eDP-1", Edge::Left)
        );
        assert_eq!((a.peer_from, a.peer_to), (0.0, 0.1));
        assert_eq!(
            (b.monitor.as_str(), b.edge, b.from, b.to),
            ("eDP-1", Edge::Left, 0.0, 0.1)
        );
        assert_eq!(
            (b.peer, b.peer_monitor.as_str(), b.peer_edge),
            (id(1), "HDMI-1", Edge::Right)
        );
        assert_eq!((b.peer_from, b.peer_to), (0.5, 1.0));
        assert_eq!(a.link, "l1");
        assert!(l.barriers_for(id(9)).is_empty());
    }

    #[test]
    fn position_maps_linearly_between_the_two_segments() {
        let b = &sample().barriers_for(id(1))[0];
        // local 0.5..1.0 -> remote 0.0..0.1
        assert!((b.map_pos(0.5) - 0.0).abs() < 1e-6);
        assert!((b.map_pos(0.75) - 0.05).abs() < 1e-6);
        assert!((b.map_pos(1.0) - 0.1).abs() < 1e-6);
        // outside the local segment clamps to its ends
        assert!((b.map_pos(0.2) - 0.0).abs() < 1e-6);
        assert!((b.map_pos(1.3) - 0.1).abs() < 1e-6);
        assert!((b.peer_middle() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn dead_corner_trims_the_edge_ends_and_can_empty_a_segment() {
        assert_eq!(trim_dead_corner(0.0, 1.0, 1080, 0), Some((0.0, 1.0)));
        let (a, b) = trim_dead_corner(0.0, 1.0, 1000, 8).unwrap();
        assert!((a - 0.008).abs() < 1e-6 && (b - 0.992).abs() < 1e-6);
        // a segment away from the corners is untouched
        assert_eq!(trim_dead_corner(0.25, 0.75, 1000, 32), Some((0.25, 0.75)));
        // a segment inside a corner disappears
        assert_eq!(trim_dead_corner(0.0, 0.005, 1000, 8), None);
        assert_eq!(trim_dead_corner(0.0, 1.0, 0, 8), None);
    }

    #[test]
    fn sides_for_puts_me_first() {
        let l = sample();
        let (local, remote) = l.links[0].sides_for(id(2)).unwrap();
        assert_eq!(local.peer, id(2));
        assert_eq!(remote.peer, id(1));
        assert!(l.links[0].sides_for(id(3)).is_none());
        assert_eq!(l.device(id(1)).unwrap().name, "pc-pessoal");
        assert!(l.link("l1").is_some() && l.link("l9").is_none());
    }

    #[test]
    fn read_applies_validation() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("layout.toml");
        let h = "01".repeat(32);
        let toml = format!(
            "version = 1\n[[device]]\npeer = \"{h}\"\nname = \"a\"\n\
             [[link]]\nid = \"l1\"\n\
             a = {{ peer = \"{h}\", monitor = \"DP-1\", edge = \"right\", from = 0.0, to = 1.0 }}\n\
             b = {{ peer = \"{h}\", monitor = \"HDMI-1\", edge = \"left\", from = 0.0, to = 1.0 }}\n"
        );
        std::fs::write(&p, toml).unwrap();
        assert_eq!(Layout::read(&p).unwrap_err().code(), "same_device");
    }
}
