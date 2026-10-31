use super::http::HttpClient;
use crate::envelope::decode;
use crate::nonce::next_nonce;
use crate::params::encode;
use crate::{KrakenError, RateLimiter, RestOptions};
use kraken_crypto::ApiSecret;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

const FORM: &str = "application/x-www-form-urlencoded";

/// Calls of `/0/private/*`: form encoded body starting with the nonce, signed with the API secret
pub struct PrivateClient {
    inner: HttpClient,
    api_key: String,
    secret: ApiSecret,
}

impl PrivateClient {
    /// `api_secret` is the base64 secret given by Kraken with the key
    pub fn new(
        api_key: String,
        api_secret: &str,
        options: RestOptions,
        rate_limiter: Arc<RateLimiter>,
    ) -> Result<Self, KrakenError> {
        Ok(Self {
            secret: ApiSecret::from_base64(api_secret)?,
            inner: HttpClient::new(options, rate_limiter)?,
            api_key,
        })
    }

    /// Calls `method` without parameters, returns its `result`
    pub async fn call<O: DeserializeOwned>(&self, method: &str) -> Result<O, KrakenError> {
        self.call_with::<(), O>(method, None).await
    }

    /// Calls `method` with `params` in the signed body, returns its `result`
    pub async fn call_with<I: Serialize + ?Sized, O: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&I>,
    ) -> Result<O, KrakenError> {
        let path = format!("/0/private/{method}");
        let url = format!("{}{path}", self.inner.host);
        let params = params.map(encode).transpose()?.unwrap_or_default();

        let (status, text) = self
            .inner
            .send(&path, || {
                // created in the queue: the calls are sent in the order of their nonces
                let nonce = next_nonce();
                let body = match params.is_empty() {
                    true => format!("nonce={nonce}"),
                    false => format!("nonce={nonce}&{params}"),
                };

                self.inner
                    .http
                    .post(&url)
                    .header(reqwest::header::CONTENT_TYPE, FORM)
                    .header("API-Key", &self.api_key)
                    .header("API-Sign", self.secret.sign(&path, nonce, &body))
                    .body(body)
            })
            .await?;

        decode(status, &text)
    }
}
