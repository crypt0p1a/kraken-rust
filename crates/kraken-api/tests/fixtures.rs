//! The real shapes of the answers of Kraken (`tests/fixtures`), offline:
//! - decoded strictly by the models: no field ignored, no required field missing
//! - served by a mocked Kraken to the typed APIs, end to end

mod common;
mod strict;

use common::{private_api, public_api};
use kraken_api::Decimal;
use kraken_api::account::{
    ApiKeyInfo, CreditLines, Export, ExportReport, ExportRequest, ExtendedBalance, LedgerEntry,
    LedgerType, QueryLedgersRequest, RemoveExportType, TradeBalance, TradeVolume, WalletAccounts,
    WalletStatus,
};
use kraken_api::algo::{ClosedStrategies, NewStrategy, Strategy, StrategyStatus, StrategyType};
use kraken_api::earn::{EarnAllocations, EarnFunds, EarnStrategies, LockType};
use kraken_api::market::{
    Asset, AssetClass, AssetPair, AssetPairStatus, AssetStatus, CandleInterval, CandlesRequest,
    GroupedOrderBook, GroupedOrderBookRequest, Level3OrderBook, Level3OrderBookRequest,
    OrderBookRequest, RecentSpreadsRequest, RecentTradesRequest, ServerTime, SystemState,
    SystemStatus, Ticker,
};
use kraken_api::trading::{
    AddOrderBatch, AmendOrder, AmendType, BatchOrderAnswer, CancelAllAfter, CancelledOrders,
    ClosedOrdersRequest, EditOrder, EditedOrder, NewOrder, NewOrderAnswer, OpenOrdersRequest,
    Order, OrderAmends, OrderAmendsRequest, OrderReference, OrderSide, OrderStatus, OrderType,
    Position, QueryOrdersRequest, QueryTradesRequest, TimeInForce, Trade, WebSocketsToken,
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
            display_volume: None,
            asset_class: None,
            trigger: None,
            leverage: None,
            reduce_only: None,
            self_trade_prevention: None,
            deadline: None,
            broker: None,
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

#[tokio::test]
async fn credit_lines_fixture() {
    let lines: CreditLines = decode_strict("CreditLines", &fixture("credit_lines.json"));
    assert_eq!(lines.asset_details["ZUSD"].credit, Some(dec!(50000)));
    assert_eq!(lines.asset_details["XXBT"].credit, None);

    let server = MockServer::start().await;
    serve(&server, "/0/private/CreditLines", "credit_lines.json").await;
    assert_eq!(
        private_api(&server)
            .credit_lines(&Default::default())
            .await
            .unwrap(),
        lines
    );
}

#[tokio::test]
async fn trade_balance_fixture() {
    let balance: TradeBalance = decode_strict("TradeBalance", &fixture("trade_balance.json"));
    // equity = trade balance + unrealized net
    assert_eq!(
        balance.equity,
        balance.trade_balance + balance.unrealized_net
    );
    assert_eq!(balance.free_margin, balance.equity - balance.margin);

    let server = MockServer::start().await;
    serve(&server, "/0/private/TradeBalance", "trade_balance.json").await;
    assert_eq!(
        private_api(&server)
            .trade_balance(&Default::default())
            .await
            .unwrap(),
        balance
    );
}

#[tokio::test]
async fn api_key_info_fixture() {
    let info: ApiKeyInfo = decode_strict("GetApiKeyInfo", &fixture("api_key_info.json"));
    assert!(!info.permissions.contains(&"withdraw-funds".to_string()));
    assert!(info.created_time < info.modified_time);

    let server = MockServer::start().await;
    serve(&server, "/0/private/GetApiKeyInfo", "api_key_info.json").await;
    assert_eq!(
        private_api(&server)
            .api_key_info(&Default::default())
            .await
            .unwrap(),
        info
    );
}

#[tokio::test]
async fn wallet_accounts_fixture() {
    let wallets: WalletAccounts =
        decode_strict("ListWalletAccounts", &fixture("wallet_accounts.json"));
    assert_eq!(wallets.accounts.len(), 3);
    assert!(
        wallets
            .accounts
            .iter()
            .all(|it| it.flags.active == (it.status == WalletStatus::Active))
    );

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/ListWalletAccounts",
        "wallet_accounts.json",
    )
    .await;
    assert_eq!(
        private_api(&server).wallet_accounts().await.unwrap(),
        wallets
    );
}

