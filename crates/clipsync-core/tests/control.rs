#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns
use clipsync_core::control::{self, ControlHandler};
use clipsync_core::{CoreError, Profile};
use clipsync_proto::{ControlRequest, ControlResponse};
#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[tokio::test]
async fn socket_is_created_private_and_a_live_agent_is_never_unlinked() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().to_owned())).unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let sock = p.socket_path();
    let server = tokio::spawn(async move {
        control::serve(&sock, Arc::new(H), async {
            let _ = stopped.await;
        })
        .await
    });
    for _ in 0..100 {
        if p.socket_path().exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let mode = std::fs::metadata(p.socket_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    // no temporary socket is left behind
    let leftovers: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    // a second serve loses and the first socket is still the one answering
    assert!(matches!(
        control::serve(&p.socket_path(), Arc::new(H), std::future::pending()).await,
        Err(CoreError::AgentAlreadyRunning)
    ));
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Ok(ControlResponse::Ok)
    ));
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[cfg(windows)]
fn published(p: &Profile) -> Option<String> {
    std::fs::read_to_string(p.socket_path()).ok()
}

#[cfg(windows)]
fn is_pipe_name(s: &str) -> bool {
    s.strip_prefix(r"\\.\pipe\clipsync-")
        .is_some_and(|h| h.len() == 32 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// Starts an agent and waits until it publishes a name other than `previous`.
#[cfg(windows)]
async fn spawn_agent(
    p: &Profile,
    previous: &str,
) -> (
    String,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), CoreError>>,
) {
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let file = p.socket_path();
    let server = tokio::spawn(async move {
        control::serve(&file, Arc::new(H), async {
            let _ = stopped.await;
        })
        .await
    });
    for _ in 0..200 {
        if let Some(name) = published(p).filter(|n| n != previous) {
            return (name, stop, server);
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("agent.pipe never published");
}

/// Contract §9 S5 item 4: (a) a second serve is refused, (b) no agent gives AgentNotRunning, (c) a stale
/// agent.pipe is replaced, (d) each serve draws a new name of the contract shape, (e) agent.pipe is removed
/// on shutdown.
#[cfg(windows)]
#[tokio::test]
async fn pipe_name_is_drawn_per_serve_published_and_removed() {
    use tokio::net::windows::named_pipe::ClientOptions;
    let tmp = tempfile::tempdir().unwrap();
    let p = Profile::resolve(Some(tmp.path().to_owned())).unwrap();
    assert_eq!(p.socket_path().file_name().unwrap(), "agent.pipe");
    // (b)
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Err(CoreError::AgentNotRunning)
    ));
    // (c) a well-formed name nobody serves
    let stale = format!(r"\\.\pipe\clipsync-{}", "0".repeat(32));
    std::fs::write(p.socket_path(), &stale).unwrap();
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Err(CoreError::AgentNotRunning)
    ));
    let (first, stop, server) = spawn_agent(&p, &stale).await;
    assert!(is_pipe_name(&first), "{first}");
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Ok(ControlResponse::Ok)
    ));
    assert!(matches!(
        control::request(&p, ControlRequest::Status).await,
        Ok(ControlResponse::Error { .. })
    ));
    // (a) and the live agent's name stays published
    assert!(matches!(
        control::serve(&p.socket_path(), Arc::new(H), std::future::pending()).await,
        Err(CoreError::AgentAlreadyRunning)
    ));
    assert_eq!(published(&p).as_deref(), Some(first.as_str()));
    // an oversized frame is refused before allocation, and the server keeps working
    let mut raw = ClientOptions::new().open(&first).unwrap();
    raw.write_all(&u32::MAX.to_be_bytes()).await.unwrap();
    drop(raw);
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Ok(ControlResponse::Ok)
    ));
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    // (e)
    assert!(!p.socket_path().exists(), "agent.pipe removed on shutdown");
    assert!(matches!(
        control::request(&p, ControlRequest::Pause).await,
        Err(CoreError::AgentNotRunning)
    ));
    // (d) a new serve, a new name
    let (second, stop, server) = spawn_agent(&p, "").await;
    assert!(is_pipe_name(&second) && second != first, "{second}");
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
