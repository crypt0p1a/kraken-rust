//! Every public call against a mocked Kraken server: request sent, answer decoded

mod common;

use common::{QueryIs, ok, public_api};
use kraken_api::market::{CandleInterval, CandlesRequest, SystemState};
use rust_decimal_macros::dec;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

#[tokio::test]
async fn server_time() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Time"))
        .and(QueryIs(vec![]))
        .respond_with(ok(
            json!({"unixtime": 1688669448, "rfc1123": "Thu, 06 Jul 23 18:50:48 +0000"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let time = public_api(&server).server_time().await.unwrap();
    assert_eq!(time.unix_time, 1688669448);
}

#[tokio::test]
async fn tickers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .and(QueryIs(vec![("pair", "XBTUSD,ETHUSD")]))
        .respond_with(ok(json!({"XXBTZUSD": {
            "a": ["30300.10000", "1", "1.000"], "b": ["30300.00000", "1", "1.000"],
            "c": ["30303.20000", "0.00067643"], "v": ["4083.67001100", "4412.73601799"],
            "p": ["30706.77771", "30689.13205"], "t": [34619, 38907],
            "l": ["29868.30000", "29868.30000"], "h": ["31631.00000", "31631.00000"],
            "o": "30502.80000"
        }})))
        .expect(1)
        .mount(&server)
        .await;

    let tickers = public_api(&server)
        .tickers(&["XBTUSD".into(), "ETHUSD".into()])
        .await
        .unwrap();
    assert_eq!(tickers["XXBTZUSD"].last_trade.price, dec!(30303.2));
    assert_eq!(tickers["XXBTZUSD"].trades.last_24h, 38907);
}

#[tokio::test]
async fn tickers_of_all_the_pairs_omit_the_pair() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Ticker"))
        .and(QueryIs(vec![]))
        .respond_with(ok(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    assert!(public_api(&server).tickers(&[]).await.unwrap().is_empty());
}

#[tokio::test]
async fn asset_pairs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/AssetPairs"))
        .and(QueryIs(vec![("pair", "XBTUSD")]))
        .respond_with(ok(json!({"XXBTZUSD": {
            "altname": "XBTUSD", "wsname": "XBT/USD", "base": "XXBT", "quote": "ZUSD",
            "pair_decimals": 1, "cost_decimals": 5, "lot_decimals": 8,
            "fees": [[0, 0.4]], "fees_maker": [[0, 0.25]],
            "ordermin": "0.00005", "costmin": "0.5", "tick_size": "0.1", "status": "online"
        }})))
        .expect(1)
        .mount(&server)
        .await;

    let pairs = public_api(&server)
        .asset_pairs(&["XBTUSD".into()])
        .await
        .unwrap();
    assert_eq!(pairs["XXBTZUSD"].ordermin, Some(dec!(0.00005)));
    assert_eq!(pairs["XXBTZUSD"].fees_maker[0].percent, dec!(0.25));
}

#[tokio::test]
async fn candles() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/OHLC"))
        .and(QueryIs(vec![
            ("pair", "XBTUSD"),
            ("interval", "5"),
            ("since", "1688671200"),
        ]))
        .respond_with(ok(json!({
            "XXBTZUSD": [
                [1688671200, "30306.1", "30306.2", "30305.7", "30305.7", "30306.1", "3.39", 23],
                [1688671500, "30304.5", "30304.5", "30300.0", "30300.0", "30300.7", "4.42", 18]
            ],
            "last": 1688671200
        })))
        .expect(1)
        .mount(&server)
        .await;

    let candles = public_api(&server)
        .candles(&CandlesRequest {
            pair: "XBTUSD".into(),
            interval: CandleInterval::Res5m,
            since: Some(1688671200),
        })
        .await
        .unwrap();
    assert_eq!(candles.pair, "XXBTZUSD");
    assert_eq!(candles.last, 1688671200);
    assert!(candles.candles[0].is_closed);
    assert!(!candles.candles[1].is_closed);
}

#[tokio::test]
async fn system_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/SystemStatus"))
        .and(QueryIs(vec![]))
        .respond_with(ok(
            json!({"status": "post_only", "timestamp": "2026-11-02T09:36:47Z"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let status = public_api(&server).system_status().await.unwrap();
    assert_eq!(status.status, SystemState::PostOnly);
}

#[tokio::test]
async fn maintenance_schedule() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/MaintenanceSchedule"))
        .and(QueryIs(vec![]))
        .respond_with(ok(json!({"events": [{"event_id": 34,
            "title": "REST API infrastructure upgrade",
            "expected_start_utc": "2026-05-14T02:00:00Z", "expected_end_utc": "2026-05-14T02:30:00Z",
            "time_to_start_s": 236940, "phase": "announced",
            "affected_services": ["spot_rest", "spot_ws"], "order_submission": "allowed",
            "recommended_action": "continue", "cancel_before_utc": "2026-05-14T01:55:00Z",
            "source_url": "https://status.kraken.com/incidents/d3f8h1xpqv62"}]})))
        .expect(1)
        .mount(&server)
        .await;

    let events = public_api(&server).maintenance_schedule().await.unwrap();
    assert_eq!(events[0].event_id, 34);
    assert_eq!(events[0].affected_services, ["spot_rest", "spot_ws"]);
}
