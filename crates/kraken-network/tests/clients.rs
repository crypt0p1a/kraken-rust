//! The clients against a mocked Kraken server: requests sent like Kraken expects them

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use kraken_network::{
    FundingClient, FundingMethod, KrakenError, PrivateClient, PublicClient, RateLimiter,
    RestOptions,
};
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

#[derive(Serialize)]
struct ReportId {
    id: &'static str,
}

#[tokio::test]
async fn private_files_are_returned_byte_for_byte() {
    // a zip header, then bytes which aren't UTF-8
    let file: Vec<u8> = vec![0x50, 0x4b, 0x03, 0x04, 0xff, 0xfe, 0x00, 0x80];
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/RetrieveExport"))
        .and(header("API-Key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(file.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let bytes = private_client(&server)
        .call_bytes("RetrieveExport", Some(&ReportId { id: "TCJA" }))
        .await
        .unwrap();
    assert_eq!(bytes, file);

    let request = &server.received_requests().await.unwrap()[0];
    let body = std::str::from_utf8(&request.body).unwrap();
    assert_eq!(body, format!("nonce={}&id=TCJA", nonce(request)));
    assert_eq!(
        request.headers.get("API-Sign").unwrap().to_str().unwrap(),
        expected_signature(request)
    );
}

#[tokio::test]
async fn private_files_map_the_failures() {
    let server = MockServer::start().await;
    let respond = |template: ResponseTemplate| {
        Mock::given(method("POST"))
            .and(path("/0/private/RetrieveExport"))
            .respond_with(template)
            .up_to_n_times(1)
    };
    let call = || async {
        private_client(&server)
            .call_bytes("RetrieveExport", Some(&ReportId { id: "TCJA" }))
            .await
    };

    respond(ResponseTemplate::new(200).set_body_json(json!({"error": ["EExport:Unknown export"]})))
        .mount(&server)
        .await;
    assert_eq!(
        call().await,
        Err(KrakenError::Api {
            status: 200,
            body: "EExport:Unknown export".into()
        })
    );
    server.reset().await;

    respond(ResponseTemplate::new(503).set_body_string("unavailable"))
        .mount(&server)
        .await;
    assert_eq!(
        call().await,
        Err(KrakenError::Network {
            status: 503,
            body: "unavailable".into()
        })
    );
    server.reset().await;

    // JSON where a file was expected
    respond(ok(json!({"id": "TCJA"}))).mount(&server).await;
    assert!(matches!(call().await, Err(KrakenError::Decode { .. })));
}

#[derive(Serialize)]
struct Batch {
    orders: Vec<&'static str>,
    cl_ord_ids: Vec<&'static str>,
}

#[tokio::test]
async fn private_json_calls_sign_their_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelOrderBatch"))
        .and(header("API-Key", API_KEY))
        .and(header("Content-Type", "application/json"))
        .respond_with(ok(json!({"count": 2})))
        .expect(1)
        .mount(&server)
        .await;

    let answer: Value = private_client(&server)
        .call_json(
            "CancelOrderBatch",
            &Batch {
                orders: vec!["OG5V2Y-RYKVL-DT3V3B", "OP5V2Y-RYKVL-ET3V3B"],
                cl_ord_ids: vec!["mgr1 buy"],
            },
        )
        .await
        .unwrap();
    assert_eq!(answer, json!({"count": 2}));

    let request = &server.received_requests().await.unwrap()[0];
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    let nonce = body["nonce"].as_u64().unwrap();
    assert_eq!(
        body,
        json!({"nonce": nonce, "orders": ["OG5V2Y-RYKVL-DT3V3B", "OP5V2Y-RYKVL-ET3V3B"],
            "cl_ord_ids": ["mgr1 buy"]})
    );
    // API-Sign over the exact bytes sent: path + SHA256(nonce + body)
    let digest = Sha256::digest([nonce.to_string().as_bytes(), &request.body].concat());
    let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
    mac.update(request.url.path().as_bytes());
    mac.update(&digest);
    assert_eq!(
        request.headers.get("API-Sign").unwrap().to_str().unwrap(),
        BASE64.encode(mac.finalize().into_bytes())
    );
}

#[tokio::test]
async fn private_json_calls_take_an_object() {
    let server = MockServer::start().await;
    let answer = private_client(&server)
        .call_json::<_, Value>("CancelOrderBatch", &vec!["OG5V2Y-RYKVL-DT3V3B"])
        .await;
    assert!(matches!(answer, Err(KrakenError::InvalidArgument { .. })));
    assert!(server.received_requests().await.unwrap().is_empty());
}

fn funding_client(server: &MockServer) -> FundingClient {
    FundingClient::new(API_KEY.into(), API_SECRET, options(server), rate_limiter()).unwrap()
}

/// API-Sign of the funding API: the path with its query string, then SHA256(nonce + body)
fn funding_signature(request: &Request) -> String {
    let nonce = request.headers.get("API-Nonce").unwrap().to_str().unwrap();
    let signed_path = match request.url.query() {
        Some(query) => format!("{}?{query}", request.url.path()),
        None => request.url.path().to_string(),
    };
    let digest = Sha256::digest([nonce.as_bytes(), &request.body].concat());
    let mut mac = Hmac::<Sha512>::new_from_slice(&BASE64.decode(API_SECRET).unwrap()).unwrap();
    mac.update(signed_path.as_bytes());
    mac.update(&digest);
    BASE64.encode(mac.finalize().into_bytes())
}

#[derive(Serialize)]
struct FundingQuery {
    #[serde(rename = "scope[method_id]")]
    method_id: &'static str,
    limit: u32,
}

#[tokio::test]
async fn funding_gets_sign_their_query_string() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/addresses"))
        .and(query_param("scope[method_id]", "27ede8db"))
        .and(query_param("limit", "20"))
        .and(header("API-Key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"addresses": []})))
        .expect(1)
        .mount(&server)
        .await;

    let answer: Value = funding_client(&server)
        .call(
            FundingMethod::Get,
            "/funding/v1/addresses",
            Some(&FundingQuery {
                method_id: "27ede8db",
                limit: 20,
            }),
            None::<&()>,
        )
        .await
        .unwrap();
    assert_eq!(answer, json!({"addresses": []}));

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        request.url.query(),
        Some("scope%5Bmethod_id%5D=27ede8db&limit=20")
    );
    assert!(request.body.is_empty());
    assert!(request.headers.get("Content-Type").is_none());
    assert!(request.headers.get("API-Nonce").is_some());
    assert_eq!(
        request.headers.get("API-Sign").unwrap().to_str().unwrap(),
        funding_signature(request)
    );
}

#[tokio::test]
async fn funding_writes_sign_their_json_body() {
    let server = MockServer::start().await;
    for (verb, method_name) in [
        (FundingMethod::Post, "POST"),
        (FundingMethod::Put, "PUT"),
        (FundingMethod::Delete, "DELETE"),
    ] {
        Mock::given(method(method_name))
            .and(path("/funding/v1/addresses"))
            .and(header("Content-Type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": method_name})))
            .expect(1)
            .mount(&server)
            .await;

        let answer: Value = funding_client(&server)
            .call(
                verb,
                "/funding/v1/addresses",
                None::<&()>,
                Some(&json!({"address": "bc1q", "label": "cold"})),
            )
            .await
            .unwrap();
        assert_eq!(answer, json!({"ok": method_name}));
    }

    let requests = server.received_requests().await.unwrap();
    let nonces: Vec<u64> = requests
        .iter()
        .map(|it| {
            it.headers
                .get("API-Nonce")
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect();
    assert!(nonces.windows(2).all(|it| it[0] < it[1]), "{nonces:?}");
    for request in &requests {
        assert_eq!(
            serde_json::from_slice::<Value>(&request.body).unwrap(),
            json!({"address": "bc1q", "label": "cold"})
        );
        assert_eq!(
            request.headers.get("API-Sign").unwrap().to_str().unwrap(),
            funding_signature(request)
        );
    }
}

#[tokio::test]
async fn funding_maps_the_failures() {
    let server = MockServer::start().await;
    Mock::given(path("/funding/v1/deposit/address"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"error": "TooManyDepositAddresses"})),
        )
        .mount(&server)
        .await;

    let answer = funding_client(&server)
        .call::<(), _, Value>(
            FundingMethod::Put,
            "/funding/v1/deposit/address",
            None,
            Some(&json!({})),
        )
        .await;
    match answer {
        Err(KrakenError::Api { status, body }) => {
            assert_eq!(status, 409);
            assert!(body.contains("TooManyDepositAddresses"), "{body}");
        }
        other => panic!("unexpected {other:?}"),
    }
}
