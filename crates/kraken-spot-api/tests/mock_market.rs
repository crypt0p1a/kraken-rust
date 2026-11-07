//! Every public call against a mocked Kraken server: request sent, answer decoded

mod common;

use common::{FormIs, QueryIs, Signed, ok, private_api, public_api};
use kraken_spot_api::market::{
    AssetClass, AssetStatus, AssetsRequest, CandleInterval, CandlesRequest,
    GroupedOrderBookRequest, Level3OrderBookRequest, OrderBookRequest, RecentSpreadsRequest,
    RecentTradesRequest, SystemState,
};
use kraken_spot_api::trading::{OrderSide, OrderType};
use kraken_spot_api::transparency::PostTradeRequest;
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

#[tokio::test]
async fn assets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Assets"))
        .and(QueryIs(vec![
            ("asset", "XBT,AALx"),
            ("aclass", "tokenized_asset"),
        ]))
        .respond_with(ok(
            json!({"AALx": {"aclass": "tokenized_asset", "altname": "AALx",
            "decimals": 8, "display_decimals": 6, "status": "enabled", "token_multiplier": "1"}}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let assets = public_api(&server)
        .assets(&AssetsRequest {
            assets: vec!["XBT".into(), "AALx".into()],
            aclass: Some(AssetClass::TokenizedAsset),
        })
        .await
        .unwrap();
    assert_eq!(assets["AALx"].status, AssetStatus::Enabled);
    assert_eq!(assets["AALx"].token_multiplier, Some(dec!(1)));
}

#[tokio::test]
async fn order_book() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Depth"))
        .and(QueryIs(vec![("pair", "XBTUSD"), ("count", "2")]))
        .respond_with(ok(json!({"XXBTZUSD": {
            "asks": [["77934.10000", "4.317", 1793612207], ["77934.20000", "0.002", 1793612201]],
            "bids": [["77934.00000", "0.001", 1793612198]]
        }})))
        .expect(1)
        .mount(&server)
        .await;

    let book = public_api(&server)
        .order_book(&OrderBookRequest {
            pair: "XBTUSD".into(),
            count: Some(2),
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(book.pair, "XXBTZUSD");
    assert_eq!(book.asks.len(), 2);
    assert_eq!(book.bids[0].price, dec!(77934));
}

#[tokio::test]
async fn grouped_order_book() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/GroupedBook"))
        .and(QueryIs(vec![
            ("pair", "XBTUSD"),
            ("depth", "10"),
            ("grouping", "100"),
        ]))
        .respond_with(ok(json!({"pair": "XBTUSD", "grouping": 100,
            "bids": [{"price": "77930.00000", "qty": "0.00388930"}],
            "asks": [{"price": "77940.00000", "qty": "17.27504303"}]})))
        .expect(1)
        .mount(&server)
        .await;

    let book = public_api(&server)
        .grouped_order_book(&GroupedOrderBookRequest {
            pair: "XBTUSD".into(),
            depth: Some(10),
            grouping: Some(100),
        })
        .await
        .unwrap();
    assert_eq!(book.bids[0].quantity, dec!(0.0038893));
}

#[tokio::test]
async fn level3_order_book_is_signed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Level3"))
        .and(Signed)
        .and(FormIs(vec![("pair", "XBTUSD"), ("depth", "10")]))
        .respond_with(ok(json!({"pair": "BTC/USD",
            "bids": [{"price": "77930.0", "qty": "0.5", "order_id": "O5KJU4-IEQTM-NDMS6W",
                "timestamp": 1765622008594292000u64}],
            "asks": []})))
        .expect(1)
        .mount(&server)
        .await;

    let book = private_api(&server)
        .level3_order_book(&Level3OrderBookRequest {
            pair: "XBTUSD".into(),
            depth: Some(10),
        })
        .await
        .unwrap();
    assert_eq!(book.bids[0].order_id, "O5KJU4-IEQTM-NDMS6W");
}

#[tokio::test]
async fn recent_trades() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Trades"))
        .and(QueryIs(vec![
            ("pair", "XBTUSD"),
            ("since", "1793612206000000000"),
            ("count", "1"),
        ]))
        .respond_with(ok(json!({
            "XXBTZUSD": [["77934.10000", "0.00049086", 1793612206.5751364, "s", "m", "", 113876654]],
            "last": "1793612206575136400"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let trades = public_api(&server)
        .recent_trades(&RecentTradesRequest {
            pair: "XBTUSD".into(),
            since: Some("1793612206000000000".into()),
            count: Some(1),
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(trades.last, "1793612206575136400");
    assert_eq!(trades.trades[0].side, OrderSide::Sell);
    assert_eq!(trades.trades[0].order_type, OrderType::Market);
}

#[tokio::test]
async fn recent_spreads() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/Spread"))
        .and(QueryIs(vec![("pair", "XBTUSD"), ("since", "1793611600")]))
        .respond_with(ok(json!({
            "XXBTZUSD": [[1793611608, "78058.00000", "78058.10000"]],
            "last": 1793611608
        })))
        .expect(1)
        .mount(&server)
        .await;

    let spreads = public_api(&server)
        .recent_spreads(&RecentSpreadsRequest {
            pair: "XBTUSD".into(),
            since: Some(1793611600),
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(spreads.last, 1793611608);
    assert_eq!(spreads.spreads[0].ask, dec!(78058.1));
}

#[tokio::test]
async fn pre_trade() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/PreTrade"))
        .and(QueryIs(vec![("symbol", "BTC/USD")]))
        .respond_with(ok(
            json!({"symbol": "BTC/USD", "description": "Bitcoin / US Dollar",
            "base_asset": "XBT", "base_notation": "UNIT", "quote_asset": "USD",
            "quote_notation": "MONE", "venue": "PGSL", "system": "CLOB", "bids": [], "asks": []}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let data = public_api(&server).pre_trade("BTC/USD").await.unwrap();
    assert_eq!(data.system, "CLOB");
    assert_eq!(data.base_dti_code, None);
}

#[tokio::test]
async fn post_trade() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/0/public/PostTrade"))
        .and(QueryIs(vec![
            ("symbol", "BTC/USD"),
            ("from_ts", "2026-11-02T09:00:00Z"),
            ("to_ts", "2026-11-02T10:00:00Z"),
            ("count", "10"),
        ]))
        .respond_with(ok(json!({"count": 0, "trades": []})))
        .expect(1)
        .mount(&server)
        .await;

    let data = public_api(&server)
        .post_trade(&PostTradeRequest {
            symbol: Some("BTC/USD".into()),
            from_ts: Some("2026-11-02T09:00:00Z".into()),
            to_ts: Some("2026-11-02T10:00:00Z".into()),
            count: Some(10),
        })
        .await
        .unwrap();
    assert_eq!(data.count, 0);
}
