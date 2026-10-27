//! §9 S3 item 6: P4c without a human, on a graphical session.
//! `cargo test -p clipsync-input -- --ignored portal_loopback --nocapture`
//!
//! The RemoteDesktop emulator pushes the pointer into the InputCapture barrier; the captured
//! events travel over MemHub to a fake destination. Up to 3 attempts (the spike saw 1 in 3 fail
//! to activate); the test fails only if all 3 do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(target_os = "linux")]
use clipsync_input::{
    EmulateAdapter, FakeCapture, FakeEmulate, InputConfig, PortalCapture, PortalEmulate,
    TokenStore, Zone, build_with_adapters,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requer sessão gráfica"]
async fn portal_loopback() {
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
    tokio::spawn(
        build_with_adapters(
            InputConfig::default(),
            Box::new(PortalCapture::new()),
            Box::new(FakeEmulate::new().0),
        )
        .run(a_host, a_rx),
    );
    let (fake_emu, emu_handle) = FakeEmulate::new();
    let (fake_cap, _cap_handle) = FakeCapture::new(Vec::<Zone>::new());
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
            other => println!("tentativa {attempts}: sem captura ({other:?})"),
        }
    }
    let _ = driver.stop().await;
    let stats = result.expect("barrier never activated in 3 attempts");
    println!(
        "portal_loopback: tentativas={attempts} eventos={} p50={}µs p95={}µs perdidos={} emitidos_no_destino={}",
        stats.received,
        stats.p50_us,
        stats.p95_us,
        stats.lost,
        emu_handle.emitted()
    );
    assert!(stats.received > 0);
    assert!(stats.p50_us <= stats.p95_us);
}
