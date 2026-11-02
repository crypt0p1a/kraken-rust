//! Helpers of the integration tests: a mocked Kraken server checking the requests like Kraken does

#![allow(dead_code)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use kraken_api::{PrivateApi, PublicApi, RateLimiter, RestOptions};
use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use std::time::Duration;
use wiremock::{Match, MockServer, Request, ResponseTemplate};

pub const API_KEY: &str = "integration-test-key";
/// Example secret of https://docs.kraken.com/api/docs/guides/spot-rest-auth
pub const API_SECRET: &str =
    "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==";

fn options(server: &MockServer) -> RestOptions {
    RestOptions {
        host: server.uri(),
        timeout_ms: 5_000,
        ..RestOptions::default()
    }
}

/// No waiting between the calls of the tests
fn rate_limiter() -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(1_000, Duration::from_millis(1), None))
}

pub fn public_api(server: &MockServer) -> PublicApi {
    PublicApi::new(options(server), rate_limiter()).unwrap()
}

pub fn private_api(server: &MockServer) -> PrivateApi {
    PrivateApi::new(
        API_KEY.into(),
        API_SECRET.into(),
        options(server),
        rate_limiter(),
    )
    .unwrap()
}

/// `{"error": [], "result": result}`
pub fn ok(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "error": [], "result": result }))
}

/// The form parameters of a request body
pub fn form(request: &Request) -> Vec<(String, String)> {
    serde_urlencoded::from_bytes(&request.body).unwrap_or_default()
}

/// A private call like Kraken accepts it: form encoded body starting with the nonce, API-Key and
/// a valid API-Sign
pub struct Signed;

impl Match for Signed {
    fn matches(&self, request: &Request) -> bool {
        let header = |name: &str| {
            request
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };

        let Ok(body) = std::str::from_utf8(&request.body) else {
            return false;
        };
        let Some(nonce) = body
            .strip_prefix("nonce=")
            .map(|rest| rest.split('&').next().unwrap_or_default())
        else {
            return false;
        };

        let digest = Sha256::digest(format!("{nonce}{body}").as_bytes());
        let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
        mac.update(request.url.path().as_bytes());
        mac.update(&digest);
        let expected = BASE64.encode(mac.finalize().into_bytes());

        header("API-Key").as_deref() == Some(API_KEY)
            && header("API-Sign") == Some(expected)
            && header("Content-Type").as_deref() == Some("application/x-www-form-urlencoded")
    }
}

/// A private call with a JSON body like Kraken accepts it: the nonce in the body, API-Key and a
/// valid API-Sign over the exact bytes sent
pub struct SignedJson;

impl Match for SignedJson {
    fn matches(&self, request: &Request) -> bool {
        let header = |name: &str| {
            request
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };
        let Ok(body) = serde_json::from_slice::<Value>(&request.body) else {
            return false;
        };
        let Some(nonce) = body["nonce"].as_u64() else {
            return false;
        };

        let digest = Sha256::digest([nonce.to_string().as_bytes(), &request.body].concat());
        let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
        mac.update(request.url.path().as_bytes());
        mac.update(&digest);
        let expected = BASE64.encode(mac.finalize().into_bytes());

        header("API-Key").as_deref() == Some(API_KEY)
            && header("API-Sign") == Some(expected)
            && header("Content-Type").as_deref() == Some("application/json")
    }
}

/// The JSON body is exactly this object, besides the nonce
pub struct JsonIs(pub Value);

impl Match for JsonIs {
    fn matches(&self, request: &Request) -> bool {
        let Ok(Value::Object(mut body)) = serde_json::from_slice::<Value>(&request.body) else {
            return false;
        };
        body.remove("nonce");
        Value::Object(body) == self.0
    }
}

/// A call of the funding API like Kraken accepts it: API-Key, API-Nonce and an API-Sign over the
/// path, its query string and the exact body sent
pub struct SignedFunding;

impl Match for SignedFunding {
    fn matches(&self, request: &Request) -> bool {
        let header = |name: &str| {
            request
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };
        let Some(nonce) = header("API-Nonce") else {
            return false;
        };
        let signed_path = match request.url.query() {
            Some(query) => format!("{}?{query}", request.url.path()),
            None => request.url.path().to_string(),
        };
        let digest = Sha256::digest([nonce.as_bytes(), &request.body].concat());
        let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
        mac.update(signed_path.as_bytes());
        mac.update(&digest);
        let expected = BASE64.encode(mac.finalize().into_bytes());

        header("API-Key").as_deref() == Some(API_KEY) && header("API-Sign") == Some(expected)
    }
}

/// An answer of the funding API: no envelope
pub fn plain(answer: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(answer)
}

/// The body holds exactly these parameters, besides the nonce
pub struct FormIs(pub Vec<(&'static str, &'static str)>);

impl Match for FormIs {
    fn matches(&self, request: &Request) -> bool {
        let mut actual: Vec<(String, String)> = form(request)
            .into_iter()
            .filter(|(key, _)| key != "nonce")
            .collect();
        let mut expected: Vec<(String, String)> = self
            .0
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        actual.sort();
        expected.sort();
        actual == expected
    }
}

/// The query string is exactly these parameters
pub struct QueryIs(pub Vec<(&'static str, &'static str)>);

impl Match for QueryIs {
    fn matches(&self, request: &Request) -> bool {
        let mut actual: Vec<(String, String)> = request
            .url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        let mut expected: Vec<(String, String)> = self
            .0
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        actual.sort();
        expected.sort();
        actual == expected
    }
}

/// Orders as answered by Kraken, keyed by txid
pub fn orders_fixture() -> Value {
    json!({
        "OQCLML-BW3P3-BUCMWZ": {
            "refid": null, "userref": 0, "cl_ord_id": "mgr1_buy_1", "status": "open",
            "opentm": 1688666559.8974, "starttm": 0, "expiretm": 0,
            "descr": {"pair": "XBTUSD", "type": "buy", "ordertype": "limit", "price": "30010.0",
                "price2": "0", "leverage": "none",
                "order": "buy 1.25000000 XBTUSD @ limit 30010.0", "close": ""},
            "vol": "1.25000000", "vol_exec": "0.37500000", "cost": "11253.7", "fee": "0.00000",
            "price": "30010.0", "stopprice": "0.00000", "limitprice": "0.00000", "misc": "",
            "oflags": "fciq", "trades": ["TCCCTY-WE2O6-P3NB37"]
        },
        "OB5VMB-B4U2U-DK2WRW": {
            "refid": null, "userref": 45, "status": "closed", "reason": null,
            "opentm": 1688665496.7808, "closetm": 1688665499.1922, "starttm": 0, "expiretm": 0,
            "descr": {"pair": "ETHUSD", "type": "sell", "ordertype": "market", "price": "0",
                "price2": "0", "leverage": "none", "order": "sell 0.25000000 ETHUSD @ market",
                "close": ""},
            "vol": "0.25000000", "vol_exec": "0.25000000", "cost": "7500.2", "fee": "19.5",
            "price": "30000.8", "stopprice": "0.00000", "limitprice": "0.00000", "misc": "",
            "oflags": "fcib"
        }
    })
}
