//! Contract of the private trading endpoints, against the real Kraken API. Read only: the orders
//! are only validated (`validate=true`), never placed.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_trading -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::Decimal;
use kraken_api::KrakenError;
use kraken_api::market::AssetPair;
use kraken_api::trading::{
    AmendOrder, AmendType, CloseTime, ClosedOrdersRequest, ConsolidatedPosition, NewOrder,
    NewOrderAnswer, OpenOrdersRequest, Order, OrderAmends, OrderFlag, OrderReference, OrderSide,
    OrderStatus, OrderType, Position, QueryTradesRequest, TimeInForce, Trade, TradesHistoryRequest,
    WebSocketsToken,
};
use live::{
    Presence, decode_strict, entries, le, now_seconds, private_api, private_client, public_api,
};
use rust_decimal_macros::dec;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Default)]
struct OrdersParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    trades: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ofs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    txid: Option<&'a str>,
}

/// Decodes the orders of an answer keyed by txid, checking each one
fn orders(context: &str, raw: &Value, presence: &mut Presence) -> Vec<Order> {
    entries(raw)
        .map(|(txid, value)| {
            presence.record(value);
            let order: Order = decode_strict(&format!("{context} {txid}"), value);
            check_order(txid, &order);
            Order {
                txid: txid.clone(),
                ..order
            }
        })
        .collect()
}

