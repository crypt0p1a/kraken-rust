//! Every Earn call against a mocked Kraken server: signed request, answer decoded

mod common;

use common::{FormIs, JsonIs, Signed, SignedJson, ok, private_api};
use kraken_api::earn::{
    EarnAllocationsRequest, EarnFunds, EarnStrategiesRequest, LockType, LockTypeFilter,
};
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

#[tokio::test]
async fn earn_allocations() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/Allocations"))
        .and(Signed)
        .and(FormIs(vec![
            ("converted_asset", "EUR"),
            ("hide_zero_allocations", "true"),
        ]))
        .respond_with(ok(
            json!({"converted_asset": "EUR", "total_allocated": "10.5",
            "total_rewarded": "0.1", "items": [{"strategy_id": "ESDQCOL-WTZEU-NU55QF",
            "native_asset": "USDC", "amount_allocated": {"total": {"native": "11",
            "converted": "10.5"}}, "total_rewarded": {"native": "0.1", "converted": "0.1"}}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let allocations = private_api(&server)
        .earn_allocations(&EarnAllocationsRequest {
            converted_asset: Some("EUR".into()),
            hide_zero_allocations: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(allocations.items[0].native_asset, "USDC");
    assert_eq!(allocations.items[0].payout, None);
}

#[tokio::test]
async fn earn_allocate() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/Allocate"))
        .and(Signed)
        .and(FormIs(vec![
            ("strategy_id", "ESRFUO3-Q62XD-WIOIL7"),
            ("amount", "4.3"),
        ]))
        .respond_with(ok(json!(true)))
        .expect(1)
        .mount(&server)
        .await;

    assert!(
        private_api(&server)
            .earn_allocate(&EarnFunds {
                strategy_id: "ESRFUO3-Q62XD-WIOIL7".into(),
                amount: "4.3".into(),
            })
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn earn_allocate_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/AllocateStatus"))
        .and(Signed)
        .and(FormIs(vec![("strategy_id", "ESRFUO3-Q62XD-WIOIL7")]))
        .respond_with(ok(json!({"pending": true})))
        .expect(1)
        .mount(&server)
        .await;

    assert!(
        private_api(&server)
            .earn_allocate_status("ESRFUO3-Q62XD-WIOIL7")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn earn_deallocate() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/Deallocate"))
        .and(Signed)
        .and(FormIs(vec![
            ("strategy_id", "ESRFUO3-Q62XD-WIOIL7"),
            ("amount", "1.5"),
        ]))
        .respond_with(ok(json!(true)))
        .expect(1)
        .mount(&server)
        .await;

    assert!(
        private_api(&server)
            .earn_deallocate(&EarnFunds {
                strategy_id: "ESRFUO3-Q62XD-WIOIL7".into(),
                amount: "1.5".into(),
            })
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn earn_deallocate_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/0/private/Earn/DeallocateStatus"))
        .and(Signed)
        .and(FormIs(vec![("strategy_id", "ESRFUO3-Q62XD-WIOIL7")]))
        .respond_with(ok(json!({"pending": false})))
        .expect(1)
        .mount(&server)
        .await;

    assert!(
        !private_api(&server)
            .earn_deallocate_status("ESRFUO3-Q62XD-WIOIL7")
            .await
            .unwrap()
    );
}
