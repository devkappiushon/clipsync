#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_core::control::{self, ControlHandler};
use clipsync_core::{CoreError, Profile};
use clipsync_proto::{ControlRequest, ControlResponse};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

struct H;
#[async_trait::async_trait]
impl ControlHandler for H {
    async fn handle(&self, req: ControlRequest) -> ControlResponse {
        match req {
            ControlRequest::Pause => ControlResponse::Ok,
            _ => ControlResponse::Error {
                message: "x".into(),
            },
        }
    }
}

#[tokio::test]
async fn serve_request_roundtrip_stale_socket_and_cleanup() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().to_owned())).unwrap();
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Err(CoreError::AgentNotRunning)
    ));
    std::fs::write(p.socket_path(), b"").unwrap(); // stale file, nobody listening
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Err(CoreError::AgentNotRunning)
    ));
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let sock = p.socket_path();
    let server = tokio::spawn(async move {
        control::serve(&sock, Arc::new(H), async {
            let _ = stopped.await;
        })
        .await
    });
    for _ in 0..100 {
        if std::fs::metadata(p.socket_path())
            .map(|m| m.permissions().mode() & 0o777 == 0o600)
            .unwrap_or(false)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Ok(ControlResponse::Ok)
    ));
    assert!(matches!(
        control::request(&p, ControlRequest::Status).await,
        Ok(ControlResponse::Error { .. })
    ));
    // a second agent on the same socket is refused
    let again = control::serve(&p.socket_path(), Arc::new(H), std::future::pending()).await;
    assert!(matches!(again, Err(CoreError::AgentAlreadyRunning)));
    // an oversized frame is refused before allocation, and the server keeps working
    let mut raw = tokio::net::UnixStream::connect(p.socket_path())
        .await
        .unwrap();
    raw.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
    drop(raw);
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Ok(ControlResponse::Ok)
    ));
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(!p.socket_path().exists(), "socket removed on shutdown");
}
