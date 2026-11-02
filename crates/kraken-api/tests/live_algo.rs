//! Contract of the algorithmic trading endpoints, against the real Kraken API: no strategy is
//! created or cancelled, the new ones are only validated (`validate=true`).
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_algo -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::Decimal;
use kraken_api::algo::{Strategy, StrategyStatus};
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
