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
    TraversalChanged(TraversalChanged),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraversalChanged {
    pub traversal: crate::traversal::Traversal,
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
    #[serde(default = "one")]
    pub occurrences: u64,
    #[serde(default)]
    pub last_unix_ms: u64,
}

fn one() -> u64 {
    1
}

#[derive(Clone, PartialEq, Eq)]
struct BlockKey {
    peer: Option<PeerId>,
    module: Option<ModuleKind>,
    reason: String,
}
impl From<&BlockRecord> for BlockKey {
    fn from(block: &BlockRecord) -> Self {
        Self {
            peer: block.peer,
            module: block.module,
            reason: block.reason.clone(),
        }
    }
}
fn block_delta(was: &[BlockRecord], now: &BlockRecord) -> u64 {
    let previous = was
        .iter()
        .find(|old| old.peer == now.peer && old.module == now.module && old.reason == now.reason);
    match previous {
        Some(old) if old.at_unix_ms == now.at_unix_ms && now.occurrences >= old.occurrences => {
            now.occurrences - old.occurrences
        }
        _ => now.occurrences,
    }
}

const BLOCK_WINDOW: std::time::Duration = std::time::Duration::from_secs(30);
const BLOCK_DIVERSITY: usize = 20;
#[derive(Default)]
struct BlockNotifications {
    last_sent: Option<tokio::time::Instant>,
    pending_occurrences: u64,
    pending_keys: Vec<BlockKey>,
    pending_peers: Vec<PeerId>,
    keys_overflow: bool,
    peers_overflow: bool,
}
impl BlockNotifications {
    fn clear_pending(&mut self) {
        self.pending_occurrences = 0;
        self.pending_keys.clear();
        self.pending_peers.clear();
        self.keys_overflow = false;
        self.peers_overflow = false;
    }
    fn observe<'a>(&mut self, deltas: impl Iterator<Item = (&'a BlockRecord, u64)>, enabled: bool) {
        if !enabled {
            self.clear_pending();
            return;
        }
        for (block, count) in deltas {
            // O retorno local tem evento próprio, baseado no estado atual, sem resumo duplicado.
            if count == 0
                || (block.module == Some(ModuleKind::Input)
                    && clipsync_input::msg::control_returned_notice(&block.reason).is_some())
            {
                continue;
            }
            self.pending_occurrences = self.pending_occurrences.saturating_add(count);
            let key = BlockKey::from(block);
            if !self.pending_keys.contains(&key) {
                if self.pending_keys.len() < BLOCK_DIVERSITY {
                    self.pending_keys.push(key);
                } else {
                    self.keys_overflow = true;
                }
            }
            if let Some(peer) = block.peer
                && !self.pending_peers.contains(&peer)
            {
                if self.pending_peers.len() < BLOCK_DIVERSITY {
                    self.pending_peers.push(peer);
                } else {
                    self.peers_overflow = true;
                }
            }
        }
    }
    fn flush(&mut self, now: tokio::time::Instant, enabled: bool) -> Option<Notification> {
        if !enabled {
            self.clear_pending();
            return None;
        }
        if self.pending_occurrences == 0
            || self.last_sent.is_some_and(|last| {
                now.checked_duration_since(last).unwrap_or_default() < BLOCK_WINDOW
            })
        {
            return None;
        }
        let notice = Notification {
            kind: NotificationKind::Block,
            title: msg::NOTIFY_BLOCK.into(),
            body: msg::notify_block_summary(
                self.pending_occurrences,
                self.pending_keys.len(),
                self.pending_peers.len(),
                self.keys_overflow,
                self.peers_overflow,
            ),
        };
        self.last_sent = Some(now);
        self.clear_pending();
        Some(notice)
    }
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
    /// Retorno local de segurança: imediato, seguindo a preferência `block`.
    ControlReturned,
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
    input_block: Option<String>,
    control_returned_id: u64,
    /// The agent's `recent_removals` (peers that removed this device), newest first.
    removals: Vec<RemovalRecord>,
    /// The clipboard box by id (32 hex), without previews; `None` when the module is off.
    inbox: Option<BTreeMap<String, ClipReceived>>,
    /// What the input module answered to `InputState`.
    input: Option<InputState>,
    traversal: crate::traversal::Traversal,
}

