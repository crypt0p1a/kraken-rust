//! The real shapes of the answers of Kraken (`tests/fixtures`), offline:
//! - decoded strictly by the models: no field ignored, no required field missing
//! - served by a mocked Kraken to the typed APIs, end to end

mod common;
mod strict;

use common::{private_api, public_api};
use kraken_api::Decimal;
use kraken_api::account::{ExtendedBalance, TradeVolume};
use kraken_api::market::{
    Asset, AssetClass, AssetPair, AssetPairStatus, AssetStatus, CandleInterval, CandlesRequest,
    GroupedOrderBook, GroupedOrderBookRequest, Level3OrderBook, Level3OrderBookRequest,
    OrderBookRequest, RecentSpreadsRequest, RecentTradesRequest, ServerTime, SystemState,
    SystemStatus, Ticker,
};
use kraken_api::trading::{
    ClosedOrdersRequest, NewOrder, NewOrderAnswer, OpenOrdersRequest, Order, OrderSide,
    OrderStatus, OrderType, QueryOrdersRequest, TimeInForce,
};
use kraken_api::transparency::{PostTradeData, PostTradeRequest, PreTradeData};
use rust_decimal_macros::dec;
use serde_json::Value;
use std::collections::HashMap;
use strict::{decode_strict, fixture, fixture_answer};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn serve(server: &MockServer, endpoint: &str, name: &str) {
    Mock::given(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture_answer(name)))
        .mount(server)
        .await;
}

fn each<T: serde::de::DeserializeOwned>(context: &str, value: &Value) -> HashMap<String, T> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, item)| {
            (
                key.clone(),
                decode_strict(&format!("{context} {key}"), item),
            )
        })
        .collect()
}

// strict decoding

#[test]
fn time_is_decoded_strictly() {
    let time: ServerTime = decode_strict("Time", &fixture("time.json"));
    assert!(time.unix_time > 1_700_000_000);
    assert!(time.rfc1123.ends_with("+0000"));
}

#[test]
fn tickers_are_decoded_strictly() {
    let tickers: HashMap<String, Ticker> = each("Ticker", &fixture("ticker.json"));

    assert_eq!(tickers.len(), 4);
    assert!(tickers.keys().any(|it| it.ends_with(":BTNL")));
    assert!(
        tickers.values().any(|it| it.bid.price == Decimal::ZERO),
        "a ticker without bid"
    );
    let btc = &tickers["XXBTZUSD"];
    assert!(btc.bid.price > Decimal::ZERO && btc.bid.price <= btc.ask.price);
    assert!(btc.trades.last_24h > 0);
}

#[test]
fn asset_pairs_are_decoded_strictly() {
    let pairs: HashMap<String, AssetPair> = each("AssetPairs", &fixture("asset_pairs.json"));

    let statuses: Vec<_> = pairs.values().filter_map(|it| it.status).collect();
    for status in [
        AssetPairStatus::Online,
        AssetPairStatus::CancelOnly,
        AssetPairStatus::PostOnly,
    ] {
        assert!(statuses.contains(&status), "{status:?} not in the fixture");
    }
    assert!(pairs.values().any(|it| it.long_position_limit.is_some()));
    assert!(pairs.values().any(|it| it.long_position_limit.is_none()));
    assert_eq!(pairs["XXBTZUSD"].altname, "XBTUSD");
    assert!(
        pairs
            .values()
            .all(|it| it.lot.is_some() && it.execution_venue.is_some())
    );
}

#[test]
fn candles_are_complete_rows_of_8_values() {
    let result = fixture("ohlc.json");
    let rows = result["XXBTZUSD"].as_array().unwrap();
    assert!(rows.iter().all(|it| it.as_array().unwrap().len() == 8));
    assert_eq!(result["last"], rows[rows.len() - 2][0]);
}

#[test]
fn balances_are_decoded_strictly() {
    let balances: HashMap<String, ExtendedBalance> = each("BalanceEx", &fixture("balance_ex.json"));

    assert_eq!(balances["ZEUR"].available(), dec!(950.42));
    assert_eq!(balances["XETH"].available(), Decimal::ZERO);
    // 25 + 10 of credit - 2.5 used
    assert_eq!(balances["USDC"].available(), dec!(32.5));
}

