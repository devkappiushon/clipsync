//! Local control channel: client side (`request`), server side (`serve`) plus the framing both sides share.
//! Frame = u32 big-endian length + JSON body, at most `MAX_FRAME_BYTES`; one request, one response, close.
//! Unix: socket `DIR/agent.sock` (0600). Windows: named pipe `\\.\pipe\clipsync-<16 hex>` derived from
//! the profile directory (`pipe_name`); same frames on both.
use crate::CoreError;
use crate::profile::Profile;
use clipsync_proto::{ControlRequest, ControlResponse, MAX_FRAME_BYTES};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient, ServerOptions};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// Win32 ERROR_PIPE_BUSY: every instance of the pipe is serving someone; retry until CONNECT_TIMEOUT.
#[cfg(windows)]
const ERROR_PIPE_BUSY: i32 = 231;
#[cfg(windows)]
const PIPE_BUSY_RETRY: Duration = Duration::from_millis(20);
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
    let mut stream = match tokio::time::timeout(CONNECT_TIMEOUT, connect(&p.socket_path())).await {
        Err(_) => return Err(CoreError::AgentNotRunning),
        Ok(s) => s?,
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

/// Nobody listening on the socket: the agent is not running.
#[cfg(unix)]
fn is_absent(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
    )
}

#[cfg(unix)]
async fn connect(socket: &Path) -> Result<UnixStream, CoreError> {
    match UnixStream::connect(socket).await {
        Ok(s) => Ok(s),
        Err(e) if is_absent(&e) => Err(CoreError::AgentNotRunning),
        Err(e) => Err(e.into()),
    }
}

/// A busy pipe has a live agent behind it: wait for a free instance (the caller bounds the wait).
/// Known gap: a pipe created first by another local user would be accepted here; checking the server's
/// owner needs Win32 calls (unsafe or a new dependency), pending a decision.
#[cfg(windows)]
async fn connect(socket: &Path) -> Result<NamedPipeClient, CoreError> {
    let name = pipe_name(socket);
    loop {
        match ClientOptions::new().open(&name) {
            Ok(c) => return Ok(c),
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                tokio::time::sleep(PIPE_BUSY_RETRY).await;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CoreError::AgentNotRunning);
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// True when an agent answers on the control channel of `socket`'s profile.
#[cfg(unix)]
pub(crate) async fn agent_alive(socket: &Path) -> bool {
    socket.exists() && UnixStream::connect(socket).await.is_ok()
}

/// True when an agent answers on the control channel of `socket`'s profile.
#[cfg(windows)]
pub(crate) async fn agent_alive(socket: &Path) -> bool {
    match ClientOptions::new().open(pipe_name(socket)) {
        Ok(_) => true,
        Err(e) => e.raw_os_error() == Some(ERROR_PIPE_BUSY),
    }
}

/// `\\.\pipe\clipsync-<16 hex>`: the first 8 bytes of SHA-256 over the profile directory (canonical
/// when it exists), so the agent and the CLI of one profile meet on one name and two profiles never share it.
#[cfg(any(windows, test))]
fn pipe_name(socket: &Path) -> String {
    use sha2::{Digest, Sha256};
    let dir = socket.parent().unwrap_or(socket);
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let digest: [u8; 32] = Sha256::digest(dir.as_os_str().as_encoded_bytes()).into();
    format!(r"\\.\pipe\clipsync-{}", hex::encode(&digest[..8]))
}

/// Serves the control socket until `shutdown` completes, then removes the socket file.
/// A live agent already on `socket` gives `AgentAlreadyRunning`; a stale file is replaced.
#[cfg(unix)]
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
            Err(e) if is_absent(&e) => std::fs::remove_file(socket)?,
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

/// Serves the profile's named pipe until `shutdown` completes. `first_pipe_instance` makes the name a lock:
/// another agent on this profile already owns it, so creation fails and we give `AgentAlreadyRunning`.
/// Access: remote clients are refused; the default pipe DACL grants write only to the owner, SYSTEM and
/// Administrators (Everyone may open it read-only, which cannot send a request). Nothing to clean up on exit:
/// the pipe disappears with its last handle.
#[cfg(windows)]
pub async fn serve(
    socket: &Path,
    handler: Arc<dyn ControlHandler>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    let name = pipe_name(socket);
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => CoreError::AgentAlreadyRunning,
            _ => e.into(),
        })?;
    let next_instance = || {
        ServerOptions::new()
            .reject_remote_clients(true)
            .create(&name)
    };
    let mut tasks = tokio::task::JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => break,
            connected = server.connect() => match connected {
                // The next instance exists before this one is handed off: clients see "busy", never "absent".
                Ok(()) => {
                    let client = std::mem::replace(&mut server, next_instance()?);
                    tasks.spawn(serve_one(client, handler.clone()));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "control pipe connect failed");
                    server = next_instance()?;
                }
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    tasks.abort_all();
    Ok(())
}

async fn serve_one<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    handler: Arc<dyn ControlHandler>,
) {
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
#[cfg(unix)]
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

#[cfg(test)]
mod tests {
    use super::pipe_name;
    use std::path::Path;

    #[test]
    fn pipe_name_is_stable_per_profile_and_distinct_across_profiles() {
        let a = pipe_name(Path::new("/nonexistent/profile-a/agent.sock"));
        assert_eq!(a, pipe_name(Path::new("/nonexistent/profile-a/agent.sock")));
        assert_ne!(a, pipe_name(Path::new("/nonexistent/profile-b/agent.sock")));
        let hash = a.strip_prefix(r"\\.\pipe\clipsync-").unwrap_or_default();
        assert_eq!(hash.len(), 16, "{a}");
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{a}");
    }
}
