//! Events (§12.5): `{event, data}` on `clipsync://event`. Owned by Rivet.
//!
//! 1 to 6, 12 (zones, offer) and 13 come from the poll below: every `UiOptions::poll` it looks
//! at the agent (`Status`, `ClipInbox` with clipboard on, `InputState` with input on) and
//! compares with the previous look, so an external agent produces the same events. 7 to 11 and
//! 12 (saved) are born where the crate acts.

use std::sync::{Arc, Weak};

use serde::{Deserialize, Serialize};

use crate::Ctx;
use crate::clipboard::ItemState;
use crate::devices::Device;
use crate::prefs::Prefs;
use crate::status::{self, AgentMode, ModuleName, Modules};

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
}

impl Snapshot {
    pub(crate) async fn take(ctx: &Ctx) -> Self {
        let o = status::observe(ctx).await;
        let paused = match &o.status {
            Some(s) => s.paused,
            None => ctx.profile.load_state().is_ok_and(|s| s.paused),
        };
        Self {
            agent: AgentStateData {
                agent: o.agent,
                error: o.agent_error,
            },
            paused,
        }
    }

    fn diff(&self, now: &Snapshot) -> Vec<UiEvent> {
        let mut out = Vec::new();
        if now.paused != self.paused {
            out.push(UiEvent::PauseChanged(PauseChanged { paused: now.paused }));
        }
        if now.agent != self.agent {
            out.push(UiEvent::AgentState(now.agent.clone()));
        }
        out
    }
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
        let now = Snapshot::take(&c).await;
        for ev in last.diff(&now) {
            c.emit(ev);
        }
        last = now;
        drop::<Arc<Ctx>>(c);
    }
}
