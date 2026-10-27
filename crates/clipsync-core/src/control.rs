//! Local control socket: client side (`request`) plus the framing both sides share.
//! Frame = u32 big-endian length + JSON body, at most `MAX_FRAME_BYTES`; one request, one response, close.
use crate::CoreError;
use crate::profile::Profile;
use clipsync_proto::{ControlRequest, ControlResponse, MAX_FRAME_BYTES};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// Long enough for `probe input --loopback --seconds N` and a `ClipSend` that waits for acks.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);

fn invalid(msg: impl Into<String>) -> CoreError {
    CoreError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        msg.into(),
    ))
}

/// Reads one frame. The length is checked BEFORE the body is allocated.
pub async fn read_message<T: DeserializeOwned, R: AsyncRead + Unpin>(
    r: &mut R,
    max: usize,
) -> Result<T, CoreError> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > max {
        return Err(invalid(format!("frame too large: {len} > {max}")));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).await?;
    serde_json::from_slice(&body).map_err(|e| invalid(format!("decode: {e}")))
}

pub async fn write_message<T: Serialize, W: AsyncWrite + Unpin>(
    w: &mut W,
    msg: &T,
    max: usize,
) -> Result<(), CoreError> {
    let body = serde_json::to_vec(msg).map_err(|e| invalid(format!("encode: {e}")))?;
    if body.len() > max {
        return Err(invalid(format!("frame too large: {} > {max}", body.len())));
    }
    let len = u32::try_from(body.len()).map_err(|_| invalid("frame too large"))?;
    w.write_all(&len.to_be_bytes()).await?;
    w.write_all(&body).await?;
    w.flush().await?;
    Ok(())
}

/// Sends `req` to the agent of profile `p`. `CoreError::AgentNotRunning` when nobody listens on the socket.
pub async fn request(p: &Profile, req: ControlRequest) -> Result<ControlResponse, CoreError> {
    let connect = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(p.socket_path()));
    let mut stream = match connect.await {
        Err(_) => return Err(CoreError::AgentNotRunning),
        Ok(Ok(s)) => s,
        Ok(Err(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Err(CoreError::AgentNotRunning);
        }
        Ok(Err(e)) => return Err(e.into()),
    };
    let exchange = async {
        write_message(&mut stream, &req, MAX_FRAME_BYTES).await?;
        read_message::<ControlResponse, _>(&mut stream, MAX_FRAME_BYTES).await
    };
    tokio::time::timeout(RESPONSE_TIMEOUT, exchange)
        .await
        .map_err(|_| {
            CoreError::Io(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "timeout waiting for the agent",
            ))
        })?
}

/// What the agent implements; `serve` does the socket work around it.
#[async_trait::async_trait]
pub trait ControlHandler: Send + Sync + 'static {
    async fn handle(&self, req: ControlRequest) -> ControlResponse;
}

/// Time a client has to deliver its request once connected.
const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Serves the control socket until `shutdown` completes, then removes the socket file.
/// A live agent already on `socket` gives `AgentAlreadyRunning`; a stale file is replaced.
pub async fn serve(
    socket: &Path,
    handler: Arc<dyn ControlHandler>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    // Only "nobody answers" proves the socket is stale. Any other connect error (a full backlog, say)
    // means a live agent may be there, and its socket must not be deleted.
    if socket.exists() {
        match UnixStream::connect(socket).await {
            Ok(_) => return Err(CoreError::AgentAlreadyRunning),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                std::fs::remove_file(socket)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let listener = bind_private(socket)?;
    let mut tasks = tokio::task::JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => { tasks.spawn(serve_one(stream, handler.clone())); }
                Err(e) => tracing::warn!(error = %e, "control socket accept failed"),
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    tasks.abort_all();
    let _ = std::fs::remove_file(socket);
    Ok(())
}

async fn serve_one(mut stream: UnixStream, handler: Arc<dyn ControlHandler>) {
    let req = match tokio::time::timeout(
        REQUEST_READ_TIMEOUT,
        read_message::<ControlRequest, _>(&mut stream, MAX_FRAME_BYTES),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::debug!(error = %e, "control request rejected");
            let reply = ControlResponse::Error {
                message: "invalid request".into(),
            };
            let _ = write_message(&mut stream, &reply, MAX_FRAME_BYTES).await;
            return;
        }
        Err(_) => return,
    };
    let resp = handler.handle(req).await;
    if let Err(e) = write_message(&mut stream, &resp, MAX_FRAME_BYTES).await {
        tracing::debug!(error = %e, "control response not delivered");
    }
}

/// Binds under a temporary name, restricts it to 0600, then links it to `socket`: the final name never
/// exists with a wider mode, and `hard_link` fails if another agent got there first.
fn bind_private(socket: &Path) -> Result<UnixListener, CoreError> {
    let tmp = socket.with_extension(format!("tmp-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let listener = UnixListener::bind(&tmp)?;
    let linked = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
        .and_then(|()| std::fs::hard_link(&tmp, socket));
    let _ = std::fs::remove_file(&tmp);
    match linked {
        Ok(()) => Ok(listener),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(CoreError::AgentAlreadyRunning)
        }
        Err(e) => Err(e.into()),
    }
}
