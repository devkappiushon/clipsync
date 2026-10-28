//! Events (§12.5): `{event, data}` on `clipsync://event`. Owned by Rivet.
//!
//! 1 to 6, 12 (zones, offer) and 13 come from the poll below: every `UiOptions::poll` it looks
//! at the agent (`Status`, `ClipInbox` with clipboard on, `InputState` with input on) and
//! compares with the previous look, so an external agent produces the same events. 7 to 11 and
//! 12 (saved) are born where the crate acts.

use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use clipsync_proto::{BlockRecord, InputState, ModuleKind, PeerId, PeerState, RemovalRecord};
use serde::{Deserialize, Serialize};

use crate::clipboard::{self, ItemState};
use crate::devices::Device;
use crate::prefs::{Notifications, Prefs};
use crate::status::{self, AgentMode, ModuleName, Modules};
use crate::{Ctx, clean, lock, msg};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum UiEvent {
    PeerConnected(PeerConnected),
    PeerDisconnected(PeerRef),
    PeerBlocked(PeerBlocked),
    ClipReceived(ClipReceived),
    PauseChanged(PauseChanged),
    AgentState(AgentStateData),
    PairCodeReady(PairCodeReady),
    PairResult(PairResult),
    Notification(Notification),
    PrefsChanged(PrefsChanged),
    Resync(Resync),
    LayoutChanged(LayoutChanged),
    ScreenLockChanged(ScreenLockChanged),
}

