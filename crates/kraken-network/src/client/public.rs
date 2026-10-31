use super::http::HttpClient;
use crate::envelope::decode;
use crate::params::encode;
use crate::{KrakenError, RateLimiter, RestOptions};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Calls of `/0/public/*`: no credentials, parameters in the query string
pub struct PublicClient {
    inner: HttpClient,
}

impl PublicClient {
    pub fn new(options: RestOptions, rate_limiter: Arc<RateLimiter>) -> Result<Self, KrakenError> {
        Ok(Self {
            inner: HttpClient::new(options, rate_limiter)?,
        })
    }

    /// Calls `method` without parameters, returns its `result`
    pub async fn call<O: DeserializeOwned>(&self, method: &str) -> Result<O, KrakenError> {
        self.call_with::<(), O>(method, None).await
    }

    /// Calls `method` with `params` in the query string, returns its `result`
    pub async fn call_with<I: Serialize + ?Sized, O: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&I>,
    ) -> Result<O, KrakenError> {
        let path = format!("/0/public/{method}");
        let query = params.map(encode).transpose()?.unwrap_or_default();
        let url = match query.is_empty() {
            true => format!("{}{path}", self.inner.host),
            false => format!("{}{path}?{query}", self.inner.host),
        };

        let (status, text) = self.inner.send(&path, || self.inner.http.get(&url)).await?;

        decode(status, &text)
    }
}
