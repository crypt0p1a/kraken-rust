use super::http::HttpClient;
use crate::nonce::next_nonce;
use crate::params::encode;
use crate::{KrakenError, RateLimiter, RestOptions};
use kraken_crypto::ApiSecret;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// The HTTP methods of the funding API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FundingMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl FundingMethod {
    fn reqwest(self) -> reqwest::Method {
        match self {
            FundingMethod::Get => reqwest::Method::GET,
            FundingMethod::Post => reqwest::Method::POST,
            FundingMethod::Put => reqwest::Method::PUT,
            FundingMethod::Delete => reqwest::Method::DELETE,
        }
    }
}

/// Calls of the funding API, `/funding/v1/*` and `/funding/v2/*`: unlike `/0/private`, signed
/// GET, POST, PUT and DELETE, the nonce in the `API-Nonce` header, a JSON body, and plain JSON
/// answers without the `{"error", "result"}` envelope
///
/// `API-Sign` = base64(HMAC-SHA512(path + "?" + query + SHA256(nonce + body), secret)): the query
/// string and the body are signed as they are sent
pub struct FundingClient {
    inner: HttpClient,
    api_key: String,
    secret: ApiSecret,
}

impl FundingClient {
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

    /// Calls `path` (e.g. "/funding/v1/withdrawals") with the `query` parameters and the JSON
    /// `body`, returns its answer
    pub async fn call<Q, B, O>(
        &self,
        method: FundingMethod,
        path: &str,
        query: Option<&Q>,
        body: Option<&B>,
    ) -> Result<O, KrakenError>
    where
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
        O: DeserializeOwned,
    {
        let query = query.map(encode).transpose()?.unwrap_or_default();
        let signed_path = match query.is_empty() {
            true => path.to_string(),
            false => format!("{path}?{query}"),
        };
        let url = format!("{}{signed_path}", self.inner.host);
        let body = body
            .map(serde_json::to_string)
            .transpose()
            .map_err(KrakenError::decode)?;

        let (status, answer) = self
            .inner
            .send_bytes(path, || {
                // created in the queue: the calls are sent in the order of their nonces
                let nonce = next_nonce();
                let signature =
                    self.secret
                        .sign(&signed_path, nonce, body.as_deref().unwrap_or_default());
                let request = self
                    .inner
                    .http
                    .request(method.reqwest(), &url)
                    .header("API-Key", &self.api_key)
                    .header("API-Nonce", nonce.to_string())
                    .header("API-Sign", signature);
                match &body {
                    Some(body) => request
                        .header(reqwest::header::CONTENT_TYPE, "application/json")
                        .body(body.clone()),
                    None => request,
                }
            })
            .await?;

        decode(status, &answer)
    }
}

/// A 2xx answer is the result itself, an empty one being `null`. The format of the errors isn't
/// documented: their body is kept as it is
fn decode<O: DeserializeOwned>(status: u16, answer: &[u8]) -> Result<O, KrakenError> {
    let text = String::from_utf8_lossy(answer);
    if !(200..300).contains(&status) {
        return Err(match serde_json::from_slice::<serde_json::Value>(answer) {
            Ok(_) => KrakenError::Api {
                status,
                body: text.into_owned(),
            },
            Err(_) => KrakenError::Network {
                status,
                body: text.into_owned(),
            },
        });
    }
    let text = if text.trim().is_empty() {
        "null"
    } else {
        &text
    };
    serde_json::from_str(text).map_err(KrakenError::decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn decodes_the_plain_answers() {
        let value: Value = decode(200, br#"{"withdrawals":[]}"#).unwrap();
        assert_eq!(value, json!({"withdrawals": []}));
        // no content
        let unit: Option<Value> = decode(204, b"").unwrap();
        assert_eq!(unit, None);
    }

    #[test]
    fn maps_the_failures() {
        assert_eq!(
            decode::<Value>(400, br#"{"error":"TooManyDepositAddresses"}"#),
            Err(KrakenError::Api {
                status: 400,
                body: r#"{"error":"TooManyDepositAddresses"}"#.into()
            })
        );
        assert_eq!(
            decode::<Value>(502, b"Bad Gateway"),
            Err(KrakenError::Network {
                status: 502,
                body: "Bad Gateway".into()
            })
        );
        assert!(matches!(
            decode::<Vec<u8>>(200, br#"{"a":1}"#),
            Err(KrakenError::Decode { .. })
        ));
    }
}
