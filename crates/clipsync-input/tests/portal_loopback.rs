//! §9 S3 item 6: P4c without a human, on a graphical session.
//! `cargo test -p clipsync-input --test portal_loopback -- --ignored --nocapture`
//! runs both tests below; `portal_loopback_from_layout` is the one of §12.9 item 3 (M).
//!
//! `portal_loopback` uses the default barrier (upper half of the right edge of zone 0).
//! `portal_loopback_from_layout` writes a temporary `layout.toml` with one link and builds the
//! source module with `build_with_layout`, so the barrier is the one derived from that link, as
//! the agent does.
//!
//! The RemoteDesktop emulator pushes the pointer into the InputCapture barrier; the captured
//! events travel over MemHub to a fake destination. Up to 3 attempts (the spike saw 1 in 3 fail
//! to activate); the test fails only if all 3 do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]
use clipsync_input::{
    EmulateAdapter, FakeCapture, FakeEmulate, InputConfig, Layout, LayoutDevice, LayoutOptions,
    Link, PortalCapture, PortalEmulate, TokenStore, build_with_adapters, build_with_layout,
    fake_zone,
};
use clipsync_proto::*;
use std::time::Duration;
use tokio::sync::oneshot;

const INPUT: ModuleSet = ModuleSet {
    clipboard: false,
    input: true,
};

fn token_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .expect("HOME");
    let dir = base.join("clipsync-input-test");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A `layout.toml` with one link: the upper half of the right edge of the first monitor of the
/// source (`#0`, whatever the portal calls it) into the left edge of the destination's monitor.
/// Dead corners are off, because the test pushes the pointer into the top-right corner.
fn link_layout(src: PeerId, dst: PeerId, dst_monitor: &str) -> Layout {
    let device = |peer, name: &str, x| LayoutDevice {
        peer,
        name: name.into(),
        x,
        y: 0,
        hotkey: None,
    };
    let end = |peer, monitor: &str, edge| clipsync_input::Endpoint {
        peer,
        monitor: monitor.into(),
        edge,
        from: 0.0,
        to: 0.5,
    };
    Layout {
        rev: 1,
        options: LayoutOptions { dead_corner_px: 0 },
        devices: vec![device(src, "portal-src", 0), device(dst, "fake-dst", 2048)],
        links: vec![Link {
            id: "l1".into(),
            a: end(src, "#0", Edge::Right),
            b: end(dst, dst_monitor, Edge::Left),
        }],
    }
}

/// Automatic: the temporary layout is valid, survives the write, and derives the barrier the
/// manual test relies on.
#[test]
fn the_layout_of_the_manual_test_derives_its_barrier() {
    let (src, dst) = (PeerId([1; 32]), PeerId([2; 32]));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(clipsync_input::LAYOUT_FILE);
    link_layout(src, dst, "fake-0").write(&path).unwrap();
    let read = Layout::read(&path).unwrap().unwrap();
    let b = &read.barriers_for(src)[0];
    assert_eq!(
        (b.monitor.as_str(), b.edge, b.from, b.to),
        ("#0", Edge::Right, 0.0, 0.5)
    );
    assert_eq!(
        (b.peer, b.peer_monitor.as_str(), b.peer_edge),
        (dst, "fake-0", Edge::Left)
    );
    assert_eq!(read.barriers_for(dst)[0].peer, src);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requer sessão gráfica"]
async fn portal_loopback() {
    run_loopback("default", |_, _| {
        build_with_adapters(
            InputConfig::default(),
            Box::new(PortalCapture::new()),
            Box::new(FakeEmulate::new().0),
        )
    })
    .await;
}

/// §12.9 item 3 (M): the same loopback, with the barrier derived from a `layout.toml` link.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requer sessão gráfica"]
async fn portal_loopback_from_layout() {
    let dir = tempfile::tempdir().unwrap();
    run_loopback("layout.toml", |src, dst| {
        link_layout(src, dst, &fake_zone().name.unwrap())
            .write(&dir.path().join(clipsync_input::LAYOUT_FILE))
            .unwrap();
        build_with_layout(
            InputConfig::default(),
            Box::new(PortalCapture::new()),
            Box::new(FakeEmulate::new().0),
            dir.path(),
        )
        .expect("layout.toml accepted by the module")
    })
    .await;
}

/// `src` builds the source module from the ids of both nodes. The destination has one monitor
/// (`fake_zone`), so the link can name it and the source sends `EnterAt`.
async fn run_loopback(label: &str, src: impl FnOnce(PeerId, PeerId) -> Box<dyn Module>) {
    let hub = MemHub::new();
    let na = hub.add_node("portal-src", INPUT);
    let nb = hub.add_node("fake-dst", INPUT);
    let (b_id, a_id) = (nb.id, na.id);
    hub.link(
        a_id,
        b_id,
        PeerPerms::default(),
        PeerPerms {
            clipboard_in: false,
            clipboard_out: false,
            input_in: true,
        },
    );
    let (a_tx, a_rx, a_host) = (na.tx.clone(), na.rx, na.host.clone());
    tokio::spawn(src(a_id, b_id).run(a_host, a_rx));
    let (fake_emu, emu_handle) = FakeEmulate::new();
    let (fake_cap, _cap_handle) = FakeCapture::new(vec![fake_zone()]);
    tokio::spawn(
        build_with_adapters(
            InputConfig::default(),
            Box::new(fake_cap),
            Box::new(fake_emu),
        )
        .run(nb.host.clone(), nb.rx),
    );

    // The driver: the real RemoteDesktop portal (no dialog once the token is stored).
    let mut driver = PortalEmulate::new(TokenStore::new(&token_dir()));
    driver.start().await.expect("RemoteDesktop start");

    let mut attempts = 0u32;
    let mut result = None;
    while attempts < 3 && result.is_none() {
        attempts += 1;
        let (reply, rx) = oneshot::channel();
        a_tx.send(ModuleInput::Command {
            cmd: ModuleCommand::InputLoopback {
                to: b_id,
                seconds: 14,
            },
            reply,
        })
        .await
        .unwrap();
        // wait for the capture to be armed, then push into the top-right corner and along it
        tokio::time::sleep(Duration::from_secs(4)).await;
        for _ in 0..40 {
            driver
                .emit(&[InputEvent::Motion {
                    dx: 400.0,
                    dy: -400.0,
                }])
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let end = tokio::time::Instant::now() + Duration::from_secs(4);
        while tokio::time::Instant::now() < end {
            driver
                .emit(&[InputEvent::Motion { dx: 50.0, dy: 0.0 }])
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(ModuleReply::InputLoopback(stats))) if stats.received > 0 => {
                result = Some(stats);
            }
            other => println!("{label}: tentativa {attempts}: sem captura ({other:?})"),
        }
    }
    let _ = driver.stop().await;
    let stats = result.expect("barrier never activated in 3 attempts");
    println!(
        "portal_loopback[{label}]: tentativas={attempts} eventos={} p50={}µs p95={}µs perdidos={} emitidos_no_destino={}",
        stats.received,
        stats.p50_us,
        stats.p95_us,
        stats.lost,
        emu_handle.emitted()
    );
    assert!(stats.received > 0);
    assert!(stats.p50_us <= stats.p95_us);
}
