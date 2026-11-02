use crate::{KrakenError, RateLimiter, RestOptions};
use std::sync::Arc;
use std::time::Duration;

const USER_AGENT: &str = "Kraken Spot Trading Rust Client";

/// HTTP client, base URL and rate limiter shared by the public and private clients
pub(super) struct HttpClient {
    pub http: reqwest::Client,
    pub host: String,
    pub rate_limiter: Arc<RateLimiter>,
}

impl HttpClient {
    pub fn new(options: RestOptions, rate_limiter: Arc<RateLimiter>) -> Result<Self, KrakenError> {
        let timeout = Duration::from_millis(options.timeout_ms);
        let http = reqwest::Client::builder()
            .connect_timeout(timeout)
            .read_timeout(timeout)
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .danger_accept_invalid_certs(!options.reject_unauthorized)
            .build()?;

        Ok(Self {
            http,
            host: options.host.trim_end_matches('/').to_string(),
            rate_limiter,
        })
    }

    /// Sends the request in the queue of the rate limiter, returns the status and body
    pub async fn send(
        &self,
        tag: &str,
        request: impl FnOnce() -> reqwest::RequestBuilder,
    ) -> Result<(u16, String), KrakenError> {
        let (status, body) = self.send_bytes(tag, request).await?;
        Ok((status, String::from_utf8_lossy(&body).into_owned()))
    }

    /// [`HttpClient::send`], the body as it was received: the binary answers (zip reports)
    pub async fn send_bytes(
        &self,
        tag: &str,
        request: impl FnOnce() -> reqwest::RequestBuilder,
    ) -> Result<(u16, Vec<u8>), KrakenError> {
        self.rate_limiter
            .enqueue(tag, async {
                let response = request().send().await?;
                let status = response.status().as_u16();
                Ok((status, response.bytes().await?.to_vec()))
            })
            .await
    }
}
