//! 3 · Pareamento (§12.5): `pair_host_start`, `pair_join_start`, `pair_code_matches`,
//! `pair_finish`, `pair_cancel` and `pair_state`. Owned by Rivet.
//!
//! One attempt at a time. The core's `pairing::host` or `join` runs in a task; its `confirm`
//! callback (a blocking thread) publishes the code with `pair_code_ready` and waits for the
//! decision that `pair_code_matches` + `pair_finish` hand over. No decision in
//! `DECISION_TIMEOUT` = decline. `pair_cancel` ends the attempt at any step. The end always comes
//! as the event `pair_result`; a pairing that worked has already saved `peers.toml` and sent
//! `ReloadPeers` (core).

use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc;
use std::sync::{Arc, Weak};
use std::time::Duration;

use clipsync_core::CoreError;
use clipsync_core::pairing::{self, PairDecision, PairOutcome, PairPrompt};
use clipsync_proto::{ModuleSet, PAIR_WINDOW_SECS, PeerPerms};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::task::{AbortHandle, JoinHandle};

use crate::api::{AttemptArgs, PairFinishArgs, PairHostStartArgs, PairJoinStartArgs, code};
use crate::devices::{self, Device, Perms};
use crate::events::{PairCodeReady, PairOutcomeName, PairPeer, PairResult, UiEvent};
use crate::status::{self, Modules};
use crate::{Ctx, UiError, clean, lock, msg, unix_ms};

