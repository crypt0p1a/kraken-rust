//! Every subaccount call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, Signed, ok, private_api};
use kraken_api::subaccounts::NewSubaccount;
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
