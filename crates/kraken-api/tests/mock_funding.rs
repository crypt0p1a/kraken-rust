//! Every funding call against a mocked Kraken server: signed request (query string and JSON body),
//! answer decoded

mod common;

use common::{QueryIs, SignedFunding, plain, private_api};
use kraken_api::funding::{
    AddressDetails, ClaimedAddressesRequest, DepositsRequest, FundingAsset, FundingDirection,
    FundingFeeRequest, FundingLimitsRequest, FundingMethodsRequest, FundingScope,
    NewWithdrawalAddress, TransferStatus, WithdrawalAddressesRequest,
};
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

#[tokio::test]
async fn funding_fee() {
    let server = MockServer::start().await;
    let usdc =
        |amount: &str| json!({"asset": {"class": "currency", "name": "USDC"}, "amount": amount});
    Mock::given(method("GET"))
        .and(path(
            "/funding/v1/fees/27ede8db-804b-4d91-8e25-46b7b9668730",
        ))
        .and(SignedFunding)
        .and(QueryIs(vec![("amount", "5"), ("fee_included", "true")]))
        .respond_with(plain(json!({"fee": usdc("1"), "gross_amount": usdc("5"),
            "net_amount": usdc("4"), "fee_details": {"base_fee": usdc("1"),
            "fee_percentage": "0"}})))
        .expect(1)
        .mount(&server)
        .await;

    let fee = private_api(&server)
        .funding_fee(
            "27ede8db-804b-4d91-8e25-46b7b9668730",
            &FundingFeeRequest {
                amount: "5".into(),
                fee_included: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(fee.net_amount.amount.to_string(), "4");
    assert_eq!(fee.withdrawal_fee_token, None);
}

fn usdc() -> FundingAsset {
    FundingAsset {
        class: AssetClass::Currency,
        name: "USDC".into(),
    }
}

#[tokio::test]
async fn deposit_limits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/limits/deposit/currency/USDC"))
        .and(SignedFunding)
        .and(QueryIs(vec![("preferred_asset[name]", "EUR")]))
        .respond_with(plain(json!({"deposit_limits": [{"method_id": "742656ee",
            "limits": [{"time_window": "86400", "limit": {"limit_type": "success",
            "remaining": "9", "maximum": "10"}}]}]})))
        .expect(1)
        .mount(&server)
        .await;

    let limits = private_api(&server)
        .deposit_limits(
            &usdc(),
            &FundingLimitsRequest {
                preferred_asset_name: Some("EUR".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(limits[0].limits[0].limit.limit_type, "success");
}

#[tokio::test]
async fn withdrawal_limits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/limits/withdrawal/currency/USDC"))
        .and(SignedFunding)
        .and(QueryIs(vec![("account_id", "WX6V-JUKW-KKPB-QE36")]))
        .respond_with(plain(
            json!({"available_balance": {"asset": {"class": "currency",
            "name": "USDC"}, "amount": "100"}, "withdrawal_limits": []}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let limits = private_api(&server)
        .withdrawal_limits(
            &usdc(),
            &FundingLimitsRequest {
                account_id: Some("WX6V-JUKW-KKPB-QE36".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(limits.available_balance.amount.to_string(), "100");
}

#[tokio::test]
async fn claim_deposit_address() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/funding/v1/deposit/address"))
        .and(SignedFunding)
        .and(QueryIs(vec![]))
        .and(wiremock::matchers::body_json(
            json!({"method_id": "27ede8db"}),
        ))
        .respond_with(plain(
            json!({"address_details": {"crypto": {"address": "0x0d5cff23",
            "tag": "123"}}}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let address = private_api(&server)
        .claim_deposit_address("27ede8db", None)
        .await
        .unwrap();
    assert_eq!(
        address,
        AddressDetails::Crypto {
            address: "0x0d5cff23".into(),
            tag: Some("123".into()),
            memo: None,
        }
    );
}

#[tokio::test]
async fn claimed_deposit_addresses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v2/deposit/addresses"))
        .and(SignedFunding)
        .and(QueryIs(vec![
            ("scope[method_id]", "27ede8db"),
            ("limit", "10"),
        ]))
        .respond_with(plain(
            json!({"addresses": [{"id": "a1", "method_id": "27ede8db",
            "address_details": {"crypto": {"address": "0x0d5cff23"}}}], "next_cursor": "n"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .claimed_deposit_addresses(&ClaimedAddressesRequest {
            scope: Some(FundingScope::MethodId {
                method_id: "27ede8db".into(),
            }),
            limit: Some(10),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.addresses[0].id.as_deref(), Some("a1"));
    assert_eq!(page.next_cursor.as_deref(), Some("n"));
}

#[tokio::test]
async fn deposits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/deposits"))
        .and(SignedFunding)
        .and(QueryIs(vec![
            ("asset[class]", "currency"),
            ("start_time", "2026-07-01T00:00:00Z"),
            ("limit", "2"),
        ]))
        .respond_with(plain(
            json!({"deposits": [{"deposit_id": "FT1", "method_id": "m",
            "status": "pending"}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .deposits(&DepositsRequest {
            asset_class: Some(AssetClass::Currency),
            start_time: Some("2026-07-01T00:00:00Z".into()),
            limit: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.deposits[0].status, TransferStatus::Pending);
}

#[tokio::test]
async fn withdrawal_addresses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/funding/v1/addresses"))
        .and(SignedFunding)
        .and(QueryIs(vec![
            ("scope[network_group_id]", "f95acdb7"),
            ("limit", "20"),
        ]))
        .respond_with(plain(
            json!({"addresses": [{"address_id": "ABR6SXP-SF6CY-VJMONY",
            "scope": {"network_group_id": "f95acdb7"}, "verified": true}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .withdrawal_addresses(&WithdrawalAddressesRequest {
            scope: Some(FundingScope::NetworkGroupId {
                network_group_id: "f95acdb7".into(),
            }),
            limit: Some(20),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(page.addresses[0].verified);
}

#[tokio::test]
async fn create_withdrawal_address() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/funding/v1/addresses"))
        .and(SignedFunding)
        .and(wiremock::matchers::body_json(json!({"scope": {"method_id": "67b765fd"},
            "address_details": {"crypto": {"address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"}}})))
        .respond_with(plain(json!({"address_id": "AB7J4FF-BGM7G-V2JMIH", "verified": false})))
        .expect(1)
        .mount(&server)
        .await;

    let api = private_api(&server);
    let mut address = NewWithdrawalAddress {
        scope: FundingScope::MethodId {
            method_id: "67b765fd".into(),
        },
        address: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".into(),
        tag: None,
        memo: None,
        description: None,
    };
    let saved = api.create_withdrawal_address(&address, None).await.unwrap();
    assert_eq!(saved.address_id, "AB7J4FF-BGM7G-V2JMIH");
    assert!(!saved.verified);

    // a tag and a memo: refused before any request
    address.tag = Some("1".into());
    address.memo = Some("2".into());
    assert!(matches!(
        api.create_withdrawal_address(&address, None).await,
        Err(kraken_api::KrakenError::InvalidArgument { .. })
    ));
}
