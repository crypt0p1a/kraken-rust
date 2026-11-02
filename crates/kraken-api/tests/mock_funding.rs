//! Every funding call against a mocked Kraken server: signed request (query string and JSON body),
//! answer decoded

mod common;

use common::{QueryIs, SignedFunding, plain, private_api};
use kraken_api::funding::{FundingDirection, FundingMethodsRequest};
use kraken_api::market::AssetClass;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

#[tokio::test]
async fn funding_assets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/assets/withdraw"))
        .and(SignedFunding)
        .and(QueryIs(vec![
            ("asset_class", "currency"),
            ("account_id", "WX6V-JUKW-KKPB-QE36"),
        ]))
        .respond_with(plain(
            json!({"currency": [{"name": "USDC"}, {"name": "BTC"}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let assets = private_api(&server)
        .funding_assets(
            FundingDirection::Withdraw,
            Some(AssetClass::Currency),
            Some("WX6V-JUKW-KKPB-QE36"),
        )
        .await
        .unwrap();
    assert_eq!(assets.currency.len(), 2);
    assert!(assets.tokenized_asset.is_empty());
}

#[tokio::test]
async fn funding_methods() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/methods/deposit"))
        .and(SignedFunding)
        .and(QueryIs(vec![
            ("asset[class]", "currency"),
            ("asset[name]", "USDC"),
        ]))
        .respond_with(plain(
            json!({"methods": [{"asset": {"class": "currency", "name": "USDC"},
            "method_id": "27ede8db", "method_name": "Standard",
            "fees": {"base": {"asset": {"class": "currency", "name": "USDC"}, "amount": "0"},
            "included": true}}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let api = private_api(&server);
    let methods = api
        .funding_methods(
            FundingDirection::Deposit,
            &FundingMethodsRequest {
                asset_class: Some(AssetClass::Currency),
                asset_name: Some("USDC".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(methods.methods[0].method_id, "27ede8db");
    assert_eq!(methods.next_cursor, None);

    // a name without its class: refused before any request
    let without_class = FundingMethodsRequest {
        asset_name: Some("USDC".into()),
        ..Default::default()
    };
    assert!(matches!(
        api.funding_methods(FundingDirection::Deposit, &without_class)
            .await,
        Err(kraken_api::KrakenError::InvalidArgument { .. })
    ));
}

#[tokio::test]
async fn funding_networks() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/networks"))
        .and(SignedFunding)
        .and(QueryIs(vec![]))
        .respond_with(plain(
            json!({"networks": [{"network_id": "n1", "name": "Ethereum"}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let networks = private_api(&server).funding_networks(None).await.unwrap();
    assert_eq!(networks.networks[0].name, "Ethereum");
    assert!(networks.network_groups.is_empty());
}
