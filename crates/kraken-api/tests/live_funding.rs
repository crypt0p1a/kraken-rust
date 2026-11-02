//! Contract of the funding endpoints (Funding Beta), against the real Kraken API, read only: no
//! address is claimed, saved or deleted, no fund is withdrawn. Skipped without the "Funds
//! permissions - Query" permission.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_funding -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::funding::{
    ClaimedAddresses, DepositLimits, FundingAssets, FundingDirection, FundingFee, FundingMethods,
    FundingNetworks, WithdrawalLimits,
};
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

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn funding_networks() {
    let Some(raw) = get("/funding/v1/networks").await else {
        return;
    };
    let networks: FundingNetworks = decode_strict("funding networks", &raw);
    assert!(!networks.networks.is_empty());
}

#[derive(serde::Serialize)]
struct Amount {
    amount: String,
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn funding_fee_of_a_withdrawal_method() {
    // a quote only: nothing is withdrawn
    let Some(raw) = get("/funding/v1/methods/withdraw").await else {
        return;
    };
    let methods: FundingMethods = decode_strict("funding methods", &raw);
    let method = &methods.methods[0];
    let amount = method
        .minimum_amount
        .unwrap_or(kraken_api::Decimal::ONE)
        .to_string();
    let raw: Value = funding_client()
        .call(
            FundingMethod::Get,
            &format!("/funding/v1/fees/{}", method.method_id),
            Some(&Amount { amount }),
            None::<&()>,
        )
        .await
        .unwrap();
    let fee: FundingFee = decode_strict("funding fee", &raw);
    assert_eq!(
        fee.gross_amount.amount - fee.fee.amount,
        fee.net_amount.amount,
        "{fee:?}"
    );
    assert!(
        fee.withdrawal_fee_token.is_some(),
        "a withdrawal has a fee token: {fee:?}"
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn deposit_limits_of_usdc() {
    let Some(raw) = get("/funding/v1/limits/deposit/currency/USDC").await else {
        return;
    };
    for value in raw["deposit_limits"].as_array().unwrap() {
        let limits: DepositLimits = decode_strict("deposit limits", value);
        assert!(!limits.method_id.is_empty());
        for timed in &limits.limits {
            assert!(timed.time_window > 0, "{timed:?}");
        }
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn withdrawal_limits_of_usdc() {
    let Some(raw) = get("/funding/v1/limits/withdrawal/currency/USDC").await else {
        return;
    };
    let limits: WithdrawalLimits = decode_strict("withdrawal limits", &raw);
    assert!(limits.available_balance.amount >= kraken_api::Decimal::ZERO);
    for method in &limits.withdrawal_limits {
        assert!(
            ["balance", "limits"].contains(&method.maximum_reason.as_str()),
            "{method:?}"
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn claimed_deposit_addresses() {
    let Some(raw) = get("/funding/v2/deposit/addresses").await else {
        return;
    };
    let page: ClaimedAddresses = decode_strict("claimed addresses", &raw);
    println!("{} claimed deposit addresses", page.addresses.len());
}
