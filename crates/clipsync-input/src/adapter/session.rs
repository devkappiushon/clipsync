//! Screen lock on Linux (contract §13.6 item 8): the `LockedHint` of this login session at
//! logind (`org.freedesktop.login1`), read at start and followed by `PropertiesChanged`. Without
//! logind the screen counts as unlocked and the diagnostics say so.
use super::CaptureEvent;
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const LOGIND: &str = "org.freedesktop.login1";

/// Follows `LockedHint`. Dropping it stops the task.
pub(crate) struct LockWatch {
    locked: Arc<AtomicBool>,
    available: bool,
    task: Option<JoinHandle<()>>,
}

impl LockWatch {
    /// Reads the hint and keeps following it. With `events`, every change (and a lock already on
    /// at the start) goes out as `CaptureEvent::SessionLocked`.
    pub(crate) async fn start(events: Option<mpsc::Sender<CaptureEvent>>) -> Self {
        let locked = Arc::new(AtomicBool::new(false));
        let session = match open_session().await {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("logind unavailable, the screen counts as unlocked: {e}");
                return Self {
                    locked,
                    available: false,
                    task: None,
                };
            }
        };
        let now = session
            .get_property::<bool>("LockedHint")
            .await
            .unwrap_or(false);
        locked.store(now, Ordering::SeqCst);
        if now && let Some(tx) = &events {
            let _ = tx.send(CaptureEvent::SessionLocked(true)).await;
        }
        let flag = locked.clone();
        let task = tokio::spawn(async move {
            let mut changes = session.receive_property_changed::<bool>("LockedHint").await;
            while let Some(change) = changes.next().await {
                let Ok(value) = change.get().await else {
                    continue;
                };
                if flag.swap(value, Ordering::SeqCst) == value {
                    continue;
                }
                if let Some(tx) = &events
                    && tx.send(CaptureEvent::SessionLocked(value)).await.is_err()
                {
                    break;
                }
            }
        });
        Self {
            locked,
            available: true,
            task: Some(task),
        }
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.locked.load(Ordering::SeqCst)
    }

    /// False when logind did not answer: the lock cannot be seen.
    pub(crate) fn available(&self) -> bool {
        self.available
    }
}

impl Drop for LockWatch {
    fn drop(&mut self) {
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}

/// The proxy of this login session: `XDG_SESSION_ID` when set, else the session of this process.
async fn open_session() -> zbus::Result<zbus::Proxy<'static>> {
    let conn = zbus::Connection::system().await?;
    let manager = zbus::Proxy::new(
        &conn,
        LOGIND,
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await?;
    let by_env: Option<zbus::zvariant::OwnedObjectPath> = match std::env::var("XDG_SESSION_ID") {
        Ok(id) if !id.is_empty() => manager.call("GetSession", &(id,)).await.ok(),
        _ => None,
    };
    let known = match by_env {
        Some(p) => Some(p),
        None => manager
            .call("GetSessionByPID", &(std::process::id(),))
            .await
            .ok(),
    };
    let path = match known {
        Some(p) => p,
        // A process started outside the login session (a service, a terminal multiplexer): the
        // graphical session of the same user, which is the one that locks.
        None => display_session(&conn, &manager).await?,
    };
    zbus::Proxy::new(&conn, LOGIND, path, "org.freedesktop.login1.Session").await
}

async fn display_session(
    conn: &zbus::Connection,
    manager: &zbus::Proxy<'_>,
) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
    use std::os::unix::fs::MetadataExt;
    let uid = std::fs::metadata("/proc/self")
        .map_err(|e| zbus::Error::Failure(e.to_string()))?
        .uid();
    let user: zbus::zvariant::OwnedObjectPath = manager.call("GetUser", &(uid,)).await?;
    let user = zbus::Proxy::new(conn, LOGIND, user, "org.freedesktop.login1.User").await?;
    let (_id, path): (String, zbus::zvariant::OwnedObjectPath) =
        user.get_property("Display").await?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::LockWatch;

    /// On a machine without logind (a container, a CI runner) the screen counts as unlocked.
    #[tokio::test]
    async fn the_hint_is_read_when_logind_answers_and_unlocked_otherwise() {
        let watch = LockWatch::start(None).await;
        if watch.available() {
            // a test run needs a logged-in desktop, so the screen is not locked
            assert!(!watch.is_locked());
        } else {
            assert!(!watch.is_locked(), "no logind means unlocked");
        }
        eprintln!("logind available: {}", watch.available());
    }
}
