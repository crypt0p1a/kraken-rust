//! Contract of the private account endpoints, against the real Kraken API, read only.
//! Credentials: KRAKEN_API_KEY / KRAKEN_API_SECRET, from the environment or `.env`.
//! `cargo test -p kraken-spot-api --test live_account -- --ignored --test-threads=1 --nocapture`

mod live;
mod strict;

use kraken_spot_api::Decimal;
use kraken_spot_api::account::{
    ApiKeyInfo, Export, ExportReport, ExportStatus, ExtendedBalance, FeeInfo, LedgerEntry,
    LedgerType, QueryLedgersRequest, TradeBalance, TradeVolume, WalletAccounts, WalletType,
};
use kraken_spot_api::market::Level3OrderBook;
use kraken_spot_api::{KrakenError, PrivateApi, RestOptions};
use live::{Presence, decode_strict, entries, env, le, private_api, private_client, unless_denied};
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

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn level3_order_book() {
    #[derive(Serialize)]
    struct Params<'a> {
        pair: &'a str,
        depth: u32,
    }
    let raw: Value = private_client()
        .call_with(
            "Level3",
            Some(&Params {
                pair: "XBTUSD",
                depth: 10,
            }),
        )
        .await
        .unwrap();
    let book: Level3OrderBook = decode_strict("Level3", &raw);
    assert!(!book.bids.is_empty() && !book.asks.is_empty());
    assert!(book.bids[0].price < book.asks[0].price, "crossed book");
    assert!(
        book.bids
            .iter()
            .chain(&book.asks)
            .all(|it| it.quantity > Decimal::ZERO && !it.order_id.is_empty())
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn balance_matches_the_extended_balance() {
    let raw: Value = private_client().call("Balance").await.unwrap();
    // exact decimals sent as strings
    for (asset, amount) in entries(&raw) {
        let amount = amount
            .as_str()
            .unwrap_or_else(|| panic!("{asset}: {amount}"));
        let amount =
            Decimal::from_str_exact(amount).unwrap_or_else(|_| panic!("{asset}: {amount}"));
        assert!(amount >= Decimal::ZERO, "{asset}: {amount}");
    }

    let api = private_api();
    let balances = api.balance(&Default::default()).await.unwrap();
    let extended = api.extended_balance().await.unwrap();
    for (asset, amount) in &balances {
        assert_eq!(
            extended.get(asset).map(|it| it.balance),
            Some(*amount),
            "{asset}"
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn trade_balance() {
    let raw: Value = private_client().call("TradeBalance").await.unwrap();
    let balance: TradeBalance = decode_strict("TradeBalance", &raw);
    assert!(balance.equivalent_balance >= Decimal::ZERO, "{balance:?}");
    // rounded by Kraken to 4 decimals
    assert!(
        (balance.equity - balance.trade_balance - balance.unrealized_net).abs()
            <= Decimal::new(1, 4),
        "{balance:?}"
    );
    assert!(balance.unexecuted_value >= Decimal::ZERO, "{balance:?}");
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn api_key_info_never_allows_withdrawals() {
    let raw: Value = private_client().call("GetApiKeyInfo").await.unwrap();
    let info: ApiKeyInfo = decode_strict("GetApiKeyInfo", &raw);
    println!(
        "GetApiKeyInfo fields: {:?}",
        raw.as_object().unwrap().keys().collect::<Vec<_>>()
    );

    assert_eq!(info.api_key, env("KRAKEN_API_KEY"));
    assert!(!info.permissions.is_empty(), "{info:?}");
    // the key of the tests must never be able to move funds out of the account
    assert!(
        !info.permissions.iter().any(|it| it.contains("withdraw")),
        "the API key of the tests can withdraw: {:?}",
        info.permissions
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn wallet_accounts_hold_the_main_one() {
    let raw: Value = private_client().call("ListWalletAccounts").await.unwrap();
    let wallets: WalletAccounts = decode_strict("ListWalletAccounts", &raw);
    assert!(
        wallets
            .accounts
            .iter()
            .any(|it| it.wallet_type == WalletType::Main),
        "{wallets:?}"
    );
    assert!(wallets.accounts.iter().all(|it| !it.account_id.is_empty()));
}

#[derive(Serialize)]
struct LedgersParams {
    without_count: bool,
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn ledgers_with_the_permission() {
    let Some(raw) = unless_denied(
        "Ledgers",
        private_client()
            .call_with::<_, Value>(
                "Ledgers",
                Some(&LedgersParams {
                    without_count: false,
                }),
            )
            .await,
    ) else {
        return;
    };
    let mut presence = Presence::default();
    for (id, value) in entries(&raw["ledger"]) {
        presence.record(value);
        let entry: LedgerEntry = decode_strict(&format!("Ledgers {id}"), value);
        assert_ne!(entry.entry_type, LedgerType::Unknown, "{id}: {value}");
        assert!(entry.fee >= Decimal::ZERO, "{id}");
    }
    presence.print("Ledgers");

    let page = private_api().ledgers(&Default::default()).await.unwrap();
    assert!(page.entries.len() <= 50);
    assert!(page.entries.windows(2).all(|it| it[0].time >= it[1].time));
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn query_ledgers_finds_the_entries_of_the_ledgers() {
    let api = private_api();
    let Some(page) = unless_denied("QueryLedgers", api.ledgers(&Default::default()).await) else {
        return;
    };
    let ids: Vec<String> = page
        .entries
        .iter()
        .take(20)
        .map(|it| it.id.clone())
        .collect();
    if ids.is_empty() {
        println!("no ledger entry on the account: nothing to query");
        return;
    }
    let queried = api
        .query_ledgers(&QueryLedgersRequest {
            ids: ids.clone(),
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(queried.len(), ids.len());
    for entry in &queried {
        let original = page.entries.iter().find(|it| it.id == entry.id).unwrap();
        assert_eq!(entry.amount, original.amount);
        assert_eq!(entry.balance, original.balance);
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn export_status_with_the_permission() {
    #[derive(Serialize)]
    struct Params {
        report: &'static str,
    }
    for report in ["trades", "ledgers"] {
        let Some(raw) = unless_denied(
            "ExportStatus",
            private_client()
                .call_with::<_, Value>("ExportStatus", Some(&Params { report }))
                .await,
        ) else {
            return;
        };
        for value in raw.as_array().unwrap() {
            let export: Export = decode_strict(&format!("ExportStatus {report}"), value);
            assert_ne!(export.status, ExportStatus::Unknown, "{value}");
        }
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API with credentials"]
async fn retrieve_a_processed_export() {
    let api = private_api();
    for report in [ExportReport::Trades, ExportReport::Ledgers] {
        let Some(exports) = unless_denied("RetrieveExport", api.export_status(report).await) else {
            return;
        };
        let Some(export) = exports
            .iter()
            .find(|it| it.status == ExportStatus::Processed && it.error.is_none())
        else {
            continue;
        };
        let zip = api.retrieve_export(&export.id).await.unwrap();
        assert!(
            zip.starts_with(b"PK"),
            "not a zip archive: {:?}",
            &zip[..zip.len().min(16)]
        );
        return;
    }
    println!("RetrieveExport: no processed report on the account");
}
