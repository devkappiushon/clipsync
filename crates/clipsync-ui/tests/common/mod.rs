//! Helpers shared by the clipsync-ui tests (Rivet): temp profiles on 127.0.0.1 with fake
//! adapters, a `Ui` with a fast poll, and waits on events. Vector's and Flux's tests may use
//! them with `mod common;`.

// Helpers outside #[test] fns are test code too; clippy.toml's allow-*-in-tests does not reach them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
use std::time::Duration;

use clipsync_core::{InitOptions, PeerEntry, Profile};
use clipsync_proto::{ModuleSet, PeerId, PeerPerms};
use clipsync_ui::{Ui, UiError, UiEvent, UiOptions};
use serde_json::{Value, json};
use tokio::sync::broadcast;

pub const WAIT: Duration = Duration::from_secs(20);
pub const POLL: Duration = Duration::from_millis(100);

pub fn free_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub fn local_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], free_port()))
}

pub const CLIPBOARD_ONLY: ModuleSet = ModuleSet {
    clipboard: true,
    input: false,
};

/// A profile made like `clipsync init --fake-adapters`, listening on 127.0.0.1.
pub fn init_profile(dir: &Path, name: &str, modules: ModuleSet) -> (Profile, PeerId) {
    let profile = Profile {
        dir: dir.join(name),
    };
    let id = profile
        .init(InitOptions {
            device_name: name.into(),
            modules,
            listen: local_addr(),
            fake_adapters: true,
        })
        .unwrap()
        .id();
    (profile, id)
}

pub const INPUT_ONLY: ModuleSet = ModuleSet {
    clipboard: false,
    input: true,
};

pub const BOTH: ModuleSet = ModuleSet {
    clipboard: true,
    input: true,
};

/// Pairs two profiles by writing each side's `peers.toml`, like a finished pairing would.
/// `a_grants` is what `a` grants `b`, and the other way round.
pub fn pair_files(a: &Profile, b: &Profile, a_grants: PeerPerms, b_grants: PeerPerms) {
    let entry = |of: &Profile, grants: PeerPerms| {
        let cfg = of.load_config().unwrap();
        PeerEntry {
            id: of.load_identity().unwrap().id(),
            name: cfg.device_name,
            addr: Some(cfg.listen),
            paired_at: "2026-10-28T12:00:00Z".into(),
            perms: grants,
        }
    };
    for (me, other, grants) in [(a, b, a_grants), (b, a, b_grants)] {
        let mut peers = me.load_peers().unwrap();
        peers.upsert(entry(other, grants));
        me.save_peers(&peers).unwrap();
    }
}

pub const NONE: PeerPerms = PeerPerms {
    clipboard_in: false,
    clipboard_out: false,
    input_in: false,
};

/// Options for the profile `base/name`; autostart and downloads also live under `base`.
pub fn opts(base: &Path, name: &str) -> UiOptions {
    UiOptions {
        profile: Some(base.join(name)),
        poll: POLL,
        listen: Some(local_addr()),
        fake_adapters: true,
        autostart_dir: Some(base.join(format!("{name}-autostart"))),
        downloads_dir: Some(base.join(format!("{name}-downloads"))),
    }
}

pub async fn start(base: &Path, name: &str) -> Ui {
    Ui::start(opts(base, name)).await.unwrap()
}

pub async fn call(ui: &Ui, req: Value) -> Result<Value, UiError> {
    ui.call(req).await
}

pub async fn ok(ui: &Ui, req: Value) -> Value {
    match ui.call(req.clone()).await {
        Ok(v) => v,
        Err(e) => panic!("{req} failed: {e:?}"),
    }
}

pub async fn err(ui: &Ui, req: Value) -> UiError {
    match ui.call(req.clone()).await {
        Ok(v) => panic!("{req} should fail, got {v}"),
        Err(e) => e,
    }
}

pub async fn app_state(ui: &Ui) -> Value {
    ok(ui, json!({ "cmd": "app_state" })).await
}

/// The next event that `want` accepts; others are skipped. Lag is not an error here.
pub async fn next_event(
    rx: &mut broadcast::Receiver<UiEvent>,
    what: &str,
    mut want: impl FnMut(&UiEvent) -> bool,
) -> UiEvent {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Ok(ev)) if want(&ev) => return ev,
            Ok(Ok(_)) | Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(e)) => panic!("events closed while waiting for {what}: {e}"),
            Err(_) => panic!("timed out waiting for {what}"),
        }
    }
}

pub async fn wait_until<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = tokio::time::Instant::now();
    while start.elapsed() < WAIT {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for: {what}");
}
