//! Every account call against a mocked Kraken server: signed request, answer decoded.
//! The nonces, signatures and error mapping themselves are tested by kraken-network

mod common;

use common::{FormIs, Signed, ok, private_api};
use kraken_api::account::{
    ApiKeyInfoRequest, BalanceRequest, RebaseMultiplier, TradeBalanceRequest,
};
use rust_decimal_macros::dec;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

#[tokio::test]
async fn api_key_info() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/GetApiKeyInfo"))
        .and(Signed)
        .and(FormIs(vec![("otp", "123456")]))
        .respond_with(ok(
            json!({"api_key_name": "bot", "api_key": "key", "nonce": "17",
            "permissions": ["query-funds", "modify-trades"], "ip_allowlist": []}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let info = private_api(&server)
        .api_key_info(&ApiKeyInfoRequest {
            otp: Some("123456".into()),
        })
        .await
        .unwrap();
    assert_eq!(info.permissions, ["query-funds", "modify-trades"]);
}

#[tokio::test]
async fn balance() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Balance"))
        .and(Signed)
        .and(FormIs(vec![("rebase_multiplier", "rebased")]))
        .respond_with(ok(json!({"ZUSD": "2970172.7962", "XXBT": "0.0000000001"})))
        .expect(1)
        .mount(&server)
        .await;

    let balances = private_api(&server)
        .balance(&BalanceRequest {
            rebase_multiplier: Some(RebaseMultiplier::Rebased),
        })
        .await
        .unwrap();
    assert_eq!(balances["ZUSD"], dec!(2970172.7962));
    assert_eq!(balances["XXBT"].to_string(), "0.0000000001");
}

#[tokio::test]
async fn credit_lines() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CreditLines"))
        .and(Signed)
        .and(FormIs(vec![]))
        .respond_with(ok(json!({
            "asset_details": {"ZUSD": {"balance": "1000.5", "hold_trade": "0", "credit": "50000"}},
            "limits_monitor": {"total_credit_usd": "50000.00", "equity_usd": "110000.00"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lines = private_api(&server)
        .credit_lines(&Default::default())
        .await
        .unwrap();
    assert_eq!(lines.asset_details["ZUSD"].credit, Some(dec!(50000)));
    assert_eq!(lines.limits_monitor.equity_usd, Some(dec!(110000)));
}

#[tokio::test]
async fn extended_balance() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/BalanceEx"))
        .and(Signed)
        .and(FormIs(vec![]))
        .respond_with(ok(json!({
            "ZUSD": {"balance": "25435.21", "hold_trade": "8249.76", "credit": "0", "credit_used": "0"},
            "XXBT": {"balance": "1.2"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let balances = private_api(&server).extended_balance().await.unwrap();
    assert_eq!(balances["ZUSD"].available(), dec!(17185.45));
    assert_eq!(balances["XXBT"].hold_trade, None);
}

#[tokio::test]
async fn trade_balance() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/TradeBalance"))
        .and(Signed)
        .and(FormIs(vec![("asset", "ZEUR")]))
        .respond_with(ok(
            json!({"eb": "100.5", "tb": "90", "m": "10", "n": "-1.5", "c": "50",
            "v": "48.5", "e": "88.5", "mf": "78.5", "mfo": "70", "ml": "885.00", "uv": "0"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let balance = private_api(&server)
        .trade_balance(&TradeBalanceRequest {
            asset: Some("ZEUR".into()),
            rebase_multiplier: None,
        })
        .await
        .unwrap();
    assert_eq!(balance.unrealized_net, dec!(-1.5));
    assert_eq!(balance.margin_level, Some(dec!(885)));
}

#[tokio::test]
async fn trade_volume() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/TradeVolume"))
        .and(Signed)
        .and(FormIs(vec![("pair", "XBTUSD,ETHUSD")]))
        .respond_with(ok(json!({
            "currency": "ZUSD", "volume": "10000.5",
            "fees": {"XXBTZUSD": {"fee": "0.2600", "nextfee": "0.2400", "nextvolume": "50000"}},
            "fees_maker": {"XXBTZUSD": {"fee": "0.1600", "nextfee": null, "nextvolume": null}}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let volume = private_api(&server)
        .trade_volume(&["XBTUSD".into(), "ETHUSD".into()])
        .await
        .unwrap();
    assert_eq!(volume.volume, dec!(10000.5));
    assert_eq!(volume.fees["XXBTZUSD"].fee, dec!(0.26));
    assert_eq!(volume.fees_maker["XXBTZUSD"].nextfee, None);
}
