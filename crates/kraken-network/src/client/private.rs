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
const JSON: &str = "application/json";

/// The parameters of a call, the nonce being added in the queue of the rate limiter
enum Body {
    /// `nonce=...&params`
    Form(String),
    /// `{"nonce": ..., params}`: the parameters a form can't hold (lists of objects)
    Json(serde_json::Map<String, serde_json::Value>),
}

impl Body {
    /// The content type and the body sent, nonce included
    fn with_nonce(&self, nonce: u64) -> Result<(&'static str, String), KrakenError> {
        Ok(match self {
            Body::Form(params) if params.is_empty() => (FORM, format!("nonce={nonce}")),
            Body::Form(params) => (FORM, format!("nonce={nonce}&{params}")),
            Body::Json(params) => {
                let mut body = serde_json::Map::new();
                body.insert("nonce".into(), nonce.into());
                body.extend(params.clone());
                (
                    JSON,
                    serde_json::to_string(&body).map_err(KrakenError::decode)?,
                )
            }
        })
    }
}

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
        let params = params.map(encode).transpose()?.unwrap_or_default();
        let (status, body) = self.send(method, Body::Form(params)).await?;
        decode(status, &String::from_utf8_lossy(&body))
    }

    /// Calls `method` with `params` in a signed JSON body, returns its `result`: the parameters a
    /// form can't hold, like lists of objects. `params` must serialize to an object
    pub async fn call_json<I: Serialize + ?Sized, O: DeserializeOwned>(
        &self,
        method: &str,
        params: &I,
    ) -> Result<O, KrakenError> {
        let params = match serde_json::to_value(params).map_err(KrakenError::decode)? {
            serde_json::Value::Object(params) => params,
            other => {
                return Err(KrakenError::InvalidArgument {
                    reason: format!("the JSON parameters must be an object, got {other}"),
                });
            }
        };
        let (status, body) = self.send(method, Body::Json(params)).await?;
        decode(status, &String::from_utf8_lossy(&body))
    }

    /// Calls `method` answering a file (e.g. the zip of RetrieveExport), returns its bytes. Its
    /// errors are still answered in JSON: `{"error": [...]}`
    pub async fn call_bytes<I: Serialize + ?Sized>(
        &self,
        method: &str,
        params: Option<&I>,
    ) -> Result<Vec<u8>, KrakenError> {
        let params = params.map(encode).transpose()?.unwrap_or_default();
        let (status, body) = self.send(method, Body::Form(params)).await?;
        if status != 200 || body.first() == Some(&b'{') {
            let text = String::from_utf8_lossy(&body);
            // an error, or JSON where a file was expected
            decode::<serde_json::Value>(status, &text)?;
            return Err(KrakenError::Decode {
                reason: format!("expected a file, got {text}"),
            });
        }
        Ok(body)
    }

    /// The signed request: the status and the body of the answer
    async fn send(&self, method: &str, params: Body) -> Result<(u16, Vec<u8>), KrakenError> {
        let path = format!("/0/private/{method}");
        let url = format!("{}{path}", self.inner.host);
        // the nonce of a JSON body can't fail to serialize: checked before the queue
        params.with_nonce(0)?;

        self.inner
            .send_bytes(&path, || {
                // created in the queue: the calls are sent in the order of their nonces
                let nonce = next_nonce();
                let (content_type, body) = params
                    .with_nonce(nonce)
                    .expect("serializable, checked before the queue");

                self.inner
                    .http
                    .post(&url)
                    .header(reqwest::header::CONTENT_TYPE, content_type)
                    .header("API-Key", &self.api_key)
                    .header("API-Sign", self.secret.sign(&path, nonce, &body))
                    .body(body)
            })
            .await
    }
}
