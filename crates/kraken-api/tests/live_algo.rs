//! Contract of the algorithmic trading endpoints, against the real Kraken API: no strategy is
//! created or cancelled, the new ones are only validated (`validate=true`).
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_algo -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::algo::{
    ClosedStrategies, ClosedStrategiesRequest, NewStrategy, Strategy, StrategyStatus, StrategyType,
};
use kraken_api::trading::{OrderSide, OrderType};
use kraken_api::{Decimal, KrakenError};
use live::{decode_strict, entries, private_api, private_client};
use serde_json::Value;

fn check_strategy(context: &str, strategy: &Strategy) {
    let details = &strategy.details;
    assert!(
        strategy.strategy_id.starts_with("AS"),
        "{context}: {strategy:?}"
    );
    assert_ne!(
        strategy.status,
        StrategyStatus::Unknown,
        "{context}: {strategy:?}"
    );
    assert!(
        details.filled_volume <= details.volume,
        "{context}: {strategy:?}"
    );
    assert!(
        details.total_fee >= Decimal::ZERO,
        "{context}: {strategy:?}"
    );
    assert!(
        details.start_time < details.expire_time,
        "{context}: {strategy:?}"
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn open_strategies() {
    let raw: Value = private_client().call("GetOpenStrategies").await.unwrap();
    for (id, value) in entries(&raw["open"]) {
        let strategy: Strategy = decode_strict(&format!("GetOpenStrategies {id}"), value);
        check_strategy(id, &strategy);
        assert!(
            matches!(
                strategy.status,
                StrategyStatus::Pending | StrategyStatus::Open
            ),
            "{strategy:?}"
        );
    }
    let typed = private_api().open_strategies(None).await.unwrap();
    assert_eq!(typed.len(), raw["open"].as_object().unwrap().len());
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn query_strategies_finds_the_open_ones() {
    let api = private_api();
    let open = api.open_strategies(None).await.unwrap();
    let ids: Vec<String> = open
        .iter()
        .take(20)
        .map(|it| it.strategy_id.clone())
        .collect();
    // an unknown id is omitted
    let unknown = api
        .query_strategies(&["ASAAAAA-AAAAA-AAAAAA".into()])
        .await
        .unwrap();
    assert!(unknown.is_empty(), "{unknown:?}");
    if ids.is_empty() {
        println!("no open strategy on the account: nothing to query");
        return;
    }
    let queried = api.query_strategies(&ids).await.unwrap();
    assert_eq!(queried.len(), ids.len());
    for strategy in &queried {
        check_strategy(&strategy.strategy_id, strategy);
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn closed_strategies_are_paginated() {
    #[derive(serde::Serialize)]
    struct Params {
        limit: u32,
    }
    let raw: Value = private_client()
        .call_with("GetClosedStrategies", Some(&Params { limit: 2 }))
        .await
        .unwrap();
    let page: ClosedStrategies = decode_strict("GetClosedStrategies", &raw);
    for strategy in &page.strategies {
        check_strategy(&strategy.strategy_id, strategy);
        assert!(
            matches!(
                strategy.status,
                StrategyStatus::Closed | StrategyStatus::Expired | StrategyStatus::Cancelled
            ),
            "{strategy:?}"
        );
    }
    let Some(cursor) = page.next_cursor.clone() else {
        return;
    };
    let next = private_api()
        .closed_strategies(&ClosedStrategiesRequest {
            limit: Some(2),
            cursor: Some(cursor),
            ..Default::default()
        })
        .await
        .unwrap();
    for strategy in &next.strategies {
        assert!(
            page.strategies
                .iter()
                .all(|it| it.strategy_id != strategy.strategy_id)
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn validates_a_new_strategy() {
    // validated only: no strategy is created
    let id = private_api()
        .add_strategy(&NewStrategy {
            strategy_type: StrategyType::Twap,
            symbol: "BTC/USD".into(),
            asset_class: "forex".into(),
            side: OrderSide::Buy,
            volume: "0.001".into(),
            order_type: OrderType::Market,
            price: None,
            start_time: None,
            expire_time: "+3600".into(),
            validate: Some(true),
        })
        .await
        .unwrap();
    assert!(id.is_empty(), "a validated strategy has no id: {id}");
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn cancel_an_unknown_strategy_is_refused() {
    // no strategy of the account is touched: this id doesn't exist
    match private_api().cancel_strategy("ASAAAAA-AAAAA-AAAAAA").await {
        Err(KrakenError::Api { body, .. }) => assert!(body.starts_with('E'), "{body}"),
        other => panic!("unexpected {other:?}"),
    }
}
