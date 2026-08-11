use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, Notify};
use tokio::time::{Duration, Instant};

pub struct RateLimitError {
    pub retry_after: Duration,
    pub global: bool,
}

impl RateLimitError {
    pub fn new(retry_after: Duration, global: bool) -> Self {
        Self {
            retry_after,
            global,
        }
    }
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rate limit error: retry_after: {:?}, global: {}",
            self.retry_after, self.global
        )
    }
}

impl Debug for RateLimitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RateLimitError {{ retry_after: {:?}, global: {} }}",
            self.retry_after, self.global
        )
    }
}

impl std::error::Error for RateLimitError {}

#[derive(Clone)]
pub(crate) struct RateLimiter {
    retry_until: Arc<Mutex<Option<Instant>>>,
    notify: Arc<Notify>,
    route_mutex: Arc<Mutex<()>>,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        RateLimiter {
            retry_until: Arc::new(Mutex::new(None)),
            notify: Arc::new(Notify::new()),
            route_mutex: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) async fn wait_if_needed(&self) {
        loop {
            let now = Instant::now();
            let retry_time = {
                let retry_until = self.retry_until.lock().await;
                *retry_until
            };

            if let Some(time) = retry_time {
                if time > now {
                    let duration = time - now;
                    tokio::select! {
                        _ = tokio::time::sleep(duration) => {},
                        _ = self.notify.notified() => {},
                    }
                } else {
                    let mut retry_until = self.retry_until.lock().await;
                    *retry_until = None;
                    return;
                }
            } else {
                return;
            }
        }
    }

    pub(crate) async fn lock_route(&self) -> MutexGuard<'_, ()> {
        let guard = self.route_mutex.lock().await;
        self.wait_if_needed().await;
        guard
    }

    pub(crate) async fn update(&self, retry_after: Duration) {
        let mut retry_until = self.retry_until.lock().await;
        let new_retry_until = Instant::now() + retry_after;
        *retry_until = Some(new_retry_until);
        self.notify.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimiter;
    use tokio::sync::oneshot;
    use tokio::sync::oneshot::error::TryRecvError;
    use tokio::time::{Duration, advance};

    #[tokio::test(start_paused = true)]
    async fn route_lock_rechecks_cooldown_after_waiting() {
        let limiter = RateLimiter::new();
        let first_guard = limiter.lock_route().await;
        let waiting_limiter = limiter.clone();
        let (started_tx, started_rx) = oneshot::channel();
        let (acquired_tx, mut acquired_rx) = oneshot::channel();

        tokio::spawn(async move {
            started_tx.send(()).unwrap();
            let _guard = waiting_limiter.lock_route().await;
            acquired_tx.send(()).unwrap();
        });

        started_rx.await.unwrap();
        tokio::task::yield_now().await;
        limiter.update(Duration::from_secs(60)).await;
        drop(first_guard);
        tokio::task::yield_now().await;

        assert_eq!(acquired_rx.try_recv(), Err(TryRecvError::Empty));
        advance(Duration::from_secs(59)).await;
        tokio::task::yield_now().await;
        assert_eq!(acquired_rx.try_recv(), Err(TryRecvError::Empty));

        advance(Duration::from_secs(1)).await;
        acquired_rx.await.unwrap();
    }
}
