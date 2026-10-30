//! Screen lock on Linux (contract §13.6 item 8): the `LockedHint` of this login session at
//! logind (`org.freedesktop.login1`), read at start and followed by `PropertiesChanged`. Without
//! a usable logind snapshot the gate stays closed and the diagnostics say why.
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
    /// Publishes an explicit initial snapshot, then follows changes. Unknown means closed.
    pub(crate) async fn start(events: Option<mpsc::Sender<CaptureEvent>>) -> Self {
        let locked = Arc::new(AtomicBool::new(true));
        let session = match open_session().await {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!("logind unavailable: {e}");
                snapshot(&events, None).await;
                return Self {
                    locked,
                    available: false,
                    task: None,
                };
            }
        };
        let now = match session.get_property::<bool>("LockedHint").await {
            Ok(now) => now,
            Err(_) => {
                snapshot(&events, None).await;
                return Self {
                    locked,
                    available: false,
                    task: None,
                };
            }
        };
        locked.store(now, Ordering::SeqCst);
        snapshot(&events, Some(now)).await;
        let flag = locked.clone();
        let task = tokio::spawn(async move {
            let mut changes = session.receive_property_changed::<bool>("LockedHint").await;
            while let Some(change) = changes.next().await {
                let value = match change.get().await {
                    Ok(value) => value,
                    Err(_) => {
                        flag.store(true, Ordering::SeqCst);
                        snapshot(&events, None).await;
                        continue;
                    }
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

async fn snapshot(events: &Option<mpsc::Sender<CaptureEvent>>, locked: Option<bool>) {
    if let Some(tx) = events {
        let _ = tx
            .send(CaptureEvent::SessionLocked(locked.unwrap_or(true)))
            .await;
        if locked.is_none() {
            let _ = tx
                .send(CaptureEvent::Error(
                    crate::msg::SCREEN_STATE_UNAVAILABLE.to_owned(),
                ))
                .await;
        }
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

    #[tokio::test]
    async fn every_new_watcher_publishes_an_explicit_initial_snapshot() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let _watch = LockWatch::start(Some(tx)).await;
        let first = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await;
        assert!(
            matches!(first, Ok(Some(super::CaptureEvent::SessionLocked(_)))),
            "new producer must publish an initial lock snapshot"
        );
    }

    /// No login-session answer is a closed gate, including headless CI.
    #[tokio::test]
    async fn the_hint_is_read_when_logind_answers_and_closed_otherwise() {
        let watch = LockWatch::start(None).await;
        if !watch.available() {
            assert!(watch.is_locked(), "unavailable is not unlocked");
        }
    }
    #[tokio::test]
    async fn snapshots_preserve_false_true_and_unknown_without_inventing_unlock() {
        for state in [Some(false), Some(true), None] {
            let (tx, mut rx) = tokio::sync::mpsc::channel(4);
            super::snapshot(&Some(tx), state).await;
            assert!(
                matches!(rx.recv().await, Some(super::CaptureEvent::SessionLocked(value)) if value == state.unwrap_or(true))
            );
            if state.is_none() {
                assert!(
                    matches!(rx.recv().await, Some(super::CaptureEvent::Error(text)) if text == crate::msg::SCREEN_STATE_UNAVAILABLE)
                );
            }
        }
    }
}
