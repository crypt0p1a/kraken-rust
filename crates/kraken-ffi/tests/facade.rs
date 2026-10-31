//! The UniFFI objects, called like the foreign bindings do, against a mocked Kraken

use kraken_ffi::{
    KrakenLogger, KrakenPrivateApi, KrakenPublicApi, KrakenRateLimiter, default_rest_options,
};
use kraken_network::{KrakenError, RestOptions};
use serde_json::json;
use std::sync::{Arc, Mutex};
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A logger implemented on the foreign side
#[derive(Default)]
struct Recorder {
    logs: Mutex<Vec<(String, String)>>,
}

impl KrakenLogger for Recorder {
    fn log(&self, tag: String, message: String) {
        self.logs.lock().unwrap().push((tag, message));
    }
}

fn options(server: &MockServer) -> RestOptions {
    RestOptions {
        host: server.uri(),
        ..default_rest_options()
    }
}

fn ok(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "error": [], "result": result }))
}

#[test]
fn exposes_the_default_options() {
    let options = default_rest_options();
    assert_eq!(options.host, "https://api.kraken.com");
    assert_eq!(options.timeout_ms, 30_000);
    assert!(options.reject_unauthorized);
}

#[tokio::test]
async fn the_public_api_calls_kraken_through_the_shared_rate_limiter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Time"))
        .respond_with(ok(
            json!({"unixtime": 1, "rfc1123": "Thu, 01 Jan 70 00:00:01 +0000"}),
        ))
        .expect(3)
        .mount(&server)
        .await;

    let recorder = Arc::new(Recorder::default());
    // a single token: the 2nd and 3rd calls wait for it, which is logged
    let limiter = KrakenRateLimiter::new(1, 1, Some(recorder.clone()));
    let api = KrakenPublicApi::new(options(&server), limiter).unwrap();

    for _ in 0..3 {
        assert_eq!(api.server_time().await.unwrap().unix_time, 1);
    }
    let logs = recorder.logs.lock().unwrap();
    assert!(
        logs.iter().all(|(tag, _)| tag == "[RateLimitQueue]"),
        "{logs:?}"
    );
    assert!(
        logs.iter()
            .any(|(_, message)| message.contains("No token available")),
        "{logs:?}"
    );
}

#[tokio::test]
async fn the_public_api_maps_the_errors() {
    let server = MockServer::start().await;
    Mock::given(path("/0/public/Ticker"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"error": ["EQuery:Unknown asset pair"]})),
        )
        .mount(&server)
        .await;

    let api = KrakenPublicApi::new(options(&server), KrakenRateLimiter::new(10, 1, None)).unwrap();
    assert_eq!(
        api.tickers(vec!["NOPEUSD".into()]).await,
        Err(KrakenError::Api {
            status: 200,
            body: "EQuery:Unknown asset pair".into()
        })
    );
}

#[tokio::test]
async fn the_private_api_signs_its_calls() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/BalanceEx"))
        .and(header_exists("API-Sign"))
        .respond_with(ok(json!({"ZEUR": {"balance": "10", "hold_trade": "4"}})))
        .expect(1)
        .mount(&server)
        .await;

    let api = KrakenPrivateApi::new(
        "key".into(),
        "c2VjcmV0".into(),
        options(&server),
        KrakenRateLimiter::new(10, 1, None),
    )
    .unwrap();

    let balances = api.extended_balance().await.unwrap();
    assert_eq!(balances["ZEUR"].available(), 6.0);
}

#[test]
fn the_private_api_refuses_an_invalid_secret() {
    let api = KrakenPrivateApi::new(
        "key".into(),
        "not base64!".into(),
        default_rest_options(),
        KrakenRateLimiter::new(10, 1, None),
    );
    assert!(matches!(api, Err(KrakenError::InvalidArgument { .. })));
}