#[test]
fn trade_volume_is_decoded_strictly() {
    let volume: TradeVolume = decode_strict("TradeVolume", &fixture("trade_volume.json"));

    assert_eq!(volume.currency, "ZUSD");
    assert_eq!(
        volume
            .inputs
            .as_ref()
            .and_then(|it| it.domain_spot_volume_30d),
        Some(dec!(5321.45))
    );
    assert_eq!(volume.fees.len(), 2);
    assert_eq!(volume.fees_maker["XXBTZUSD"].fee, dec!(0.25));
    assert_eq!(
        volume.fees["XETHZUSD"].nextfuturesvolume,
        Some(dec!(100000))
    );
}

#[test]
fn closed_orders_are_decoded_strictly() {
    let result = fixture("closed_orders.json");
    let orders: HashMap<String, Order> = each("ClosedOrders", &result["closed"]);

    assert_eq!(result["count"], 3);
    assert!(orders.values().all(|it| it.status.is_final()));
    assert!(
        orders
            .values()
            .all(|it| it.time_in_force == Some(TimeInForce::Gtc))
    );
    let canceled = &orders["OAAAAA-BBBBB-CCCCC2"];
    assert_eq!(canceled.status, OrderStatus::Canceled);
    assert_eq!(canceled.reason.as_deref(), Some("User requested"));
    assert_eq!(canceled.misc, "partial");
    let market = &orders["OAAAAA-BBBBB-CCCCC3"];
    assert_eq!(market.description.order_type, OrderType::Market);
    assert_eq!(market.oflags, "viqc,fciq");
    assert_eq!(market.stop_price, Some(Decimal::ZERO));
}

#[test]
fn open_orders_are_decoded_strictly() {
    let result = fixture("open_orders.json");
    let orders: HashMap<String, Order> = each("OpenOrders", &result["open"]);

    let order = &orders["OAAAAA-BBBBB-DDDDD1"];
    assert_eq!(order.status, OrderStatus::Open);
    assert_eq!(order.cl_ord_id.as_deref(), Some("mgr1_buy_1"));
    assert_eq!(order.userref, Some(4242));
    assert_eq!(order.time_in_force, Some(TimeInForce::Gtd));
    assert_eq!(order.expire_time, Some(1761993200.0));
}

#[test]
fn new_order_answers_are_decoded_strictly() {
    let validated: NewOrderAnswer = decode_strict("AddOrder", &fixture("add_order_validate.json"));
    assert!(validated.txids.is_empty());
    assert_eq!(validated.description.close, None);

    let placed: NewOrderAnswer = decode_strict("AddOrder", &fixture("add_order.json"));
    assert_eq!(placed.txids, ["OAAAAA-BBBBB-EEEEE1"]);
    assert!(placed.description.close.is_some());
}

// end to end, through a mocked Kraken

#[tokio::test]
async fn public_calls_read_the_real_shapes() {
    let server = MockServer::start().await;
    serve(&server, "/0/public/Time", "time.json").await;
    serve(&server, "/0/public/Ticker", "ticker.json").await;
    serve(&server, "/0/public/AssetPairs", "asset_pairs.json").await;
    serve(&server, "/0/public/OHLC", "ohlc.json").await;
    let api = public_api(&server);

    assert!(api.server_time().await.unwrap().unix_time > 0);
    assert_eq!(api.tickers(&[]).await.unwrap().len(), 4);
    assert_eq!(api.asset_pairs(&[]).await.unwrap().len(), 5);

    let candles = api
        .candles(&CandlesRequest {
            pair: "XBTUSD".into(),
            interval: CandleInterval::Res1d,
            since: None,
        })
        .await
        .unwrap();
    assert_eq!(candles.candles.len(), 4);
    assert_eq!(candles.last, candles.candles[2].time);
    assert!(!candles.candles[3].is_closed && candles.candles[2].is_closed);
    assert!(
        candles
            .candles
            .windows(2)
            .all(|it| it[1].time - it[0].time == 86_400)
    );
}

