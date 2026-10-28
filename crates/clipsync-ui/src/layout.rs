//! 4 · Layout de telas (§12.5). Owned by VECTOR (§12.1).
//!
//! `layout.toml` is the single source (§12.10 decision 5). Every command reads it, adds the
//! devices the profile knows (this one and the paired peers) and drops what points to a peer
//! that is gone, so the screen never shows a device that cannot be reached. Every write goes
//! through `save`: validate, refuse a clash with a manual `[[input.barrier]]`, write through
//! tmp + rename, tell the agent to reload (`InputReload`, a no-op when no agent is up) and emit
//! `layout_changed`. Monitors come from the agent's `InputState`; without an agent the screen
//! still gets the file, with `monitors_source: "none"`.
//!
//! Own error codes (§12.5 lists the rest): `input_off` when the input module is off or not
//! available here, `no_layout` when there is nothing to offer yet.

use std::path::PathBuf;

use clipsync_core::Config;
use clipsync_input::layout as file;
use clipsync_input::{InputConfig, LayoutError, trim_dead_corner};
use clipsync_proto::{
    AgentStatus, Edge as ProtoEdge, InputState, LayoutOffer, MAX_LAYOUT_LINKS, ModuleCommand,
    ModuleKind, ModuleReply, OfferEndpoint, OfferState, PeerId, PeerState, ZoneInfo,
};
use serde::{Deserialize, Serialize};

use crate::agent::INPUT_SUPPORTED;
use crate::api::{self, code as base};
use crate::devices::DeviceState;
use crate::events::{LayoutChanged, LayoutReason, UiEvent};
use crate::{Ctx, UiError, clean, msg, status};

/// Own codes of this screen.
pub mod code {
    pub const INPUT_OFF: &str = "input_off";
    pub const NO_LAYOUT: &str = "no_layout";
    pub const NO_OFFER: &str = "no_offer";
    pub const STALE_OFFER: &str = "stale_offer";
    pub const SAME_DEVICE: &str = "same_device";
    pub const INTERNAL_EDGE_LOCKED: &str = "internal_edge_locked";
    pub const EMPTY_SEGMENT: &str = "empty_segment";
    pub const SEGMENT_OUT_OF_RANGE: &str = "segment_out_of_range";
    pub const SEGMENT_OVERLAP: &str = "segment_overlap";
    pub const STALE_REV: &str = "stale_rev";
    pub const LINK_NOT_FOUND: &str = "link_not_found";
    pub const HOTKEY_TAKEN: &str = "hotkey_taken";
    pub const OUT_OF_RANGE: &str = "out_of_range";
}

/// Portuguese text of this screen (the module's own refusals keep the text of `clipsync_input::msg`).
pub(crate) mod txt {
    pub const INPUT_OFF: &str = "teclado e mouse estão desligados neste dispositivo";
    pub const NO_LAYOUT: &str =
        "o layout ainda não foi salvo: ligue duas bordas ou mova um dispositivo antes de copiar";
    pub const NO_OFFER: &str = "não há oferta de layout pendente";
    pub const SAME_DEVICE: &str =
        "as duas pontas estão no mesmo dispositivo: ligue bordas de dispositivos diferentes";
    pub const INTERNAL_EDGE_LOCKED: &str =
        "essa borda fica entre dois monitores do mesmo dispositivo e não cruza para outro";
    pub const EMPTY_SEGMENT: &str = "o trecho ficou vazio: o começo precisa vir antes do fim";
    pub const SEGMENT_OUT_OF_RANGE: &str = "o trecho sai da borda: use valores de 0 a 1";
    pub const STALE_REV: &str =
        "o layout mudou desde que a tela foi carregada: recarregue e tente de novo";
    pub const LINK_NOT_FOUND: &str = "essa ligação não existe mais";
    pub const BAD_MONITOR: &str = "esse monitor não existe no dispositivo";
    pub const BAD_HOTKEY: &str = "o atalho vai de 1 a 9";
    pub const BAD_POSITION: &str = "posição fora da área do canvas";
    pub const TOO_MANY_LINKS: &str = "limite de ligações atingido: remova uma antes de criar outra";
    pub const DEAD_CORNER_RANGE: &str = "cantos mortos vão de 0 a 32 px";
    pub const DEAD_CORNER_EMPTIES: &str = "com esse valor uma ligação ficaria sem trecho: encurte os cantos mortos ou ajuste a ligação";
    pub const OFFER_OFFLINE: &str = "dispositivo offline: a oferta não foi enviada";
    pub const OFFER_DENIED: &str = "o outro dispositivo não aceitou a oferta: ele precisa permitir que este controle o teclado e o mouse dele";

    pub fn stale_offer(from: &str, rev: u64) -> String {
        format!(
            "a oferta mudou: agora quem oferece é {from} (revisão {rev}): confira e responda de novo"
        )
    }

    pub fn hotkey_taken(key: u8, owner: &str) -> String {
        format!("Win+Alt+{key} já é de {owner}")
    }

    pub fn applied_but_not_loaded(why: &str) -> String {
        format!("layout salvo, mas o agente não aplicou: {why}")
    }

    pub fn overlap(other: &str) -> String {
        format!("esse trecho já está ligado por {other}: encurte um dos dois ou remova a ligação")
    }
}

/// Grid of the canvas; devices snap to it (§12.5 `layout_set_device`).
pub const SNAP_PX: u32 = 16;
/// Gap between two devices that nobody placed yet.
const AUTO_STEP: i64 = 2048;

/// One writer at a time: read-modify-write of `layout.toml` must not interleave.
static WRITE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ---------------------------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl From<Edge> for ProtoEdge {
    fn from(e: Edge) -> Self {
        match e {
            Edge::Left => ProtoEdge::Left,
            Edge::Right => ProtoEdge::Right,
            Edge::Top => ProtoEdge::Top,
            Edge::Bottom => ProtoEdge::Bottom,
        }
    }
}

impl From<ProtoEdge> for Edge {
    fn from(e: ProtoEdge) -> Self {
        match e {
            ProtoEdge::Left => Edge::Left,
            ProtoEdge::Right => Edge::Right,
            ProtoEdge::Top => Edge::Top,
            ProtoEdge::Bottom => Edge::Bottom,
        }
    }
}

