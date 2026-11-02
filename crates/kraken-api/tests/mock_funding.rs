//! Every funding call against a mocked Kraken server: signed request (query string and JSON body),
//! answer decoded

mod common;

use common::{QueryIs, SignedFunding, plain, private_api};
use kraken_api::funding::FundingDirection;
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
