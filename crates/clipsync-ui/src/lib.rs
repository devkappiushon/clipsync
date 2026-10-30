//! Logic of the ClipSync screens (contract §12), without Tauri: `cargo test` runs it on Linux
//! without a graphical session. One entry, `Ui::call`, takes the JSON of a `UiRequest` and
//! answers JSON or a `UiError`; `Ui::subscribe` hands out the `UiEvent`s.
//!
//! Ownership (§12.1): Rivet owns lib, api, agent, status, onboarding, devices, pairing,
//! clipboard, diag, prefs, events and msg; Vector owns layout and input; Flux owns autostart.
//! The UI talks to the agent ONLY through `control::request`, even when the agent runs in this
//! process (§12.2 rule 1).

pub mod agent;
pub mod api;
pub mod autostart;
pub mod clipboard;
pub mod devices;
pub mod diag;
pub mod events;
pub mod input;
pub mod layout;
pub mod modules;
pub mod msg;
pub mod onboarding;
pub mod pairing;
pub mod prefs;
pub mod status;
pub mod traversal;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use clipsync_core::{Identity, Profile, control};
use clipsync_proto::{ControlRequest, ControlResponse, ModuleCommand, ModuleKind, ModuleReply};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub use api::{UiError, UiRequest};
pub use events::UiEvent;

/// Events waiting for a slow subscriber; past this it gets `Lagged` and the casca sends
/// `UiEvent::Resync` (§12.5 event 11).
const EVENT_QUEUE: usize = 256;

#[derive(Debug, Clone)]
pub struct UiOptions {
    /// None = `Profile::resolve(None)`, like the CLI.
    pub profile: Option<PathBuf>,
    /// Status poll that feeds events 1 to 6; 1 s, tests use less.
    pub poll: Duration,
    /// Tests: own port in `onboarding_create`.
    pub listen: Option<SocketAddr>,
    /// Tests: `onboarding_create` writes fake adapters.
    pub fake_adapters: bool,
    /// Tests; None = the platform default.
    pub autostart_dir: Option<PathBuf>,
    /// Tests; None = XDG_DOWNLOAD_DIR, ~/Downloads or %USERPROFILE%\Downloads.
    pub downloads_dir: Option<PathBuf>,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            profile: None,
            poll: Duration::from_secs(1),
            listen: None,
            fake_adapters: false,
            autostart_dir: None,
            downloads_dir: None,
        }
    }
}

/// Cheap to clone: everything lives behind one `Arc`.
#[derive(Clone)]
pub struct Ui {
    ctx: Arc<Ctx>,
}

/// What every screen module gets: the profile, the options, the agent slot and the event
/// sender. Screen modules reach the agent only through `request` and `ask_module`.
pub(crate) struct Ctx {
    pub(crate) profile: Profile,
    pub(crate) opts: UiOptions,
    events: broadcast::Sender<UiEvent>,
    pub(crate) agent: tokio::sync::Mutex<agent::Slot>,
    pub(crate) mutation: tokio::sync::Mutex<()>,
    pub(crate) jobs: Mutex<Vec<JoinHandle<()>>>,
    pub(crate) pause_warning: Mutex<status::PauseWarning>,
    /// Made by `onboarding_start`, written by `onboarding_create`.
    pub(crate) pending_identity: Mutex<Option<Identity>>,
    /// `ui.toml`, read once at start; `prefs_set` keeps it in step with the file.
    pub(crate) prefs: Mutex<prefs::Stored>,
    /// S6-01: one writer of peers.toml at a time in this process (`Profile::update_peers` locks
    /// the file against the agent and other processes).
    pub(crate) peers_lock: tokio::sync::Mutex<()>,
    /// The one pairing attempt, if any.
    pub(crate) pairing: Mutex<pairing::Pairing>,
    /// The fake clipboard of the in-app agent, when the profile asks for one (tests).
    pub(crate) fake_clipboard: Mutex<Option<clipsync_clipboard::FakeClipboardHandle>>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

impl Ctx {
    pub(crate) fn emit(&self, ev: UiEvent) {
        // No subscriber is fine: the front fetches everything when it starts.
        let _ = self.events.send(ev);
    }

    /// One request to the agent of this profile, in_app or external.
    pub(crate) async fn request(&self, req: ControlRequest) -> Result<ControlResponse, UiError> {
        control::request(&self.profile, req)
            .await
            .map_err(UiError::from)
    }