#[tokio::test]
async fn private_calls_read_the_real_shapes() {
    let server = MockServer::start().await;
    serve(&server, "/0/private/BalanceEx", "balance_ex.json").await;
    serve(&server, "/0/private/TradeVolume", "trade_volume.json").await;
    serve(&server, "/0/private/OpenOrders", "open_orders.json").await;
    serve(&server, "/0/private/ClosedOrders", "closed_orders.json").await;
    serve(&server, "/0/private/QueryOrders", "query_orders.json").await;
    serve(&server, "/0/private/AddOrder", "add_order_validate.json").await;
    let api = private_api(&server);

    assert_eq!(api.extended_balance().await.unwrap().len(), 4);
    assert_eq!(
        api.trade_volume(&["XBTUSD".into()])
            .await
            .unwrap()
            .fees
            .len(),
        2
    );

    let open = api
        .open_orders(&OpenOrdersRequest::default())
        .await
        .unwrap();
    assert_eq!(open[0].txid, "OAAAAA-BBBBB-DDDDD1");
    assert_eq!(api.open_orders_for_pair("XBTUSD").await.unwrap().len(), 1);
    assert!(api.open_orders_for_pair("ETHUSD").await.unwrap().is_empty());

    let closed = api
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap();
    assert_eq!(closed.count, 3);
    let txids: Vec<_> = closed.orders.iter().map(|it| it.txid.as_str()).collect();
    // most recent first
    assert_eq!(
        txids,
        [
            "OAAAAA-BBBBB-CCCCC1",
            "OAAAAA-BBBBB-CCCCC2",
            "OAAAAA-BBBBB-CCCCC3"
        ]
    );

    let queried = api
        .query_orders(&QueryOrdersRequest {
            txids: vec!["OAAAAA-BBBBB-CCCCC1".into(), "OAAAAA-BBBBB-CCCCC2".into()],
            trades: None,
            userref: None,
        })
        .await
        .unwrap();
    assert_eq!(queried.len(), 2);
    assert!(api.order("OAAAAA-BBBBB-CCCCC2").await.unwrap().is_some());
    assert!(api.order("OAAAAA-BBBBB-ZZZZZ9").await.unwrap().is_none());

    let answer = api
        .new_order(&NewOrder {
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

// market data

#[tokio::test]
async fn system_status_fixture() {
    let status: SystemStatus = decode_strict("SystemStatus", &fixture("system_status.json"));
    assert_eq!(status.status, SystemState::Online);

    let server = MockServer::start().await;
    serve(&server, "/0/public/SystemStatus", "system_status.json").await;
    assert_eq!(public_api(&server).system_status().await.unwrap(), status);
}

#[tokio::test]
async fn maintenance_schedule_fixture() {
    // nothing scheduled when the answer was recorded: no `events` at all
    assert!(
        fixture("maintenance_schedule.json")
            .as_object()
            .unwrap()
            .is_empty()
    );

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/public/MaintenanceSchedule",
        "maintenance_schedule.json",
    )
    .await;
    assert!(
        public_api(&server)
            .maintenance_schedule()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn assets_fixture() {
    let assets: HashMap<String, Asset> = each("Assets", &fixture("assets.json"));

    assert_eq!(assets["XXBT"].altname, "XBT");
    assert_eq!(assets["XXBT"].aclass, AssetClass::Currency);
    assert!(assets["XXBT"].margin_rate.is_some());
    assert!(
        assets
            .values()
            .any(|it| it.status == AssetStatus::WithdrawalOnly)
    );
    let tokenized: Vec<_> = assets
        .values()
        .filter(|it| it.aclass == AssetClass::TokenizedAsset)
        .collect();
    assert!(tokenized.iter().any(|it| it.token_multiplier.is_some()));
    assert!(
        tokenized
            .iter()
            .any(|it| it.max_collateral_balance_usd_value.is_some())
    );

    let server = MockServer::start().await;
    serve(&server, "/0/public/Assets", "assets.json").await;
    assert_eq!(
        public_api(&server)
            .assets(&Default::default())
            .await
            .unwrap(),
        assets
    );
}

/// The rows of the single pair of a fixture, or of one of its sides
fn rows(name: &str, side: Option<&str>) -> Vec<Value> {
    let result = fixture(name);
    let pair = result
        .as_object()
        .unwrap()
        .keys()
        .find(|it| *it != "last")
        .unwrap()
        .clone();
    match side {
        None => result[&pair].as_array().unwrap().clone(),
        Some(side) => result[&pair][side].as_array().unwrap().clone(),
    }
}

#[tokio::test]
async fn order_book_fixture() {
    for row in rows("depth.json", Some("asks"))
        .iter()
        .chain(&rows("depth.json", Some("bids")))
    {
        assert_eq!(row.as_array().unwrap().len(), 3, "{row}");
    }

    let server = MockServer::start().await;
    serve(&server, "/0/public/Depth", "depth.json").await;
    let book = public_api(&server)
        .order_book(&OrderBookRequest {
            pair: "XBTUSD".into(),
            count: Some(3),
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(book.pair, "XXBTZUSD");
    assert_eq!((book.asks.len(), book.bids.len()), (3, 3));
    assert!(book.bids[0].price < book.asks[0].price);
}

#[tokio::test]
async fn grouped_order_book_fixture() {
    let grouped: GroupedOrderBook = decode_strict("GroupedBook", &fixture("grouped_book.json"));
    assert_eq!(grouped.grouping, Some(100));
    assert_eq!(grouped.bids.len(), 10);
    assert!(grouped.bids[0].price < grouped.asks[0].price);

    let server = MockServer::start().await;
    serve(&server, "/0/public/GroupedBook", "grouped_book.json").await;
    let served = public_api(&server)
        .grouped_order_book(&GroupedOrderBookRequest {
            pair: "XBTUSD".into(),
            depth: None,
            grouping: None,
        })
        .await
        .unwrap();
    assert_eq!(served, grouped);
}

#[tokio::test]
async fn level3_order_book_fixture() {
    let level3: Level3OrderBook = decode_strict("Level3", &fixture("level3.json"));
    assert!(!level3.bids.is_empty() && !level3.asks.is_empty());
    assert!(level3.bids.iter().all(|it| it.order_id.len() == 19));

    let server = MockServer::start().await;
    serve(&server, "/0/private/Level3", "level3.json").await;
    let served = private_api(&server)
        .level3_order_book(&Level3OrderBookRequest {
            pair: "YFIEUR".into(),
            depth: None,
        })
        .await
        .unwrap();
    assert_eq!(served, level3);
}

#[tokio::test]
async fn recent_trades_fixture() {
    for row in rows("trades.json", None) {
        assert_eq!(row.as_array().unwrap().len(), 7, "{row}");
    }
    assert!(fixture("trades.json")["last"].is_string());

    let server = MockServer::start().await;
    serve(&server, "/0/public/Trades", "trades.json").await;
    let trades = public_api(&server)
        .recent_trades(&RecentTradesRequest {
            pair: "XBTUSD".into(),
            since: None,
            count: None,
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(trades.trades.len(), 3);
    assert!(
        trades
            .trades
            .windows(2)
            .all(|it| it[0].trade_id < it[1].trade_id)
    );
}

#[tokio::test]
async fn recent_spreads_fixture() {
    for row in rows("spread.json", None) {
        assert_eq!(row.as_array().unwrap().len(), 3, "{row}");
    }
    assert!(fixture("spread.json")["last"].is_number());

    let server = MockServer::start().await;
    serve(&server, "/0/public/Spread", "spread.json").await;
    let spreads = public_api(&server)
        .recent_spreads(&RecentSpreadsRequest {
            pair: "XBTUSD".into(),
            since: None,
            asset_class: None,
        })
        .await
        .unwrap();
    assert_eq!(spreads.spreads.len(), 3);
    assert!(spreads.spreads.iter().all(|it| it.bid <= it.ask));
}

#[tokio::test]
async fn pre_trade_fixture() {
    let pre: PreTradeData = decode_strict("PreTrade", &fixture("pre_trade.json"));
    assert_eq!(pre.symbol, "BTC/USD");
    assert!(pre.bids.iter().all(|it| it.side == "BUY"));
    assert!(pre.asks.iter().all(|it| it.side == "SELL"));

    let server = MockServer::start().await;
    serve(&server, "/0/public/PreTrade", "pre_trade.json").await;
    assert_eq!(public_api(&server).pre_trade("BTC/USD").await.unwrap(), pre);
}

#[tokio::test]
async fn post_trade_fixture() {
    let post: PostTradeData = decode_strict("PostTrade", &fixture("post_trade.json"));
    assert_eq!(post.count as usize, post.trades.len());
    // most recent first, the documentation says the opposite
    assert_eq!(
        post.last_ts.as_ref(),
        post.trades.first().map(|it| &it.trade_ts)
    );
    assert!(
        post.trades
            .windows(2)
            .all(|it| it[0].trade_ts >= it[1].trade_ts)
    );

    let server = MockServer::start().await;
    serve(&server, "/0/public/PostTrade", "post_trade.json").await;
    assert_eq!(
        public_api(&server)
            .post_trade(&PostTradeRequest::default())
            .await
            .unwrap(),
        post
    );
}

// account

#[tokio::test]
async fn balance_fixture() {
    let raw = fixture("balance.json");
    assert!(raw.as_object().unwrap().values().all(Value::is_string));

    let server = MockServer::start().await;
    serve(&server, "/0/private/Balance", "balance.json").await;
    let balances = private_api(&server)
        .balance(&Default::default())
        .await
        .unwrap();
    assert_eq!(balances.len(), 6);
    assert_eq!(balances["ZUSD"], dec!(2970172.7962));
    assert_eq!(balances["XETH"], Decimal::ZERO);
}