fn check_order(txid: &str, order: &Order) {
    assert!(txid.len() == 19 && txid.matches('-').count() == 2, "{txid}");
    assert!(
        !matches!(order.status, OrderStatus::Unknown),
        "{txid}: {order:?}"
    );
    assert!(
        !matches!(order.description.order_type, OrderType::Unknown),
        "{txid}: {order:?}"
    );
    assert!(
        !matches!(order.time_in_force, Some(TimeInForce::Unknown)),
        "{txid}: {order:?}"
    );
    assert!(order.open_time > 1_300_000_000.0, "{txid}: {order:?}");
    assert!(order.volume > Decimal::ZERO, "{txid}: {order:?}");
    assert!(
        order.volume_executed >= Decimal::ZERO && le(order.volume_executed, order.volume),
        "{txid}: {order:?}"
    );
    assert!(
        order.cost >= Decimal::ZERO
            && order.fee >= Decimal::ZERO
            && order.average_price >= Decimal::ZERO,
        "{txid}: {order:?}"
    );
    assert!(
        !order.description.pair.is_empty() && !order.description.order.is_empty(),
        "{txid}"
    );

    if order.volume_executed > Decimal::ZERO && order.average_price > Decimal::ZERO {
        let expected = order.volume_executed * order.average_price;
        assert!(
            (order.cost - expected).abs() <= expected * dec!(0.01) + dec!(0.000001),
            "{txid}: cost {} != {} × {}",
            order.cost,
            order.volume_executed,
            order.average_price
        );
    }
    if order.description.order_type == OrderType::Limit {
        assert!(
            order.description.price.is_some_and(|it| it > Decimal::ZERO),
            "{txid}: {order:?}"
        );
    }
    let side = match order.description.side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    };
    assert!(
        order.description.order.starts_with(side),
        "{txid}: {}",
        order.description.order
    );
    assert!(
        order.description.order.contains(&order.description.pair),
        "{txid}"
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn open_orders() {
    let raw: Value = private_client()
        .call_with(
            "OpenOrders",
            Some(&OrdersParams {
                trades: Some(true),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        raw.as_object().unwrap().keys().collect::<Vec<_>>(),
        ["open"]
    );

    let mut presence = Presence::default();
    for order in orders("OpenOrders", &raw["open"], &mut presence) {
        assert!(
            matches!(order.status, OrderStatus::Open | OrderStatus::Pending),
            "{order:?}"
        );
        assert!(order.close_time.is_none(), "{order:?}");
        assert!(order.volume_executed < order.volume, "{order:?}");
    }
    presence.print("OpenOrders");

    let typed = private_api()
        .open_orders(&OpenOrdersRequest::default())
        .await
        .unwrap();
    assert_eq!(typed.len(), presence.total());
    assert!(
        typed
            .windows(2)
            .all(|it| it[0].open_time >= it[1].open_time)
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn closed_orders_of_the_first_pages() {
    let mut presence = Presence::default();
    let mut seen = HashSet::new();
    let mut count = None;

    for offset in [0, 50, 100] {
        let raw: Value = private_client()
            .call_with(
                "ClosedOrders",
                Some(&OrdersParams {
                    trades: Some(true),
                    ofs: Some(offset),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        let mut keys: Vec<_> = raw.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["closed", "count"]);

        let total = raw["count"].as_i64().expect("count");
        assert!(
            count.is_none_or(|it: i64| it.abs_diff(total) < 5),
            "count moved: {count:?} {total}"
        );
        count = Some(total);

        let page = orders("ClosedOrders", &raw["closed"], &mut presence);
        assert!(page.len() <= 50);
        for order in &page {
            assert!(order.status.is_final(), "{order:?}");
            let closed = order.close_time.expect("closetm of a closed order");
            assert!(order.open_time <= closed, "{order:?}");
            if order.volume_executed > Decimal::ZERO {
                assert!(
                    order.trades.as_ref().is_some_and(|it| !it.is_empty()),
                    "{order:?}"
                );
            }
            assert!(
                seen.insert(order.txid.clone()),
                "{} in two pages",
                order.txid
            );
        }
        if page.len() < 50 {
            break;
        }
    }

    presence.print("ClosedOrders");
    presence.assert_always(
        "ClosedOrders",
        &[
            "status", "opentm", "closetm", "descr", "vol", "vol_exec", "cost", "fee", "price",
            "misc", "oflags",
        ],
    );
    assert!(seen.len() as i64 <= count.unwrap());
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn closed_orders_through_the_typed_call() {
    let api = private_api();
    let first = api
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap();

    assert!(first.orders.len() <= 50 && first.count >= first.orders.len() as i64);
    assert!(
        first
            .orders
            .windows(2)
            .all(|it| it[0].open_time >= it[1].open_time)
    );
    // without `trades`, Kraken doesn't send them
    assert!(first.orders.iter().all(|it| it.trades.is_none()));

    if first.count > 50 {
        let second = api
            .closed_orders(&ClosedOrdersRequest {
                offset: Some(50),
                ..Default::default()
            })
            .await
            .unwrap();
        let first_ids: HashSet<_> = first.orders.iter().map(|it| &it.txid).collect();
        assert!(second.orders.iter().all(|it| !first_ids.contains(&it.txid)));
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn closed_orders_between_two_dates() {
    let api = private_api();
    let all = api
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap()
        .orders;
    if all.len() < 3 {
        println!("not enough closed orders to test the dates");
        return;
    }

    // the closing times of the 3 most recent orders
    let mut closes: Vec<f64> = all.iter().filter_map(|it| it.close_time).collect();
    closes.sort_by(|a, b| b.total_cmp(a));
    let end = closes[0].ceil() as i64;
    let start = closes[2].floor() as i64 - 1;

    let between = api
        .closed_orders(&ClosedOrdersRequest {
            start: Some(start),
            end: Some(end),
            close_time: Some(CloseTime::Close),
            ..Default::default()
        })
        .await
        .unwrap();

    assert!(!between.orders.is_empty());
    for order in &between.orders {
        let closed = order.close_time.unwrap();
        assert!(
            closed > start as f64 && closed <= end as f64 + 1.0,
            "{closed} not in ]{start}, {end}]"
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn query_orders_returns_the_closed_orders() {
    let api = private_api();
    let closed = api
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap()
        .orders;
    if closed.is_empty() {
        println!("no closed order to query");
        return;
    }
    let batch: Vec<_> = closed.iter().take(20).collect();
    let txids = batch
        .iter()
        .map(|it| it.txid.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let raw: Value = private_client()
        .call_with(
            "QueryOrders",
            Some(&OrdersParams {
                txid: Some(&txids),
                trades: Some(true),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    let mut presence = Presence::default();
    let queried: HashMap<String, Order> = orders("QueryOrders", &raw, &mut presence)
        .into_iter()
        .map(|it| (it.txid.clone(), it))
        .collect();

    assert_eq!(queried.len(), batch.len());
    for order in batch {
        let same = &queried[&order.txid];
        assert_eq!(same.status, order.status);
        assert_eq!(same.volume, order.volume);
        assert_eq!(same.volume_executed, order.volume_executed);
        assert_eq!(same.cost, order.cost);
        assert_eq!(same.description.pair, order.description.pair);
        assert_eq!(same.close_time, order.close_time);
    }

    let one = api
        .order(&closed[0].txid)
        .await
        .unwrap()
        .expect("the order");
    assert_eq!(one.txid, closed[0].txid);
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn an_unknown_order_is_none() {
    assert_eq!(
        private_api().order("OAAAAA-AAAAA-AAAAAA").await.unwrap(),
        None
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn open_orders_for_pair_are_a_subset() {
    let api = private_api();
    let all = api
        .open_orders(&OpenOrdersRequest::default())
        .await
        .unwrap();
    let pair = all
        .first()
        .map(|it| it.description.pair.clone())
        .unwrap_or("XBTUSD".into());

    let of_pair = api.open_orders_for_pair(&pair).await.unwrap();
    assert!(of_pair.iter().all(|it| it.description.pair == pair));
    assert_eq!(
        of_pair.len(),
        all.iter().filter(|it| it.description.pair == pair).count()
    );
}

// new orders, validated only

/// The order, never placed
fn validate_only(order: NewOrder) -> NewOrder {
    NewOrder {
        validate: Some(true),
        ..order
    }
}

struct Market {
    pair: AssetPair,
    bid: Decimal,
    ask: Decimal,
}

async fn market(altname: &str) -> Market {
    let api = public_api();
    let pair = api
        .asset_pairs(&[altname.into()])
        .await
        .unwrap()
        .into_values()
        .next()
        .unwrap();
    let ticker = api
        .tickers(&[altname.into()])
        .await
        .unwrap()
        .into_values()
        .next()
        .unwrap();
    Market {
        pair,
        bid: ticker.bid.price,
        ask: ticker.ask.price,
    }
}

impl Market {
    fn price(&self, value: Decimal) -> String {
        let tick = self.pair.tick_size.unwrap();
        ((value / tick).floor() * tick)
            .round_dp(self.pair.pair_decimals as u32)
            .normalize()
            .to_string()
    }

    /// A volume above ordermin and costmin at this price
    fn volume(&self, price: Decimal) -> String {
        let minimum = self
            .pair
            .ordermin
            .unwrap()
            .max(self.pair.costmin.unwrap() * dec!(1.5) / price);
        minimum
            .round_dp_with_strategy(
                self.pair.lot_decimals as u32,
                rust_decimal::RoundingStrategy::AwayFromZero,
            )
            .normalize()
            .to_string()
    }

    fn order(&self, side: OrderSide, order_type: OrderType, price: Option<Decimal>) -> NewOrder {
        let reference = price.unwrap_or(self.ask);
        validate_only(NewOrder {
            pair: self.pair.altname.clone(),
            side,
            order_type,
            volume: self.volume(reference),
            price: price.map(|it| self.price(it)),
            price2: None,
            oflags: None,
            time_in_force: None,
            start_time: None,
            expire_time: None,
            cl_ord_id: None,
            userref: None,
            validate: None,
        })
    }
}

fn check_validated(context: &str, order: &NewOrder, answer: &NewOrderAnswer) {
    assert!(
        answer.txids.is_empty(),
        "{context}: an order was placed {answer:?}"
    );
    let side = match order.side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    };
    assert!(
        answer.description.order.starts_with(side),
        "{context}: {answer:?}"
    );
    assert!(
        answer.description.order.contains(&order.pair),
        "{context}: {answer:?}"
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn validates_the_raw_answer_of_a_new_order() {
    let market = market("XBTUSD").await;
    let order = market.order(OrderSide::Buy, OrderType::Limit, Some(market.bid / dec!(2)));
    assert_eq!(order.validate, Some(true));

    let raw: Value = private_client()
        .call_with("AddOrder", Some(&order))
        .await
        .unwrap();
    let answer: NewOrderAnswer = decode_strict("AddOrder", &raw);
    check_validated("raw limit buy", &order, &answer);
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn validates_every_kind_of_new_order() {
    let api = private_api();
    let market = market("XBTUSD").await;
    let low = market.bid / dec!(2);
    let high = market.ask * dec!(2);

    let orders = [
        (
            "limit buy",
            market.order(OrderSide::Buy, OrderType::Limit, Some(low)),
        ),
        (
            "market buy",
            market.order(OrderSide::Buy, OrderType::Market, None),
        ),
        (
            "post-only limit buy, fee in quote",
            NewOrder {
                oflags: Some(vec![OrderFlag::Post, OrderFlag::Fciq]),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "GTD limit buy",
            NewOrder {
                time_in_force: Some(TimeInForce::Gtd),
                expire_time: Some("+3600".into()),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "IOC limit buy",
            NewOrder {
                time_in_force: Some(TimeInForce::Ioc),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "limit buy with a client order id",
            NewOrder {
                cl_ord_id: Some("kraken-rust-test".into()),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "limit buy with a userref",
            NewOrder {
                userref: Some(4242),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "delayed limit buy",
            NewOrder {
                start_time: Some(format!("{}", now_seconds() + 3600)),
                ..market.order(OrderSide::Buy, OrderType::Limit, Some(low))
            },
        ),
        (
            "stop-loss-limit buy",
            NewOrder {
                price2: Some(market.price(high * dec!(1.01))),
                ..market.order(OrderSide::Buy, OrderType::StopLossLimit, Some(high))
            },
        ),
        (
            "take-profit buy",
            market.order(OrderSide::Buy, OrderType::TakeProfit, Some(low)),
        ),
    ];

    for (context, order) in orders {
        let order = validate_only(order);
        match api.new_order(&order).await {
            Ok(answer) => check_validated(context, &order, &answer),
            Err(error) => panic!("{context}: {error} for {order:?}"),
        }
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn maps_the_errors_of_invalid_new_orders() {
    let api = private_api();
    let market = market("XBTUSD").await;
    let low = market.bid / dec!(2);
    let valid = market.order(OrderSide::Buy, OrderType::Limit, Some(low));

    let below_minimum = NewOrder {
        volume: "0.00000001".into(),
        ..valid.clone()
    };
    let error = api.new_order(&below_minimum).await.unwrap_err();
    assert!(
        matches!(&error, KrakenError::Api { body, .. } if body.contains("minimum not met")),
        "{error:?}"
    );

    let unknown_pair = NewOrder {
        pair: "NOPEUSD".into(),
        ..valid.clone()
    };
    assert_eq!(
        api.new_order(&unknown_pair).await,
        Err(KrakenError::Api {
            status: 200,
            body: "EQuery:Unknown asset pair".into()
        })
    );

    // refused locally, without calling Kraken
    let gtd_without_expiry = NewOrder {
        time_in_force: Some(TimeInForce::Gtd),
        ..valid.clone()
    };
    assert!(matches!(
        api.new_order(&gtd_without_expiry).await,
        Err(KrakenError::InvalidArgument { .. })
    ));
}

#[derive(Serialize)]
struct TradesParams {
    ledgers: bool,
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn trades_history_is_consistent_and_paginated() {
    let raw: Value = private_client()
        .call_with("TradesHistory", Some(&TradesParams { ledgers: true }))
        .await
        .unwrap();
    let mut presence = Presence::default();
    for (txid, value) in entries(&raw["trades"]) {
        presence.record(value);
        let trade: Trade = decode_strict(&format!("TradesHistory {txid}"), value);
        assert!(
            trade.volume > Decimal::ZERO && trade.fee >= Decimal::ZERO,
            "{txid}"
        );
        // cost = price × volume, to the decimals of the pair
        let cost = trade.price * trade.volume;
        assert!(
            (trade.cost - cost).abs() <= trade.cost * dec!(0.0001) + dec!(0.01),
            "{txid}: {} vs {cost}",
            trade.cost
        );
    }
    presence.print("TradesHistory");

    let api = private_api();
    let request = |cursor: Option<String>| TradesHistoryRequest {
        limit: Some(2),
        with_cursor: Some(true),
        cursor,
        ..Default::default()
    };
    let first = api.trades_history(&request(None)).await.unwrap();
    assert!(first.trades.len() <= 2);
    assert!(first.trades.windows(2).all(|it| it[0].time >= it[1].time));
    if let Some(next) = first.cursor.and_then(|it| it.next) {
        let second = api.trades_history(&request(Some(next))).await.unwrap();
        let first_ids: HashSet<_> = first.trades.iter().map(|it| &it.txid).collect();
        assert!(second.trades.iter().all(|it| !first_ids.contains(&it.txid)));
        if let (Some(newest), Some(oldest)) = (second.trades.first(), first.trades.last()) {
            assert!(newest.time <= oldest.time);
        }
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn query_trades_finds_the_trades_of_the_history() {
    let api = private_api();
    let history = api.trades_history(&Default::default()).await.unwrap();
    let txids: Vec<String> = history
        .trades
        .iter()
        .take(20)
        .map(|it| it.txid.clone())
        .collect();
    if txids.is_empty() {
        println!("no trade on the account: nothing to query");
        return;
    }

    #[derive(Serialize)]
    struct Params {
        txid: String,
        ledgers: bool,
    }
    let raw: Value = private_client()
        .call_with(
            "QueryTrades",
            Some(&Params {
                txid: txids.join(","),
                ledgers: true,
            }),
        )
        .await
        .unwrap();
    for (txid, value) in entries(&raw) {
        let _: Trade = decode_strict(&format!("QueryTrades {txid}"), value);
    }

    let queried = api
        .query_trades(&QueryTradesRequest {
            txids: txids.clone(),
            trades: None,
            ledgers: None,
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(queried.len(), txids.len());
    for trade in &queried {
        let original = history
            .trades
            .iter()
            .find(|it| it.txid == trade.txid)
            .unwrap();
        assert_eq!(trade, original);
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn order_amends_start_with_the_original_order() {
    let closed = private_api()
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap();
    let Some(order) = closed.orders.first() else {
        println!("no closed order on the account: no amends to query");
        return;
    };

    #[derive(Serialize)]
    struct Params<'a> {
        order_id: &'a str,
    }
    let raw: Value = private_client()
        .call_with(
            "OrderAmends",
            Some(&Params {
                order_id: &order.txid,
            }),
        )
        .await
        .unwrap();
    let amends: OrderAmends = decode_strict("OrderAmends", &raw);
    assert!(amends.count >= 1, "{amends:?}");
    assert_eq!(
        amends.amends[0].amend_type,
        AmendType::Original,
        "{amends:?}"
    );
    // the current volume of the order is the one of its last version
    assert_eq!(
        amends.amends.last().unwrap().order_quantity,
        order.volume,
        "{amends:?}"
    );
}

#[derive(Serialize)]
struct PositionsParams {
    docalcs: bool,
}

#[derive(Serialize)]
struct ConsolidatedParams {
    docalcs: bool,
    consolidation: &'static str,
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn open_positions_are_decoded_strictly() {
    let raw: Value = private_client()
        .call_with("OpenPositions", Some(&PositionsParams { docalcs: true }))
        .await
        .unwrap();
    for (txid, value) in entries(&raw) {
        let position: Position = decode_strict(&format!("OpenPositions {txid}"), value);
        assert!(position.volume_closed <= position.volume, "{txid}");
    }

    let raw: Value = private_client()
        .call_with(
            "OpenPositions",
            Some(&ConsolidatedParams {
                docalcs: true,
                consolidation: "market",
            }),
        )
        .await
        .unwrap();
    // an empty object without positions
    let consolidated = raw.as_array().cloned().unwrap_or_default();
    for value in &consolidated {
        let position: ConsolidatedPosition = decode_strict("OpenPositions by market", value);
        assert!(position.positions > 0, "{position:?}");
    }
    assert_eq!(
        private_api()
            .open_positions_by_market(Some(true))
            .await
            .unwrap()
            .len(),
        consolidated.len()
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn websockets_token_with_the_permission() {
    let Some(raw) = live::unless_denied(
        "GetWebSocketsToken",
        private_client().call::<Value>("GetWebSocketsToken").await,
    ) else {
        return;
    };
    let token: WebSocketsToken = decode_strict("GetWebSocketsToken", &raw);
    assert!(!token.token.is_empty());
    assert!(token.expires > 0 && token.expires <= 900, "{token:?}");
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn cancel_an_unknown_order_is_refused() {
    // no order of the account is touched: this txid doesn't exist
    let answer = private_api()
        .cancel_order(&OrderReference::Txid {
            txid: "OAAAAA-AAAAA-AAAAAA".into(),
        })
        .await;
    match answer {
        Err(KrakenError::Api { body, .. }) => assert!(body.starts_with("EOrder:"), "{body}"),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn cancel_a_batch_of_unknown_orders() {
    // no order of the account is touched: these orders don't exist. Kraken reading the shape of
    // the batch, its error is about the orders, not about the arguments
    let answer = private_api()
        .cancel_order_batch(&[
            OrderReference::Txid {
                txid: "OAAAAA-AAAAA-AAAAAA".into(),
            },
            OrderReference::Txid {
                txid: "OBBBBB-BBBBB-BBBBBB".into(),
            },
        ])
        .await;
    match answer {
        // nothing to cancel
        Ok(cancelled) => assert_eq!(cancelled.count, 0, "{cancelled:?}"),
        Err(KrakenError::Api { body, .. }) => {
            assert!(!body.contains("Invalid arguments"), "{body}");
            assert!(body.starts_with("EOrder:"), "{body}");
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn amend_an_unknown_order_is_refused() {
    // no order of the account is touched: this txid doesn't exist
    let answer = private_api()
        .amend_order(&AmendOrder {
            txid: Some("OAAAAA-AAAAA-AAAAAA".into()),
            order_qty: Some("1".into()),
            ..Default::default()
        })
        .await;
    match answer {
        Err(KrakenError::Api { body, .. }) => assert!(body.starts_with("EOrder:"), "{body}"),
        other => panic!("unexpected {other:?}"),
    }
}
