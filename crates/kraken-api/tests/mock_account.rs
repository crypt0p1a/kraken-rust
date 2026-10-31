//! Every account call against a mocked Kraken server: signed request, answer decoded.
//! The nonces, signatures and error mapping themselves are tested by kraken-network

mod common;

use common::{FormIs, Signed, ok, private_api};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

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
    assert!((balances["ZUSD"].available() - 17185.45).abs() < 1e-9);
    assert_eq!(balances["XXBT"].hold_trade, None);
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
    assert_eq!(volume.volume, 10000.5);
    assert_eq!(volume.fees["XXBTZUSD"].fee, 0.26);
    assert_eq!(volume.fees_maker["XXBTZUSD"].nextfee, None);
}