impl UiEvent {
    /// What the casca sends when its receiver lagged (`RecvError::Lagged`): the front fetches
    /// everything again.
    pub fn resync() -> Self {
        UiEvent::Resync(Resync {})
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PeerConnected {
    pub peer_id: String,
    pub short: String,
    pub name: String,
    pub addr: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PeerRef {
    pub peer_id: String,
    pub short: String,
    pub name: String,
}

/// A new entry of the agent's `recent_blocks`. A module's own block has no peer.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PeerBlocked {
    pub peer_id: Option<String>,
    pub name: Option<String>,
    pub module: Option<ModuleName>,
    pub reason: String,
    pub at_unix_ms: u64,
}

/// No preview: content never travels in an event.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClipReceived {
    /// 32 hex.
    pub id: String,
    pub short: String,
    pub from: String,
    pub bytes: usize,
    pub state: ItemState,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PauseChanged {
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentStateData {
    pub agent: AgentMode,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairPeer {
    pub name: String,
    pub short: String,
    pub modules: Modules,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairCodeReady {
    pub attempt_id: u64,
    /// "123 456"
    pub code: String,
    pub peer: PairPeer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairOutcomeName {
    Paired,
    DeclinedLocal,
    DeclinedPeer,
    Failed,
    Expired,
    Cancelled,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairResult {
    pub attempt_id: u64,
    pub outcome: PairOutcomeName,
    pub device: Option<Device>,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    ItemReceived,
    DeviceConnected,
    Block,
    /// v1.16: a peer removed this device (follows `notifications.block`).
    DeviceRemoved,
}

/// Already filtered by the preferences; never clipboard content (§12.2 rule 7).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Notification {
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrefsChanged {
    pub prefs: Prefs,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Resync {}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayoutReason {
    Saved,
    Zones,
    Offer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LayoutChanged {
    pub rev: u64,
    pub reason: LayoutReason,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScreenLockChanged {
    pub screen_locked: bool,
    pub active_peer_id: Option<String>,
}

/// What one tick of the poll saw.
pub(crate) struct Snapshot {
    agent: AgentStateData,
    paused: bool,
    /// An agent answered `Status` in this tick.
    live: bool,
    /// Connected peers: `peer_connected` and `peer_disconnected` compare these.
    connected: BTreeMap<PeerId, PeerConnected>,
    /// The agent's `recent_blocks`, newest first.
    blocks: Vec<BlockRecord>,
    /// The agent's `recent_removals` (peers that removed this device), newest first.
    removals: Vec<RemovalRecord>,
    /// The clipboard box by id (32 hex), without previews; `None` when the module is off.
    inbox: Option<BTreeMap<String, ClipReceived>>,
    /// What the input module answered to `InputState`.
    input: Option<InputState>,
}

impl Snapshot {
    pub(crate) async fn take(ctx: &Ctx) -> Self {
        let o = status::observe(ctx).await;
        let paused = match &o.status {
            Some(s) => s.paused,
            None => ctx.profile.load_state().is_ok_and(|s| s.paused),
        };
        let removals = o
            .status
            .as_ref()
            .map(|s| s.recent_removals.clone())
            .unwrap_or_default();
        let (connected, blocks) = match &o.status {
            Some(s) => (
                s.peers
                    .iter()
                    .filter(|p| p.state == PeerState::Connected)
                    .map(|p| {
                        let ev = PeerConnected {
                            peer_id: p.id.to_hex(),
                            short: p.id.short(),
                            name: clean(&p.name),
                            addr: p.addr.map(|a| a.to_string()),
                        };
                        (p.id, ev)
                    })
                    .collect(),
                s.recent_blocks.clone(),
            ),
            None => (BTreeMap::new(), Vec::new()),
        };
        let clipboard_on = o.status.as_ref().is_some_and(|s| {
            s.modules
                .iter()
                .any(|m| m.kind == ModuleKind::Clipboard && m.enabled)
        });
        let inbox = if clipboard_on {
            clipboard::inbox(ctx).await.ok().map(|items| {
                items
                    .iter()
                    .map(|i| {
                        let item = clipboard::InboxItem::new(i);
                        let ev = ClipReceived {
                            id: item.id.clone(),
                            short: item.short,
                            from: item.from,
                            bytes: item.bytes,
                            state: item.state,
                        };
                        (item.id, ev)
                    })
                    .collect()
            })
        } else {
            None
        };
        Self {
            agent: AgentStateData {
                agent: o.agent,
                error: o.agent_error,
            },
            paused,
            live: o.status.is_some(),
            connected,
            blocks,
            removals,
            inbox,
            input: o.input,
        }
    }

    /// Blocks in `now` that this look did not have, oldest first.
    fn new_blocks<'a>(&self, now: &'a Snapshot) -> impl Iterator<Item = &'a BlockRecord> {
        fresh(&self.blocks, &now.blocks)
    }

    /// `layout_changed` (zones, offer) and `screen_lock_changed`, when both looks have an
    /// `InputState` to compare.
    fn input_events(&self, now: &Snapshot, layout_rev: impl Fn() -> u64) -> Vec<UiEvent> {
        let (Some(was), Some(is)) = (&self.input, &now.input) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if was.local != is.local || was.peers != is.peers {
            out.push(UiEvent::LayoutChanged(LayoutChanged {
                rev: layout_rev(),
                reason: LayoutReason::Zones,
            }));
        }
        if is.pending_offer.is_some() && was.pending_offer != is.pending_offer {
            out.push(UiEvent::LayoutChanged(LayoutChanged {
                rev: layout_rev(),
                reason: LayoutReason::Offer,
            }));
        }
        if (was.screen_locked, was.active_peer) != (is.screen_locked, is.active_peer) {
            out.push(UiEvent::ScreenLockChanged(ScreenLockChanged {
                screen_locked: is.screen_locked,
                active_peer_id: is.active_peer.map(|p| p.to_hex()),
            }));
        }
        out
    }

    fn diff(&self, now: &Snapshot, notify: Notifications) -> Vec<UiEvent> {
        let mut out = Vec::new();
        if now.paused != self.paused {
            out.push(UiEvent::PauseChanged(PauseChanged { paused: now.paused }));
        }
        if now.agent != self.agent {
            out.push(UiEvent::AgentState(now.agent.clone()));
        }
        for (id, p) in &self.connected {
            if !now.connected.contains_key(id) {
                out.push(UiEvent::PeerDisconnected(PeerRef {
                    peer_id: p.peer_id.clone(),
                    short: p.short.clone(),
                    name: p.name.clone(),
                }));
            }
        }
        for (id, p) in &now.connected {
            if !self.connected.contains_key(id) {
                out.push(UiEvent::PeerConnected(p.clone()));
                if notify.device_connected {
                    out.push(UiEvent::Notification(Notification {
                        kind: NotificationKind::DeviceConnected,
                        title: msg::NOTIFY_CONNECTED.to_string(),
                        body: msg::notify_connected(&p.name),
                    }));
                }
            }
        }
        // Once per removal, no peer_blocked with it; peer_disconnected comes from `connected`.
        for r in fresh(&self.removals, &now.removals) {
            if notify.block {
                out.push(UiEvent::Notification(Notification {
                    kind: NotificationKind::DeviceRemoved,
                    title: msg::NOTIFY_REMOVED.to_string(),
                    body: msg::notify_removed(&clean(&r.peer_name)),
                }));
            }
        }
        for b in self.new_blocks(now) {
            let ev = PeerBlocked {
                peer_id: b.peer.map(|id| id.to_hex()),
                name: b.peer_name.as_deref().map(clean),
                module: b.module.map(ModuleName::from),
                reason: clean(&b.reason),
                at_unix_ms: b.at_unix_ms,
            };
            if notify.block {
                out.push(UiEvent::Notification(Notification {
                    kind: NotificationKind::Block,
                    title: msg::NOTIFY_BLOCK.to_string(),
                    body: msg::notify_block(ev.name.as_deref(), &ev.reason),
                }));
            }
            out.push(UiEvent::PeerBlocked(ev));
        }
        if let (Some(was), Some(is)) = (&self.inbox, &now.inbox) {
            for (id, item) in is {
                if !was.contains_key(id) {
                    out.push(UiEvent::ClipReceived(item.clone()));
                    if notify.item_received {
                        out.push(UiEvent::Notification(Notification {
                            kind: NotificationKind::ItemReceived,
                            title: msg::NOTIFY_ITEM.to_string(),
                            body: msg::notify_item(&item.from, item.bytes),
                        }));
                    }
                }
            }
        }
        out
    }
}

/// Entries of a newest-first ring (`now`) that `was` did not have, oldest first: the ones before
/// our newest; a ring that lost it (new agent) is all new.
fn fresh<'a, T: PartialEq>(was: &[T], now: &'a [T]) -> impl Iterator<Item = &'a T> {
    let n = match was.first() {
        Some(top) => now.iter().position(|x| x == top).unwrap_or(now.len()),
        None => now.len(),
    };
    now[..n].iter().rev()
}

/// The `rev` of `layout.toml` (0 when there is none yet).
fn layout_rev(ctx: &Ctx) -> u64 {
    clipsync_input::layout::Layout::read(&ctx.profile.dir.join("layout.toml"))
        .ok()
        .flatten()
        .map_or(0, |l| l.rev)
}

/// Runs until the `Ui` is gone (or `shutdown` aborts it). `first` is the look taken inside
/// `Ui::start`, so a change made right after `start` is never swallowed by the baseline.
pub(crate) async fn poll(ctx: Weak<Ctx>, first: Snapshot) {
    let mut last = first;
    loop {
        let Some(every) = ctx.upgrade().map(|c| c.opts.poll) else {
            return;
        };
        tokio::time::sleep(every).await;
        let Some(c) = ctx.upgrade() else { return };
        let mut now = Snapshot::take(&c).await;
        if !now.live {
            // No agent this tick: keep the ring, so the same blocks are not new next time.
            now.blocks = std::mem::take(&mut last.blocks);
            now.removals = std::mem::take(&mut last.removals);
        }
        if now.inbox.is_none() {
            // A box that could not be read this tick is not a box emptied.
            now.inbox = last.inbox.take();
        }
        let notify = lock(&c.prefs).notifications;
        let layout_rev = || layout_rev(&c);
        for ev in last
            .diff(&now, notify)
            .into_iter()
            .chain(last.input_events(&now, layout_rev))
        {
            c.emit(ev);
        }
        last = now;
        drop::<Arc<Ctx>>(c);
    }
}
