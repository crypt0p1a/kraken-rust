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
    assert_eq!(balances["ZEUR"].available(), kraken_api::Decimal::from(6));
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

/// Serves the fixtures of kraken-api: the real shapes of the answers
async fn serve_fixtures(server: &MockServer) {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../kraken-api/tests/fixtures");
    for (endpoint, file) in [
        ("/0/public/Time", "time.json"),
        ("/0/public/Ticker", "ticker.json"),
        ("/0/public/AssetPairs", "asset_pairs.json"),
        ("/0/public/OHLC", "ohlc.json"),
        ("/0/private/BalanceEx", "balance_ex.json"),
        ("/0/private/TradeVolume", "trade_volume.json"),
        ("/0/private/OpenOrders", "open_orders.json"),
        ("/0/private/ClosedOrders", "closed_orders.json"),
        ("/0/private/QueryOrders", "query_orders.json"),
        ("/0/private/AddOrder", "add_order_validate.json"),
    ] {
        let body: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fixtures.join(file)).unwrap()).unwrap();
        Mock::given(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn every_public_method_is_exported() {
    let server = MockServer::start().await;
    serve_fixtures(&server).await;
    let api = KrakenPublicApi::new(options(&server), KrakenRateLimiter::new(100, 1, None)).unwrap();

    assert!(api.server_time().await.unwrap().unix_time > 0);
    assert_eq!(api.tickers(vec![]).await.unwrap().len(), 4);
    assert_eq!(
        api.asset_pairs(vec!["XBTUSD".into()]).await.unwrap().len(),
        5
    );
    let candles = api
        .candles(kraken_api::market::CandlesRequest {
            pair: "XBTUSD".into(),
            interval: kraken_api::market::CandleInterval::Res1d,
            since: None,
        })
        .await
        .unwrap();
    assert_eq!(candles.candles.len(), 4);
}

#[tokio::test]
async fn every_private_method_is_exported() {
    use kraken_api::trading::{
        ClosedOrdersRequest, NewOrder, OpenOrdersRequest, OrderSide, OrderType, QueryOrdersRequest,
    };

    let server = MockServer::start().await;
    serve_fixtures(&server).await;
    let api = KrakenPrivateApi::new(
        "key".into(),
        "c2VjcmV0".into(),
        options(&server),
        KrakenRateLimiter::new(100, 1, None),
    )
    .unwrap();

    assert_eq!(api.extended_balance().await.unwrap().len(), 4);
    assert_eq!(
        api.trade_volume(vec!["XBTUSD".into()])
            .await
            .unwrap()
            .fees
            .len(),
        2
    );
    assert_eq!(
        api.open_orders(OpenOrdersRequest::default())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        api.open_orders_for_pair("XBTUSD".into())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        api.closed_orders(ClosedOrdersRequest::default())
            .await
            .unwrap()
            .count,
        3
    );
    let queried = api
        .query_orders(QueryOrdersRequest {
            txids: vec!["OAAAAA-BBBBB-CCCCC1".into()],
            trades: None,
            userref: None,
        })
        .await
        .unwrap();
    assert_eq!(queried.len(), 2);
    assert!(
        api.order("OAAAAA-BBBBB-CCCCC1".into())
            .await
            .unwrap()
            .is_some()
    );

    let answer = api
        .new_order(NewOrder {
            pair: "XBTUSD".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            volume: "0.0001".into(),
            price: Some("38000.0".into()),
            price2: None,
            oflags: None,
            time_in_force: None,
            start_time: None,
            expire_time: None,
            cl_ord_id: None,
            userref: None,
            validate: Some(true),
        })
        .await
        .unwrap();
    assert!(answer.txids.is_empty());
}
