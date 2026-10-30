#[derive(Default)]
struct PrepareGate {
    entered: tokio::sync::Notify,
    stop_fails: std::sync::atomic::AtomicBool,
    complete: tokio::sync::Notify,
    pending: std::sync::atomic::AtomicBool,
    cancelled: std::sync::atomic::AtomicBool,
}
struct PreparingCapture {
    inner: crate::FakeCapture,
    gate: std::sync::Arc<PrepareGate>,
}
#[async_trait::async_trait]
impl crate::adapter::CaptureAdapter for PreparingCapture {
    fn name(&self) -> &'static str {
        "prepare-gate"
    }
    fn known_monitor_observation(
        &self,
    ) -> Result<crate::inventory::ObservedMonitors, crate::adapter::InputError> {
        self.inner.known_monitor_observation()
    }
    async fn prepare_monitor_observation(
        &mut self,
    ) -> Result<crate::inventory::ObservedMonitors, crate::adapter::InputError> {
        struct Pending(std::sync::Arc<PrepareGate>);
        impl Drop for Pending {
            fn drop(&mut self) {
                self.0
                    .pending
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                self.0
                    .cancelled
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let _pending = Pending(self.gate.clone());
        self.gate
            .pending
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.gate.entered.notify_one();
        self.gate.complete.notified().await;
        self.inner.known_monitor_observation()
    }
    fn known_zones(&self) -> Vec<crate::adapter::Zone> {
        self.inner.known_zones()
    }
    async fn zones(&mut self) -> Result<Vec<crate::adapter::Zone>, crate::adapter::InputError> {
        self.inner.zones().await
    }
    async fn start(
        &mut self,
        b: &[crate::adapter::BarrierSpec],
    ) -> Result<tokio::sync::mpsc::Receiver<crate::adapter::CaptureEvent>, crate::adapter::InputError>
    {
        self.inner.start(b).await
    }
    async fn release(&mut self) -> Result<(), crate::adapter::InputError> {
        self.inner.release().await
    }
    async fn stop(&mut self) -> Result<(), crate::adapter::InputError> {
        if self
            .gate
            .stop_fails
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(crate::adapter::InputError::Other("stop failed".into()));
        }
        self.inner.stop().await
    }
}
