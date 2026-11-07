//! UniFFI export of the Kraken Spot REST API
//!
//! The models and [`KrakenError`](kraken_network::KrakenError) are exported by `kraken-spot-api` and
//! `kraken-network` (their `uniffi` feature), the APIs are wrapped here as objects with async
//! methods. Same shape as the `ffi` module of `cex-spot-trading`.

uniffi::setup_scaffolding!("kraken_ffi");

mod private;
mod public;

pub use private::KrakenPrivateApi;
pub use public::KrakenPublicApi;

use kraken_network::{Logger, RateLimiter, RestOptions};
use std::sync::Arc;
use std::time::Duration;

/// Receives the logs of the rate limiter
#[uniffi::export(with_foreign)]
pub trait KrakenLogger: Send + Sync {
    fn log(&self, tag: String, message: String);
}

/// The default options of the HTTP client
#[uniffi::export]
pub fn default_rest_options() -> RestOptions {
    RestOptions::default()
}

/// Token bucket shared by the APIs given to their constructor
#[derive(uniffi::Object)]
pub struct KrakenRateLimiter {
    pub(crate) inner: Arc<RateLimiter>,
}

#[uniffi::export]
impl KrakenRateLimiter {
    /// `pool_maximum` tokens at most, a new one every `token_interval_ms`
    #[uniffi::constructor]
    pub fn new(
        pool_maximum: u32,
        token_interval_ms: u64,
        logger: Option<Arc<dyn KrakenLogger>>,
    ) -> Arc<Self> {
        let logger = logger.map(|logger| -> Logger {
            Arc::new(move |tag: &str, message: &str| logger.log(tag.into(), message.into()))
        });

        Arc::new(Self {
            inner: Arc::new(RateLimiter::new(
                pool_maximum,
                Duration::from_millis(token_interval_ms),
                logger,
            )),
        })
    }
}
