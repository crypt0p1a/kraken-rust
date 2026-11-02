//! Contract of the Earn endpoints, against the real Kraken API, read only: no fund is allocated
//! or deallocated.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_earn -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::Decimal;
use kraken_api::earn::{EarnAllocations, EarnStrategy, LockType};
use live::{Presence, decode_strict, private_client};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct Page {
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

/// Every strategy available to the account, page after page
pub async fn every_strategy() -> Vec<Value> {
    let mut strategies = vec![];
    let mut cursor = None;
    loop {
        let raw: Value = private_client()
            .call_json("Earn/Strategies", &Page { cursor })
            .await
            .unwrap();
        strategies.extend(raw["items"].as_array().unwrap().iter().cloned());
        match raw["next_cursor"].as_str() {
            Some(next) if !next.is_empty() => cursor = Some(next.to_string()),
            _ => return strategies,
        }
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn earn_strategies_are_decoded_strictly() {
    let mut presence = Presence::default();
    let strategies = every_strategy().await;
    for value in &strategies {
        presence.record(value);
        let strategy: EarnStrategy = decode_strict("Earn/Strategies", value);
        assert_ne!(strategy.lock_type, LockType::Unknown, "{value}");
        assert!(!strategy.asset.is_empty(), "{value}");
    }
    println!("{} Earn strategies", strategies.len());
    presence.print("Earn/Strategies");
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn earn_allocations_are_decoded_strictly() {
    let raw: Value = private_client().call("Earn/Allocations").await.unwrap();
    let allocations: EarnAllocations = decode_strict("Earn/Allocations", &raw);
    assert!(!allocations.converted_asset.is_empty());
    for allocation in &allocations.items {
        let amounts = &allocation.amount_allocated;
        assert!(amounts.total.native >= Decimal::ZERO, "{allocation:?}");
        assert!(
            allocation.total_rewarded.native >= Decimal::ZERO,
            "{allocation:?}"
        );
    }
    println!("{} Earn allocations", allocations.items.len());
}
