//! Every algorithmic trading call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, Signed, ok, private_api};
use kraken_api::algo::{ClosedStrategiesRequest, NewStrategy, StrategyStatus, StrategyType};
use kraken_api::trading::{OrderSide, OrderType};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

/// A TWAP as answered by Kraken
fn twap(id: &str, status: &str) -> serde_json::Value {
    json!({"strategy_id": id, "status": status, "details": {"strategy_type": "twap",
        "side": "buy", "volume": "0.01000000", "filled_volume": "0", "avg_price": "",
        "total_cost": "0", "total_fee": "0", "starttm": "1781169958", "expiretm": "1781173558",
        "symbol": "BTC/USD", "asset_class": "forex", "ordertype": "market"}})
}

#[tokio::test]
async fn open_strategies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/GetOpenStrategies"))
        .and(Signed)
        .and(FormIs(vec![("strategy_type", "twap")]))
        .respond_with(ok(
            json!({"open": {"ASPBVCI-6MKBA-QE4VF6": twap("ASPBVCI-6MKBA-QE4VF6", "pending")}}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let strategies = private_api(&server)
        .open_strategies(Some(StrategyType::Twap))
        .await
        .unwrap();
    assert_eq!(strategies[0].status, StrategyStatus::Pending);
}

#[tokio::test]
async fn query_strategies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/QueryStrategies"))
        .and(Signed)
        .and(FormIs(vec![(
            "strategy_ids",
            "ASPBVCI-6MKBA-QE4VF6,AS-UNKNOWN",
        )]))
        .respond_with(ok(json!({"strategies": {
            "ASPBVCI-6MKBA-QE4VF6": twap("ASPBVCI-6MKBA-QE4VF6", "open")
        }})))
        .expect(1)
        .mount(&server)
        .await;

    let strategies = private_api(&server)
        .query_strategies(&["ASPBVCI-6MKBA-QE4VF6".into(), "AS-UNKNOWN".into()])
        .await
        .unwrap();
    assert_eq!(strategies.len(), 1, "the unknown id is omitted");
    assert_eq!(strategies[0].status, StrategyStatus::Open);
}

#[tokio::test]
async fn closed_strategies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/GetClosedStrategies"))
        .and(Signed)
        .and(FormIs(vec![("status", "expired"), ("limit", "1")]))
        .respond_with(ok(
            json!({"closed": [twap("ASPBVCI-6MKBA-QE4VF6", "expired")],
            "next_cursor": "abc"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .closed_strategies(&ClosedStrategiesRequest {
            status: Some(StrategyStatus::Expired),
            limit: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.strategies[0].status, StrategyStatus::Expired);
    assert_eq!(page.next_cursor.as_deref(), Some("abc"));
}

fn twap_limit(validate: Option<bool>) -> NewStrategy {
    NewStrategy {
        strategy_type: StrategyType::Twap,
        symbol: "BTC/USD".into(),
        asset_class: "forex".into(),
        side: OrderSide::Buy,
        volume: "0.01".into(),
        order_type: OrderType::Limit,
        price: Some("60000.0".into()),
        start_time: None,
        expire_time: "+3600".into(),
        validate,
    }
}

#[tokio::test]
async fn add_strategy() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/AddStrategy"))
        .and(Signed)
        .and(FormIs(vec![
            ("strategy_type", "twap"),
            ("symbol", "BTC/USD"),
            ("asset_class", "forex"),
            ("side", "buy"),
            ("volume", "0.01"),
            ("ordertype", "limit"),
            ("price", "60000.0"),
            ("expiretm", "+3600"),
        ]))
        .respond_with(ok(json!({"strategy_id": "ASMBA64-R23M2-J7JRJV"})))
        .expect(1)
        .mount(&server)
        .await;

    let id = private_api(&server)
        .add_strategy(&twap_limit(None))
        .await
        .unwrap();
    assert_eq!(id, "ASMBA64-R23M2-J7JRJV");
}

#[tokio::test]
async fn cancel_strategy() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CancelStrategy"))
        .and(Signed)
        .and(FormIs(vec![("strategy_id", "ASMBA64-R23M2-J7JRJV")]))
        .respond_with(ok(json!({"strategy_id": "ASMBA64-R23M2-J7JRJV"})))
        .expect(1)
        .mount(&server)
        .await;

    let id = private_api(&server)
        .cancel_strategy("ASMBA64-R23M2-J7JRJV")
        .await
        .unwrap();
    assert_eq!(id, "ASMBA64-R23M2-J7JRJV");
}