    /// A module command. `ModuleReply::Error` comes back as `Ok`: each screen maps the module's
    /// own messages to its codes (`api::module_error` is the default).
    pub(crate) async fn ask_module(
        &self,
        kind: ModuleKind,
        cmd: ModuleCommand,
    ) -> Result<ModuleReply, UiError> {
        match self.request(ControlRequest::Module { kind, cmd }).await? {
            ControlResponse::Module(r) => Ok(r),
            ControlResponse::Error { message } => Err(UiError::internal(&clean(&message))),
            _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
        }
    }

    pub(crate) fn require_profile(&self) -> Result<(), UiError> {
        if self.profile.is_initialized() {
            Ok(())
        } else {
            Err(UiError::not_initialized())
        }
    }
}

impl Ui {
    /// Needs a multi-thread tokio runtime in context (the casca uses Tauri's). Starts the agent
    /// when the profile exists and turns the status poll on.
    pub async fn start(opts: UiOptions) -> Result<Ui, UiError> {
        let profile = Profile::resolve(opts.profile.clone())?;
        let stored_prefs = prefs::load(&profile.dir);
        let (events, _) = broadcast::channel(EVENT_QUEUE);
        let ctx = Arc::new(Ctx {
            profile,
            opts,
            events,
            agent: tokio::sync::Mutex::new(agent::Slot::Stopped),
            mutation: tokio::sync::Mutex::new(()),
            jobs: Mutex::new(Vec::new()),
            pause_warning: Mutex::new(status::PauseWarning::default()),
            pending_identity: Mutex::new(None),
            prefs: Mutex::new(stored_prefs),
            peers_lock: tokio::sync::Mutex::new(()),
            pairing: Mutex::new(pairing::Pairing::default()),
            fake_clipboard: Mutex::new(None),
            poller: Mutex::new(None),
        });
        if ctx.profile.is_initialized()
            && let Err(e) = agent::start(&ctx).await
        {
            // Recorded in the slot: app_state shows agent "failed" with this message.
            tracing::warn!(code = %e.code, "{}", e.message);
        }
        let first = events::Snapshot::take(&ctx).await;
        let poller = tokio::spawn(events::poll(Arc::downgrade(&ctx), first));
        *lock(&ctx.poller) = Some(poller);
        Ok(Ui { ctx })
    }

    /// The single entry: the JSON of a `UiRequest`. Wrong shape = `bad_request`; unknown `cmd`
    /// = `unknown_command`.
    pub async fn call(&self, req: serde_json::Value) -> Result<serde_json::Value, UiError> {
        let is_modules = matches!(
            req.get("cmd").and_then(serde_json::Value::as_str),
            Some("modules_set" | "clipboard_set_mode")
        );
        let req = api::parse(req).map_err(|error| {
            if is_modules {
                error.with_details(serde_json::json!({"effects": modules::Effects::default(), "observed": modules::Observed::default()}))
            } else { error }
        })?;
        api::route(&self.ctx, req).await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<UiEvent> {
        self.ctx.events.subscribe()
    }

    /// Stops the poll and the in_app agent (peers get Bye). An external agent is left alone.
    pub async fn shutdown(&self) {
        if let Some(p) = lock(&self.ctx.poller).take() {
            p.abort();
        }
        let jobs = std::mem::take(&mut *lock(&self.ctx.jobs));
        for job in jobs {
            let _ = job.await;
        }
        let _mutation = self.ctx.mutation.lock().await;
        agent::shutdown(&self.ctx).await;
    }

    /// Tests: the fake clipboard of the in-app agent, when its profile uses `adapter = "fake"`.
    #[doc(hidden)]
    pub fn fake_clipboard(&self) -> Option<clipsync_clipboard::FakeClipboardHandle> {
        lock(&self.ctx.fake_clipboard).clone()
    }
}

/// Wall clock in ms since the Unix epoch (§12.2 rule 4: times are `*_unix_ms`).
pub(crate) fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The CLI's `clean` (§12.2 rule 3): text that came from a peer is data. Controls (C0, DEL, C1)
/// and Unicode formatting characters become `?`; line breaks and tabs become a space.
pub fn clean(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\n' | '\r' | '\t' => ' ',
            c if c.is_control()
                || matches!(c, '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}') => '?',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_replaces_controls_c1_and_bidi_and_keeps_text() {
        assert_eq!(clean("ev\u{1b}[2Jil"), "ev?[2Jil");
        assert_eq!(clean("a\u{9b}b"), "a?b");
        assert_eq!(clean("a\u{202e}b\u{200b}c\u{feff}"), "a?b?c?");
        assert_eq!(clean("linha1\nlinha2\tx"), "linha1 linha2 x");
        assert_eq!(clean("olá ção ✓"), "olá ção ✓");
    }
}
