//! Calls the real Kraken public API: `cargo test -p kraken-api --test live_public -- --ignored`

use kraken_api::market::{CandleInterval, CandlesRequest};
use kraken_api::{PublicApi, RateLimiter, RestOptions};
use std::sync::{Arc, LazyLock};

/// Shared like in an application: Kraken limits the calls per client
static RATE_LIMITER: LazyLock<Arc<RateLimiter>> = LazyLock::new(Default::default);

fn api() -> PublicApi {
    PublicApi::new(RestOptions::default(), RATE_LIMITER.clone()).unwrap()
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn server_time() {
    let time = api().server_time().await.unwrap();
    assert!(time.unix_time > 0);
    assert!(!time.rfc1123.is_empty());
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn tickers_pairs_and_candles() {
    let api = api();
    let pairs = vec!["XBTUSD".to_string()];

    let tickers = api.tickers(&pairs).await.unwrap();
    assert!(tickers["XXBTZUSD"].last_trade.price > 0.0);

    let asset_pairs = api.asset_pairs(&pairs).await.unwrap();
    assert_eq!(asset_pairs["XXBTZUSD"].altname, "XBTUSD");

    let candles = api
        .candles(&CandlesRequest {
            pair: "XBTUSD".into(),
            interval: CandleInterval::Res5m,
            since: None,
        })
        .await
        .unwrap();
    assert_eq!(candles.pair, "XXBTZUSD");
    assert!(!candles.candles.is_empty());
    assert!(!candles.candles.last().unwrap().is_closed);
}
