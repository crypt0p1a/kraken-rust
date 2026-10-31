use crate::KrakenError;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

const PREFIX: &str = "[RateLimitQueue]";

/// Receives the logs of the [`RateLimiter`]: tag, message
pub type Logger = Arc<dyn Fn(&str, &str) + Send + Sync>;

struct Bucket {
    tokens: u32,
    last_refill: Instant,
}

/// Token bucket shared by the clients: each call consumes a token, a new token is created every
/// `token_interval` up to `pool_maximum` tokens. The calls are executed one after the other, which
/// also keeps the Kraken nonces in increasing order.
///
/// The default matches the Starter tier of Kraken: a counter of 15, decreased by 0.33 per second.
/// See <https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits>
pub struct RateLimiter {
    pool_maximum: u32,
    token_interval: Duration,
    bucket: Mutex<Bucket>,
    logger: Option<Logger>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(15, Duration::from_secs(3), None)
    }
}

impl RateLimiter {
    pub fn new(pool_maximum: u32, token_interval: Duration, logger: Option<Logger>) -> Self {
        Self {
            pool_maximum,
            // a zero interval would never refill: at least one millisecond
            token_interval: token_interval.max(Duration::from_millis(1)),
            bucket: Mutex::new(Bucket {
                tokens: pool_maximum,
                last_refill: Instant::now(),
            }),
            logger,
        }
    }

    fn log(&self, message: &str) {
        if let Some(logger) = &self.logger {
            logger(PREFIX, message);
        }
    }

    fn refill(&self, bucket: &mut Bucket) {
        let created = bucket.last_refill.elapsed().as_nanos() / self.token_interval.as_nanos();

        if created == 0 {
            return;
        }

        let created = u32::try_from(created).unwrap_or(u32::MAX);
        bucket.tokens = bucket.tokens.saturating_add(created).min(self.pool_maximum);
        bucket.last_refill = if bucket.tokens == self.pool_maximum {
            Instant::now()
        } else {
            bucket.last_refill + self.token_interval * created
        };

        self.log(&format!("pool is now {}", bucket.tokens));
    }

    /// Waits for a token, then executes the call while holding the queue
    pub async fn enqueue<T, F>(&self, tag: &str, call: F) -> Result<T, KrakenError>
    where
        F: Future<Output = Result<T, KrakenError>>,
    {
        let mut bucket = self.bucket.lock().await;

        loop {
            self.refill(&mut bucket);

            if bucket.tokens > 0 {
                break;
            }

            self.log(&format!(
                "{tag} :: No token available (max:{})",
                self.pool_maximum
            ));

            let next_token = bucket.last_refill + self.token_interval;
            tokio::time::sleep_until(next_token).await;
        }

        bucket.tokens -= 1;

        let result = call.await;

        if let Err(error) = &result {
            self.log(&format!(
                "{tag} :: Error: {error} / tokens: {}",
                bucket.tokens
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn waits_for_a_token_when_the_pool_is_empty() {
        let limiter = RateLimiter::new(2, Duration::from_secs(2), None);
        let start = Instant::now();

        for _ in 0..2 {
            limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        }
        assert_eq!(start.elapsed(), Duration::ZERO);

        limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        assert_eq!(start.elapsed(), Duration::from_secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn refills_up_to_the_pool_maximum() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1), None);

        for _ in 0..3 {
            limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        }

        tokio::time::advance(Duration::from_secs(60)).await;

        let start = Instant::now();
        for _ in 0..3 {
            limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        }
        assert_eq!(start.elapsed(), Duration::ZERO);

        limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        assert_eq!(start.elapsed(), Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn consumes_a_token_for_failed_calls() {
        let limiter = RateLimiter::new(1, Duration::from_secs(5), None);

        let failure: Result<(), _> = limiter
            .enqueue("test", async {
                Err(KrakenError::Transport {
                    reason: "offline".into(),
                })
            })
            .await;
        assert!(failure.is_err());

        let start = Instant::now();
        limiter.enqueue("test", async { Ok(()) }).await.unwrap();
        assert_eq!(start.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn executes_the_concurrent_calls_one_after_the_other() {
        let limiter = Arc::new(RateLimiter::new(1, Duration::from_secs(1), None));
        let running = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let start = Instant::now();

        let tasks: Vec<_> = (0..3)
            .map(|_| {
                let limiter = limiter.clone();
                let running = running.clone();
                tokio::spawn(async move {
                    limiter
                        .enqueue("test", async {
                            let others = running.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            assert_eq!(others, 0, "two calls at once");
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                            Ok(Instant::now())
                        })
                        .await
                        .unwrap()
                })
            })
            .collect();

        let mut ends = Vec::new();
        for task in tasks {
            ends.push(task.await.unwrap() - start);
        }
        ends.sort();
        // one token at first, then one per second
        assert!(ends[0] < Duration::from_secs(1));
        assert!(ends[1] >= Duration::from_secs(1) && ends[2] >= Duration::from_secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn logs_the_waits_and_the_failures() {
        let logs = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = logs.clone();
        let logger: Logger = Arc::new(move |tag: &str, message: &str| {
            sink.lock().unwrap().push(format!("{tag} {message}"));
        });
        let limiter = RateLimiter::new(1, Duration::from_secs(1), Some(logger));

        limiter
            .enqueue("/0/public/Time", async { Ok(()) })
            .await
            .unwrap();
        let _ = limiter
            .enqueue::<(), _>("/0/public/Time", async {
                Err(KrakenError::Transport {
                    reason: "offline".into(),
                })
            })
            .await;

        let logs = logs.lock().unwrap();
        assert!(
            logs.iter().any(|it| it.contains("No token available")),
            "{logs:?}"
        );
        assert!(logs.iter().any(|it| it.contains("offline")), "{logs:?}");
        assert!(
            logs.iter().all(|it| it.starts_with("[RateLimitQueue]")),
            "{logs:?}"
        );
    }

    #[test]
    fn a_zero_interval_still_refills() {
        let limiter = RateLimiter::new(1, Duration::ZERO, None);
        assert_eq!(limiter.token_interval, Duration::from_millis(1));
    }
}
