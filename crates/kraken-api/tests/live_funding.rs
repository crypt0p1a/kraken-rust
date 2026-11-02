//! Contract of the funding endpoints (Funding Beta), against the real Kraken API, read only: no
//! address is claimed, saved or deleted, no fund is withdrawn. Skipped without the "Funds
//! permissions - Query" permission.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_funding -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::funding::{FundingAssets, FundingDirection, FundingMethods};
use kraken_network::FundingMethod;
use live::{decode_strict, funding_client, private_api, unless_denied};
use serde_json::Value;

/// The raw answer of a GET of the funding API, None without its permission
async fn get(path: &str) -> Option<Value> {
    unless_denied(
        path,
        funding_client()
            .call(FundingMethod::Get, path, None::<&()>, None::<&()>)
            .await,
    )
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn funding_assets_of_both_directions() {
    for direction in ["deposit", "withdraw"] {
        let Some(raw) = get(&format!("/funding/v1/assets/{direction}")).await else {
            return;
        };
        let assets: FundingAssets = decode_strict(&format!("funding assets {direction}"), &raw);
        assert!(!assets.currency.is_empty(), "{direction}: {raw}");
    }
    let typed = private_api()
        .funding_assets(FundingDirection::Withdraw, None, None)
        .await
        .unwrap();
    assert!(!typed.currency.is_empty());
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn funding_methods_of_both_directions() {
    for direction in ["deposit", "withdraw"] {
        let Some(raw) = get(&format!("/funding/v1/methods/{direction}")).await else {
            return;
        };
        let methods: FundingMethods = decode_strict(&format!("funding methods {direction}"), &raw);
        assert!(!methods.methods.is_empty(), "{direction}");
        for method in &methods.methods {
            // the details of a direction only come with its methods
            assert!(
                method.deposit.is_none() || direction == "deposit",
                "{method:?}"
            );
            assert!(!method.withdrawal || direction == "withdraw", "{method:?}");
        }
    }
}
