//! Contract of the private account endpoints, against the real Kraken API, read only.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-api --test live_account -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_api::Decimal;
use kraken_api::account::{ExtendedBalance, FeeInfo, TradeVolume};
use kraken_api::{KrakenError, PrivateApi, RestOptions};
use live::{Presence, decode_strict, entries, le, private_api, private_client};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Serialize)]
struct PairParam<'a> {
    pair: &'a str,
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn rejects_an_unknown_key() {
    // no credentials required: a signed request reaches Kraken and its error is mapped
    let api = PrivateApi::new(
        "unknown".into(),
        "c2VjcmV0".into(),
        RestOptions::default(),
        Arc::new(Default::default()),
    )
    .unwrap();

    match api.extended_balance().await {
        Err(KrakenError::Api { body, .. }) => assert!(body.starts_with("EAPI:"), "{body}"),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn extended_balance_of_every_asset() {
    let raw: Value = private_client().call("BalanceEx").await.unwrap();
    let mut presence = Presence::default();

    for (asset, value) in entries(&raw) {
        presence.record(value);
        let balance: ExtendedBalance = decode_strict(&format!("BalanceEx {asset}"), value);

        assert!(balance.balance >= Decimal::ZERO, "{asset}: {balance:?}");
        let hold = balance.hold_trade.unwrap_or_default();
        assert!(hold >= Decimal::ZERO, "{asset}: {balance:?}");
        assert!(
            le(hold, balance.balance + balance.credit.unwrap_or_default()),
            "{asset}: more held than owned {balance:?}"
        );
        assert!(balance.available() >= Decimal::ZERO, "{asset}: {balance:?}");
    }

    presence.print("BalanceEx");
    presence.assert_always("BalanceEx", &["balance"]);

    let typed = private_api().extended_balance().await.unwrap();
    assert_eq!(typed.len(), presence.total());
}

fn check_fee(context: &str, fee: &FeeInfo) {
    assert!(
        (Decimal::ZERO..=Decimal::ONE_HUNDRED).contains(&fee.fee),
        "{context}: {fee:?}"
    );
    if let (Some(min), Some(max)) = (fee.minfee, fee.maxfee) {
        assert!(le(min, fee.fee) && le(fee.fee, max), "{context}: {fee:?}");
    }
    if let Some(next) = fee.nextfee {
        assert!(
            le(next, fee.fee),
            "{context}: the next tier is more expensive {fee:?}"
        );
    }
    if let (Some(tier), Some(next)) = (fee.tiervolume, fee.nextvolume) {
        assert!(tier < next, "{context}: {fee:?}");
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn trade_volume_and_fees_of_several_pairs() {
    let raw: Value = private_client()
        .call_with(
            "TradeVolume",
            Some(&PairParam {
                pair: "XBTUSD,ETHUSD,SOLEUR",
            }),
        )
        .await
        .unwrap();
    let volume: TradeVolume = decode_strict("TradeVolume", &raw);
    println!(
        "TradeVolume: {}",
        serde_json::to_string_pretty(&raw).unwrap().len()
    );

    assert!(volume.volume >= Decimal::ZERO);
    assert!(
        volume.currency.starts_with('Z') || volume.currency.len() == 3,
        "{}",
        volume.currency
    );

    let mut taker: Vec<_> = volume.fees.keys().cloned().collect();
    taker.sort();
    assert_eq!(taker, ["SOLEUR", "XETHZUSD", "XXBTZUSD"]);
    let mut maker: Vec<_> = volume.fees_maker.keys().cloned().collect();
    maker.sort();
    assert_eq!(maker, taker);

    for (pair, fee) in &volume.fees {
        check_fee(&format!("taker {pair}"), fee);
        check_fee(&format!("maker {pair}"), &volume.fees_maker[pair]);
        assert!(
            le(volume.fees_maker[pair].fee, fee.fee),
            "{pair}: maker above taker"
        );
        if let Some(tier) = fee.tiervolume {
            assert!(le(tier, volume.volume.max(tier)), "{pair}");
        }
    }

    if let Some(inputs) = &volume.inputs {
        for value in [
            inputs.domain_spot_volume_30d,
            inputs.domain_futures_volume_30d,
            inputs.domain_assets_on_platform,
        ]
        .into_iter()
        .flatten()
        {
            assert!(value >= Decimal::ZERO, "{inputs:?}");
        }
    }

    // same answer through the typed call
    let typed = private_api()
        .trade_volume(&["XBTUSD".into(), "ETHUSD".into(), "SOLEUR".into()])
        .await
        .unwrap();
    assert_eq!(typed.fees.len(), 3);
    assert_eq!(typed.currency, volume.currency);
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn trade_volume_without_pairs_has_no_fees() {
    let raw: Value = private_client().call("TradeVolume").await.unwrap();
    let volume: TradeVolume = decode_strict("TradeVolume", &raw);

    assert!(
        volume.fees.is_empty() && volume.fees_maker.is_empty(),
        "{volume:?}"
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn trade_volume_of_an_unknown_pair_is_an_error() {
    let error = private_api()
        .trade_volume(&["NOPEUSD".into()])
        .await
        .unwrap_err();
    assert!(
        matches!(&error, KrakenError::Api { body, .. } if body.starts_with("EQuery:")),
        "{error:?}"
    );
}
