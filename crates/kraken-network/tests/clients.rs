//! The clients against a mocked Kraken server: requests sent like Kraken expects them

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use kraken_network::{KrakenError, PrivateClient, PublicClient, RateLimiter, RestOptions};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const API_KEY: &str = "network-test-key";
const API_SECRET: &str = "c2VjcmV0LW9mLXRoZS1uZXR3b3JrLXRlc3Rz";

fn options(server: &MockServer) -> RestOptions {
    RestOptions {
        host: server.uri(),
        ..RestOptions::default()
    }
}

fn rate_limiter() -> Arc<RateLimiter> {
    Arc::new(RateLimiter::new(1_000, Duration::from_millis(1), None))
}

fn public_client(server: &MockServer) -> PublicClient {
    PublicClient::new(options(server), rate_limiter()).unwrap()
}

fn private_client(server: &MockServer) -> PrivateClient {
    PrivateClient::new(API_KEY.into(), API_SECRET, options(server), rate_limiter()).unwrap()
}

fn ok(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "error": [], "result": result }))
}

/// Recomputes the signature, independently of kraken-crypto
fn expected_signature(request: &Request) -> String {
    let body = std::str::from_utf8(&request.body).unwrap();
    let nonce = body
        .strip_prefix("nonce=")
        .and_then(|rest| rest.split('&').next())
        .unwrap();

    let digest = Sha256::digest(format!("{nonce}{body}").as_bytes());
    let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
    mac.update(request.url.path().as_bytes());
    mac.update(&digest);
    BASE64.encode(mac.finalize().into_bytes())
}

fn nonce(request: &Request) -> u64 {
    let body = std::str::from_utf8(&request.body).unwrap();
    body.strip_prefix("nonce=")
        .and_then(|rest| rest.split('&').next())
        .and_then(|nonce| nonce.parse().ok())
        .unwrap()
}

#[derive(Serialize)]
struct Params {
    pair: &'static str,
    count: u32,
}

#[tokio::test]
async fn public_calls_are_gets_with_a_query_string() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .and(query_param("pair", "XBTUSD"))
        .and(query_param("count", "2"))
        .respond_with(ok(json!({"value": 42})))
        .expect(1)
        .mount(&server)
        .await;

    let result: Value = public_client(&server)
        .call_with(
            "Ticker",
            Some(&Params {
                pair: "XBTUSD",
                count: 2,
            }),
        )
        .await
        .unwrap();
    assert_eq!(result, json!({"value": 42}));
}

#[tokio::test]
async fn public_calls_without_parameters_have_no_query_string() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Time"))
        .respond_with(ok(json!(1)))
        .expect(1)
        .mount(&server)
        .await;

    let _: Value = public_client(&server).call("Time").await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn private_calls_are_signed_posts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/AddOrder"))
        .and(header("API-Key", API_KEY))
        .and(header("Content-Type", "application/x-www-form-urlencoded"))
        .respond_with(ok(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let _: Value = private_client(&server)
        .call_with(
            "AddOrder",
            Some(&Params {
                pair: "XBTUSD",
                count: 2,
            }),
        )
        .await
        .unwrap();

    let request = &server.received_requests().await.unwrap()[0];
    let body = std::str::from_utf8(&request.body).unwrap();
    assert_eq!(
        body,
        format!("nonce={}&pair=XBTUSD&count=2", nonce(request))
    );
    assert_eq!(
        request.headers.get("API-Sign").unwrap().to_str().unwrap(),
        expected_signature(request)
    );
}

#[tokio::test]
async fn private_calls_without_parameters_only_send_the_nonce() {
    let server = MockServer::start().await;
    Mock::given(path("/0/private/BalanceEx"))
        .respond_with(ok(json!({})))
        .expect(5)
        .mount(&server)
        .await;

    let client = private_client(&server);
    for _ in 0..5 {
        let _: Value = client.call("BalanceEx").await.unwrap();
    }

    let requests = server.received_requests().await.unwrap();
    let nonces: Vec<u64> = requests.iter().map(nonce).collect();
    assert!(
        nonces.windows(2).all(|pair| pair[0] < pair[1]),
        "{nonces:?}"
    );

    for request in &requests {
        assert_eq!(
            request.body,
            format!("nonce={}", nonce(request)).into_bytes()
        );
        assert_eq!(
            request.headers.get("API-Sign").unwrap().to_str().unwrap(),
            expected_signature(request)
        );
    }
}

#[tokio::test]
async fn maps_the_failures() {
    let server = MockServer::start().await;
    Mock::given(path("/0/public/Errors"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"error": ["EAPI:Invalid key", "EGeneral:Too many requests"]}),
            ),
        )
        .mount(&server)
        .await;
    Mock::given(path("/0/public/Down"))
        .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
        .mount(&server)
        .await;
    Mock::given(path("/0/public/Garbage"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;
    Mock::given(path("/0/public/Empty"))
        .and(body_string(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"error": []})))
        .mount(&server)
        .await;

    let client = public_client(&server);
    assert_eq!(
        client.call::<Value>("Errors").await,
        Err(KrakenError::Api {
            status: 200,
            body: "EAPI:Invalid key; EGeneral:Too many requests".into()
        })
    );
    assert_eq!(
        client.call::<Value>("Down").await,
        Err(KrakenError::Network {
            status: 503,
            body: "unavailable".into()
        })
    );
    assert!(matches!(
        client.call::<Value>("Garbage").await,
        Err(KrakenError::Decode { .. })
    ));
    assert!(matches!(
        client.call::<Value>("Empty").await,
        Err(KrakenError::Api { .. })
    ));
}

#[tokio::test]
async fn maps_the_unreachable_server() {
    let client = PublicClient::new(
        RestOptions {
            host: "http://127.0.0.1:1".into(),
            timeout_ms: 1_000,
            ..RestOptions::default()
        },
        rate_limiter(),
    )
    .unwrap();

    assert!(matches!(
        client.call::<Value>("Time").await,
        Err(KrakenError::Transport { .. })
    ));
}

#[test]
fn rejects_an_invalid_secret() {
    let client = PrivateClient::new(
        API_KEY.into(),
        "not base64!",
        RestOptions::default(),
        rate_limiter(),
    );
    assert!(matches!(client, Err(KrakenError::InvalidArgument { .. })));
}
