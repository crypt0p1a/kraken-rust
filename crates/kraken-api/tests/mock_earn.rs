//! Every Earn call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{JsonIs, SignedJson, ok, private_api};
use kraken_api::earn::{EarnStrategiesRequest, LockType, LockTypeFilter};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

#[tokio::test]
async fn earn_strategies_are_signed_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/Strategies"))
        .and(SignedJson)
        .and(JsonIs(json!({"asset": "DOT", "lock_type": ["instant"]})))
        .respond_with(ok(
            json!({"items": [{"id": "ESRFUO3-Q62XD-WIOIL7", "asset": "DOT",
            "lock_type": {"type": "instant", "payout_frequency": 604800},
            "can_allocate": true, "can_deallocate": true}], "next_cursor": null}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let page = private_api(&server)
        .earn_strategies(&EarnStrategiesRequest {
            asset: Some("DOT".into()),
            lock_type: vec![LockTypeFilter::Instant],
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(matches!(page.items[0].lock_type, LockType::Instant { .. }));
    assert_eq!(page.next_cursor, None);
}
