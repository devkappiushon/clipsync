//! Real bounded sends and an owned OS thread; stop deliberately cannot interrupt an in-flight
//! send, like PortalCapture. The rescue channel is used ONLY after a regression has timed out.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::adapter::{BarrierSpec, CaptureAdapter, CaptureEvent, InputError, Zone};
use clipsync_proto::InputEvent;
use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tokio::sync::{Notify, mpsc, oneshot};

#[derive(Default)]
pub(crate) struct Handle {
    pub sender: Mutex<Option<mpsc::Sender<CaptureEvent>>>,
    requests: Mutex<Option<mpsc::UnboundedSender<CaptureEvent>>>,
    rescue: Mutex<Option<oneshot::Sender<()>>>,
    pub starts: AtomicUsize,
    pub joined: AtomicUsize,
    pub releasing: Notify,
    pub release_open: AtomicBool,
    pub release: Notify,
    pub blocked: Notify,
}
impl Handle {
    pub fn sender(&self) -> mpsc::Sender<CaptureEvent> {
        self.sender.lock().unwrap().as_ref().unwrap().clone()
    }
    pub async fn fill_and_block(&self) {
        let tx = self.sender();
        let input = || CaptureEvent::Input {
            t_mono_us: 0,
            ev: InputEvent::Motion { dx: 9.0, dy: 0.0 },
        };
        while tx.try_send(input()).is_ok() {}
        assert_eq!(tx.capacity(), 0);
        self.requests
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(input())
            .unwrap();
        self.blocked.notified().await;
        assert_eq!(
            tx.capacity(),
            0,
            "producer was not waiting on a full channel"
        );
    }
    pub fn rescue(&self) {
        if let Some(tx) = self.rescue.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}
pub(crate) struct Capture {
    handle: Arc<Handle>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Capture {
    pub fn new(handle: Arc<Handle>) -> Self {
        Self {
            handle,
            stop: None,
            thread: None,
        }
    }
}
#[async_trait::async_trait]
impl CaptureAdapter for Capture {
    fn name(&self) -> &'static str {
        "blocked-send-thread"
    }
    async fn zones(&mut self) -> Result<Vec<Zone>, InputError> {
        Ok(vec![crate::fake_zone()])
    }
    fn known_zones(&self) -> Vec<Zone> {
        vec![crate::fake_zone()]
    }
    async fn start(
        &mut self,
        _: &[BarrierSpec],
    ) -> Result<mpsc::Receiver<CaptureEvent>, InputError> {
        assert!(
            self.thread.is_none(),
            "old producer must be joined before start"
        );
        let (tx, rx) = mpsc::channel(4096);
        let (requests, mut commands) = mpsc::unbounded_channel();
        let (stop, mut stopped) = oneshot::channel();
        let (rescue, rescued) = oneshot::channel();
        *self.handle.sender.lock().unwrap() = Some(tx.clone());
        *self.handle.requests.lock().unwrap() = Some(requests);
        *self.handle.rescue.lock().unwrap() = Some(rescue);
        self.stop = Some(stop);
        let h = self.handle.clone();
        self.thread = Some(std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                tokio::select! {
                    _ = rescued => (),
                    _ = async {
                        loop {
                            tokio::select! {
                                biased;
                                _ = &mut stopped => break,
                                ev = commands.recv() => {
                                    let Some(ev) = ev else { break };
                                    let mut send = Box::pin(tx.send(ev));
                                    let sent = std::future::poll_fn(|cx| {
                                        let result = send.as_mut().poll(cx);
                                        if result.is_pending() { h.blocked.notify_one(); }
                                        result
                                    }).await;
                                    if sent.is_err() { break; }
                                }
                            }
                        }
                    } => (),
                }
            });
        }));
        self.handle.starts.fetch_add(1, Ordering::SeqCst);
        Ok(rx)
    }
    async fn release(&mut self) -> Result<(), InputError> {
        self.handle.releasing.notify_one();
        if !self.handle.release_open.load(Ordering::SeqCst) {
            self.handle.release.notified().await;
        }
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), InputError> {
        self.stop = None;
        if let Some(t) = self.thread.take() {
            tokio::task::spawn_blocking(move || t.join())
                .await
                .unwrap()
                .unwrap();
            self.handle.joined.fetch_add(1, Ordering::SeqCst);
        }
        *self.handle.sender.lock().unwrap() = None;
        *self.handle.requests.lock().unwrap() = None;
        Ok(())
    }
}
