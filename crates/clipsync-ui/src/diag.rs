//! 6 · Diagnóstico (§12.5): `diagnostics_get`, `support_bundle_preview` and
//! `support_bundle_export`. Owned by Rivet.
//!
//! The support bundle is plain text with the identifiers taken out: peer ids (64 hex and short),
//! peer names, IP addresses and this device's name. Never in it: the key, the raw `peers.toml`,
//! `portal-tokens.toml` or clipboard content (the box is not read at all).

use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use clipsync_proto::{AgentStatus, DiagStatus, InputState};
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
    #[serde(default = "one")]
    pub occurrences: u64,
    #[serde(default)]
    pub last_unix_ms: u64,
}

fn one() -> u64 {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    #[serde(default)]
    pub log_path: String,
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
async fn checks(profile_dir: &Path, status: Option<&AgentStatus>) -> Vec<DiagItem> {
    let (sys, clip, input) = tokio::join!(
        tokio::time::timeout(CHECK_TIMEOUT, clipsync_core::diag::system()),
        tokio::time::timeout(CHECK_TIMEOUT, clipsync_clipboard::diagnose()),
        tokio::time::timeout(
            CHECK_TIMEOUT,
            clipsync_input::diagnose_for(profile_dir, status.map(|s| s.id), status)
        ),
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

/// O layout do teclado anunciado pelo agente substitui a sondagem de mesmo nome.
/// Atalhos e configuração de saída vêm do helper contextual compartilhado com a CLI.
fn with_agent_state(mut items: Vec<DiagItem>, ours: Vec<DiagItem>) -> Vec<DiagItem> {
    items.retain(|i| !ours.iter().any(|o| o.area == i.area && o.name == i.name));
    items.extend(ours);
    items
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
    let ours = vec![keyboard_item(status.is_some(), observed.input.as_ref())];
    let items = with_agent_state(checks(&ctx.profile.dir, status.as_ref()).await, ours);
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
                    occurrences: b.occurrences,
                    last_unix_ms: if b.last_unix_ms == 0 {
                        b.at_unix_ms
                    } else {
                        b.last_unix_ms
                    },
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
            log_path: clipsync_core::logging::log_path(&ctx.profile.dir)
                .display()
                .to_string(),
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
        let count = if b.occurrences == u64::MAX {
            format!("pelo menos {}", b.occurrences)
        } else {
            b.occurrences.to_string()
        };
        let last = if b.last_unix_ms == 0 {
            b.at_unix_ms
        } else {
            b.last_unix_ms
        };
        t.push_str(&format!(
            "primeiro={} · último={} · {} ocorrências observadas · {} · {} · {}\n",
            b.at_unix_ms,
            last,
            count,
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
    let mut peer_numbers: Vec<String> = g.devices.iter().map(|d| d.peer_id.clone()).collect();
    for (id, name, addr) in extra {
        let hex = id.to_hex();
        let n = match peer_numbers.iter().position(|known| *known == hex) {
            Some(index) => index + 1,
            None => {
                peer_numbers.push(hex.clone());
                let n = peer_numbers.len();
                needles.push((hex, msg::redacted_id(n), RedactionKind::PeerId));
                needles.push((id.short(), msg::redacted_id(n), RedactionKind::PeerId));
                n
            }
        };
        // Cada observação pode conter outro nome ou endereço do mesmo par.
        if !name.is_empty()
            && !needles
                .iter()
                .any(|(known, _, kind)| *kind == RedactionKind::PeerName && *known == name)
        {
            needles.push((name, msg::redacted_peer(n), RedactionKind::PeerName));
        }
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
    // O redator também aceita texto codificado em JSON, cujas aspas e barras
    // aparecem escapadas. Conserve também a forma usada no texto comum.
    let escaped: Vec<_> = needles
        .iter()
        .filter_map(|(needle, with, kind)| {
            let encoded = serde_json::to_string(needle).ok()?;
            let escaped = encoded.strip_prefix('"')?.strip_suffix('"')?;
            (escaped != needle).then(|| (escaped.to_owned(), with.clone(), *kind))
        })
        .collect();
    needles.extend(escaped);
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

const LOG_BYTES: usize = 256 * 1024;
const LOG_LINE_BYTES: usize = 16 * 1024;
const LOG_LINES: usize = 400;
const LOG_HEADER: &str = "\n[log]\n";
const LOG_TRUNCATED: &str = "[linha truncada: limite da cauda]";
const LOG_UNREADABLE: &str = "[linha ilegível]";
const LOG_SYMLINK: &str = "log não lido: link simbólico";
static LOG_READ: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

type RedactionCounts = std::collections::BTreeMap<RedactionKind, usize>;

/// Lê no máximo a janela final, sem percorrer o prefixo nem alocar uma linha sem teto.
fn log_tail(profile_dir: &Path, max: usize) -> Vec<String> {
    let max = max.min(LOG_LINES);
    if max == 0 {
        return Vec::new();
    }
    let path = clipsync_core::logging::log_path(profile_dir);
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    // Limite aceito (§17): as verificações recusam links presentes, mas não são
    // atômicas com open. Uma troca local entre check/open ainda pode vencer a corrida.
    for (component, directory) in [(parent, true), (path.as_path(), false)] {
        let Ok(meta) = std::fs::symlink_metadata(component) else {
            return Vec::new();
        };
        if meta.file_type().is_symlink() {
            return vec![LOG_SYMLINK.into()];
        }
        if (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
            return Vec::new();
        }
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let Ok(meta) = file.metadata() else {
        return Vec::new();
    };
    if !meta.is_file() {
        return Vec::new();
    }
    let start = meta.len().saturating_sub(LOG_BYTES as u64);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    // Também limita caso o escritor acrescente linhas durante a leitura.
    if file
        .take(meta.len() - start)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Vec::new();
    }
    let mut tail = VecDeque::new();
    let mut lines = bytes.split(|byte| *byte == b'\n').peekable();
    let mut partial = start > 0;
    while let Some(line) = lines.next() {
        if line.is_empty() && lines.peek().is_none() {
            break;
        }
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let text = if partial || line.len() > LOG_LINE_BYTES {
            LOG_TRUNCATED.into()
        } else {
            std::str::from_utf8(line)
                .unwrap_or(LOG_UNREADABLE)
                .to_owned()
        };
        partial = false;
        if tail.len() == max {
            tail.pop_front();
        }
        tail.push_back(text);
    }
    tail.into_iter().collect()
}

const LOG_TEXT_OMITTED: &str = "<texto omitido>";

fn log_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 24 {
        return false;
    }
    for (index, byte) in bytes.iter().copied().enumerate() {
        let punctuation = match index {
            4 | 7 => Some(b'-'),
            10 => Some(b'T'),
            13 | 16 => Some(b':'),
            19 => Some(b'.'),
            23 => Some(b'Z'),
            _ => None,
        };
        if punctuation.map_or_else(|| !byte.is_ascii_digit(), |expected| byte != expected) {
            return false;
        }
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0_u32, |n, byte| n * 10 + u32::from(byte - b'0'))
    };
    let year = number(0, 4);
    let days = match number(5, 7) {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&number(8, 10))
        && number(11, 13) < 24
        && number(14, 16) < 60
        && number(17, 19) < 60
}

/// Lista literal, não um padrão de caracteres: texto que parece enum também
/// pode conter dados privados. Um valor novo fica omitido até entrar no contrato.
fn known_log_text(key: &str, text: &str) -> bool {
    match key {
        "evento" => matches!(
            text,
            "inventory.prepared"
                | "inventory.sent"
                | "inventory.received"
                | "inventory.missing"
                | "routes.built"
                | "routes.empty"
                | "capture.start"
                | "capture.failed"
                | "capture.ready"
                | "attempt.begin"
                | "barrier.activated"
                | "attempt.filtered"
                | "session.open"
                | "session.sent"
                | "session.active"
                | "session.refused"
                | "datagram.first"
                | "return.received"
                | "return.sent"
                | "return.failed"
                | "session.closed"
                | "enter.received"
                | "enter.refused"
                | "emulate.start"
                | "emulate.positioned"
                | "emulate.error"
                | "agent.up"
                | "peer.connected"
                | "peer.disconnected"
                | "module.input.start"
                | "module.input.stop"
                | "portal.session.created"
                | "portal.connect_eis"
                | "hooks.installed"
                | "hooks.dropped"
                | "legado"
                | "log.truncated"
        ),
        "nivel" => matches!(text, "info" | "warn" | "error"),
        "alvo" => matches!(
            text,
            "clipsync_core"
                | "clipsync_input"
                | "clipsync_clipboard"
                | "clipsync_proto"
                | "clipsync_ui"
                | "clipsync_cli"
                | "clipsync_app"
                | "clipsync_win"
                | "clipsync_core::agent"
                | "clipsync_core::logging"
                | "clipsync_core::transport"
                | "clipsync_input::capture"
                | "clipsync_input::module"
                | "clipsync_input::module::organization"
                | "clipsync_input::adapter::portal"
                | "clipsync_input::adapter::win::capture"
                | "clipsync_input::adapter::win::emulate"
        ),
        "motivo" => matches!(
            text,
            "paused"
                | "locked"
                | "screen_locked"
                | "unknown_barrier"
                | "cleanup_pending"
                | "inventory_changed"
                | "peer_offline"
                | "no_activation"
                | "not_connected"
                | "not_authorized"
                | "remote_module_disabled"
                | "local_module_disabled"
                | "too_large"
                | "transport"
                | "remote_too_old"
                | "return"
                | "refused"
                | "error"
                | "pause"
                | "lock"
                | "stop"
                | "peer_down"
                | "switch"
                | "inventory"
                | "portal_denied"
                | "adapter_failed"
                | "suspended"
                | "no_local_inventory"
                | "local_without_return"
                | "no_organization"
                | "unresolved_legacy"
                | "no_remote_inventory"
                | "remote_not_ready"
                | "local_monitor_missing"
                | "remote_monitor_missing"
                | "zone_not_found"
                | "no_links"
                | "unsupported_topology"
                | "monitor_not_found"
                | "module_disabled"
                | "invalid"
                | "duplicate"
                | "stale"
                | "expired"
                | "apply_failed"
                | "version_mismatch"
                | "send_failed"
        ),
        "mensagem" => matches!(text, "enter_routes" | "enter_span" | "enter_at" | "enter"),
        "origem" => matches!(text, "barrier" | "switch" | "test"),
        "adaptador" => matches!(
            text,
            "fake" | "portal-inputcapture" | "portal-remotedesktop" | "win-hooks" | "win-sendinput"
        ),
        "portal" => matches!(text, "input_capture" | "remote_desktop"),
        "ts" => log_timestamp(text),
        "monitor" | "uids" => text.len() == 32 && text.bytes().all(|byte| byte.is_ascii_hexdigit()),
        _ => false,
    }
}

fn redact_log_text(key: &str, text: &str, counts: &mut RedactionCounts) -> String {
    if matches!(key, "erro" | "message") {
        return LOG_TEXT_OMITTED.into();
    }
    let short_id = text.len() == 19
        && text.bytes().enumerate().all(|(i, byte)| {
            if matches!(i, 4 | 9 | 14) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        });
    if short_id {
        *counts.entry(RedactionKind::PeerId).or_default() += 1;
        "<id>".into()
    } else if known_log_text(key, text) {
        text.into()
    } else {
        LOG_TEXT_OMITTED.into()
    }
}

// §17.2 e desenho §4.6: campos extras não entram no pacote, mesmo se o produtor
// antigo ou um arquivo substituído trouxer clipboard, tecla ou objeto arbitrário.
const LOG_FIELDS: &[&str] = &[
    "v",
    "ts",
    "nivel",
    "host",
    "alvo",
    "evento",
    "message",
    "mensagem",
    "id",
    "peer",
    "nome",
    "pares",
    "monitores",
    "uids",
    "revisao",
    "rotas",
    "barreiras",
    "motivo",
    "adaptador",
    "portal",
    "attempt",
    "origem",
    "barreira",
    "pos",
    "session",
    "rota",
    "erro",
    "totais",
    "contagem",
    "enviada",
    "reutilizado",
    "descartados",
    "bytes",
];

fn redact_log_fields(
    fields: serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    counts: &mut RedactionCounts,
) -> serde_json::Map<String, serde_json::Value> {
    fields
        .into_iter()
        .filter_map(|(key, value)| {
            if !allowed.contains(&key.as_str()) {
                return None;
            }
            redact_log_value(&key, value, counts).map(|value| (key, value))
        })
        .collect()
}

fn redact_log_value(
    key: &str,
    value: serde_json::Value,
    counts: &mut RedactionCounts,
) -> Option<serde_json::Value> {
    use serde_json::Value;
    let identity = match key {
        "host" => Some(("<dispositivo>", RedactionKind::DeviceName)),
        "peer" | "id" => Some(("<id>", RedactionKind::PeerId)),
        "nome" => Some(("<nome>", RedactionKind::PeerName)),
        "pares" => Some(("<pares>", RedactionKind::PeerId)),
        _ => None,
    };
    if let Some((marker, kind)) = identity {
        *counts.entry(kind).or_default() += 1;
        return Some(Value::String(marker.into()));
    }
    match value {
        Value::Object(fields) => {
            let allowed: &[&str] = match key {
                "totais" => &[
                    "enviados",
                    "descartados",
                    "recebidos",
                    "perdidos",
                    "fora_de_ordem",
                ],
                "pos" => &["x", "y"],
                _ => return None,
            };
            Some(Value::Object(redact_log_fields(fields, allowed, counts)))
        }
        Value::Array(values) => {
            let values = match key {
                "barreiras" => values
                    .into_iter()
                    .filter_map(|value| {
                        let Value::Object(fields) = value else {
                            return None;
                        };
                        Some(Value::Object(redact_log_fields(
                            fields,
                            &["peer", "monitor", "borda", "trecho"],
                            counts,
                        )))
                    })
                    .collect(),
                "uids" => values
                    .into_iter()
                    .filter_map(|value| match value {
                        Value::String(text) => {
                            Some(Value::String(redact_log_text(key, &text, counts)))
                        }
                        Value::Number(_) => Some(value),
                        _ => None,
                    })
                    .collect(),
                "trecho" => values.into_iter().filter(Value::is_number).collect(),
                _ => return None,
            };
            Some(Value::Array(values))
        }
        Value::String(text) if !matches!(key, "totais" | "barreiras" | "uids" | "trecho") => {
            Some(Value::String(redact_log_text(key, &text, counts)))
        }
        Value::String(_) => None,
        other => Some(other),
    }
}

fn redact_log_line(line: &str, counts: &mut RedactionCounts) -> String {
    if [LOG_TRUNCATED, LOG_UNREADABLE, LOG_SYMLINK].contains(&line) {
        return line.into();
    }
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(line) else {
        return LOG_UNREADABLE.into();
    };
    let value = serde_json::Value::Object(redact_log_fields(fields, LOG_FIELDS, counts));
    serde_json::to_string(&value).unwrap_or_else(|_| LOG_UNREADABLE.into())
}

fn preview_with_log(g: &Gathered, profile_dir: &Path) -> BundlePreview {
    let mut preview = redact(&raw_text(g), g);
    let mut counts = RedactionCounts::new();
    let mut lines = VecDeque::new();
    let mut bytes = LOG_HEADER.len();
    for raw in log_tail(profile_dir, LOG_LINES) {
        let line = redact_log_line(&raw, &mut counts);
        let line = if line.len() > LOG_LINE_BYTES {
            LOG_TRUNCATED.into()
        } else {
            line
        };
        bytes += line.len() + 1;
        lines.push_back(line);
        while bytes > LOG_BYTES {
            if let Some(line) = lines.pop_front() {
                bytes -= line.len() + 1;
            }
        }
    }
    if !lines.is_empty() {
        preview.text.push_str(LOG_HEADER);
        for line in lines {
            preview.text.push_str(&line);
            preview.text.push('\n');
        }
    }
    for redaction in preview.redactions {
        *counts.entry(redaction.kind).or_default() += redaction.count;
    }
    preview.redactions = counts
        .into_iter()
        .map(|(kind, count)| Redaction { kind, count })
        .collect();
    preview
}

pub(crate) async fn bundle_preview(ctx: &Ctx) -> Result<BundlePreview, UiError> {
    let g = gather(ctx).await;
    let dir = ctx.profile.dir.clone();
    let permit = LOG_READ
        .acquire()
        .await
        .map_err(|_| UiError::new(code::IO, "leitura do log indisponível"))?;
    let preview = tokio::task::spawn_blocking(move || {
        // A guarda fica na tarefa: cancelar o chamador não libera outra leitura
        // enquanto esta ainda usa o arquivo e a janela limitada de memória.
        let _permit = permit;
        preview_with_log(&g, &dir)
    })
    .await
    .map_err(|_| UiError::new(code::IO, "não foi possível preparar o pacote de suporte"))?;
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
            inventories: vec![],
            inventory_problem: None,
            pending_monitor_offer: None,
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

    #[test]
    fn observed_keyboard_item_replaces_the_probe_without_touching_other_areas() {
        let theirs = input_item("layout do teclado", ItemStatus::Ok, "anterior".into());
        let other = DiagItem {
            area: "sistema".into(),
            ..theirs.clone()
        };
        let items = with_agent_state(vec![theirs, other], vec![keyboard_item(false, None)]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].area, "sistema");
        assert_eq!(items[1].detail, "agente parado");
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
                log_path: String::new(),
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
    #[test]
    fn legacy_block_defaults_are_normalized_in_support_text() {
        let old: Block = serde_json::from_value(serde_json::json!({"at_unix_ms":42,"peer_id":null,"peer_name":null,"module":null,"reason":"negativa"})).unwrap();
        assert_eq!(old.occurrences, 1);
        assert_eq!(old.last_unix_ms, 0);
        let mut gathered = Gathered {
            diag: Diagnostics {
                log_path: String::new(),
                items: vec![],
                network: Network {
                    listen: None,
                    addresses: vec![],
                    peers: vec![],
                },
                blocks: vec![old],
            },
            status: None,
            devices: vec![],
            device_name: None,
        };
        let text = raw_text(&gathered);
        assert!(text.contains("primeiro=42 · último=42 · 1 ocorrências observadas"));
        gathered.diag.blocks[0].occurrences = u64::MAX;
        assert!(
            raw_text(&gathered).contains("pelo menos 18446744073709551615 ocorrências observadas")
        );
    }
    #[test]
    fn cauda_do_log_preserva_as_ultimas_400_linhas() {
        let dir = tempfile::tempdir().unwrap();
        let path = clipsync_core::logging::log_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lines: Vec<_> = (0..410).map(|n| format!("linha-{n:03}")).collect();
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let tail = log_tail(dir.path(), 400);
        assert_eq!(tail, lines[10..]);
        assert!(log_tail(dir.path(), 0).is_empty());
        std::fs::write(&path, "").unwrap();
        assert!(log_tail(dir.path(), 400).is_empty());
        std::fs::remove_file(&path).unwrap();
        assert!(log_tail(dir.path(), 400).is_empty());
        std::fs::create_dir(&path).unwrap();
        assert!(log_tail(dir.path(), 400).is_empty());
    }

    #[tokio::test]
    async fn pacote_inclui_cauda_redigida_e_nao_cria_secao_sem_log() {
        use clipsync_core::{InitOptions, PeerEntry};
        use clipsync_proto::{ModuleSet, PeerId, PeerPerms};
        let dir = tempfile::Builder::new()
            .prefix("csdiag-")
            .tempdir()
            .unwrap();
        let ui = crate::Ui::start(crate::UiOptions {
            profile: Some(dir.path().join("p")),
            poll: Duration::from_secs(600),
            fake_adapters: true,
            downloads_dir: Some(dir.path().join("downloads")),
            autostart_dir: Some(dir.path().join("autostart")),
            ..Default::default()
        })
        .await
        .unwrap();
        // Inicializa só os arquivos após criar a UI, para o teste não iniciar agente.
        let profile = &ui.ctx.profile;
        profile
            .init(InitOptions {
                device_name: "desktop-privado".into(),
                modules: ModuleSet {
                    clipboard: true,
                    input: false,
                },
                listen: "127.0.0.1:0".parse().unwrap(),
                fake_adapters: true,
            })
            .unwrap();
        let id = PeerId([0x1b; 32]);
        let mut peers = profile.load_peers().unwrap();
        peers.upsert(PeerEntry {
            id,
            name: "notebook-privado".into(),
            addr: Some("[fd00::1234]:47800".parse().unwrap()),
            paired_at: "2026-11-04T14:00:00Z".into(),
            perms: PeerPerms {
                clipboard_in: false,
                clipboard_out: false,
                input_in: false,
            },
        });
        profile.save_peers(&peers).unwrap();
        let before = bundle_preview(&ui.ctx).await.unwrap();
        assert!(!before.text.contains("[log]"));
        assert!(!profile.dir.join("logs").exists());
        let path = clipsync_core::logging::log_path(&profile.dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lines: Vec<_> = (0..410).map(|n| serde_json::json!({"v":1,"evento":"peer.connected","host":"desktop-privado","peer":id.short(),"contagem":n,"mensagem":format!("linha-{n:03} notebook-privado {} 10.55.44.33 fd00::1234",id.to_hex())}).to_string()).collect();
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        let log = preview.text.split("\n[log]\n").nth(1).unwrap();
        assert_eq!(log.lines().count(), 400);
        let first: serde_json::Value = serde_json::from_str(log.lines().next().unwrap()).unwrap();
        let last: serde_json::Value = serde_json::from_str(log.lines().last().unwrap()).unwrap();
        assert_eq!(first["contagem"], 10);
        assert_eq!(last["contagem"], 409);
        assert_eq!(first["mensagem"], "<texto omitido>");
        for needle in [
            id.to_hex(),
            id.short(),
            "desktop-privado".into(),
            "notebook-privado".into(),
            "10.55.44.33".into(),
            "fd00::1234".into(),
        ] {
            assert!(!preview.text.contains(&needle), "vazou {needle}");
        }
        assert!(preview.redactions.iter().all(|r| r.count > 0));
        let diagnostics = serde_json::to_value(get(&ui.ctx).await.unwrap()).unwrap();
        assert_eq!(diagnostics["log_path"], path.display().to_string());
        // A exportação conserva exatamente a prévia, mesmo se o log mudar depois.
        std::fs::write(&path, "linha posterior\n").unwrap();
        let exported = bundle_export(&ui.ctx).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(exported.path).unwrap(),
            preview.text
        );
        ui.shutdown().await;
    }

    #[test]
    fn diagnostics_antigo_aceita_caminho_omitido() {
        let old: Diagnostics = serde_json::from_value(serde_json::json!({"items":[],"network":{"listen":null,"addresses":[],"peers":[]},"blocks":[]})).unwrap();
        assert_eq!(serde_json::to_value(old).unwrap()["log_path"], "");
    }
    #[test]
    fn redige_nome_escapado_na_cauda_json() {
        for name in [r#"desktop-"privado""#, r"notebook\privado"] {
            let g = Gathered {
                diag: serde_json::from_value(serde_json::json!({"items":[],"network":{"listen":null,"addresses":[],"peers":[]},"blocks":[]})).unwrap(),
                status: None, devices: vec![], device_name: Some(name.into()),
            };
            let raw = serde_json::json!({"evento":"agent.up","host":name}).to_string();
            let result = redact(&raw, &g);
            let log: serde_json::Value = serde_json::from_str(&result.text).unwrap();
            assert_eq!(log["host"], msg::REDACTED_DEVICE);
            assert!(
                result
                    .redactions
                    .iter()
                    .any(|r| r.kind == RedactionKind::DeviceName && r.count == 1)
            );
        }
    }
    async fn ui_for_log_test(dir: &Path) -> crate::Ui {
        crate::Ui::start(crate::UiOptions {
            profile: Some(dir.join("p")),
            poll: Duration::from_secs(600),
            fake_adapters: true,
            downloads_dir: Some(dir.join("downloads")),
            autostart_dir: Some(dir.join("autostart")),
            ..Default::default()
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn cauda_redige_identidades_historicas_ips_caminhos_e_descarta_campos() {
        let dir = tempfile::tempdir().unwrap();
        let ui = ui_for_log_test(dir.path()).await;
        let path = clipsync_core::logging::log_path(&ui.ctx.profile.dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let raw = serde_json::json!({
            "v":1, "ts":"2026-11-04T14:00:00Z", "nivel":"info", "alvo":"clipsync_core", "evento":"agent.up",
            "host":"host-historico", "id":"0123456789abcdef", "peer":"par-removido", "nome":"nome-\"historico\"",
            "pares":["outro-par-removido"], "session":42,
            "erro":"falhou /home/usuario-antigo/.local/token e C:\\Users\\usuario-win\\segredo em 203.0.113.9:47800 (2001:db8::abcd)",
            "message":"conexão [2001:db8:abcd::1234]:47800, ::ffff:192.0.2.8",
            "mensagem":"versão 0.5.0; falha de transporte",
            "clipboard_text":"conteudo-proibido", "tecla":"tecla-proibida", "keycode":999,
            "barreiras":[{"peer":"par-da-barreira", "monitor":7, "borda":"left", "trecho":[0.1,0.9], "clipboard_text":"aninhado-proibido"}],
            "totais":{"enviados":2,"descartados":1,"keycode":998}
        });
        std::fs::write(
            &path,
            format!("{raw}\ntexto cru proibido\n{{JSON inválido\n"),
        )
        .unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        let log = preview.text.split("\n[log]\n").nth(1).unwrap();
        let rows: Vec<_> = log.lines().collect();
        let value: serde_json::Value = serde_json::from_str(rows[0]).unwrap();
        assert!(
            value.get("clipboard_text").is_none(),
            "campo proibido passou"
        );
        assert!(value.get("tecla").is_none());
        assert!(value.get("keycode").is_none());
        assert!(value["barreiras"][0].get("clipboard_text").is_none());
        assert!(value["totais"].get("keycode").is_none());
        assert_eq!(value["session"], 42);
        assert_eq!(value["totais"]["enviados"], 2);
        assert_eq!(
            value["barreiras"][0]["trecho"],
            serde_json::json!([0.1, 0.9])
        );
        assert_eq!(value["mensagem"], "<texto omitido>");
        assert_eq!(value["erro"], "<texto omitido>");
        assert_eq!(value["message"], "<texto omitido>");
        for private in [
            "host-historico",
            "0123456789abcdef",
            "par-removido",
            "historico",
            "par-da-barreira",
            "usuario-antigo",
            "usuario-win",
            "203.0.113.9",
            "2001:db8::abcd",
            "2001:db8:abcd::1234",
            "192.0.2.8",
            "conteudo-proibido",
            "aninhado-proibido",
            "texto cru proibido",
        ] {
            assert!(!log.contains(private), "vazou {private}");
        }
        assert_eq!(&rows[1..], &["[linha ilegível]", "[linha ilegível]"]);
        let exported = bundle_export(&ui.ctx).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(exported.path).unwrap(),
            preview.text
        );
        ui.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cauda_recusa_links_no_arquivo_e_no_diretorio() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let ui = ui_for_log_test(dir.path()).await;
        let path = clipsync_core::logging::log_path(&ui.ctx.profile.dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let external = dir.path().join("externo");
        std::fs::create_dir(&external).unwrap();
        let target = external.join("clipsync.jsonl");
        std::fs::write(&target, "{\"evento\":\"externo-nao-ler\"}\n").unwrap();
        symlink(&target, &path).unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        assert!(preview.text.contains("log não lido: link simbólico"));
        assert!(!preview.text.contains("externo-nao-ler"));
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
        symlink(&external, path.parent().unwrap()).unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        assert!(preview.text.contains("log não lido: link simbólico"));
        assert!(!preview.text.contains("externo-nao-ler"));
        ui.shutdown().await;
    }

    #[test]
    fn cauda_le_so_a_janela_final_e_limita_linhas_grandes() {
        let dir = tempfile::tempdir().unwrap();
        let path = clipsync_core::logging::log_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!(
                "prefixo-fora-da-janela\n{}\nultimo\n",
                "x".repeat(300 * 1024)
            ),
        )
        .unwrap();
        let tail = log_tail(dir.path(), 400);
        assert!(
            !tail
                .iter()
                .any(|line| line.contains("prefixo-fora-da-janela"))
        );
        assert!(tail.iter().any(|line| line.contains("linha truncada")));
        assert_eq!(tail.last().unwrap(), "ultimo");
        assert!(tail.iter().all(|line| line.len() <= 16 * 1024));
        std::fs::write(&path, format!("{}\nultimo\n", "x".repeat(128 * 1024))).unwrap();
        let tail = log_tail(dir.path(), 400);
        assert!(tail[0].contains("linha truncada"));
        assert!(tail.iter().all(|line| line.len() <= 16 * 1024));
    }

    #[tokio::test]
    async fn cauda_limita_tamanho_da_secao_apos_redigir() {
        let dir = tempfile::tempdir().unwrap();
        let ui = ui_for_log_test(dir.path()).await;
        let path = clipsync_core::logging::log_path(&ui.ctx.profile.dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Cada string curta não permitida cresce para o marcador de omissão.
        let fields: serde_json::Map<String, serde_json::Value> = LOG_FIELDS
            .iter()
            .map(|key| ((*key).into(), serde_json::json!("?")))
            .collect();
        let line = serde_json::Value::Object(fields).to_string();
        std::fs::write(&path, (line + "\n").repeat(500)).unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        let log = preview.text.split("\n[log]\n").nth(1).unwrap();
        assert!(log.len() + "\n[log]\n".len() <= 256 * 1024);
        assert!(log.lines().count() <= 400);
        assert!(log.contains("<texto omitido>"));
        assert!(
            log.lines().count() < 400,
            "o teto de bytes deve cortar linhas expandidas"
        );
        ui.shutdown().await;
    }
    #[test]
    fn cauda_redige_valores_sem_trocar_chaves_iguais_a_nomes() {
        let dir = tempfile::tempdir().unwrap();
        let path = clipsync_core::logging::log_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"evento":"agent.up","peer":"historico","mensagem":"peer"}"#,
        )
        .unwrap();
        let g = Gathered {
            diag: serde_json::from_value(serde_json::json!({"items":[],"network":{"listen":null,"addresses":[],"peers":[]},"blocks":[]})).unwrap(),
            status: None, devices: vec![], device_name: Some("peer".into()),
        };
        let preview = preview_with_log(&g, dir.path());
        let log = preview.text.split(LOG_HEADER).nth(1).unwrap();
        let row: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert!(
            row.get("peer").is_some(),
            "a redação do nome alterou a chave do esquema"
        );
        assert_eq!(row["mensagem"], "<texto omitido>");
        assert!(
            row.as_object()
                .unwrap()
                .keys()
                .all(|key| LOG_FIELDS.contains(&key.as_str()))
        );
    }
    async fn check_log_free_text_omitted(field: &str, sensitive: &str) {
        let dir = tempfile::tempdir().unwrap();
        let ui = ui_for_log_test(dir.path()).await;
        let path = clipsync_core::logging::log_path(&ui.ctx.profile.dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut row = serde_json::json!({"v":1,"evento":"emulate.error","session":42});
        row[field] = serde_json::json!(sensitive);
        std::fs::write(&path, format!("{row}\n")).unwrap();
        let preview = bundle_preview(&ui.ctx).await.unwrap();
        let log = preview.text.split(LOG_HEADER).nth(1).unwrap();
        let value: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(
            value[field], "<texto omitido>",
            "texto livre sobrevivente em {field}"
        );
        assert_eq!(value["evento"], "emulate.error");
        assert_eq!(value["session"], 42);
        assert!(!log.contains(sensitive));
        let exported = bundle_export(&ui.ctx).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(exported.path).unwrap(),
            preview.text
        );
        ui.shutdown().await;
    }

    #[tokio::test]
    async fn cauda_omite_ipv6_com_escopo_inteiro() {
        check_log_free_text_omitted("erro", "[fe80::1234%7]:47800").await;
    }

    #[tokio::test]
    async fn cauda_omite_ipv6_com_prefixo_inteiro() {
        check_log_free_text_omitted("mensagem", "servidor:2001:db8:7654::1 indisponível").await;
    }

    #[tokio::test]
    async fn cauda_omite_caminho_linux_com_espacos_inteiro() {
        check_log_free_text_omitted(
            "message",
            r#"falha ao abrir "/home/pessoa-sintetica/Documento sigiloso.txt""#,
        )
        .await;
    }

    #[tokio::test]
    async fn cauda_omite_caminho_windows_com_espacos_inteiro() {
        check_log_free_text_omitted("erro", r#"falha ao abrir "C:\Users\Nome Sobrenome""#).await;
    }

    #[test]
    fn cauda_preserva_so_valores_confirmados_do_esquema() {
        let dir = tempfile::tempdir().unwrap();
        let path = clipsync_core::logging::log_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let g = Gathered {
            diag: serde_json::from_value(serde_json::json!({"items":[],"network":{"listen":null,"addresses":[],"peers":[]},"blocks":[]})).unwrap(),
            status: None, devices: vec![], device_name: None,
        };
        let uid = "0123456789abcdef0123456789abcdef";
        let raw = serde_json::json!({
            "v":1, "ts":"2026-11-04T14:00:00.123Z", "evento":"session.open", "nivel":"info",
            "alvo":"clipsync_input::capture", "motivo":"inventory_changed", "mensagem":"enter_routes",
            "origem":"switch", "adaptador":"portal-inputcapture", "portal":"input_capture",
            "erro":"aaaa-bbbb-cccc-dddd", "message":"enter", "session":42, "enviada":false,
            "host":uid, "peer":"aaaa-bbbb-cccc-dddd", "id":uid, "nome":"historico", "pares":[uid],
            "uids":[uid,"aaaa-bbbb-cccc-dddd","monitor-particular"],
            "barreiras":[{"monitor":uid,"peer":"aaaa-bbbb-cccc-dddd","trecho":[0,1]}]
        });
        std::fs::write(&path, raw.to_string()).unwrap();
        let preview = preview_with_log(&g, dir.path());
        let row: serde_json::Value =
            serde_json::from_str(preview.text.split(LOG_HEADER).nth(1).unwrap().trim()).unwrap();
        for field in [
            "ts",
            "evento",
            "nivel",
            "alvo",
            "motivo",
            "mensagem",
            "origem",
            "adaptador",
            "portal",
            "session",
            "enviada",
        ] {
            assert_eq!(row[field], raw[field], "campo seguro {field}");
        }
        assert_eq!(row["erro"], "<texto omitido>");
        assert_eq!(row["message"], "<texto omitido>");
        assert_eq!(row["uids"][0], uid);
        assert_eq!(row["uids"][1], "<id>");
        assert_eq!(row["uids"][2], "<texto omitido>");
        assert_eq!(row["barreiras"][0]["monitor"], uid);
        for field in ["host", "peer", "id", "nome", "pares"] {
            assert_ne!(row[field], raw[field]);
        }
        for field in [
            "evento",
            "nivel",
            "alvo",
            "motivo",
            "mensagem",
            "origem",
            "adaptador",
            "portal",
            "session",
        ] {
            let mut invalid = raw.clone();
            invalid[field] = serde_json::json!("servidor:2001:db8::1");
            std::fs::write(&path, invalid.to_string()).unwrap();
            let preview = preview_with_log(&g, dir.path());
            let row: serde_json::Value =
                serde_json::from_str(preview.text.split(LOG_HEADER).nth(1).unwrap().trim())
                    .unwrap();
            assert_eq!(
                row[field], "<texto omitido>",
                "chave {field} não autoriza texto livre"
            );
        }
    }

    #[test]
    fn cauda_recusa_timestamp_e_uid_fora_do_formato_estrito() {
        let dir = tempfile::tempdir().unwrap();
        let path = clipsync_core::logging::log_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let g = Gathered {
            diag: serde_json::from_value(serde_json::json!({"items":[],"network":{"listen":null,"addresses":[],"peers":[]},"blocks":[]})).unwrap(),
            status: None, devices: vec![], device_name: None,
        };
        for ts in [
            "2026-11-04T14:00:00Z",
            "2026-11-04T14:00:00.123+00:00",
            "2026-02-29T14:00:00.123Z",
            "2026-11-04T24:00:00.123Z",
            "2026-11-04T14:00:00.123Z segredo",
        ] {
            let raw = serde_json::json!({"v":1,"evento":"agent.up","ts":ts,"uids":["0123456789abcdef0123456789abcdef EXTRA","0123456789abcdef0123456789abcdeG"],"barreiras":[{"monitor":"monitor pessoal"}]});
            std::fs::write(&path, raw.to_string()).unwrap();
            let preview = preview_with_log(&g, dir.path());
            let row: serde_json::Value =
                serde_json::from_str(preview.text.split(LOG_HEADER).nth(1).unwrap().trim())
                    .unwrap();
            assert_eq!(row["ts"], "<texto omitido>", "timestamp {ts}");
            assert_eq!(
                row["uids"],
                serde_json::json!(["<texto omitido>", "<texto omitido>"])
            );
            assert_eq!(row["barreiras"][0]["monitor"], "<texto omitido>");
        }
    }
}
