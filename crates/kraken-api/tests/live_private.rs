//! Calls the real Kraken private API, read only. Requires KRAKEN_API_KEY and KRAKEN_API_SECRET
//! (with the Query Funds, Query Open/Closed Orders and Create & Modify Orders permissions), from
//! the environment or from the `.env` file at the root of the repository (see `.env.example`):
//!
//! `KRAKEN_API_KEY=… KRAKEN_API_SECRET=… cargo test -p kraken-api --test live_private -- --ignored --test-threads=1`
//!
//! The order is only validated (`validate=true`), never placed.

use kraken_api::trading::{
    ClosedOrdersRequest, NewOrder, OpenOrdersRequest, OrderSide, OrderType, QueryOrdersRequest,
};
use kraken_api::{KrakenError, PrivateApi, PublicApi, RateLimiter, RestOptions};
use std::sync::{Arc, LazyLock};

static RATE_LIMITER: LazyLock<Arc<RateLimiter>> = LazyLock::new(Default::default);

/// The variable from the environment, or else from the `.env` file at the root of the repository
fn env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .or_else(|| from_dotenv(name))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("{name} is required by the live private tests, see .env.example"))
}

fn from_dotenv(name: &str) -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let content = std::fs::read_to_string(path).ok()?;

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == name)
        .map(|(_, value)| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
}

fn api() -> PrivateApi {
    PrivateApi::new(
        env("KRAKEN_API_KEY"),
        env("KRAKEN_API_SECRET"),
        RestOptions::default(),
        RATE_LIMITER.clone(),
    )
    .unwrap()
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn rejects_an_unknown_key() {
    // no credentials required: checks the signed request reaches Kraken and its error is mapped
    let api = PrivateApi::new(
        "unknown".into(),
        "c2VjcmV0".into(),
        RestOptions::default(),
        RATE_LIMITER.clone(),
    )
    .unwrap();

    match api.extended_balance().await {
        Err(KrakenError::Api { body, .. }) => assert!(body.starts_with("EAPI:"), "{body}"),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn balance_and_fees() {
    let api = api();

    api.extended_balance().await.unwrap();

    let volume = api.trade_volume(&["XBTUSD".into()]).await.unwrap();
    assert!(volume.fees.contains_key("XXBTZUSD"));
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn orders() {
    let api = api();

    api.open_orders(&OpenOrdersRequest::default())
        .await
        .unwrap();
    let closed = api
        .closed_orders(&ClosedOrdersRequest::default())
        .await
        .unwrap();

    if let Some(first) = closed.orders.first() {
        let queried = api
            .query_orders(&QueryOrdersRequest {
                txids: vec![first.txid.clone()],
                trades: None,
                userref: None,
            })
            .await
            .unwrap();
        assert_eq!(queried[0].txid, first.txid);
        assert_eq!(
            api.order(&first.txid).await.unwrap().unwrap().txid,
            first.txid
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn validates_a_new_order() {
    let public_api = PublicApi::new(RestOptions::default(), RATE_LIMITER.clone()).unwrap();
    let pair = public_api.asset_pairs(&["XBTUSD".into()]).await.unwrap()["XXBTZUSD"].clone();
    let ticker = public_api.tickers(&["XBTUSD".into()]).await.unwrap()["XXBTZUSD"].clone();

    // far below the market, at the minimum volume
    let price = format!(
        "{:.1$}",
        ticker.bid.price / 2.0,
        pair.pair_decimals as usize
    );
    let answer = api()
        .new_order(&NewOrder {
            pair: "XBTUSD".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            volume: format!("{:.1$}", pair.ordermin.unwrap(), pair.lot_decimals as usize),
            price: Some(price),
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

    assert!(
        answer.txids.is_empty(),
        "the order was placed: {:?}",
        answer.txids
    );
}
