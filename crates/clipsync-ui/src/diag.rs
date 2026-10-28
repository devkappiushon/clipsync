//! 6 · Diagnóstico (§12.5): `diagnostics_get`, `support_bundle_preview` and
//! `support_bundle_export`. Owned by Rivet.
//!
//! The support bundle is plain text with the identifiers taken out: peer ids (64 hex and short),
//! peer names, IP addresses and this device's name. Never in it: the key, the raw `peers.toml`,
//! `portal-tokens.toml` or clipboard content (the box is not read at all).

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use clipsync_proto::{AgentStatus, DiagStatus, InputState, ModuleKind, PeerId};
use serde::{Deserialize, Serialize};

use crate::api::code;
use crate::autostart::fsio;
use crate::devices::{Device, DeviceState};
use crate::status::{self, ModuleName};
use crate::{Ctx, UiError, clean, msg, pairing};

/// A check that hangs (a portal that never answers) must not hold the screen.
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Ok,
    Warn,
    Missing,
    Error,
}

impl From<DiagStatus> for ItemStatus {
    fn from(s: DiagStatus) -> Self {
        match s {
            DiagStatus::Ok => ItemStatus::Ok,
            DiagStatus::Warn => ItemStatus::Warn,
            DiagStatus::Missing => ItemStatus::Missing,
            DiagStatus::Error => ItemStatus::Error,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiagItem {
    pub area: String,
    pub name: String,
    pub status: ItemStatus,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetPeer {
    pub peer_id: String,
    pub name: String,
    pub state: DeviceState,
    pub addr: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Network {
    /// "IP:PORT" of the agent (or of config.toml when it is stopped).
    pub listen: Option<String>,
    pub addresses: Vec<String>,
    pub peers: Vec<NetPeer>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub at_unix_ms: u64,
    pub peer_id: Option<String>,
    pub peer_name: Option<String>,
    pub module: Option<ModuleName>,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub items: Vec<DiagItem>,
    pub network: Network,
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RedactionKind {
    PeerId,
    PeerName,
    Ip,
    DeviceName,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Redaction {
    pub kind: RedactionKind,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundlePreview {
    pub text: String,
    pub redactions: Vec<Redaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Exported {
    pub path: String,
    pub bytes: usize,
}

fn timed_out(area: &str) -> Vec<clipsync_proto::DiagItem> {
    vec![clipsync_proto::DiagItem {
        area: area.into(),
        name: msg::DIAG_CHECK.into(),
        status: DiagStatus::Error,
        detail: msg::DIAG_TIMEOUT.into(),
    }]
}

/// The CLI's `clipsync diagnose`: system, clipboard and input, each bounded in time.
async fn checks() -> Vec<DiagItem> {
    let (sys, clip, input) = tokio::join!(
        tokio::time::timeout(CHECK_TIMEOUT, clipsync_core::diag::system()),
        tokio::time::timeout(CHECK_TIMEOUT, clipsync_clipboard::diagnose()),
        tokio::time::timeout(CHECK_TIMEOUT, clipsync_input::diagnose()),
    );
    let mut all = sys.unwrap_or_else(|_| timed_out("sistema"));
    all.extend(clip.unwrap_or_else(|_| timed_out("clipboard")));
    all.extend(input.unwrap_or_else(|_| timed_out("input")));
    all.into_iter()
        .map(|i| DiagItem {
            area: clean(&i.area),
            name: clean(&i.name),
            status: i.status.into(),
            detail: clean(&i.detail),
        })
        .collect()
}

fn input_item(name: &str, status: ItemStatus, detail: String) -> DiagItem {
    DiagItem {
        area: "input".into(),
        name: name.into(),
        status,
        detail,
    }
}

/// The KLIDs the contract names (§13.7 item 3); any other shows as it came.
fn keyboard_name(layout: &str) -> String {
    let known = match layout {
        "00010416" => Some("Português (Brasil ABNT2)"),
        "00000409" => Some("EUA"),
        _ => None,
    };
    let layout = clean(layout);
    match known {
        Some(k) => format!("{layout} · {k}"),
        None => layout,
    }
}

/// "layout do teclado": what the input module of the running agent announced in `InputState`
/// (the `xkb_symbols` name the EIS gives on Linux, the KLID on Windows).
fn keyboard_item(agent_running: bool, input: Option<&InputState>) -> DiagItem {
    let layout = input.and_then(|i| i.keyboard_layout.as_deref());
    let (status, detail) = match (agent_running, layout) {
        (false, _) => (ItemStatus::Warn, msg::DIAG_AGENT_STOPPED.to_string()),
        (true, None) => (ItemStatus::Warn, msg::DIAG_NOT_ANNOUNCED.to_string()),
        (true, Some(l)) => (ItemStatus::Ok, keyboard_name(l)),
    };
    input_item(msg::DIAG_KEYBOARD, status, detail)
}

/// "atalhos" (Windows): active while the hooks exist, that is with the agent running, the input
/// module on and at least one link of this machine in `layout.toml` (§13.2 rule 2a).
fn shortcuts_item(agent_running: bool, input_on: bool, local_link: bool) -> DiagItem {
    let (status, detail) = if !agent_running {
        (ItemStatus::Warn, msg::DIAG_SHORTCUTS_NO_AGENT)
    } else if !input_on {
        (ItemStatus::Warn, msg::DIAG_SHORTCUTS_INPUT_OFF)
    } else if !local_link {
        (ItemStatus::Warn, msg::DIAG_SHORTCUTS_NO_LINK)
    } else {
        (ItemStatus::Ok, msg::DIAG_SHORTCUTS_ON)
    };
    input_item(msg::DIAG_SHORTCUTS, status, detail.to_string())
}

/// `clipsync_input::diagnose()` names "layout do teclado" and "atalhos" too on Windows, without
/// the agent: its "atalhos" is always ok. Ours carry the agent's state (§13.9), so they replace
/// the input items of the same name, and each name shows once.
fn with_agent_state(mut items: Vec<DiagItem>, ours: Vec<DiagItem>) -> Vec<DiagItem> {
    items.retain(|i| !ours.iter().any(|o| o.area == i.area && o.name == i.name));
    items.extend(ours);
    items
}

/// Whether `layout.toml` links a monitor of this machine. An unreadable file links nothing.
fn has_local_link(ctx: &Ctx) -> bool {
    let me: Option<PeerId> = ctx.profile.load_identity().ok().map(|i| i.id());
    let layout = clipsync_input::Layout::read(&ctx.profile.dir.join(clipsync_input::LAYOUT_FILE));
    match (me, layout) {
        (Some(me), Ok(Some(l))) => l.links.iter().any(|k| k.a.peer == me || k.b.peer == me),
        _ => false,
    }
}

/// What the screen and the bundle share.
struct Gathered {
    diag: Diagnostics,
    status: Option<AgentStatus>,
    devices: Vec<Device>,
    device_name: Option<String>,
}

async fn gather(ctx: &Ctx) -> Gathered {
    let observed = status::observe(ctx).await;
    let status = observed.status;
    let mut ours = vec![keyboard_item(status.is_some(), observed.input.as_ref())];
    if cfg!(windows) {
        let input_on = status.as_ref().is_some_and(|s| {
            s.modules
                .iter()
                .any(|m| m.kind == ModuleKind::Input && m.enabled)
        });
        ours.push(shortcuts_item(
            status.is_some(),
            input_on,
            has_local_link(ctx),
        ));
    }
    let items = with_agent_state(checks().await, ours);
    let cfg = ctx.profile.load_config().ok();
    let peers = ctx.profile.load_peers().unwrap_or_default();
    let devices: Vec<Device> = peers
        .peer
        .iter()
        .map(|e| {
            let live = status
                .as_ref()
                .and_then(|s| s.peers.iter().find(|p| p.id == e.id));
            Device::new(e, live)
        })
        .collect();
    let listen = status
        .as_ref()
        .map(|s| s.listen)
        .or(cfg.as_ref().map(|c| c.listen));
    let blocks = status
        .as_ref()
        .map(|s| {
            s.recent_blocks
                .iter()
                .map(|b| Block {
                    at_unix_ms: b.at_unix_ms,
                    peer_id: b.peer.map(|id| id.to_hex()),
                    peer_name: b.peer_name.as_deref().map(clean),
                    module: b.module.map(ModuleName::from),
                    reason: clean(&b.reason),
                })
                .collect()
        })
        .unwrap_or_default();
    let device_name = status
        .as_ref()
        .map(|s| s.device_name.clone())
        .or(cfg.map(|c| c.device_name));
    Gathered {
        diag: Diagnostics {
            items,
            network: Network {
                listen: listen.map(|l| l.to_string()),
                addresses: listen.map(pairing::addresses).unwrap_or_default(),
                peers: devices
                    .iter()
                    .map(|d| NetPeer {
                        peer_id: d.peer_id.clone(),
                        name: d.name.clone(),
                        state: d.state,
                        addr: d.addr.clone(),
                    })
                    .collect(),
            },
            blocks,
        },
        status,
        devices,
        device_name,
    }
}

/// Works without a profile too: the system checks still help.
pub(crate) async fn get(ctx: &Ctx) -> Result<Diagnostics, UiError> {
    Ok(gather(ctx).await.diag)
}

fn state_pt(s: DeviceState) -> &'static str {
    match s {
        DeviceState::Connected => "conectado",
        DeviceState::Connecting => "conectando",
        DeviceState::Offline => "offline",
        DeviceState::Blocked => "bloqueado",
    }
}

fn status_pt(s: ItemStatus) -> &'static str {
    match s {
        ItemStatus::Ok => "ok",
        ItemStatus::Warn => "aviso",
        ItemStatus::Missing => "ausente",
        ItemStatus::Error => "erro",
    }
}

/// The bundle before redaction. Peers appear by their identifiers here; `redact` swaps them.
fn raw_text(g: &Gathered) -> String {
    let mut t = String::new();
    t.push_str("ClipSync · pacote de suporte\n");
    t.push_str(&format!(
        "versão {} · {}\n",
        env!("CARGO_PKG_VERSION"),
        if cfg!(windows) { "windows" } else { "linux" }
    ));
    t.push_str(&format!("gerado em (unix ms): {}\n", crate::unix_ms()));
    match &g.status {
        Some(s) => {
            t.push_str(&format!("agente: rodando · pausado: {}\n", s.paused));
            for m in &s.modules {
                t.push_str(&format!(
                    "módulo {:?}: {} · adaptador {} · último evento: {} · último bloqueio: {}\n",
                    m.kind,
                    if m.enabled { "ligado" } else { "desligado" },
                    m.adapter.as_deref().map_or("-".into(), clean),
                    m.last_event.as_deref().map_or("-".into(), clean),
                    m.last_block.as_deref().map_or("-".into(), clean),
                ));
            }
        }
        None => t.push_str("agente: parado\n"),
    }
    t.push_str("\n[verificações]\n");
    for i in &g.diag.items {
        t.push_str(&format!(
            "{} · {} · {}: {}\n",
            status_pt(i.status),
            i.area,
            i.name,
            i.detail
        ));
    }
    t.push_str("\n[rede]\n");
    let n = &g.diag.network;
    t.push_str(&format!("escuta: {}\n", n.listen.as_deref().unwrap_or("-")));
    t.push_str(&format!("endereços: {}\n", n.addresses.join(", ")));
    for d in &g.devices {
        t.push_str(&format!(
            "par {} ({}) · {} · {} · versão {} · permissões: entrada de clipboard {}, saída de clipboard {}, controle {}\n",
            d.name,
            d.peer_id,
            state_pt(d.state),
            d.addr.as_deref().unwrap_or("-"),
            d.remote_version.map_or("-".into(), |v| v.to_string()),
            d.perms.clipboard_in,
            d.perms.clipboard_out,
            d.perms.input_in,
        ));
    }
    t.push_str("\n[últimos bloqueios]\n");
    for b in &g.diag.blocks {
        t.push_str(&format!(
            "{} · {} · {} · {}\n",
            b.at_unix_ms,
            b.peer_name.as_deref().unwrap_or("-"),
            b.module.map_or("-", |m| match m {
                ModuleName::Clipboard => "clipboard",
                ModuleName::Input => "input",
            }),
            b.reason
        ));
    }
    t
}

/// Names and ids with at least this many characters are swapped wherever they appear, even inside
/// a longer word (`ana-notebook`, `casa.local`): overmatching only hides more.
const SUBSTRING_MIN_CHARS: usize = 4;

/// Shorter ones would eat ordinary words (`ana` in `banana`), so they go only when the characters
/// around them are not letters or digits: `-`, `_` and `.` count as separators, so `pc-da-ana` is
/// still caught.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

fn replace_words(text: &str, needle: &str, with: &str) -> (String, usize) {
    if needle.is_empty() {
        return (text.to_string(), 0);
    }
    if needle.chars().count() >= SUBSTRING_MIN_CHARS {
        return (text.replace(needle, with), text.matches(needle).count());
    }
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut rest = text;
    let mut prev: Option<char> = None;
    while let Some(at) = rest.find(needle) {
        let before = rest[..at].chars().next_back().or(prev);
        let after = rest[at + needle.len()..].chars().next();
        out.push_str(&rest[..at]);
        if before.is_none_or(|c| !is_word_char(c)) && after.is_none_or(|c| !is_word_char(c)) {
            out.push_str(with);
            count += 1;
        } else {
            out.push_str(needle);
        }
        prev = needle.chars().next_back();
        rest = &rest[at + needle.len()..];
    }
    out.push_str(rest);
    (out, count)
}

/// Every IPv4 address in free text (dotted quads, maybe followed by a port).
fn replace_ipv4(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String, count: &mut usize| {
        if token.parse::<Ipv4Addr>().is_ok() {
            out.push_str(msg::REDACTED_IP);
            *count += 1;
        } else {
            out.push_str(token);
        }
        token.clear();
    };
    for c in text.chars() {
        if c.is_ascii_digit() || c == '.' {
            token.push(c);
        } else {
            flush(&mut token, &mut out, &mut count);
            out.push(c);
        }
    }
    flush(&mut token, &mut out, &mut count);
    (out, count)
}

/// The secrets of this profile, longest first, so a name that contains another goes first.
fn redact(raw: &str, g: &Gathered) -> BundlePreview {
    let mut needles: Vec<(String, String, RedactionKind)> = Vec::new();
    let mut ips: Vec<IpAddr> = Vec::new();
    for (i, d) in g.devices.iter().enumerate() {
        let n = i + 1;
        needles.push((
            d.peer_id.clone(),
            msg::redacted_id(n),
            RedactionKind::PeerId,
        ));
        needles.push((d.short.clone(), msg::redacted_id(n), RedactionKind::PeerId));
        needles.push((
            d.name.clone(),
            msg::redacted_peer(n),
            RedactionKind::PeerName,
        ));
        if let Some(a) = d
            .addr
            .as_deref()
            .and_then(|a| a.parse::<std::net::SocketAddr>().ok())
        {
            ips.push(a.ip());
        }
    }
    // Peers named only in the agent's status or in a block (already removed from peers.toml).
    let extra = g.status.iter().flat_map(|s| {
        s.peers
            .iter()
            .map(|p| (p.id, clean(&p.name), p.addr))
            .chain(s.recent_blocks.iter().filter_map(|b| {
                b.peer.map(|id| {
                    (
                        id,
                        b.peer_name.as_deref().map(clean).unwrap_or_default(),
                        None,
                    )
                })
            }))
    });
    let mut n = g.devices.len();
    for (id, name, addr) in extra {
        if needles.iter().any(|(s, _, _)| *s == id.to_hex()) {
            continue;
        }
        n += 1;
        needles.push((id.to_hex(), msg::redacted_id(n), RedactionKind::PeerId));
        needles.push((id.short(), msg::redacted_id(n), RedactionKind::PeerId));
        needles.push((name, msg::redacted_peer(n), RedactionKind::PeerName));
        ips.extend(addr.map(|a| a.ip()));
    }
    if let Some(name) = &g.device_name {
        needles.push((
            clean(name),
            msg::REDACTED_DEVICE.into(),
            RedactionKind::DeviceName,
        ));
    }
    if let Some(l) = g
        .diag
        .network
        .listen
        .as_deref()
        .and_then(|a| a.parse::<std::net::SocketAddr>().ok())
    {
        ips.push(l.ip());
    }
    ips.extend(
        g.diag
            .network
            .addresses
            .iter()
            .filter_map(|a| a.parse::<std::net::SocketAddr>().ok())
            .map(|a| a.ip()),
    );
    for ip in ips.into_iter().filter(IpAddr::is_ipv6) {
        needles.push((ip.to_string(), msg::REDACTED_IP.into(), RedactionKind::Ip));
    }
    needles.retain(|(s, _, _)| !s.is_empty());
    needles.sort_by_key(|n| std::cmp::Reverse(n.0.len()));

    let mut counts = std::collections::BTreeMap::<RedactionKind, usize>::new();
    let mut text = raw.to_string();
    for (needle, with, kind) in &needles {
        let (t, c) = replace_words(&text, needle, with);
        text = t;
        *counts.entry(*kind).or_default() += c;
    }
    let (text, c) = replace_ipv4(&text);
    *counts.entry(RedactionKind::Ip).or_default() += c;
    BundlePreview {
        text,
        redactions: counts
            .into_iter()
            .map(|(kind, count)| Redaction { kind, count })
            .collect(),
    }
}

const BUNDLE_NAME_TAKEN: &str =
    "já existem arquivos demais com o nome do pacote de suporte nesta pasta";

/// How long the text the user saw stays the one that `support_bundle_export` writes.
const APPROVED_TTL: Duration = Duration::from_secs(600);

/// The last preview of each profile: the export writes exactly this text (§12.5 "o mesmo texto"),
/// not a new one with another timestamp or another status. Redacted text only. Kept here because
/// the shared context (`lib.rs`) is Rivet's.
static APPROVED: Mutex<Vec<(PathBuf, Instant, String)>> = Mutex::new(Vec::new());

fn remember_preview(dir: &Path, text: &str) {
    let mut all = APPROVED.lock().unwrap_or_else(PoisonError::into_inner);
    all.retain(|(d, at, _)| d != dir && at.elapsed() < APPROVED_TTL);
    all.push((dir.to_path_buf(), Instant::now(), text.to_string()));
}

/// The previewed text of this profile, once: a second export builds a new one.
fn take_preview(dir: &Path) -> Option<String> {
    let mut all = APPROVED.lock().unwrap_or_else(PoisonError::into_inner);
    all.retain(|(_, at, _)| at.elapsed() < APPROVED_TTL);
    let at = all.iter().position(|(d, _, _)| d == dir)?;
    Some(all.swap_remove(at).2)
}

pub(crate) async fn bundle_preview(ctx: &Ctx) -> Result<BundlePreview, UiError> {
    let g = gather(ctx).await;
    let preview = redact(&raw_text(&g), &g);
    remember_preview(&ctx.profile.dir, &preview.text);
    Ok(preview)
}

/// `XDG_DOWNLOAD_DIR`, then the user-dirs file (pt-BR systems may say "Transferências"), then
/// `~/Downloads`; on Windows `%USERPROFILE%\Downloads`.
fn downloads_dir(ctx: &Ctx) -> Option<PathBuf> {
    if let Some(d) = &ctx.opts.downloads_dir {
        return Some(d.clone());
    }
    if let Some(d) = std::env::var_os("XDG_DOWNLOAD_DIR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(d));
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .filter(|v| !v.is_empty())
            .map(|h| PathBuf::from(h).join("Downloads"))
    }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(std::env::var_os("HOME").filter(|v| !v.is_empty())?);
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map_or_else(|| home.join(".config"), PathBuf::from);
        let from_user_dirs = std::fs::read_to_string(config.join("user-dirs.dirs"))
            .ok()
            .and_then(|text| {
                text.lines().find_map(|l| {
                    let v = l
                        .trim()
                        .strip_prefix("XDG_DOWNLOAD_DIR=")?
                        .trim_matches('"');
                    let v = v.replace("$HOME", &home.to_string_lossy());
                    Some(PathBuf::from(v)).filter(|p| p.is_absolute())
                })
            });
        Some(from_user_dirs.unwrap_or_else(|| home.join("Downloads")))
    }
}

/// `AAAAMMDD-HHMMSS` (UTC) of a unix time in ms: days to civil date (H. Hinnant's
/// `civil_from_days`), no date crate.
fn stamp(unix_ms: u64) -> String {
    let secs = unix_ms / 1000;
    let rem = secs % 86_400;
    let z = i64::try_from(secs / 86_400).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// `clipsync-suporte-AAAAMMDD-HHMMSS.txt` (UTC), no dialog.
fn file_name() -> String {
    format!("clipsync-suporte-{}.txt", stamp(crate::unix_ms()))
}

/// Creates the bundle file without replacing or following anything: `clipsync-suporte-...txt`,
/// then `-2`, `-3`... when the name is taken. Mode 0600.
fn create_bundle_file(dir: &Path, name: &str) -> std::io::Result<(PathBuf, std::fs::File)> {
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, "txt"));
    for n in 1..=9 {
        let candidate = if n == 1 {
            name.to_string()
        } else {
            format!("{stem}-{n}.{ext}")
        };
        let path = dir.join(candidate);
        match fsio::create_new(&path, 0o600) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        BUNDLE_NAME_TAKEN,
    ))
}

/// Writes the text the user previewed (a fresh one when no preview is on record).
pub(crate) async fn bundle_export(ctx: &Ctx) -> Result<Exported, UiError> {
    let text = match take_preview(&ctx.profile.dir) {
        Some(text) => text,
        None => {
            let preview = bundle_preview(ctx).await?;
            let _ = take_preview(&ctx.profile.dir); // this one is being written now
            preview.text
        }
    };
    let dir = downloads_dir(ctx).ok_or_else(|| UiError::new(code::IO, msg::NO_DOWNLOADS))?;
    let io_error = |e: std::io::Error| UiError::new(code::IO, msg::io(&clean(&e.to_string())));
    std::fs::create_dir_all(&dir).map_err(io_error)?;
    let (path, mut file) = create_bundle_file(&dir, &file_name()).map_err(io_error)?;
    let written =
        std::io::Write::write_all(&mut file, text.as_bytes()).and_then(|()| file.sync_all());
    drop(file);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&path);
        return Err(io_error(e));
    }
    Ok(Exported {
        path: path.display().to_string(),
        bytes: text.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(layout: Option<&str>) -> InputState {
        InputState {
            local: vec![],
            peers: vec![],
            screen_locked: false,
            active_peer: None,
            pending_offer: None,
            keyboard_layout: layout.map(str::to_string),
            resume_hotkey_refused: false,
            switch_supported: true,
        }
    }

    #[test]
    fn keyboard_layout_says_why_it_is_missing_and_names_the_known_klids() {
        let pick = |i: DiagItem| (i.area, i.name, i.status, i.detail);
        assert_eq!(
            pick(keyboard_item(false, None)),
            (
                "input".into(),
                "layout do teclado".into(),
                ItemStatus::Warn,
                "agente parado".into()
            )
        );
        for input in [None, Some(state(None))] {
            let i = keyboard_item(true, input.as_ref());
            assert_eq!(
                (i.status, i.detail.as_str()),
                (ItemStatus::Warn, "ainda não anunciado")
            );
        }
        let ok = |l: &str| keyboard_item(true, Some(&state(Some(l))));
        assert_eq!(ok("00010416").detail, "00010416 · Português (Brasil ABNT2)");
        assert_eq!(ok("00000409").detail, "00000409 · EUA");
        assert_eq!(ok("br(abnt2)").detail, "br(abnt2)");
        assert_eq!(ok("br(abnt2)").status, ItemStatus::Ok);
    }

    /// CI 40757283200 (8cc14c4, Windows): the input crate's own "atalhos" (always ok) came first,
    /// so the screen showed the shortcuts active without a local link.
    #[test]
    fn the_agent_state_items_replace_the_input_crate_items_of_the_same_name() {
        let theirs = |name: &str| input_item(name, ItemStatus::Ok, "do crate de input".into());
        let checks = vec![
            input_item("captura", ItemStatus::Ok, "win-hooks".into()),
            theirs("layout do teclado"),
            theirs("atalhos"),
        ];
        let ours = vec![
            keyboard_item(false, None),
            shortcuts_item(true, true, false),
        ];
        let items = with_agent_state(checks, ours);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["captura", "layout do teclado", "atalhos"]);
        assert_eq!(items[1].detail, "agente parado");
        assert_eq!(
            (items[2].status, items[2].detail.as_str()),
            (
                ItemStatus::Warn,
                "inativos: nenhuma ligação desta máquina no layout"
            )
        );
        // Another area with the same name stays.
        let other = DiagItem {
            area: "sistema".into(),
            ..theirs("atalhos")
        };
        assert_eq!(
            with_agent_state(vec![other], vec![shortcuts_item(true, true, true)]).len(),
            2
        );
    }

    #[test]
    fn shortcuts_are_active_only_with_agent_input_and_a_local_link() {
        let detail = |a, i, l| shortcuts_item(a, i, l).detail;
        assert_eq!(shortcuts_item(true, true, true).status, ItemStatus::Ok);
        assert_eq!(
            detail(true, true, true),
            "ativos: Win+Alt+1 a 9, Win+Alt+L e Win+Alt+P"
        );
        assert_eq!(detail(false, true, true), "inativos: agente parado");
        assert_eq!(
            detail(true, false, true),
            "inativos: módulo de input desligado"
        );
        assert_eq!(
            detail(true, true, false),
            "inativos: nenhuma ligação desta máquina no layout"
        );
        for (a, i, l) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert_eq!(shortcuts_item(a, i, l).status, ItemStatus::Warn);
        }
    }

    #[test]
    fn short_names_are_replaced_only_as_whole_words() {
        assert_eq!(
            replace_words("a: recusado · casa de a", "a", "<p>"),
            ("<p>: recusado · casa de <p>".to_string(), 2)
        );
        // `-`, `_` and `.` separate words; letters and digits do not
        assert_eq!(
            replace_words("pc-da-ana e ana e banana e ana_2 e ana.local", "ana", "<p>"),
            (
                "pc-da-<p> e <p> e banana e <p>_2 e <p>.local".to_string(),
                4
            )
        );
        assert_eq!(
            replace_words("ana1 e 1ana", "ana", "<p>"),
            ("ana1 e 1ana".to_string(), 0)
        );
    }

    #[test]
    fn long_names_are_replaced_inside_compound_words_too() {
        let (t, n) = replace_words(
            "notebook-do-bruno.local e xnotebook-do-brunoy e notebook-do-bruno",
            "notebook-do-bruno",
            "<p>",
        );
        assert_eq!(t, "<p>.local e x<p>y e <p>");
        assert_eq!(n, 3);
        assert_eq!(
            replace_words("sem nada", "casa", "<p>"),
            ("sem nada".to_string(), 0)
        );
        assert_eq!(replace_words("casa", "", "<p>"), ("casa".to_string(), 0));
    }

    fn device(name: &str, id_hex: &str) -> Device {
        Device {
            peer_id: id_hex.to_string(),
            short: id_hex[..8].to_string(),
            name: name.to_string(),
            state: DeviceState::Offline,
            blocked_reason: None,
            addr: Some("192.168.0.77:47800".into()),
            perms: crate::devices::Perms::default(),
            remote_modules: crate::status::Modules::default(),
            remote_version: None,
            paired_at: None,
        }
    }

    /// S6-15: no known name, id or address survives in the final text, even glued to other words.
    #[test]
    fn no_known_name_survives_in_the_final_text() {
        let bruno = "b".repeat(64);
        let ana = "a1".repeat(32);
        let g = Gathered {
            diag: Diagnostics {
                items: Vec::new(),
                network: Network {
                    listen: Some("192.168.0.5:47800".into()),
                    addresses: vec!["192.168.0.5:47800".into()],
                    peers: Vec::new(),
                },
                blocks: Vec::new(),
            },
            status: None,
            devices: vec![device("notebook-do-bruno", &bruno), device("ana", &ana)],
            device_name: Some("casa.local".into()),
        };
        let mut raw = raw_text(&g);
        raw.push_str(
            "\nnotas: notebook-do-bruno.lan · xnotebook-do-brunoy · pc-da-ana · ana_2 · \
             casa.local-2 · meu-casa.local · id=",
        );
        raw.push_str(&format!("{bruno}{} · curto={}", &ana[..8], &bruno[..8]));
        let text = redact(&raw, &g).text;
        for secret in [
            "notebook-do-bruno",
            "casa.local",
            "192.168.0.5",
            "192.168.0.77",
            bruno.as_str(),
            ana.as_str(),
            &bruno[..8],
            &ana[..8],
        ] {
            assert!(!text.contains(secret), "{secret} survived:\n{text}");
        }
        assert!(
            !text.contains("pc-da-ana") && !text.contains("ana_2"),
            "{text}"
        );
    }

    #[test]
    fn the_last_preview_is_taken_once_and_expires_per_profile() {
        let a = Path::new("/perfil-a");
        let b = Path::new("/perfil-b");
        remember_preview(a, "texto aprovado");
        assert_eq!(take_preview(b), None, "another profile has its own");
        assert_eq!(take_preview(a).as_deref(), Some("texto aprovado"));
        assert_eq!(take_preview(a), None, "only once");
        remember_preview(a, "um");
        remember_preview(a, "dois");
        assert_eq!(take_preview(a).as_deref(), Some("dois"), "the newest wins");
    }

    #[cfg(unix)]
    #[test]
    fn the_bundle_file_is_new_private_and_never_replaces_another() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("vitima.txt");
        std::fs::write(&victim, b"intacta").unwrap();
        let name = "clipsync-suporte-20261028-034000.txt";
        // a symlink where the bundle would go is not followed: the next name is used
        std::os::unix::fs::symlink(&victim, dir.path().join(name)).unwrap();
        let (path, _) = create_bundle_file(dir.path(), name).unwrap();
        assert_eq!(
            path,
            dir.path().join("clipsync-suporte-20261028-034000-2.txt")
        );
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(std::fs::read(&victim).unwrap(), b"intacta");
        let (third, _) = create_bundle_file(dir.path(), name).unwrap();
        assert_eq!(
            third,
            dir.path().join("clipsync-suporte-20261028-034000-3.txt")
        );
    }

    #[test]
    fn stamps_are_utc_civil_dates() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(951_782_400_000), "20000229-000000");
        assert_eq!(stamp(1_793_158_800_000), "20261028-034000");
        assert_eq!(stamp(4_102_444_799_000), "20991231-235959");
    }

    #[test]
    fn every_ipv4_goes_and_versions_stay() {
        let (t, n) = replace_ipv4("escuta 192.168.0.10:47800 · 127.0.0.1 · versão 0.1.0");
        assert_eq!(
            t,
            format!("escuta {0}:47800 · {0} · versão 0.1.0", msg::REDACTED_IP)
        );
        assert_eq!(n, 2);
    }
}