// trades

#[tokio::test]
async fn trades_history_fixture() {
    let result = fixture("trades_history.json");
    let trades: HashMap<String, Trade> = each("TradesHistory", &result["trades"]);
    assert_eq!(result["count"], 3);
    let position = &trades["TKH2SE-M7IF5-CFI7LT"];
    assert_eq!(position.position_status.as_deref(), Some("closed"));
    // net = closed cost - cost - fees
    assert_eq!(
        position.net,
        Some(
            position.closed_cost.unwrap()
                - position.cost
                - position.fee
                - position.closed_fee.unwrap()
        )
    );

    let server = MockServer::start().await;
    serve(&server, "/0/private/TradesHistory", "trades_history.json").await;
    let page = private_api(&server)
        .trades_history(&Default::default())
        .await
        .unwrap();
    assert_eq!(page.count, 3);
    assert!(page.trades.windows(2).all(|it| it[0].time >= it[1].time));
    assert!(
        page.trades
            .iter()
            .all(|it| trades[&it.txid].trade_id == it.trade_id)
    );
}

#[tokio::test]
async fn query_trades_fixture() {
    let trades: HashMap<String, Trade> = each("QueryTrades", &fixture("query_trades.json"));

    let server = MockServer::start().await;
    serve(&server, "/0/private/QueryTrades", "query_trades.json").await;
    let served = private_api(&server)
        .query_trades(&QueryTradesRequest {
            txids: trades.keys().cloned().collect(),
            trades: None,
            ledgers: None,
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(served.len(), trades.len());
    for trade in &served {
        assert_eq!(trade.trade_id, trades[&trade.txid].trade_id);
    }
}

#[tokio::test]
async fn order_amends_fixture() {
    let amends: OrderAmends = decode_strict("OrderAmends", &fixture("order_amends.json"));
    assert_eq!(amends.count as usize, amends.amends.len());
    assert_eq!(amends.amends[0].amend_type, AmendType::Original);
    assert!(
        amends
            .amends
            .windows(2)
            .all(|it| it[0].timestamp < it[1].timestamp)
    );

    let server = MockServer::start().await;
    serve(&server, "/0/private/OrderAmends", "order_amends.json").await;
    let served = private_api(&server)
        .order_amends(&OrderAmendsRequest {
            order_id: "OQCLML-BW3P3-BUCMWZ".into(),
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(served, amends);
}

#[tokio::test]
async fn open_positions_fixture() {
    let positions: HashMap<String, Position> =
        each("OpenPositions", &fixture("open_positions.json"));
    assert!(positions.values().all(|it| it.volume_closed <= it.volume));
    assert!(
        positions
            .values()
            .all(|it| it.net.is_some_and(|net| net > Decimal::ZERO))
    );

    let server = MockServer::start().await;
    serve(&server, "/0/private/OpenPositions", "open_positions.json").await;
    let served = private_api(&server)
        .open_positions(&Default::default())
        .await
        .unwrap();
    assert_eq!(served.len(), 3);
    assert_eq!(served[0].txid, "TYMRFG-URRG5-2ZTQSD");
    assert!(served.windows(2).all(|it| it[0].time >= it[1].time));
}

#[tokio::test]
async fn ledgers_fixture() {
    let result = fixture("ledgers.json");
    let entries: HashMap<String, LedgerEntry> = each("Ledgers", &result["ledger"]);
    assert_eq!(result["count"], 5);
    // the two entries of a trade share its refid
    let trade = entries
        .values()
        .filter(|it| it.refid == "TJKLXX-PGMUI-4NTLXU")
        .count();
    assert_eq!(trade, 2);
    assert!(
        entries
            .values()
            .any(|it| it.entry_type == LedgerType::Unknown)
    );
    assert!(entries.values().any(|it| it.amount_token.is_some()));

    let server = MockServer::start().await;
    serve(&server, "/0/private/Ledgers", "ledgers.json").await;
    let page = private_api(&server)
        .ledgers(&Default::default())
        .await
        .unwrap();
    assert_eq!(page.count, Some(5));
    assert!(page.entries.windows(2).all(|it| it[0].time >= it[1].time));
}

#[tokio::test]
async fn query_ledgers_fixture() {
    let entries: HashMap<String, LedgerEntry> =
        each("QueryLedgers", &fixture("query_ledgers.json"));

    let server = MockServer::start().await;
    serve(&server, "/0/private/QueryLedgers", "query_ledgers.json").await;
    let served = private_api(&server)
        .query_ledgers(&QueryLedgersRequest {
            ids: entries.keys().cloned().collect(),
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(served.len(), entries.len());
    for entry in &served {
        assert_eq!(entry.refid, entries[&entry.id].refid);
    }
}

// exports

#[tokio::test]
async fn add_export_fixture() {
    let result = fixture("add_export.json");
    assert_eq!(
        result.as_object().unwrap().len(),
        1,
        "only the id: {result}"
    );

    let server = MockServer::start().await;
    serve(&server, "/0/private/AddExport", "add_export.json").await;
    let id = private_api(&server)
        .add_export(&ExportRequest {
            report: ExportReport::Ledgers,
            format: None,
            description: "ledgers".into(),
            fields: vec![],
            start_time: None,
            end_time: None,
        })
        .await
        .unwrap();
    assert_eq!(id, "TCJA");
}

#[tokio::test]
async fn export_status_fixture() {
    let raw = fixture("export_status.json");
    let exports: Vec<Export> = raw
        .as_array()
        .unwrap()
        .iter()
        .map(|it| decode_strict("ExportStatus", it))
        .collect();
    assert!(exports.iter().any(|it| it.error.is_some()));
    assert!(exports.iter().any(|it| it.asset_classes.is_some()));

    let server = MockServer::start().await;
    serve(&server, "/0/private/ExportStatus", "export_status.json").await;
    assert_eq!(
        private_api(&server)
            .export_status(ExportReport::Trades)
            .await
            .unwrap(),
        exports
    );
}

#[tokio::test]
async fn remove_export_fixture() {
    let server = MockServer::start().await;
    serve(&server, "/0/private/RemoveExport", "remove_export.json").await;
    assert!(
        private_api(&server)
            .remove_export("TCJA", RemoveExportType::Cancel)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn websockets_token_fixture() {
    let token: WebSocketsToken =
        decode_strict("GetWebSocketsToken", &fixture("websockets_token.json"));

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/GetWebSocketsToken",
        "websockets_token.json",
    )
    .await;
    assert_eq!(
        private_api(&server).websockets_token().await.unwrap(),
        token
    );
}

#[tokio::test]
async fn cancel_order_fixture() {
    let cancelled: CancelledOrders = decode_strict("CancelOrder", &fixture("cancel_order.json"));

    let server = MockServer::start().await;
    serve(&server, "/0/private/CancelOrder", "cancel_order.json").await;
    let served = private_api(&server)
        .cancel_order(&OrderReference::Userref { userref: 45 })
        .await
        .unwrap();
    assert_eq!(served, cancelled);
}

#[tokio::test]
async fn cancel_all_fixture() {
    // the same answer as CancelOrder
    let server = MockServer::start().await;
    serve(&server, "/0/private/CancelAll", "cancel_order.json").await;
    let cancelled = private_api(&server).cancel_all().await.unwrap();
    assert_eq!(cancelled.count, 2);
}

#[tokio::test]
async fn cancel_all_orders_after_fixture() {
    let timer: CancelAllAfter = decode_strict(
        "CancelAllOrdersAfter",
        &fixture("cancel_all_orders_after.json"),
    );
    assert!(timer.current_time < timer.trigger_time);

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/CancelAllOrdersAfter",
        "cancel_all_orders_after.json",
    )
    .await;
    assert_eq!(
        private_api(&server)
            .cancel_all_orders_after(60)
            .await
            .unwrap(),
        timer
    );
}

#[tokio::test]
async fn cancel_order_batch_fixture() {
    // example of the documentation: no `pending`
    let cancelled: CancelledOrders =
        decode_strict("CancelOrderBatch", &fixture("cancel_order_batch.json"));
    assert_eq!(cancelled.pending, None);

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/CancelOrderBatch",
        "cancel_order_batch.json",
    )
    .await;
    let served = private_api(&server)
        .cancel_order_batch(&[OrderReference::Txid {
            txid: "OG5V2Y-RYKVL-DT3V3B".into(),
        }])
        .await
        .unwrap();
    assert_eq!(served, cancelled);
}

#[tokio::test]
async fn amend_order_fixture() {
    let server = MockServer::start().await;
    serve(&server, "/0/private/AmendOrder", "amend_order.json").await;
    let amend_id = private_api(&server)
        .amend_order(&AmendOrder {
            txid: Some("OQCLML-BW3P3-BUCMWZ".into()),
            order_qty: Some("1.0".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(amend_id, "TJSMEH-AA67V-YUSQ6O");
}

#[tokio::test]
async fn edit_order_fixture() {
    let edited: EditedOrder = decode_strict("EditOrder", &fixture("edit_order.json"));
    assert_eq!(edited.status, "ok");
    assert_ne!(edited.txid, edited.original_txid);

    let server = MockServer::start().await;
    serve(&server, "/0/private/EditOrder", "edit_order.json").await;
    let served = private_api(&server)
        .edit_order(&EditOrder {
            txid: "OHYO67-6LP66-HMQ437".into(),
            pair: "XBTGBP".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(served, edited);
}

#[tokio::test]
async fn add_order_batch_fixture() {
    let raw = fixture("add_order_batch.json");
    let answers: Vec<BatchOrderAnswer> = raw["orders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| decode_strict("AddOrderBatch", it))
        .collect();
    // each order either placed or failed
    assert!(
        answers
            .iter()
            .all(|it| it.txid.is_some() != it.error.is_some())
    );

    let server = MockServer::start().await;
    serve(&server, "/0/private/AddOrderBatch", "add_order_batch.json").await;
    let served = private_api(&server)
        .add_order_batch(&AddOrderBatch {
            pair: "XBTUSD".into(),
            orders: vec![],
            asset_class: None,
            deadline: None,
            validate: None,
            broker: None,
        })
        .await
        .unwrap();
    assert_eq!(served, answers);
}

// algo

#[tokio::test]
async fn open_strategies_fixture() {
    let strategies: HashMap<String, Strategy> = each(
        "GetOpenStrategies",
        &fixture("open_strategies.json")["open"],
    );
    assert!(strategies.iter().all(|(id, it)| *id == it.strategy_id));
    // avg_price = total_cost / filled_volume
    let open = &strategies["ASMBA64-R23M2-J7JRJV"].details;
    assert_eq!(open.avg_price, Some(open.total_cost / open.filled_volume));

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/GetOpenStrategies",
        "open_strategies.json",
    )
    .await;
    let served = private_api(&server).open_strategies(None).await.unwrap();
    assert_eq!(served.len(), 2);
    assert!(
        served
            .windows(2)
            .all(|it| it[0].details.start_time >= it[1].details.start_time)
    );
}

#[tokio::test]
async fn query_strategies_fixture() {
    let strategies: HashMap<String, Strategy> = each(
        "QueryStrategies",
        &fixture("query_strategies.json")["strategies"],
    );

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/QueryStrategies",
        "query_strategies.json",
    )
    .await;
    let served = private_api(&server)
        .query_strategies(&strategies.keys().cloned().collect::<Vec<_>>())
        .await
        .unwrap();
    assert!(served.iter().all(|it| strategies[&it.strategy_id] == *it));
}

#[tokio::test]
async fn closed_strategies_fixture() {
    let page: ClosedStrategies =
        decode_strict("GetClosedStrategies", &fixture("closed_strategies.json"));
    assert!(page.next_cursor.is_some());
    // a fully executed strategy filled its whole volume
    let closed = page
        .strategies
        .iter()
        .find(|it| it.status == StrategyStatus::Closed)
        .unwrap();
    assert_eq!(closed.details.filled_volume, closed.details.volume);

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/GetClosedStrategies",
        "closed_strategies.json",
    )
    .await;
    assert_eq!(
        private_api(&server)
            .closed_strategies(&Default::default())
            .await
            .unwrap(),
        page
    );
}

#[tokio::test]
async fn add_strategy_fixture() {
    let server = MockServer::start().await;
    serve(&server, "/0/private/AddStrategy", "add_strategy.json").await;
    let id = private_api(&server)
        .add_strategy(&NewStrategy {
            strategy_type: StrategyType::Twap,
            symbol: "BTC/USD".into(),
            asset_class: "forex".into(),
            side: OrderSide::Sell,
            volume: "2".into(),
            order_type: OrderType::Market,
            price: None,
            start_time: None,
            expire_time: "+7200".into(),
            validate: None,
        })
        .await
        .unwrap();
    assert!(id.starts_with("AS"));
}

#[tokio::test]
async fn cancel_strategy_fixture() {
    // the same answer as AddStrategy
    let server = MockServer::start().await;
    serve(&server, "/0/private/CancelStrategy", "add_strategy.json").await;
    let id = private_api(&server)
        .cancel_strategy("ASMBA64-R23M2-J7JRJV")
        .await
        .unwrap();
    assert_eq!(id, "ASMBA64-R23M2-J7JRJV");
}

// earn

#[tokio::test]
async fn earn_strategies_fixture() {
    let page: EarnStrategies = decode_strict("Earn/Strategies", &fixture("earn_strategies.json"));
    assert!(
        page.items
            .iter()
            .all(|it| it.lock_type != LockType::Unknown)
    );
    // a strategy which can't be allocated tells why
    assert!(
        page.items
            .iter()
            .filter(|it| !it.can_allocate)
            .all(|it| !it.allocation_restriction_info.is_empty())
    );

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/Earn/Strategies",
        "earn_strategies.json",
    )
    .await;
    assert_eq!(
        private_api(&server)
            .earn_strategies(&Default::default())
            .await
            .unwrap(),
        page
    );
}

#[tokio::test]
async fn earn_allocations_fixture() {
    let allocations: EarnAllocations =
        decode_strict("Earn/Allocations", &fixture("earn_allocations.json"));
    // the total of the account is the sum of its strategies
    let total: Decimal = allocations
        .items
        .iter()
        .map(|it| it.amount_allocated.total.converted)
        .sum();
    assert_eq!(total, allocations.total_allocated);
    for state in allocations
        .items
        .iter()
        .flat_map(|it| {
            [
                &it.amount_allocated.bonding,
                &it.amount_allocated.exit_queue,
                &it.amount_allocated.unbonding,
            ]
        })
        .flatten()
    {
        assert_eq!(state.allocation_count as usize, state.allocations.len());
    }

    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/Earn/Allocations",
        "earn_allocations.json",
    )
    .await;
    assert_eq!(
        private_api(&server)
            .earn_allocations(&Default::default())
            .await
            .unwrap(),
        allocations
    );
}

#[tokio::test]
async fn earn_allocate_fixture() {
    let server = MockServer::start().await;
    serve(&server, "/0/private/Earn/Allocate", "earn_allocate.json").await;
    let started = private_api(&server)
        .earn_allocate(&EarnFunds {
            strategy_id: "ESRFUO3-Q62XD-WIOIL7".into(),
            amount: "4.3".into(),
        })
        .await
        .unwrap();
    assert!(started);
}

#[tokio::test]
async fn earn_allocate_status_fixture() {
    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/Earn/AllocateStatus",
        "earn_status.json",
    )
    .await;
    let pending = private_api(&server)
        .earn_allocate_status("ESRFUO3-Q62XD-WIOIL7")
        .await
        .unwrap();
    assert!(!pending);
}

#[tokio::test]
async fn earn_deallocate_fixture() {
    // the same answer as Earn/Allocate
    let server = MockServer::start().await;
    serve(&server, "/0/private/Earn/Deallocate", "earn_allocate.json").await;
    let started = private_api(&server)
        .earn_deallocate(&EarnFunds {
            strategy_id: "ESRFUO3-Q62XD-WIOIL7".into(),
            amount: "1.5".into(),
        })
        .await
        .unwrap();
    assert!(started);
}

#[tokio::test]
async fn earn_deallocate_status_fixture() {
    let server = MockServer::start().await;
    serve(
        &server,
        "/0/private/Earn/DeallocateStatus",
        "earn_status.json",
    )
    .await;
    let pending = private_api(&server)
        .earn_deallocate_status("ESRFUO3-Q62XD-WIOIL7")
        .await
        .unwrap();
    assert!(!pending);
}
