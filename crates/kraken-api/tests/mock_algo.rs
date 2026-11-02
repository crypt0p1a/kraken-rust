//! Every algorithmic trading call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, Signed, ok, private_api};
use kraken_api::algo::{StrategyStatus, StrategyType};
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