impl Snapshot {
    pub(crate) async fn take(ctx: &Ctx) -> Self {
        let o = status::observe(ctx).await;
        let traversal = crate::traversal::observe(ctx, &o);
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
            input_block: o
                .status
                .as_ref()
                .and_then(|s| s.modules.iter().find(|m| m.kind == ModuleKind::Input))
                .and_then(|m| m.last_block.clone()),
            control_returned_id: o
                .status
                .as_ref()
                .and_then(|s| s.modules.iter().find(|m| m.kind == ModuleKind::Input))
                .map_or(0, |m| m.control_returned_id),
            removals,
            inbox,
            input: o.input,
            traversal,
        }
    }

    /// Blocks in `now` that this look did not have, oldest first.
    fn new_blocks<'a>(&self, now: &'a Snapshot) -> impl Iterator<Item = &'a BlockRecord> {
        self.new_block_deltas(now).map(|(block, _)| block)
    }

    fn new_block_deltas<'a>(
        &self,
        now: &'a Snapshot,
    ) -> impl Iterator<Item = (&'a BlockRecord, u64)> {
        now.blocks.iter().rev().filter_map(|block| {
            let delta = block_delta(&self.blocks, block);
            (delta > 0).then_some((block, delta))
        })
    }

    fn retain_unobserved(&mut self, previous: &Snapshot) {
        if !self.live {
            self.blocks = previous.blocks.clone();
            self.input_block = previous.input_block.clone();
            self.control_returned_id = previous.control_returned_id;
            self.removals = previous.removals.clone();
        }
        if self.inbox.is_none() {
            self.inbox = previous.inbox.clone();
        }
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
        let previous_return = self
            .input_block
            .as_deref()
            .and_then(clipsync_input::msg::control_returned_notice);
        let current_return = now
            .input_block
            .as_deref()
            .and_then(clipsync_input::msg::control_returned_notice);
        if notify.block
            && (self.control_returned_id != now.control_returned_id
                || previous_return != current_return)
            && let Some(text) = current_return
        {
            out.push(UiEvent::Notification(Notification {
                kind: NotificationKind::ControlReturned,
                title: "ClipSync".into(),
                body: text.into(),
            }));
        }
        if now.traversal != self.traversal {
            out.push(UiEvent::TraversalChanged(TraversalChanged {
                traversal: now.traversal.clone(),
            }));
        }
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
                occurrences: b.occurrences,
                last_unix_ms: if b.last_unix_ms == 0 {
                    b.at_unix_ms
                } else {
                    b.last_unix_ms
                },
            };
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
    let mut block_notifications = BlockNotifications::default();
    loop {
        let Some(every) = ctx.upgrade().map(|c| c.opts.poll) else {
            return;
        };
        tokio::time::sleep(every).await;
        let Some(c) = ctx.upgrade() else { return };
        let mut now = Snapshot::take(&c).await;
        now.retain_unobserved(&last);
        let notify = lock(&c.prefs).notifications;
        block_notifications.observe(last.new_block_deltas(&now), notify.block);
        if let Some(notification) =
            block_notifications.flush(tokio::time::Instant::now(), notify.block)
        {
            c.emit(UiEvent::Notification(notification));
        }
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

#[cfg(test)]
mod block_notice_tests {
    use super::*;
    fn block(reason: &str, count: u64) -> BlockRecord {
        BlockRecord {
            peer: Some(PeerId([1; 32])),
            peer_name: Some("nome".into()),
            module: Some(ModuleKind::Input),
            reason: reason.into(),
            at_unix_ms: 10,
            last_unix_ms: 20,
            occurrences: count,
        }
    }
    fn snapshot(blocks: Vec<BlockRecord>) -> Snapshot {
        Snapshot {
            agent: AgentStateData {
                agent: AgentMode::InApp,
                error: None,
            },
            paused: false,
            live: true,
            connected: BTreeMap::new(),
            blocks,
            input_block: None,
            control_returned_id: 0,
            removals: vec![],
            inbox: None,
            input: None,
            traversal: crate::traversal::Traversal::default(),
        }
    }
    const RETURNED: &str = "Controle devolvido a este computador: o outro parou de responder. As próximas teclas serão digitadas aqui.";
    const FAILED: &str = "Controle devolvido a este computador: o outro não aceitou o mouse e o teclado. As próximas teclas serão digitadas aqui.";
    fn return_notices(was: &Snapshot, now: &Snapshot, enabled: bool) -> Vec<Notification> {
        let prefs = Notifications {
            block: enabled,
            ..Notifications::default()
        };
        was.diff(now, prefs)
            .into_iter()
            .filter_map(|e| match e {
                UiEvent::Notification(n)
                    if serde_json::to_value(n.kind).unwrap() == "control_returned" =>
                {
                    Some(n)
                }
                _ => None,
            })
            .collect()
    }
    // Respostas JSON controladas; coleta e comparação usam o caminho real da UI.
    struct ReturnStatus(std::sync::Mutex<serde_json::Value>);

    impl clipsync_core::control::ControlHandler for ReturnStatus {
        fn handle<'life0, 'async_trait>(
            &'life0 self,
            req: clipsync_proto::ControlRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = clipsync_proto::ControlResponse>
                    + Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                match req {
                    clipsync_proto::ControlRequest::Status => {
                        serde_json::from_value(lock(&self.0).clone()).unwrap()
                    }
                    _ => {
                        clipsync_proto::ControlResponse::Module(clipsync_proto::ModuleReply::Error(
                            "sem estado de entrada neste ensaio".into(),
                        ))
                    }
                }
            })
        }
    }

    #[tokio::test]
    async fn a1_same_return_after_unobserved_recovery_notifies_again() {
        let dir = tempfile::Builder::new().prefix("a1-").tempdir().unwrap();
        let ui = crate::Ui::start(crate::UiOptions {
            profile: Some(dir.path().to_owned()),
            ..crate::UiOptions::default()
        })
        .await
        .unwrap();
        if let Some(poller) = lock(&ui.ctx.poller).take() {
            poller.abort();
        }
        let source = Arc::new(ReturnStatus(std::sync::Mutex::new(serde_json::json!({
            "Status": {
                "device_name": "ensaio", "id": PeerId([1; 32]),
                "listen": "127.0.0.1:0", "paused": false, "peers": [],
                "modules": [{
                    "kind": ModuleKind::Input, "enabled": true,
                    "adapter": null, "authorized_peers": [], "last_event": null,
                    "last_block": FAILED, "control_returned_id": 1
                }]
            }
        }))));
        let listener = clipsync_core::control::bind(&ui.ctx.profile.socket_path())
            .await
            .unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let handler = source.clone();
        let server = tokio::spawn(async move {
            listener
                .serve(handler, async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let previous = Snapshot::take(&ui.ctx).await;
        assert_eq!(previous.input_block.as_deref(), Some(FAILED));
        // O mesmo par recupera e falha de novo antes da próxima consulta.
        {
            let mut json = lock(&source.0);
            json["Status"]["modules"][0]["last_block"] = serde_json::Value::Null;
            json["Status"]["modules"][0]["last_block"] = FAILED.into();
            json["Status"]["modules"][0]["control_returned_id"] = 2.into();
        }
        let current = Snapshot::take(&ui.ctx).await;
        let notices = return_notices(&previous, &current, true);
        // Outro retorno recuperado antes de consultar não pode gerar aviso atrasado.
        {
            let mut json = lock(&source.0);
            json["Status"]["modules"][0]["control_returned_id"] = 3.into();
            json["Status"]["modules"][0]["last_block"] = serde_json::Value::Null;
        }
        let recovered = Snapshot::take(&ui.ctx).await;
        let _ = stop.send(());
        server.await.unwrap();
        ui.shutdown().await;
        assert_eq!(
            notices.len(),
            1,
            "nova devolução com frase igual precisa de novo aviso"
        );
        assert_eq!(notices[0].body, FAILED);
        assert!(return_notices(&current, &current, true).is_empty());
        assert!(return_notices(&previous, &current, false).is_empty());
        assert!(return_notices(&current, &recovered, true).is_empty());
    }

    #[test]
    fn a1_current_return_is_immediate_and_repeats_after_recovery() {
        let good = snapshot(vec![block(RETURNED, 1)]);
        for phrase in [RETURNED, FAILED] {
            let mut failed = snapshot(vec![]);
            failed.input_block = Some(phrase.into());
            let notices = return_notices(&good, &failed, true);
            assert_eq!(notices.len(), 1);
            assert_eq!(notices[0].body, phrase);
            assert!(return_notices(&failed, &failed, true).is_empty());
            assert!(return_notices(&failed, &good, true).is_empty());
            assert_eq!(return_notices(&good, &failed, true).len(), 1);
            assert!(return_notices(&good, &failed, false).is_empty());
        }
        let mut other = snapshot(vec![]);
        for text in ["texto livre", &format!("{RETURNED} extra")] {
            other.input_block = Some(text.into());
            assert!(return_notices(&good, &other, true).is_empty());
        }
    }
    #[test]
    fn a1_composed_block_notifies_only_the_literal_return_once() {
        let good = snapshot(vec![]);
        let mut failed = snapshot(vec![]);
        failed.input_block = Some(format!("{RETURNED} · outro motivo · teclado indisponível"));
        let notifications = return_notices(&good, &failed, true);
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].body, RETURNED);
        let mut changed = snapshot(vec![]);
        changed.input_block = Some(format!("{RETURNED} · outro motivo atualizado"));
        assert!(return_notices(&failed, &changed, true).is_empty());
        let mut notices = BlockNotifications::default();
        let record = block(failed.input_block.as_deref().unwrap(), 1);
        notices.observe(std::iter::once((&record, 1)), true);
        assert!(notices.flush(tokio::time::Instant::now(), true).is_none());
    }
    #[test]
    fn a1_return_notice_does_not_also_enter_generic_aggregation() {
        let mut notices = BlockNotifications::default();
        let record = block(RETURNED, 1);
        notices.observe(std::iter::once((&record, 1)), true);
        assert!(notices.flush(tokio::time::Instant::now(), true).is_none());
    }
    #[test]
    fn a1_missing_observation_does_not_replay_return() {
        let mut failed = snapshot(vec![]);
        failed.input_block = Some(RETURNED.into());
        failed.control_returned_id = 42;
        let mut missing = snapshot(vec![]);
        missing.live = false;
        missing.retain_unobserved(&failed);
        assert!(return_notices(&missing, &failed, true).is_empty());
    }
    #[test]
    fn unchanged_name_and_order_do_not_make_new_groups() {
        let was = snapshot(vec![block("A", 2), block("B", 3)]);
        let mut renamed = block("A", 2);
        renamed.peer_name = Some("renomeado".into());
        renamed.last_unix_ms = 99;
        let now = snapshot(vec![block("B", 3), renamed]);
        assert_eq!(was.new_blocks(&now).count(), 0);
    }
    #[test]
    fn growth_reports_only_changed_group() {
        let was = snapshot(vec![block("A", 2), block("B", 3)]);
        let now = snapshot(vec![block("B", 3), block("A", 5)]);
        assert_eq!(was.new_blocks(&now).count(), 1);
    }
    #[test]
    fn peer_blocked_does_not_make_its_own_notification() {
        let was = snapshot(vec![]);
        let now = snapshot(vec![block("A", 1), block("B", 1)]);
        let events = was.diff(&now, Notifications::default());
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, UiEvent::PeerBlocked(_)))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e,UiEvent::Notification(n) if n.kind==NotificationKind::Block))
                .count(),
            0
        );
    }
    #[test]
    fn monotonic_window_flushes_pending_on_empty_tick() {
        let start = tokio::time::Instant::now();
        let mut notices = BlockNotifications::default();
        let empty = snapshot(vec![]);
        let first = snapshot(vec![block("A", 1)]);
        notices.observe(empty.new_block_deltas(&first), true);
        assert!(notices.flush(start, true).is_some());
        let grown = snapshot(vec![block("A", 4)]);
        notices.observe(first.new_block_deltas(&grown), true);
        assert_eq!(notices.pending_occurrences, 3);
        assert!(
            notices
                .flush(start + std::time::Duration::from_millis(29_999), true)
                .is_none()
        );
        // Resync só pede leitura; não substitui o estado de avisos do poll.
        let _ = UiEvent::resync();
        notices.observe(grown.new_block_deltas(&grown), true);
        let summary = notices.flush(start + BLOCK_WINDOW, true).unwrap();
        assert!(summary.body.contains("3 ocorrências observadas"));
        assert!(notices.flush(start + BLOCK_WINDOW, true).is_none());
        assert!(notices.last_sent.is_some());
    }
    #[test]
    fn absent_agent_retains_baseline_without_replaying_groups() {
        let previous = snapshot(vec![block("A", 9)]);
        let mut absent = snapshot(vec![]);
        absent.live = false;
        absent.retain_unobserved(&previous);
        assert_eq!(previous.blocks.len(), 1);
        assert_eq!(previous.new_blocks(&absent).count(), 0);
        let returned = snapshot(vec![block("A", 9)]);
        assert_eq!(absent.new_blocks(&returned).count(), 0);
    }
    #[test]
    fn preference_discards_pending_without_reset_or_replay() {
        let start = tokio::time::Instant::now();
        let mut notices = BlockNotifications::default();
        let first = block("A", 1);
        notices.observe(std::iter::once((&first, 1)), true);
        assert!(notices.flush(start, true).is_some());
        notices.observe(std::iter::once((&first, 4)), true);
        notices.observe(std::iter::once((&first, 8)), false);
        assert_eq!(notices.pending_occurrences, 0);
        assert_eq!(notices.last_sent, Some(start));
        let baseline = snapshot(vec![block("A", 13)]);
        notices.observe(baseline.new_block_deltas(&baseline), true);
        assert!(notices.flush(start + BLOCK_WINDOW, true).is_none());
        notices.observe(std::iter::once((&first, 1)), true);
        assert!(
            notices
                .flush(start + std::time::Duration::from_secs(2), true)
                .is_none()
        );
        let summary = notices.flush(start + BLOCK_WINDOW, true).unwrap();
        assert!(summary.body.contains("1 ocorrências observadas"));
    }
    #[test]
    fn recreation_and_saturation_never_underflow_or_invent_delta() {
        let old = vec![block("A", 9)];
        assert_eq!(block_delta(&old, &block("A", 2)), 2);
        let mut recreated = block("A", 10);
        recreated.at_unix_ms = 3;
        assert_eq!(block_delta(&old, &recreated), 10);
        assert_eq!(
            block_delta(&[block("A", u64::MAX - 1)], &block("A", u64::MAX)),
            1
        );
        assert_eq!(
            block_delta(&[block("A", u64::MAX)], &block("A", u64::MAX)),
            0
        );
    }
    #[test]
    fn diversity_and_occurrences_stay_bounded_and_honest() {
        let mut notices = BlockNotifications::default();
        for n in 0..21u8 {
            let mut entry = block(&format!("causa{n}"), 1);
            entry.peer = Some(PeerId([n; 32]));
            notices.observe(std::iter::once((&entry, u64::MAX)), true);
        }
        assert_eq!(notices.pending_occurrences, u64::MAX);
        assert_eq!(notices.pending_keys.len(), 20);
        assert_eq!(notices.pending_peers.len(), 20);
        assert!(notices.keys_overflow && notices.peers_overflow);
        let summary = notices.flush(tokio::time::Instant::now(), true).unwrap();
        assert!(summary.body.contains("pelo menos 21 causas"));
        assert!(summary.body.contains("pelo menos 21 dispositivos"));
        assert!(summary.body.contains("ocorrências observadas"));
        let mut local = block("local", 1);
        local.peer = None;
        notices.observe(std::iter::once((&local, 1)), true);
        assert!(notices.pending_peers.is_empty());
    }
    #[test]
    fn removal_stays_independent_of_block_aggregation() {
        let old = snapshot(vec![]);
        let mut now = snapshot(vec![block("A", 1)]);
        now.removals.push(RemovalRecord {
            at_unix_ms: 1,
            peer: PeerId([2; 32]),
            peer_name: "removido".into(),
        });
        let events = old.diff(&now, Notifications::default());
        assert!(events.iter().any(
            |e| matches!(e,UiEvent::Notification(n) if n.kind==NotificationKind::DeviceRemoved)
        ));
        assert!(
            !events
                .iter()
                .any(|e| matches!(e,UiEvent::Notification(n) if n.kind==NotificationKind::Block))
        );
        let UiEvent::PeerBlocked(group) = events
            .iter()
            .find(|e| matches!(e, UiEvent::PeerBlocked(_)))
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(group.occurrences, 1);
        assert_eq!(group.last_unix_ms, 20);
    }
}
