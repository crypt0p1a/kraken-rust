//! Every subaccount call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, Signed, ok, private_api};
use kraken_spot_api::subaccounts::{AccountTransfer, NewSubaccount};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

#[tokio::test]
async fn create_subaccount() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/CreateSubaccount"))
        .and(Signed)
        .and(FormIs(vec![
            ("username", "bot-1"),
            ("email", "bot@example.com"),
        ]))
        .respond_with(ok(json!(true)))
        .expect(1)
        .mount(&server)
        .await;

    let created = private_api(&server)
        .create_subaccount(&NewSubaccount {
            username: "bot-1".into(),
            email: "bot@example.com".into(),
        })
        .await
        .unwrap();
    assert!(created);
}

#[tokio::test]
async fn account_transfer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/AccountTransfer"))
        .and(Signed)
        .and(FormIs(vec![
            ("asset", "XBT"),
            ("amount", "0.5"),
            ("from", "ABCD 1234 EFGH 5678"),
            ("to", "IJKL 0987 MNOP 6543"),
        ]))
        .respond_with(ok(
            json!({"transfer_id": "TOH3AS2-LPCWR8-JDQGEU", "status": "pending"}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let result = private_api(&server)
        .account_transfer(&AccountTransfer {
            asset: "XBT".into(),
            asset_class: None,
            amount: "0.5".into(),
            from: "ABCD 1234 EFGH 5678".into(),
            to: "IJKL 0987 MNOP 6543".into(),
        })
        .await
        .unwrap();
    assert_eq!(result.status, "pending");
}
