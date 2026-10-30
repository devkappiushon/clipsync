//! Owned, bounded input network jobs. No payload or key diagnostics.
use clipsync_proto::SendError;
use std::future::Future;
use std::pin::Pin;
pub(crate) type Sending = Pin<Box<dyn Future<Output = Result<(), SendError>> + Send>>;
use std::time::Duration;
use tokio::task::JoinSet;
pub(crate) const SEND_WAIT: Duration = Duration::from_secs(2);
#[derive(Default)]
pub(crate) struct Network {
    jobs: JoinSet<()>,
}
impl Network {
    pub fn has_capacity(&mut self) -> bool {
        while self.jobs.try_join_next().is_some() {}
        self.jobs.len() < 32
    }
    pub fn spawn<T: Send + 'static>(
        &mut self,
        job: impl Future<Output = T> + Send + 'static,
    ) -> bool {
        self.spawn_for(SEND_WAIT, job)
    }
    pub fn spawn_for<T: Send + 'static>(
        &mut self,
        budget: Duration,
        job: impl Future<Output = T> + Send + 'static,
    ) -> bool {
        while self.jobs.try_join_next().is_some() {}
        if self.jobs.len() >= 32 {
            return false;
        }
        self.jobs.spawn(async move {
            let _ = tokio::time::timeout(budget, job).await;
        });
        true
    }
    pub async fn stop(&mut self) {
        self.jobs.abort_all();
        while self.jobs.join_next().await.is_some() {}
    }
}
