//! Local control channel: client side (`request`), server side (`serve`) plus the framing both sides share.
//! Frame = u32 big-endian length + JSON body, at most `MAX_FRAME_BYTES`; one request, one response, close.
//! Unix: socket `DIR/agent.sock` (0600). Windows: named pipe `\\.\pipe\clipsync-<32 random hex>`, drawn per
//! `serve` and published in `DIR\agent.pipe` (contract §11 rule 5); same frames on both.
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
#[cfg(windows)]
async fn connect(socket: &Path) -> Result<NamedPipeClient, CoreError> {
    let name = read_pipe_name(socket)?.ok_or(CoreError::AgentNotRunning)?;
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

#[cfg(any(windows, test))]
const PIPE_PREFIX: &str = r"\\.\pipe\clipsync-";

/// A fresh name per `serve`. The `\\.\pipe\` namespace is global and any local user can list it: a name
/// fixed or derived from the profile path could be created first by someone else, who would then receive
/// what `request` sends. A name drawn at startup cannot be taken ahead of time.
#[cfg(any(windows, test))]
fn new_pipe_name() -> String {
    format!("{PIPE_PREFIX}{}", hex::encode(rand::random::<[u8; 16]>()))
}

#[cfg(any(windows, test))]
fn is_pipe_name(s: &str) -> bool {
    s.strip_prefix(PIPE_PREFIX)
        .is_some_and(|h| h.len() == 32 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// The name published in `DIR\agent.pipe`; `None` when there is no such file.
#[cfg(windows)]
fn read_pipe_name(file: &Path) -> Result<Option<String>, CoreError> {
    match std::fs::read_to_string(file) {
        Ok(name) if is_pipe_name(&name) => Ok(Some(name)),
        Ok(_) => Err(invalid(format!(
            "{}: not a clipsync pipe name",
            file.display()
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(windows)]
enum PipeProbe {
    Absent,
    Stale,
    Live,
}

/// Reads `DIR\agent.pipe` and tries its name: NotFound means a stale file; any other error is surfaced.
#[cfg(windows)]
fn probe(file: &Path) -> Result<PipeProbe, CoreError> {
    let Some(name) = read_pipe_name(file)? else {
        return Ok(PipeProbe::Absent);
    };
    match ClientOptions::new().open(&name) {
        Ok(_) => Ok(PipeProbe::Live),
        Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => Ok(PipeProbe::Live),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PipeProbe::Stale),
        Err(e) => Err(e.into()),
    }
}

/// The control channel, bound and published but not answering yet: `agent::run` binds it BEFORE
/// the mesh starts, so whoever sees a peer come up (a module, a test, the UI) already finds the
/// agent on `socket`. Requests that arrive before `serve` wait for it. Dropped without serving,
/// it unpublishes `socket` like `serve` does on exit.
pub struct ControlListener {
    #[cfg(unix)]
    listener: UnixListener,
    #[cfg(windows)]
    name: String,
    #[cfg(windows)]
    server: tokio::net::windows::named_pipe::NamedPipeServer,
    published: Published,
}

/// Removes the published file on drop. Windows: only while it still holds our pipe name (a file
/// another agent rewrote is left alone).
struct Published {
    socket: std::path::PathBuf,
    #[cfg(windows)]
    name: String,
}

impl Drop for Published {
    fn drop(&mut self) {
        #[cfg(unix)]
        let _ = std::fs::remove_file(&self.socket);
        #[cfg(windows)]
        if matches!(read_pipe_name(&self.socket), Ok(Some(n)) if n == self.name) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }
}

/// Binds the control socket. A live agent already on `socket` gives `AgentAlreadyRunning`; a stale
/// file is replaced.
#[cfg(unix)]
pub async fn bind(socket: &Path) -> Result<ControlListener, CoreError> {
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
    Ok(ControlListener {
        listener,
        published: Published {
            socket: socket.to_path_buf(),
        },
    })
}

/// Creates a fresh named pipe and publishes its name in `socket` (`DIR\agent.pipe`): a live agent
/// named there gives `AgentAlreadyRunning`, a stale name is replaced.
/// Access: remote clients are refused; the default pipe DACL lets no other user create an instance or
/// connect for writing; `agent.pipe` is readable only through the profile's ACL.
#[cfg(windows)]
pub async fn bind(socket: &Path) -> Result<ControlListener, CoreError> {
    if let PipeProbe::Live = probe(socket)? {
        return Err(CoreError::AgentAlreadyRunning);
    }
    let name = new_pipe_name();
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)?;
    // Published only once the pipe exists: whoever reads the file finds a server.
    crate::profile::write_private(socket, name.as_bytes())?;
    Ok(ControlListener {
        name: name.clone(),
        server,
        published: Published {
            socket: socket.to_path_buf(),
            name,
        },
    })
}

impl ControlListener {
    /// Answers requests until `shutdown` completes, then unpublishes the socket.
    #[cfg(unix)]
    pub async fn serve(
        self,
        handler: Arc<dyn ControlHandler>,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<(), CoreError> {
        let ControlListener {
            listener,
            published,
        } = self;
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
        drop(published);
        Ok(())
    }

    /// Answers requests until `shutdown` completes, then unpublishes the pipe name.
    #[cfg(windows)]
    pub async fn serve(
        self,
        handler: Arc<dyn ControlHandler>,
        shutdown: impl Future<Output = ()> + Send,
    ) -> Result<(), CoreError> {
        let ControlListener {
            name,
            server,
            published,
        } = self;
        let served = accept_pipes(&name, server, handler, shutdown).await;
        drop(published);
        served
    }
}

/// `bind` then `ControlListener::serve`: serves until `shutdown` completes, then removes the
/// socket file (Windows: the `agent.pipe` file, while it still names our pipe).
pub async fn serve(
    socket: &Path,
    handler: Arc<dyn ControlHandler>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    bind(socket).await?.serve(handler, shutdown).await
}

#[cfg(windows)]
async fn accept_pipes(
    name: &str,
    mut server: tokio::net::windows::named_pipe::NamedPipeServer,
    handler: Arc<dyn ControlHandler>,
    shutdown: impl Future<Output = ()> + Send,
) -> Result<(), CoreError> {
    let next_instance = || {
        ServerOptions::new()
            .reject_remote_clients(true)
            .create(name)
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
    use super::{is_pipe_name, new_pipe_name};

    #[test]
    fn pipe_name_is_random_and_matches_the_contract_shape() {
        let a = new_pipe_name();
        assert!(is_pipe_name(&a), "{a}");
        assert_ne!(a, new_pipe_name());
        let hex = a.strip_prefix(r"\\.\pipe\clipsync-").unwrap_or_default();
        assert_eq!(hex.len(), 32);
        for bad in [
            r"\\.\pipe\clipsync-",
            r"\\.\pipe\clipsync-0123456789ABCDEF0123456789abcdef",
            r"\\.\pipe\other-0123456789abcdef0123456789abcdef",
            r"\\server\pipe\clipsync-0123456789abcdef0123456789abcdef",
        ] {
            assert!(!is_pipe_name(bad), "{bad}");
        }
        assert!(!is_pipe_name(&format!("{a}\n")));
    }
}
