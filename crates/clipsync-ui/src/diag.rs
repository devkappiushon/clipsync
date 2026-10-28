//! 6 · Diagnóstico (§12.5): `diagnostics_get`, `support_bundle_preview` and
//! `support_bundle_export`. Owned by Rivet.
//!
//! The support bundle is plain text with the identifiers taken out: peer ids (64 hex and short),
//! peer names, IP addresses and this device's name. Never in it: the key, the raw `peers.toml`,
//! `portal-tokens.toml` or clipboard content (the box is not read at all).

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use clipsync_proto::{AgentStatus, DiagStatus};
use serde::{Deserialize, Serialize};

use crate::api::code;
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

/// What the screen and the bundle share.
struct Gathered {
    diag: Diagnostics,
    status: Option<AgentStatus>,
    devices: Vec<Device>,
    device_name: Option<String>,
}

async fn gather(ctx: &Ctx) -> Gathered {
    let items = checks().await;
    let status = status::agent_status(ctx).await;
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

/// A name or id counts only as a whole word: the characters around it are not letters, digits,
/// `-`, `_` or `.`. Overmatching only hides more.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | '.')
}

fn replace_words(text: &str, needle: &str, with: &str) -> (String, usize) {
    if needle.is_empty() {
        return (text.to_string(), 0);
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

pub(crate) async fn bundle_preview(ctx: &Ctx) -> Result<BundlePreview, UiError> {
    let g = gather(ctx).await;
    Ok(redact(&raw_text(&g), &g))
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

pub(crate) async fn bundle_export(ctx: &Ctx) -> Result<Exported, UiError> {
    let preview = bundle_preview(ctx).await?;
    let dir = downloads_dir(ctx).ok_or_else(|| UiError::new(code::IO, msg::NO_DOWNLOADS))?;
    let path = dir.join(file_name());
    std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, preview.text.as_bytes()))
        .map_err(|e| UiError::new(code::IO, msg::io(&clean(&e.to_string()))))?;
    Ok(Exported {
        path: path.display().to_string(),
        bytes: preview.text.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_replaced_only_as_whole_words() {
        assert_eq!(
            replace_words("a: recusado · casa de a", "a", "<p>"),
            ("<p>: recusado · casa de <p>".to_string(), 2)
        );
        assert_eq!(
            replace_words("pc-da-ana e ana", "ana", "<p>"),
            ("pc-da-ana e <p>".to_string(), 1)
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