/// The user's side of "Sem decisão em 120 s = recusa".
const DECISION_TIMEOUT: Duration = Duration::from_secs(PAIR_WINDOW_SECS);
/// What `pairing::host` reports when nobody joined in time.
const WINDOW_EXPIRED: &str = "window expired";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Idle,
    Listening,
    Connecting,
    Code,
    Permissions,
    WaitingPeer,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostStarted {
    pub attempt_id: u64,
    pub port: u16,
    /// "IP:PORT" this machine can be reached at.
    pub addresses: Vec<String>,
    pub expires_unix_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JoinStarted {
    pub attempt_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodeMatched {
    pub step: Step,
    /// What can be granted: the modules that are on here.
    pub available: Modules,
    /// Everything off (prompt-mestre §2.1): the user turns on what they want.
    pub defaults: Perms,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Finished {
    pub step: Step,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cancelled {
    pub cancelled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PairState {
    pub step: Step,
    pub attempt_id: Option<u64>,
    pub port: Option<u16>,
    pub addresses: Vec<String>,
    pub expires_unix_ms: Option<u64>,
    pub code: Option<String>,
    pub peer: Option<PairPeer>,
}

pub(crate) struct Attempt {
    id: u64,
    step: Step,
    port: Option<u16>,
    addresses: Vec<String>,
    expires_unix_ms: Option<u64>,
    code: Option<String>,
    peer: Option<PairPeer>,
    peer_modules: ModuleSet,
    /// From `pair_code_ready` until `pair_finish` hands the decision over.
    decide: Option<mpsc::SyncSender<PairDecision>>,
    cancelled: bool,
    task: AbortHandle,
}

/// The slot in `Ctx`.
#[derive(Default)]
pub(crate) struct Pairing {
    last_id: u64,
    current: Option<Attempt>,
}

fn decline() -> PairDecision {
    PairDecision {
        accept: false,
        grants: PeerPerms::default(),
    }
}

/// "IP:PORT" for the screen: the bound address itself, or every interface address (no loopback,
/// no link-local) when the agent listens on all of them.
pub(crate) fn addresses(bound: SocketAddr) -> Vec<String> {
    if !bound.ip().is_unspecified() {
        return vec![bound.to_string()];
    }
    let mut ips: Vec<IpAddr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback() && !i.is_link_local() && i.ip().is_ipv4() == bound.is_ipv4())
        .map(|i| i.ip())
        .collect();
    ips.sort();
    ips.dedup();
    ips.into_iter()
        .map(|ip| SocketAddr::new(ip, bound.port()).to_string())
        .collect()
}

fn no_attempt() -> UiError {
    UiError::new(code::NO_ATTEMPT, msg::NO_ATTEMPT).with_field("attempt_id")
}

fn wrong_step() -> UiError {
    UiError::new(code::WRONG_STEP, msg::WRONG_STEP)
}

fn current(p: &mut Pairing, id: u64) -> Result<&mut Attempt, UiError> {
    p.current
        .as_mut()
        .filter(|a| a.id == id)
        .ok_or_else(no_attempt)
}

/// The `confirm` of the core: runs on a blocking thread, so it may wait for the user.
fn confirm_fn(
    ctx: Weak<Ctx>,
    id: u64,
) -> impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static {
    move |prompt: &PairPrompt| {
        let (tx, rx) = mpsc::sync_channel(1);
        let Some(c) = ctx.upgrade() else {
            return decline();
        };
        let published = code_ready(&c, id, prompt, tx);
        drop(c);
        if !published {
            return decline();
        }
        // Disconnected (the attempt is gone) or timed out: decline.
        rx.recv_timeout(DECISION_TIMEOUT)
            .unwrap_or_else(|_| decline())
    }
}

fn code_ready(ctx: &Ctx, id: u64, prompt: &PairPrompt, tx: mpsc::SyncSender<PairDecision>) -> bool {
    let peer = PairPeer {
        name: clean(&prompt.peer_name),
        short: prompt.peer_id.short(),
        modules: prompt.peer_modules.into(),
    };
    {
        let mut p = lock(&ctx.pairing);
        let Some(a) = p.current.as_mut().filter(|a| a.id == id && !a.cancelled) else {
            return false;
        };
        a.step = Step::Code;
        a.code = Some(prompt.code.clone());
        a.peer = Some(peer.clone());
        a.peer_modules = prompt.peer_modules;
        a.decide = Some(tx);
    }
    ctx.emit(UiEvent::PairCodeReady(PairCodeReady {
        attempt_id: id,
        code: prompt.code.clone(),
        peer,
    }));
    true
}

type PairTask = JoinHandle<Result<PairOutcome, CoreError>>;

/// Takes the slot (`pair_busy` when taken) and spawns the attempt made by `run(id)`.
fn begin<F>(
    ctx: &Arc<Ctx>,
    step: Step,
    run: impl FnOnce(u64) -> F,
) -> Result<(u64, PairTask), UiError>
where
    F: Future<Output = Result<PairOutcome, CoreError>> + Send + 'static,
{
    let mut p = lock(&ctx.pairing);
    if p.current.is_some() {
        return Err(UiError::new(code::PAIR_BUSY, msg::PAIR_BUSY));
    }
    p.last_id += 1;
    let id = p.last_id;
    let task = tokio::spawn(run(id));
    p.current = Some(Attempt {
        id,
        step,
        port: None,
        addresses: Vec::new(),
        expires_unix_ms: None,
        code: None,
        peer: None,
        peer_modules: ModuleSet::default(),
        decide: None,
        cancelled: false,
        task: task.abort_handle(),
    });
    Ok((id, task))
}

pub(crate) async fn host_start(
    ctx: &Arc<Ctx>,
    args: PairHostStartArgs,
) -> Result<HostStarted, UiError> {
    // §12.5 (v1.17, S6-11): the same 2 minutes the user has to decide.
    if !(1..=PAIR_WINDOW_SECS).contains(&args.window_secs) {
        return Err(UiError::bad_request("window_secs: 1..120").with_field("window_secs"));
    }
    ctx.require_profile()?;
    let window = Duration::from_secs(args.window_secs);
    let (listen_tx, listen_rx) = oneshot::channel::<SocketAddr>();
    let (id, task) = begin(ctx, Step::Listening, |id| {
        let profile = ctx.profile.clone();
        let confirm = confirm_fn(Arc::downgrade(ctx), id);
        async move {
            let on_listen = move |addr| {
                let _ = listen_tx.send(addr);
            };
            pairing::host(&profile, args.port, window, on_listen, confirm).await
        }
    })?;
    let Ok(bound) = listen_rx.await else {
        // `host` returned before listening: the bind (or the profile) failed.
        let res = task.await;
        let mut p = lock(&ctx.pairing);
        if p.current.as_ref().is_some_and(|a| a.id == id) {
            p.current = None;
        }
        return Err(match res {
            Ok(Err(CoreError::Transport(d))) => {
                UiError::new(code::BIND, msg::bind(&clean(&d))).with_field("port")
            }
            Ok(Err(e)) => e.into(),
            Ok(Ok(_)) => UiError::internal(msg::UNEXPECTED_REPLY),
            Err(_) => no_attempt(),
        });
    };
    let started = HostStarted {
        attempt_id: id,
        port: bound.port(),
        addresses: addresses(bound),
        expires_unix_ms: unix_ms().saturating_add(args.window_secs.saturating_mul(1000)),
    };
    if let Ok(a) = current(&mut lock(&ctx.pairing), id) {
        a.port = Some(started.port);
        a.addresses = started.addresses.clone();
        a.expires_unix_ms = Some(started.expires_unix_ms);
    }
    tokio::spawn(watch(Arc::downgrade(ctx), id, task));
    Ok(started)
}

pub(crate) async fn join_start(
    ctx: &Arc<Ctx>,
    args: PairJoinStartArgs,
) -> Result<JoinStarted, UiError> {
    let addr = args
        .addr
        .trim()
        .parse::<SocketAddr>()
        .ok()
        .filter(|a| a.port() != 0 && !a.ip().is_unspecified())
        .ok_or_else(|| UiError::new(code::BAD_ADDR, msg::BAD_ADDR).with_field("addr"))?;
    ctx.require_profile()?;
    let (id, task) = begin(ctx, Step::Connecting, |id| {
        let profile = ctx.profile.clone();
        let confirm = confirm_fn(Arc::downgrade(ctx), id);
        async move { pairing::join(&profile, addr, confirm).await }
    })?;
    tokio::spawn(watch(Arc::downgrade(ctx), id, task));
    Ok(JoinStarted { attempt_id: id })
}

pub(crate) async fn code_matches(ctx: &Ctx, args: AttemptArgs) -> Result<CodeMatched, UiError> {
    {
        let mut p = lock(&ctx.pairing);
        let a = current(&mut p, args.attempt_id)?;
        if a.step != Step::Code {
            return Err(wrong_step());
        }
        a.step = Step::Permissions;
    }
    let local = ctx.profile.load_config()?.modules;
    Ok(CodeMatched {
        step: Step::Permissions,
        available: local.into(),
        defaults: Perms::default(),
    })
}

/// Grants for a module that is off here are dropped by the core before saving.
pub(crate) fn finish(ctx: &Ctx, args: PairFinishArgs) -> Result<Finished, UiError> {
    let mut p = lock(&ctx.pairing);
    let a = current(&mut p, args.attempt_id)?;
    if a.step != Step::Permissions {
        return Err(wrong_step());
    }
    devices::check_full_trust(false, args.grants.input_in, args.acknowledge_full_trust)?;
    let tx = a.decide.take().ok_or_else(wrong_step)?;
    a.step = Step::WaitingPeer;
    // A thread that already gave up (timeout) declined: `pair_result` tells.
    let _ = tx.try_send(PairDecision {
        accept: true,
        grants: args.grants.into(),
    });
    Ok(Finished {
        step: Step::WaitingPeer,
    })
}

/// Ends the attempt before the local decision is sent: the connection drops, nothing is saved,
/// and `pair_result` says `cancelled`. In `waiting_peer` the decision already left and the core
/// may be saving `peers.toml` right now, so the end belongs to the other machine (§12.5 v1.17,
/// S6-10): `wrong_step`.
pub(crate) fn cancel(ctx: &Ctx, args: AttemptArgs) -> Result<Cancelled, UiError> {
    let mut p = lock(&ctx.pairing);
    let a = current(&mut p, args.attempt_id)?;
    if a.step == Step::WaitingPeer {
        return Err(UiError::new(code::WRONG_STEP, msg::PAIR_ALREADY_DECIDED));
    }
    a.cancelled = true;
    a.decide = None;
    a.task.abort();
    Ok(Cancelled { cancelled: true })
}

pub(crate) fn state(ctx: &Ctx) -> PairState {
    let p = lock(&ctx.pairing);
    match &p.current {
        Some(a) => PairState {
            step: a.step,
            attempt_id: Some(a.id),
            port: a.port,
            addresses: a.addresses.clone(),
            expires_unix_ms: a.expires_unix_ms,
            code: a.code.clone(),
            peer: a.peer.clone(),
        },
        None => PairState {
            step: Step::Idle,
            attempt_id: None,
            port: None,
            addresses: Vec::new(),
            expires_unix_ms: None,
            code: None,
            peer: None,
        },
    }
}

/// The reason of a `failed` result, in Portuguese; the core's own words only as a detail.
fn failure(e: CoreError) -> String {
    match e {
        CoreError::Transport(_) => msg::PAIR_NO_ANSWER.into(),
        CoreError::Pairing(d) if d.starts_with("closed by peer") => msg::PAIR_PEER_CLOSED.into(),
        CoreError::Pairing(d) if d.contains("timeout") => msg::PAIR_TIMEOUT.into(),
        CoreError::Pairing(d) => msg::pair_failed(&clean(&d)),
        other => UiError::from(other).message,
    }
}

/// Waits for the attempt, frees the slot and emits `pair_result`.
async fn watch(ctx: Weak<Ctx>, id: u64, task: PairTask) {
    let res = task.await;
    let Some(ctx) = ctx.upgrade() else { return };
    let attempt = {
        let mut p = lock(&ctx.pairing);
        if p.current.as_ref().is_some_and(|a| a.id == id) {
            p.current.take()
        } else {
            None
        }
    };
    let cancelled = attempt.as_ref().is_some_and(|a| a.cancelled);
    let peer_modules = attempt.map_or(ModuleSet::default(), |a| a.peer_modules);
    let (outcome, device, reason) = match res {
        Ok(Ok(PairOutcome::Paired(entry))) => {
            let live = status::agent_status(&ctx).await;
            let l = live
                .as_ref()
                .and_then(|s| s.peers.iter().find(|p| p.id == entry.id));
            let mut device = Device::new(&entry, l);
            if l.is_none_or(|l| l.remote_version == 0) {
                device.remote_modules = peer_modules.into();
            }
            (PairOutcomeName::Paired, Some(device), None)
        }
        Ok(Ok(PairOutcome::Declined { by_peer: true })) => (
            PairOutcomeName::DeclinedPeer,
            None,
            Some(msg::PAIR_DECLINED_PEER.to_string()),
        ),
        _ if cancelled => (PairOutcomeName::Cancelled, None, None),
        Ok(Ok(PairOutcome::Declined { by_peer: false })) => (
            PairOutcomeName::DeclinedLocal,
            None,
            Some(msg::PAIR_NO_DECISION.to_string()),
        ),
        Ok(Err(CoreError::Pairing(d))) if d == WINDOW_EXPIRED => (
            PairOutcomeName::Expired,
            None,
            Some(msg::PAIR_EXPIRED.to_string()),
        ),
        Ok(Err(e)) => (PairOutcomeName::Failed, None, Some(failure(e))),
        Err(e) if e.is_cancelled() => (PairOutcomeName::Cancelled, None, None),
        Err(e) => (
            PairOutcomeName::Failed,
            None,
            Some(msg::pair_failed(&clean(&e.to_string()))),
        ),
    };
    ctx.emit(UiEvent::PairResult(PairResult {
        attempt_id: id,
        outcome,
        device,
        reason,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_address_is_shown_as_is_and_any_lists_no_loopback() {
        let one: SocketAddr = "127.0.0.1:53122".parse().unwrap();
        assert_eq!(addresses(one), vec!["127.0.0.1:53122".to_string()]);
        let any: SocketAddr = "0.0.0.0:53122".parse().unwrap();
        for a in addresses(any) {
            let a: SocketAddr = a.parse().unwrap();
            assert!(a.is_ipv4() && !a.ip().is_loopback(), "{a}");
            assert_eq!(a.port(), 53122);
        }
    }
}