/// `monitor` = the monitor ref ("DP-1" or "#0"); `from` and `to` are fractions 0..1.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// 64 hex.
    pub peer_id: String,
    pub monitor: String,
    pub edge: Edge,
    pub from: f64,
    pub to: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub id: String,
    pub a: Endpoint,
    pub b: Endpoint,
    /// `"internal_edge_locked"` when the link crosses an edge between two monitors of one device.
    pub problem: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Monitor {
    #[serde(rename = "ref")]
    pub monitor_ref: String,
    pub name: String,
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MonitorsSource {
    Live,
    Cached,
    #[serde(rename = "none")]
    Absent,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LayoutDevice {
    pub peer_id: String,
    pub name: String,
    pub is_self: bool,
    pub state: DeviceState,
    pub x: i32,
    pub y: i32,
    pub hotkey: Option<u8>,
    pub monitors_source: MonitorsSource,
    pub monitors: Vec<Monitor>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub dead_corner_px: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PendingOffer {
    pub from_peer_id: String,
    pub from_name: String,
    pub rev: u64,
    /// Ids are local to the offer (`o1`, `o2`...): they become real ids only if accepted.
    pub links: Vec<Link>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub rev: u64,
    pub self_id: String,
    pub snap_px: u32,
    pub options: Options,
    pub screen_locked: bool,
    pub active_peer_id: Option<String>,
    pub devices: Vec<LayoutDevice>,
    pub locked_edges: Vec<Endpoint>,
    pub links: Vec<Link>,
    pub pending_offer: Option<PendingOffer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Warning {
    pub code: String,
    /// "a" | "b".
    pub side: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetLinkReply {
    pub link: Link,
    pub layout_rev: u64,
    pub warnings: Vec<Warning>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RevReply {
    pub layout_rev: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetDeviceReply {
    pub device: LayoutDevice,
    pub layout_rev: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetOptionsReply {
    pub options: Options,
    pub layout_rev: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopyStatus {
    Offered,
    Offline,
    Denied,
    Unsupported,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CopyResult {
    pub peer_id: String,
    pub name: String,
    pub status: CopyStatus,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CopyReply {
    pub rev: u64,
    pub results: Vec<CopyResult>,
}

// ---------------------------------------------------------------------------------------------
// Arguments (deserialized by `UiRequest` with `deny_unknown_fields`)
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetLinkArgs {
    /// null creates.
    pub id: Option<String>,
    pub a: Endpoint,
    pub b: Endpoint,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RemoveLinkArgs {
    pub id: String,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetDeviceArgs {
    pub peer_id: String,
    pub x: i32,
    pub y: i32,
    /// null or 1..9.
    pub hotkey: Option<u8>,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetOptionsArgs {
    /// 0..32.
    pub dead_corner_px: u32,
    pub base_rev: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CopyToPeersArgs {
    /// null = every paired peer.
    pub peer_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OfferAnswerArgs {
    pub accept: bool,
    /// 64 hex: who made the offer the screen showed (`pending_offer.from_peer_id`).
    pub from_peer_id: String,
    /// The revision of that offer (`pending_offer.rev`).
    pub rev: u64,
}

// ---------------------------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------------------------

/// `0.1_f32` as `0.1`, not as `0.10000000149011612` (what a plain `as f64` would show).
fn clean_f(v: f32) -> f64 {
    format!("{v}").parse().unwrap_or(f64::from(v))
}

fn round3(v: f64) -> f32 {
    ((v * 1000.0).round() / 1000.0) as f32
}

/// Nearest multiple of `SNAP_PX`, ties up.
fn snap(v: i64) -> i64 {
    let s = i64::from(SNAP_PX);
    (v + s / 2).div_euclid(s) * s
}

fn saturating_i32(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

fn monitor_of(z: &ZoneInfo) -> Monitor {
    let name = z.name.as_deref().map(clean).filter(|n| !n.is_empty());
    Monitor {
        monitor_ref: name.clone().unwrap_or_else(|| format!("#{}", z.index)),
        name: name.unwrap_or_else(|| format!("#{}", z.index)),
        index: z.index,
        x: z.x,
        y: z.y,
        width: z.width,
        height: z.height,
        scale: z.scale,
    }
}

fn segment(peer: PeerId, m: &Monitor, edge: ProtoEdge, from_px: i64, to_px: i64) -> file::Endpoint {
    let len = match edge {
        ProtoEdge::Left | ProtoEdge::Right => m.height,
        ProtoEdge::Top | ProtoEdge::Bottom => m.width,
    };
    let len = f64::from(len.max(1));
    file::Endpoint {
        peer,
        monitor: m.monitor_ref.clone(),
        edge,
        from: round3(from_px as f64 / len),
        to: round3(to_px as f64 / len),
    }
}

/// Edges where two monitors of the same device touch: the cursor crosses them on its own, so no
/// link may sit there (§12.5, "borda interna").
fn internal_edges(peer: PeerId, ms: &[Monitor]) -> Vec<file::Endpoint> {
    let mut out = Vec::new();
    for a in ms {
        for b in ms {
            if a.monitor_ref == b.monitor_ref {
                continue;
            }
            let (ax, ay) = (i64::from(a.x), i64::from(a.y));
            let (bx, by) = (i64::from(b.x), i64::from(b.y));
            let (ax2, ay2) = (ax + i64::from(a.width), ay + i64::from(a.height));
            let (bx2, by2) = (bx + i64::from(b.width), by + i64::from(b.height));
            if ax2 == bx {
                let (lo, hi) = (ay.max(by), ay2.min(by2));
                if hi > lo {
                    out.push(segment(peer, a, ProtoEdge::Right, lo - ay, hi - ay));
                    out.push(segment(peer, b, ProtoEdge::Left, lo - by, hi - by));
                }
            }
            if ay2 == by {
                let (lo, hi) = (ax.max(bx), ax2.min(bx2));
                if hi > lo {
                    out.push(segment(peer, a, ProtoEdge::Bottom, lo - ax, hi - ax));
                    out.push(segment(peer, b, ProtoEdge::Top, lo - bx, hi - bx));
                }
            }
        }
    }
    out
}

/// Same peer, monitor and edge, and the open intervals meet.
fn overlaps(a: &file::Endpoint, b: &file::Endpoint) -> bool {
    a.peer == b.peer && a.monitor == b.monitor && a.edge == b.edge && a.from < b.to && b.from < a.to
}

fn touches_locked(locked: &[file::Endpoint], e: &file::Endpoint) -> bool {
    locked.iter().any(|l| overlaps(l, e))
}

fn in_range(e: &file::Endpoint) -> bool {
    let ok = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
    ok(e.from) && ok(e.to)
}

/// `l1`, `l2`...: the lowest number that no link uses.
fn next_id(links: &[file::Link]) -> String {
    let mut n = 1u32;
    loop {
        let id = format!("l{n}");
        if links.iter().all(|l| l.id != id) {
            return id;
        }
        n += 1;
    }
}

/// Dead corners cut `dead` px off each END OF THE EDGE. `None` when nothing of the segment is
/// left; the flag says whether it was cut. Without the monitor size nothing is cut.
fn trim_side(e: file::Endpoint, len_px: Option<u32>, dead: u32) -> Option<(file::Endpoint, bool)> {
    let Some(len) = len_px else {
        return Some((e, false));
    };
    let (from, to) = trim_dead_corner(e.from, e.to, len, dead)?;
    let cut = from != e.from || to != e.to;
    Some((file::Endpoint { from, to, ..e }, cut))
}

struct Row {
    id: PeerId,
    name: String,
}

/// The file plus what the profile knows: this device and the paired peers always have an entry
/// (placed to the right of the others when nobody placed them), names follow the profile, and
/// whatever points to an unknown peer is dropped.
fn normalize(mut l: file::Layout, me: &Row, peers: &[Row]) -> file::Layout {
    let known = |p: &PeerId| *p == me.id || peers.iter().any(|r| r.id == *p);
    l.devices.retain(|d| known(&d.peer));
    l.links.retain(|k| known(&k.a.peer) && known(&k.b.peer));
    for r in std::iter::once(me).chain(peers) {
        match l.devices.iter().position(|d| d.peer == r.id) {
            Some(i) => l.devices[i].name = r.name.clone(),
            None => {
                let x = l
                    .devices
                    .iter()
                    .map(|d| i64::from(d.x))
                    .max()
                    .map_or(0, |m| snap(m + AUTO_STEP));
                l.devices.push(file::LayoutDevice {
                    peer: r.id,
                    name: r.name.clone(),
                    x: i32::try_from(x).unwrap_or(0),
                    y: 0,
                    hotkey: None,
                });
            }
        }
    }
    l
}

fn layout_error(e: &LayoutError) -> UiError {
    let text = clean(&e.to_string());
    let err = |c: &str| UiError::new(c, text.clone());
    match e {
        LayoutError::SameDevice { .. } => err(code::SAME_DEVICE),
        LayoutError::EmptySegment { .. } => err(code::EMPTY_SEGMENT),
        LayoutError::SegmentOutOfRange { .. } => err(code::SEGMENT_OUT_OF_RANGE),
        LayoutError::SegmentOverlap { link, other } => err(code::SEGMENT_OVERLAP)
            .with_details(serde_json::json!({ "link": link, "other": other })),
        LayoutError::Io(_) => err(base::IO),
        _ => err(base::CONFIG),
    }
}

/// The module's own refusals, by the consts of `clipsync_input::msg`.
fn module_error(text: &str) -> UiError {
    use clipsync_input::msg as m;
    match text {
        t if t == m::NO_LAYOUT => UiError::new(code::NO_LAYOUT, txt::NO_LAYOUT),
        t if t == clipsync_proto::msg::MODULE_DISABLED => {
            UiError::new(code::INPUT_OFF, txt::INPUT_OFF)
        }
        t => UiError::new(base::INTERNAL, clean(t)),
    }
}

// ---------------------------------------------------------------------------------------------
// The world one command sees
// ---------------------------------------------------------------------------------------------

pub(crate) struct World {
    me: Row,
    peers: Vec<Row>,
    layout: file::Layout,
    /// `layout.toml` exists on disk.
    saved: bool,
    state: Option<InputState>,
    status: Option<AgentStatus>,
}

fn path(ctx: &Ctx) -> PathBuf {
    ctx.profile.dir.join(clipsync_input::LAYOUT_FILE)
}

/// The profile is set up and the input module is on and exists on this platform.
pub(crate) fn require_input(ctx: &Ctx) -> Result<Config, UiError> {
    ctx.require_profile()?;
    let cfg = ctx.profile.load_config()?;
    if cfg.modules.input && INPUT_SUPPORTED {
        Ok(cfg)
    } else {
        Err(UiError::new(code::INPUT_OFF, txt::INPUT_OFF))
    }
}

/// `Some` when an input module answered.
async fn input_state(ctx: &Ctx) -> Option<InputState> {
    match ctx
        .ask_module(ModuleKind::Input, ModuleCommand::InputState)
        .await
    {
        Ok(ModuleReply::InputState(s)) => Some(s),
        _ => None,
    }
}

async fn ask(ctx: &Ctx, cmd: ModuleCommand) -> Result<ModuleReply, UiError> {
    match ctx.ask_module(ModuleKind::Input, cmd).await? {
        ModuleReply::Error(t) => Err(module_error(&t)),
        r => Ok(r),
    }
}

/// `InputReload` after a save. No agent is fine (the file is read at the next start).
async fn reload(ctx: &Ctx) -> Result<(), UiError> {
    match ctx
        .ask_module(ModuleKind::Input, ModuleCommand::InputReload)
        .await
    {
        Ok(ModuleReply::Done) => Ok(()),
        Ok(ModuleReply::Error(t)) => Err(UiError::new(
            base::CONFIG,
            txt::applied_but_not_loaded(&clean(&t)),
        )),
        Ok(_) => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
        Err(e) if e.code == base::AGENT_NOT_RUNNING => Ok(()),
        Err(e) => Err(e),
    }
}

impl World {
    pub(crate) async fn load(ctx: &Ctx) -> Result<World, UiError> {
        let me = Row {
            id: ctx.profile.load_identity()?.id(),
            name: ctx.profile.load_config()?.device_name,
        };
        let peers: Vec<Row> = ctx
            .profile
            .load_peers()?
            .peer
            .into_iter()
            .map(|p| Row {
                id: p.id,
                name: p.name,
            })
            .collect();
        let read = file::Layout::read(&path(ctx)).map_err(|e| layout_error(&e))?;
        let saved = read.is_some();
        let layout = normalize(read.unwrap_or_default(), &me, &peers);
        Ok(World {
            me,
            peers,
            layout,
            saved,
            state: input_state(ctx).await,
            status: status::agent_status(ctx).await,
        })
    }

    fn knows(&self, p: PeerId) -> bool {
        p == self.me.id || self.peers.iter().any(|r| r.id == p)
    }

    fn name_of(&self, p: PeerId) -> String {
        if p == self.me.id {
            return clean(&self.me.name);
        }
        self.peers
            .iter()
            .find(|r| r.id == p)
            .map_or_else(|| p.short(), |r| clean(&r.name))
    }

    fn peer_state(&self, p: PeerId) -> DeviceState {
        if p == self.me.id {
            return DeviceState::Connected;
        }
        let live = self
            .status
            .as_ref()
            .and_then(|s| s.peers.iter().find(|x| x.id == p));
        match live.map(|x| &x.state) {
            Some(PeerState::Connected) => DeviceState::Connected,
            Some(PeerState::Connecting) => DeviceState::Connecting,
            Some(PeerState::Blocked { .. }) => DeviceState::Blocked,
            Some(PeerState::Disconnected) | None => DeviceState::Offline,
        }
    }

    /// Zones are kept after a peer leaves, so a peer that is not connected shows `cached`.
    fn monitors(&self, p: PeerId) -> (Vec<Monitor>, MonitorsSource) {
        let Some(state) = &self.state else {
            return (Vec::new(), MonitorsSource::Absent);
        };
        let zones: &[ZoneInfo] = if p == self.me.id {
            &state.local
        } else {
            state
                .peers
                .iter()
                .find(|z| z.peer == p)
                .map_or(&[], |z| z.zones.as_slice())
        };
        if zones.is_empty() {
            return (Vec::new(), MonitorsSource::Absent);
        }
        let mut ms: Vec<Monitor> = zones.iter().map(monitor_of).collect();
        ms.sort_by_key(|m| m.index);
        let source = if self.peer_state(p) == DeviceState::Connected {
            MonitorsSource::Live
        } else {
            MonitorsSource::Cached
        };
        (ms, source)
    }

    /// Length in px of the edge `e` sits on, when the monitor is known.
    fn edge_len(&self, e: &file::Endpoint) -> Option<u32> {
        let (ms, _) = self.monitors(e.peer);
        let m = ms.iter().find(|m| m.monitor_ref == e.monitor)?;
        Some(match e.edge {
            ProtoEdge::Left | ProtoEdge::Right => m.height,
            ProtoEdge::Top | ProtoEdge::Bottom => m.width,
        })
    }

    fn locked_edges(&self) -> Vec<file::Endpoint> {
        self.layout
            .devices
            .iter()
            .flat_map(|d| internal_edges(d.peer, &self.monitors(d.peer).0))
            .collect()
    }

    fn device_view(&self, d: &file::LayoutDevice) -> LayoutDevice {
        let (monitors, monitors_source) = self.monitors(d.peer);
        LayoutDevice {
            peer_id: d.peer.to_hex(),
            name: self.name_of(d.peer),
            is_self: d.peer == self.me.id,
            state: self.peer_state(d.peer),
            x: d.x,
            y: d.y,
            hotkey: d.hotkey,
            monitors_source,
            monitors,
        }
    }

    fn view(&self) -> Layout {
        let locked = self.locked_edges();
        let links = self
            .layout
            .links
            .iter()
            .map(|k| {
                let bad = touches_locked(&locked, &k.a) || touches_locked(&locked, &k.b);
                link_view(k, bad.then_some(code::INTERNAL_EDGE_LOCKED))
            })
            .collect();
        let st = self.state.as_ref();
        Layout {
            rev: self.layout.rev,
            self_id: self.me.id.to_hex(),
            snap_px: SNAP_PX,
            options: Options {
                dead_corner_px: self.layout.options.dead_corner_px,
            },
            screen_locked: st.is_some_and(|s| s.screen_locked),
            active_peer_id: st.and_then(|s| s.active_peer).map(|p| p.to_hex()),
            devices: self
                .layout
                .devices
                .iter()
                .map(|d| self.device_view(d))
                .collect(),
            locked_edges: locked.iter().map(endpoint_view).collect(),
            links,
            pending_offer: st
                .and_then(|s| s.pending_offer.as_ref())
                .map(|p| self.offer_view(p.from, &p.offer)),
        }
    }

    fn offer_view(&self, from: PeerId, offer: &LayoutOffer) -> PendingOffer {
        let links = offer
            .links
            .iter()
            .enumerate()
            .map(|(i, l)| Link {
                id: format!("o{}", i + 1),
                a: offer_endpoint_view(&l.a),
                b: offer_endpoint_view(&l.b),
                problem: None,
            })
            .collect();
        PendingOffer {
            from_peer_id: from.to_hex(),
            from_name: self.name_of(from),
            rev: offer.rev,
            links,
        }
    }
}

fn endpoint_view(e: &file::Endpoint) -> Endpoint {
    Endpoint {
        peer_id: e.peer.to_hex(),
        monitor: clean(&e.monitor),
        edge: e.edge.into(),
        from: clean_f(e.from),
        to: clean_f(e.to),
    }
}

fn offer_endpoint_view(e: &OfferEndpoint) -> Endpoint {
    Endpoint {
        peer_id: e.peer.to_hex(),
        monitor: clean(&e.monitor),
        edge: e.edge.into(),
        from: clean_f(e.from),
        to: clean_f(e.to),
    }
}

fn link_view(k: &file::Link, problem: Option<&str>) -> Link {
    Link {
        id: k.id.clone(),
        a: endpoint_view(&k.a),
        b: endpoint_view(&k.b),
        problem: problem.map(str::to_owned),
    }
}

/// The one write path: validate, refuse a manual-barrier clash, write, reload the agent, emit.
/// The new `rev` is `layout.rev + 1`. When the agent cannot apply the file, it stays saved and
/// the error says so.
async fn save(ctx: &Ctx, cfg: &Config, mut layout: file::Layout) -> Result<u64, UiError> {
    layout.rev += 1;
    layout.validate().map_err(|e| layout_error(&e))?;
    let input = InputConfig::from_table(cfg.input.as_ref())
        .map_err(|e| UiError::new(base::CONFIG, msg::config(&clean(&e.to_string()))))?;
    layout
        .ensure_no_manual_barriers(&input)
        .map_err(|e| layout_error(&e))?;
    layout.write(&path(ctx)).map_err(|e| layout_error(&e))?;
    let rev = layout.rev;
    let reloaded = reload(ctx).await;
    ctx.emit(UiEvent::LayoutChanged(LayoutChanged {
        rev,
        reason: LayoutReason::Saved,
    }));
    reloaded?;
    Ok(rev)
}

fn stale(current: u64) -> UiError {
    UiError::new(code::STALE_REV, txt::STALE_REV)
        .with_details(serde_json::json!({ "rev": current }))
}

fn unknown_peer(field: &str) -> UiError {
    UiError::new(base::UNKNOWN_PEER, msg::UNKNOWN_PEER).with_field(field)
}

/// A peer id from the front: 64 hex and a device the profile knows.
fn known_peer(w: &World, s: &str, field: &str) -> Result<PeerId, UiError> {
    let p = api::peer_id(s, field)?;
    if w.knows(p) {
        Ok(p)
    } else {
        Err(unknown_peer(field))
    }
}

fn parse_endpoint(w: &World, e: &Endpoint, side: &str) -> Result<file::Endpoint, UiError> {
    let peer = known_peer(w, &e.peer_id, &format!("{side}.peer_id"))?;
    let (ms, _) = w.monitors(peer);
    let monitor_ok = !e.monitor.is_empty()
        && e.monitor.len() <= clipsync_proto::MAX_NAME_BYTES
        && (ms.is_empty() || ms.iter().any(|m| m.monitor_ref == e.monitor));
    if !monitor_ok {
        return Err(UiError::new(base::BAD_REQUEST, txt::BAD_MONITOR)
            .with_field(&format!("{side}.monitor")));
    }
    Ok(file::Endpoint {
        peer,
        monitor: e.monitor.clone(),
        edge: e.edge.into(),
        from: e.from as f32,
        to: e.to as f32,
    })
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

pub(crate) async fn get(ctx: &Ctx) -> Result<Layout, UiError> {
    require_input(ctx)?;
    Ok(World::load(ctx).await?.view())
}

/// Validated in the order of §12.5: `same_device`, `internal_edge_locked`, `empty_segment`,
/// `segment_out_of_range`, `segment_overlap`, `stale_rev`. Shape errors (`bad_request`,
/// `unknown_peer`) come first, and `link_not_found` after the revision.
pub(crate) async fn set_link(ctx: &Ctx, args: SetLinkArgs) -> Result<SetLinkReply, UiError> {
    let cfg = require_input(ctx)?;
    let _guard = WRITE.lock().await;
    let mut w = World::load(ctx).await?;
    let a = parse_endpoint(&w, &args.a, "a")?;
    let b = parse_endpoint(&w, &args.b, "b")?;
    let sides = [("a", &a), ("b", &b)];

    if a.peer == b.peer {
        return Err(UiError::new(code::SAME_DEVICE, txt::SAME_DEVICE).with_field("b"));
    }
    let locked = w.locked_edges();
    for (side, e) in sides {
        if touches_locked(&locked, e) {
            return Err(
                UiError::new(code::INTERNAL_EDGE_LOCKED, txt::INTERNAL_EDGE_LOCKED)
                    .with_field(side),
            );
        }
    }
    for (side, e) in sides {
        if e.from >= e.to {
            return Err(UiError::new(code::EMPTY_SEGMENT, txt::EMPTY_SEGMENT).with_field(side));
        }
    }
    for (side, e) in sides {
        if !in_range(e) {
            return Err(
                UiError::new(code::SEGMENT_OUT_OF_RANGE, txt::SEGMENT_OUT_OF_RANGE)
                    .with_field(side),
            );
        }
    }
    for (side, e) in sides {
        let other = w
            .layout
            .links
            .iter()
            .filter(|k| args.id.as_deref() != Some(k.id.as_str()))
            .find(|k| overlaps(&k.a, e) || overlaps(&k.b, e));
        if let Some(k) = other {
            return Err(UiError::new(code::SEGMENT_OVERLAP, txt::overlap(&k.id))
                .with_field(side)
                .with_details(serde_json::json!({ "link": k.id })));
        }
    }
    if args.base_rev != w.layout.rev {
        return Err(stale(w.layout.rev));
    }
    let editing = match &args.id {
        Some(id) => Some(
            w.layout
                .links
                .iter()
                .position(|k| k.id == *id)
                .ok_or_else(|| UiError::new(code::LINK_NOT_FOUND, txt::LINK_NOT_FOUND))?,
        ),
        None => None,
    };
    if editing.is_none() && w.layout.links.len() >= MAX_LAYOUT_LINKS {
        return Err(UiError::new(base::BAD_REQUEST, txt::TOO_MANY_LINKS));
    }

    let dead = w.layout.options.dead_corner_px;
    let mut warnings = Vec::new();
    let mut trimmed = Vec::new();
    for (side, e) in [("a", a.clone()), ("b", b.clone())] {
        let len = w.edge_len(&e);
        let (e, cut) = trim_side(e, len, dead).ok_or_else(|| {
            UiError::new(code::EMPTY_SEGMENT, txt::EMPTY_SEGMENT).with_field(side)
        })?;
        if cut {
            warnings.push(Warning {
                code: "trimmed_by_dead_corner".to_owned(),
                side: side.to_owned(),
            });
        }
        trimmed.push(e);
    }
    let (b, a) = (trimmed.remove(1), trimmed.remove(0));

    let id = args.id.clone().unwrap_or_else(|| next_id(&w.layout.links));
    let link = file::Link { id, a, b };
    match editing {
        Some(i) => w.layout.links[i] = link.clone(),
        None => w.layout.links.push(link.clone()),
    }
    let layout_rev = save(ctx, &cfg, w.layout).await?;
    Ok(SetLinkReply {
        link: link_view(&link, None),
        layout_rev,
        warnings,
    })
}

pub(crate) async fn remove_link(ctx: &Ctx, args: RemoveLinkArgs) -> Result<RevReply, UiError> {
    let cfg = require_input(ctx)?;
    let _guard = WRITE.lock().await;
    let mut w = World::load(ctx).await?;
    let Some(i) = w.layout.links.iter().position(|k| k.id == args.id) else {
        return Err(UiError::new(code::LINK_NOT_FOUND, txt::LINK_NOT_FOUND).with_field("id"));
    };
    if args.base_rev != w.layout.rev {
        return Err(stale(w.layout.rev));
    }
    w.layout.links.remove(i);
    Ok(RevReply {
        layout_rev: save(ctx, &cfg, w.layout).await?,
    })
}

pub(crate) async fn set_device(ctx: &Ctx, args: SetDeviceArgs) -> Result<SetDeviceReply, UiError> {
    let cfg = require_input(ctx)?;
    let _guard = WRITE.lock().await;
    let mut w = World::load(ctx).await?;
    let peer = known_peer(&w, &args.peer_id, "peer_id")?;
    if let Some(n) = args.hotkey
        && !(1..=file::MAX_HOTKEY).contains(&n)
    {
        return Err(UiError::new(base::BAD_REQUEST, txt::BAD_HOTKEY).with_field("hotkey"));
    }
    let (x, y) = (snap(i64::from(args.x)), snap(i64::from(args.y)));
    let limit = i64::from(file::MAX_CANVAS_PX);
    if x.abs() > limit || y.abs() > limit {
        return Err(UiError::new(base::BAD_REQUEST, txt::BAD_POSITION).with_field("x"));
    }
    if let Some(n) = args.hotkey
        && let Some(owner) = w
            .layout
            .devices
            .iter()
            .find(|d| d.peer != peer && d.hotkey == Some(n))
    {
        return Err(UiError::new(
            code::HOTKEY_TAKEN,
            txt::hotkey_taken(n, &w.name_of(owner.peer)),
        )
        .with_field("hotkey")
        .with_details(serde_json::json!({ "peer_id": owner.peer.to_hex() })));
    }
    if args.base_rev != w.layout.rev {
        return Err(stale(w.layout.rev));
    }
    if let Some(d) = w.layout.devices.iter_mut().find(|d| d.peer == peer) {
        d.x = i32::try_from(x).unwrap_or(0);
        d.y = i32::try_from(y).unwrap_or(0);
        d.hotkey = args.hotkey;
    }
    let layout_rev = save(ctx, &cfg, w.layout.clone()).await?;
    let device = w
        .layout
        .devices
        .iter()
        .find(|d| d.peer == peer)
        .map(|d| w.device_view(d))
        .ok_or_else(|| unknown_peer("peer_id"))?;
    Ok(SetDeviceReply { device, layout_rev })
}

pub(crate) async fn set_options(
    ctx: &Ctx,
    args: SetOptionsArgs,
) -> Result<SetOptionsReply, UiError> {
    let cfg = require_input(ctx)?;
    let _guard = WRITE.lock().await;
    let mut w = World::load(ctx).await?;
    if args.dead_corner_px > file::MAX_DEAD_CORNER_PX {
        return Err(
            UiError::new(code::OUT_OF_RANGE, txt::DEAD_CORNER_RANGE).with_field("dead_corner_px")
        );
    }
    let emptied: Vec<&str> = w
        .layout
        .links
        .iter()
        .filter(|k| {
            [&k.a, &k.b].into_iter().any(|e| {
                w.edge_len(e).is_some_and(|len| {
                    trim_dead_corner(e.from, e.to, len, args.dead_corner_px).is_none()
                })
            })
        })
        .map(|k| k.id.as_str())
        .collect();
    if !emptied.is_empty() {
        return Err(UiError::new(code::EMPTY_SEGMENT, txt::DEAD_CORNER_EMPTIES)
            .with_details(serde_json::json!({ "links": emptied })));
    }
    if args.base_rev != w.layout.rev {
        return Err(stale(w.layout.rev));
    }
    w.layout.options.dead_corner_px = args.dead_corner_px;
    let layout_rev = save(ctx, &cfg, w.layout).await?;
    Ok(SetOptionsReply {
        options: Options {
            dead_corner_px: args.dead_corner_px,
        },
        layout_rev,
    })
}

fn copy_result(w: &World, peer: PeerId, state: OfferState) -> CopyResult {
    let (status, reason) = match state {
        OfferState::Offered => (CopyStatus::Offered, None),
        OfferState::Offline => (CopyStatus::Offline, Some(txt::OFFER_OFFLINE)),
        OfferState::Denied => (CopyStatus::Denied, Some(txt::OFFER_DENIED)),
        OfferState::Unsupported => (
            CopyStatus::Unsupported,
            Some(clipsync_input::msg::PEER_TOO_OLD),
        ),
    };
    CopyResult {
        peer_id: peer.to_hex(),
        name: w.name_of(peer),
        status,
        reason: reason.map(str::to_owned),
    }
}

/// Sends the saved layout to the peers (`InputOffer`). The peer decides; `denied` is a peer that
/// did not grant this device control of it.
pub(crate) async fn copy_to_peers(ctx: &Ctx, args: CopyToPeersArgs) -> Result<CopyReply, UiError> {
    require_input(ctx)?;
    let w = World::load(ctx).await?;
    let mut targets: Vec<PeerId> = Vec::new();
    match &args.peer_ids {
        None => targets.extend(w.peers.iter().map(|r| r.id)),
        Some(list) => {
            for s in list {
                let p = api::peer_id(s, "peer_ids")?;
                if p == w.me.id || !w.knows(p) {
                    return Err(unknown_peer("peer_ids"));
                }
                if !targets.contains(&p) {
                    targets.push(p);
                }
            }
        }
    }
    if !w.saved {
        return Err(UiError::new(code::NO_LAYOUT, txt::NO_LAYOUT));
    }
    if targets.is_empty() {
        return Ok(CopyReply {
            rev: w.layout.rev,
            results: Vec::new(),
        });
    }
    let outcomes = match ask(
        ctx,
        ModuleCommand::InputOffer {
            to: targets.clone(),
        },
    )
    .await?
    {
        ModuleReply::InputOffered(o) => o,
        _ => return Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    };
    let results = targets
        .iter()
        .map(|p| {
            let state = outcomes
                .iter()
                .find(|o| o.peer == *p)
                .map_or(OfferState::Offline, |o| o.state);
            copy_result(&w, *p, state)
        })
        .collect();
    Ok(CopyReply {
        rev: w.layout.rev,
        results,
    })
}

/// The offer replaces the links and the device positions with the sender's; it keeps this
/// device's hotkeys and options. Links that point to a device unknown here are dropped.
fn apply_offer(l: &mut file::Layout, offer: &LayoutOffer, known: impl Fn(PeerId) -> bool) {
    for od in &offer.devices {
        if let Some(d) = l.devices.iter_mut().find(|d| d.peer == od.peer) {
            // Saturate instead of wrapping to 0: `validate` then refuses what is out of the canvas.
            d.x = saturating_i32(snap(i64::from(od.x)));
            d.y = saturating_i32(snap(i64::from(od.y)));
        }
    }
    l.links.clear();
    for ol in &offer.links {
        if !known(ol.a.peer) || !known(ol.b.peer) {
            continue;
        }
        let from_offer = |e: &OfferEndpoint| file::Endpoint {
            peer: e.peer,
            monitor: e.monitor.clone(),
            edge: e.edge,
            from: e.from,
            to: e.to,
        };
        let id = next_id(&l.links);
        l.links.push(file::Link {
            id,
            a: from_offer(&ol.a),
            b: from_offer(&ol.b),
        });
    }
}

/// The answer names the offer the user saw (`from_peer_id` and `rev`). The module keeps one
/// offer and any peer with `input_in` can replace it, so an answer to another offer than the
/// pending one is `stale_offer`, whether it accepts or declines, and it says which one is
/// pending. What is applied is always the offer read here, the one that was checked. A newer
/// offer that lands after the check can still be dropped by the final clear: that loses an
/// offer, it never applies one the user did not see.
pub(crate) async fn offer_answer(ctx: &Ctx, args: OfferAnswerArgs) -> Result<RevReply, UiError> {
    let cfg = require_input(ctx)?;
    let seen_from = api::peer_id(&args.from_peer_id, "from_peer_id")?;
    let _guard = WRITE.lock().await;
    let state = match ask(ctx, ModuleCommand::InputState).await? {
        ModuleReply::InputState(s) => s,
        _ => return Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    };
    let Some(pending) = state.pending_offer else {
        return Err(UiError::new(code::NO_OFFER, txt::NO_OFFER));
    };
    let mut w = World::load(ctx).await?;
    if pending.from != seen_from || pending.offer.rev != args.rev {
        return Err(UiError::new(
            code::STALE_OFFER,
            txt::stale_offer(&w.name_of(pending.from), pending.offer.rev),
        )
        .with_details(serde_json::json!({
            "from_peer_id": pending.from.to_hex(),
            "rev": pending.offer.rev,
        })));
    }
    if !args.accept {
        ask(ctx, ModuleCommand::InputOfferClear).await?;
        return Ok(RevReply {
            layout_rev: w.layout.rev,
        });
    }
    let mut layout = std::mem::take(&mut w.layout);
    apply_offer(&mut layout, &pending.offer, |p| w.knows(p));
    let layout_rev = save(ctx, &cfg, layout).await?;
    ask(ctx, ModuleCommand::InputOfferClear).await?;
    Ok(RevReply { layout_rev })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> PeerId {
        PeerId([n; 32])
    }

    fn mon(name: &str, index: u32, x: i32, y: i32, w: u32, h: u32) -> Monitor {
        Monitor {
            monitor_ref: name.to_owned(),
            name: name.to_owned(),
            index,
            x,
            y,
            width: w,
            height: h,
            scale: 1.0,
        }
    }

    fn ep(n: u8, monitor: &str, edge: ProtoEdge, from: f32, to: f32) -> file::Endpoint {
        file::Endpoint {
            peer: id(n),
            monitor: monitor.to_owned(),
            edge,
            from,
            to,
        }
    }

    fn row(n: u8, name: &str) -> Row {
        Row {
            id: id(n),
            name: name.to_owned(),
        }
    }

    #[test]
    fn snap_goes_to_the_nearest_multiple_and_ties_up() {
        assert_eq!(snap(0), 0);
        assert_eq!(snap(7), 0);
        assert_eq!(snap(8), 16);
        assert_eq!(snap(23), 16);
        assert_eq!(snap(-7), 0);
        assert_eq!(snap(-9), -16);
        assert_eq!(snap(4640), 4640);
    }

    #[test]
    fn side_by_side_monitors_lock_the_shared_edge_on_both() {
        // DP-1 2560x1440 at 0,0 and HDMI-1 1920x1080 at 2560,180: they share y 180..1260.
        let ms = [
            mon("DP-1", 0, 0, 0, 2560, 1440),
            mon("HDMI-1", 1, 2560, 180, 1920, 1080),
        ];
        let e = internal_edges(id(1), &ms);
        assert_eq!(e.len(), 2);
        let right = e.iter().find(|x| x.monitor == "DP-1").unwrap();
        assert_eq!(right.edge, ProtoEdge::Right);
        assert!((right.from - 0.125).abs() < 1e-6 && (right.to - 0.875).abs() < 1e-6);
        let left = e.iter().find(|x| x.monitor == "HDMI-1").unwrap();
        assert_eq!(left.edge, ProtoEdge::Left);
        assert_eq!((left.from, left.to), (0.0, 1.0));
    }

    #[test]
    fn stacked_monitors_lock_bottom_and_top_and_a_gap_locks_nothing() {
        let stacked = [
            mon("A", 0, 0, 0, 1920, 1080),
            mon("B", 1, 0, 1080, 1920, 1080),
        ];
        let e = internal_edges(id(1), &stacked);
        assert_eq!(
            e.iter()
                .map(|x| (x.monitor.as_str(), x.edge))
                .collect::<Vec<_>>(),
            vec![("A", ProtoEdge::Bottom), ("B", ProtoEdge::Top)]
        );
        let gap = [
            mon("A", 0, 0, 0, 1920, 1080),
            mon("B", 1, 1936, 0, 1920, 1080),
        ];
        assert!(internal_edges(id(1), &gap).is_empty());
        let corner_only = [mon("A", 0, 0, 0, 100, 100), mon("B", 1, 100, 100, 100, 100)];
        assert!(internal_edges(id(1), &corner_only).is_empty());
        assert!(internal_edges(id(1), &[mon("A", 0, 0, 0, 1920, 1080)]).is_empty());
    }

    #[test]
    fn a_link_on_a_locked_edge_is_detected_only_where_it_overlaps() {
        let locked = vec![ep(1, "DP-1", ProtoEdge::Right, 0.125, 0.875)];
        assert!(touches_locked(
            &locked,
            &ep(1, "DP-1", ProtoEdge::Right, 0.0, 0.2)
        ));
        assert!(!touches_locked(
            &locked,
            &ep(1, "DP-1", ProtoEdge::Right, 0.875, 1.0)
        ));
        assert!(!touches_locked(
            &locked,
            &ep(1, "DP-1", ProtoEdge::Left, 0.0, 1.0)
        ));
        assert!(!touches_locked(
            &locked,
            &ep(2, "DP-1", ProtoEdge::Right, 0.0, 1.0)
        ));
    }

    #[test]
    fn dead_corners_cut_the_ends_of_the_edge_and_report_the_cut() {
        let e = ep(1, "DP-1", ProtoEdge::Right, 0.0, 1.0);
        let (t, cut) = trim_side(e.clone(), Some(1000), 8).unwrap();
        assert!(cut && (t.from - 0.008).abs() < 1e-6 && (t.to - 0.992).abs() < 1e-6);
        let mid = ep(1, "DP-1", ProtoEdge::Right, 0.25, 0.75);
        let (t, cut) = trim_side(mid.clone(), Some(1000), 8).unwrap();
        assert!(!cut && t == mid);
        assert!(trim_side(ep(1, "DP-1", ProtoEdge::Right, 0.0, 0.005), Some(1000), 8).is_none());
        // Unknown monitor size: nothing to cut, nothing to refuse.
        let (t, cut) = trim_side(e.clone(), None, 8).unwrap();
        assert!(!cut && t == e);
    }

    #[test]
    fn link_ids_take_the_lowest_free_number() {
        let link = |id: &str| file::Link {
            id: id.to_owned(),
            a: ep(1, "A", ProtoEdge::Right, 0.0, 1.0),
            b: ep(2, "B", ProtoEdge::Left, 0.0, 1.0),
        };
        assert_eq!(next_id(&[]), "l1");
        assert_eq!(next_id(&[link("l1"), link("l2")]), "l3");
        assert_eq!(next_id(&[link("l2")]), "l1");
        assert_eq!(next_id(&[link("custom")]), "l1");
    }

    #[test]
    fn normalize_adds_the_known_devices_and_drops_the_gone_ones() {
        let (me, peers) = (row(1, "eu"), vec![row(2, "par")]);
        let l = normalize(file::Layout::default(), &me, &peers);
        assert_eq!(l.devices.len(), 2);
        assert_eq!((l.devices[0].peer, l.devices[0].x), (id(1), 0));
        assert_eq!((l.devices[1].peer, l.devices[1].x), (id(2), 2048));
        assert!(l.devices.iter().all(|d| d.x % 16 == 0 && d.y == 0));

        // A revoked peer takes its device and its links with it; names follow the profile.
        let mut l = l;
        l.devices.push(file::LayoutDevice {
            peer: id(9),
            name: "sumiu".into(),
            x: 64,
            y: 0,
            hotkey: Some(3),
        });
        l.devices[1].name = "nome-velho".into();
        l.links.push(file::Link {
            id: "l1".into(),
            a: ep(1, "A", ProtoEdge::Right, 0.0, 1.0),
            b: ep(9, "B", ProtoEdge::Left, 0.0, 1.0),
        });
        let l = normalize(l, &me, &peers);
        assert_eq!(l.devices.len(), 2);
        assert!(l.links.is_empty());
        assert_eq!(l.devices[1].name, "par");
        l.validate().unwrap();
    }

    #[test]
    fn an_offer_replaces_links_and_positions_but_keeps_hotkeys() {
        let (me, peers) = (row(1, "eu"), vec![row(2, "par")]);
        let mut l = normalize(file::Layout::default(), &me, &peers);
        l.devices[1].hotkey = Some(4);
        let offer = LayoutOffer {
            rev: 5,
            devices: vec![
                clipsync_proto::OfferDevice {
                    peer: id(1),
                    x: 0,
                    y: 0,
                },
                clipsync_proto::OfferDevice {
                    peer: id(2),
                    x: 1930,
                    y: -9,
                },
                clipsync_proto::OfferDevice {
                    peer: id(7),
                    x: 5,
                    y: 5,
                },
            ],
            links: vec![
                clipsync_proto::OfferLink {
                    a: OfferEndpoint {
                        peer: id(1),
                        monitor: "A".into(),
                        edge: ProtoEdge::Right,
                        from: 0.0,
                        to: 1.0,
                    },
                    b: OfferEndpoint {
                        peer: id(2),
                        monitor: "B".into(),
                        edge: ProtoEdge::Left,
                        from: 0.0,
                        to: 1.0,
                    },
                },
                clipsync_proto::OfferLink {
                    a: OfferEndpoint {
                        peer: id(1),
                        monitor: "A".into(),
                        edge: ProtoEdge::Top,
                        from: 0.0,
                        to: 1.0,
                    },
                    b: OfferEndpoint {
                        peer: id(7),
                        monitor: "C".into(),
                        edge: ProtoEdge::Bottom,
                        from: 0.0,
                        to: 1.0,
                    },
                },
            ],
        };
        apply_offer(&mut l, &offer, |p| p == id(1) || p == id(2));
        assert_eq!(l.links.len(), 1, "the link to an unknown device is dropped");
        assert_eq!(l.links[0].id, "l1");
        assert_eq!((l.devices[1].x, l.devices[1].y), (1936, -16), "snapped");
        assert_eq!(l.devices[1].hotkey, Some(4));
        l.validate().unwrap();
    }

    #[test]
    fn an_offer_with_absurd_positions_does_not_pass_validate() {
        let (me, peers) = (row(1, "eu"), vec![row(2, "par")]);
        let device = |peer, x, y| clipsync_proto::OfferDevice { peer, x, y };
        for (x, y) in [
            (2_000_000, 0),
            (0, -2_000_000),
            (i32::MAX, i32::MAX),
            (i32::MIN, 0),
        ] {
            let mut l = normalize(file::Layout::default(), &me, &peers);
            let offer = LayoutOffer {
                rev: 1,
                devices: vec![device(id(2), x, y)],
                links: Vec::new(),
            };
            apply_offer(&mut l, &offer, |_| true);
            assert_ne!(
                (l.devices[1].x, l.devices[1].y),
                (0, 0),
                "must not wrap to 0 ({x},{y})"
            );
            let e = l.validate().unwrap_err();
            assert_eq!(e.code(), "invalid_field", "{x},{y}");
        }
        // At the edge of the canvas it is accepted, snapped to the grid.
        let mut l = normalize(file::Layout::default(), &me, &peers);
        let offer = LayoutOffer {
            rev: 1,
            devices: vec![device(id(2), 999_999, -999_999)],
            links: Vec::new(),
        };
        apply_offer(&mut l, &offer, |_| true);
        l.validate().unwrap();
    }

    #[test]
    fn floats_leave_as_short_decimals() {
        let v = serde_json::to_value(clean_f(0.1)).unwrap();
        assert_eq!(v.to_string(), "0.1");
        assert_eq!(clean_f(0.0074074073).to_string(), "0.0074074073");
    }

    #[test]
    fn zones_without_a_name_are_refered_by_index() {
        let z = ZoneInfo {
            index: 2,
            name: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
            scale: 1.0,
        };
        let m = monitor_of(&z);
        assert_eq!((m.monitor_ref.as_str(), m.index), ("#2", 2));
        assert!(matches!(
            clipsync_input::zone_ref(&m.monitor_ref),
            clipsync_input::ZoneRef::Index(2)
        ));
    }
}
