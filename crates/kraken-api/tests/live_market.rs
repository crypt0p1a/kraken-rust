//! Contract of the public market endpoints, against the real Kraken API:
//! `cargo test -p kraken-api --test live_market -- --ignored --test-threads=1 --nocapture`
//!
//! Every answer is decoded strictly (no field ignored by the models, no required field missing),
//! on every pair and every interval, and its values are checked for consistency.

mod live;
mod strict;

use kraken_api::Decimal;
use kraken_api::KrakenError;
use kraken_api::market::{
    AssetPair, AssetPairStatus, CandleInterval, CandlesRequest, ServerTime, SystemState,
    SystemStatus, Ticker,
};
use live::{Presence, decode_strict, entries, le, now_seconds, public_api, public_client};
use rust_decimal_macros::dec;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

const INTERVALS: [CandleInterval; 9] = [
    CandleInterval::Res1m,
    CandleInterval::Res5m,
    CandleInterval::Res15m,
    CandleInterval::Res30m,
    CandleInterval::Res1h,
    CandleInterval::Res4h,
    CandleInterval::Res1d,
    CandleInterval::Res1w,
    CandleInterval::Res15d,
];

#[derive(Serialize)]
struct PairParam<'a> {
    pair: &'a str,
}

#[derive(Serialize)]
struct OhlcParams<'a> {
    pair: &'a str,
    interval: u32,
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn server_time() {
    let raw: Value = public_client().call("Time").await.unwrap();
    let time: ServerTime = decode_strict("Time", &raw);

    assert!((time.unix_time - now_seconds()).abs() < 60, "{time:?}");
    assert!(time.rfc1123.ends_with("+0000"), "{time:?}");
    assert_eq!(
        public_api().server_time().await.unwrap().rfc1123.len(),
        time.rfc1123.len()
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn tickers_of_every_pair() {
    let raw: Value = public_client().call("Ticker").await.unwrap();
    let mut presence = Presence::default();

    for (pair, value) in entries(&raw) {
        presence.record(value);
        let ticker: Ticker = decode_strict(&format!("Ticker {pair}"), value);

        if ticker.ask.price > Decimal::ZERO && ticker.bid.price > Decimal::ZERO {
            assert!(
                le(ticker.bid.price, ticker.ask.price),
                "{pair}: crossed book {ticker:?}"
            );
        }
        for window in [ticker.volume, ticker.vwap, ticker.low, ticker.high] {
            assert!(
                window.today >= Decimal::ZERO && window.last_24h >= Decimal::ZERO,
                "{pair}: {ticker:?}"
            );
        }
        assert!(
            ticker.trades.today >= 0 && ticker.trades.last_24h >= 0,
            "{pair}"
        );
        assert!(
            ticker.ask.lot_volume >= Decimal::ZERO && ticker.bid.lot_volume >= Decimal::ZERO,
            "{pair}"
        );
        assert!(
            ticker.last_trade.price >= Decimal::ZERO && ticker.open >= Decimal::ZERO,
            "{pair}"
        );

        if ticker.trades.last_24h > 0 {
            assert!(
                le(ticker.low.last_24h, ticker.high.last_24h),
                "{pair}: {ticker:?}"
            );
            assert!(
                le(ticker.low.last_24h, ticker.vwap.last_24h * dec!(1.000001))
                    && le(ticker.vwap.last_24h, ticker.high.last_24h * dec!(1.000001)),
                "{pair}: vwap outside of the 24h range {ticker:?}"
            );
        }
        if ticker.trades.today > 0 {
            assert!(
                le(ticker.low.today, ticker.high.today),
                "{pair}: {ticker:?}"
            );
        }
    }

    presence.print("Ticker");
    presence.assert_always("Ticker", &["a", "b", "c", "v", "p", "t", "l", "h", "o"]);
    assert!(presence.total() > 500, "only {} tickers", presence.total());

    // the typed call returns the same pairs
    let typed = public_api().tickers(&[]).await.unwrap();
    assert!(typed.len().abs_diff(presence.total()) < 10);
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn tickers_of_given_pairs_are_keyed_by_kraken_name() {
    let pairs = [
        "XBTUSD".to_string(),
        "ETHUSD".to_string(),
        "XBTEUR".to_string(),
    ];
    let tickers = public_api().tickers(&pairs).await.unwrap();

    let mut keys: Vec<_> = tickers.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["XETHZUSD", "XXBTZEUR", "XXBTZUSD"]);
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn asset_pairs_of_every_pair() {
    let raw: Value = public_client().call("AssetPairs").await.unwrap();
    let mut presence = Presence::default();
    let mut statuses = HashMap::new();

    for (key, value) in entries(&raw) {
        presence.record(value);
        let pair: AssetPair = decode_strict(&format!("AssetPairs {key}"), value);
        *statuses.entry(pair.status).or_insert(0) += 1;

        assert!(!pair.altname.is_empty() && !pair.base.is_empty() && !pair.quote.is_empty());
        assert!((0..=18).contains(&pair.pair_decimals), "{key}: {pair:?}");
        assert!((0..=18).contains(&pair.lot_decimals), "{key}: {pair:?}");
        assert!(
            pair.cost_decimals.is_some_and(|it| (0..=18).contains(&it)),
            "{key}"
        );
        assert!(
            pair.ordermin.is_some_and(|it| it > Decimal::ZERO),
            "{key}: {pair:?}"
        );
        assert!(
            pair.costmin.is_some_and(|it| it >= Decimal::ZERO),
            "{key}: {pair:?}"
        );

        // the price increment can't be finer than the decimals of the price
        let tick = pair.tick_size.expect("tick_size");
        assert!(
            le(Decimal::new(1, pair.pair_decimals as u32), tick),
            "{key}: tick {tick} too fine"
        );

        for tiers in [&pair.fees, &pair.fees_maker] {
            assert!(
                tiers.windows(2).all(|it| it[0].volume < it[1].volume),
                "{key}: {tiers:?}"
            );
            assert!(
                tiers.windows(2).all(|it| le(it[1].percent, it[0].percent)),
                "{key}"
            );
            assert!(
                tiers
                    .iter()
                    .all(|it| (Decimal::ZERO..=Decimal::ONE_HUNDRED).contains(&it.percent)),
                "{key}"
            );
        }

        assert_eq!(
            pair.long_position_limit.is_some(),
            pair.short_position_limit.is_some(),
            "{key}: one position limit without the other"
        );
        assert!(pair.leverage_buy.iter().all(|it| *it > 1), "{key}");
    }

    presence.print("AssetPairs");
    presence.assert_always(
        "AssetPairs",
        &[
            "altname",
            "wsname",
            "aclass_base",
            "base",
            "aclass_quote",
            "quote",
            "lot",
            "pair_decimals",
            "cost_decimals",
            "lot_decimals",
            "lot_multiplier",
            "fees",
            "fees_maker",
            "ordermin",
            "costmin",
            "tick_size",
            "status",
        ],
    );
    println!("statuses: {statuses:?}");
    assert!(
        !statuses.contains_key(&Some(AssetPairStatus::Unknown)),
        "{statuses:?}"
    );
    assert!(!statuses.contains_key(&None), "pairs without status");
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn every_pair_has_a_ticker() {
    let pairs = public_api().asset_pairs(&[]).await.unwrap();
    let tickers = public_api().tickers(&[]).await.unwrap();

    let missing: Vec<_> = pairs
        .keys()
        .filter(|it| !tickers.contains_key(*it))
        .collect();
    assert!(missing.is_empty(), "pairs without ticker: {missing:?}");
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn asset_pairs_of_given_pairs() {
    let pairs = public_api()
        .asset_pairs(&["XBTUSD".into(), "ETHEUR".into()])
        .await
        .unwrap();

    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs["XXBTZUSD"].altname, "XBTUSD");
    assert_eq!(pairs["XXBTZUSD"].wsname.as_deref(), Some("XBT/USD"));
    assert_eq!(pairs["XETHZEUR"].base, "XETH");
    assert_eq!(pairs["XETHZEUR"].quote, "ZEUR");
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn candles_of_every_interval() {
    for interval in INTERVALS {
        let seconds = interval.minutes() as i64 * 60;
        let context = format!("OHLC {interval:?}");

        // raw: {"XXBTZUSD": [[time, open, high, low, close, vwap, volume, count]], "last": time}
        let raw: Value = public_client()
            .call_with(
                "OHLC",
                Some(&OhlcParams {
                    pair: "XBTUSD",
                    interval: interval.minutes(),
                }),
            )
            .await
            .unwrap();
        let object = raw.as_object().unwrap();
        let mut keys: Vec<_> = object.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["XXBTZUSD", "last"], "{context}");

        for row in object["XXBTZUSD"].as_array().unwrap() {
            let row = row.as_array().unwrap();
            assert_eq!(row.len(), 8, "{context}: {row:?}");
            assert!(row[0].is_i64() && row[7].is_i64(), "{context}: {row:?}");
            assert!(row[1..7].iter().all(Value::is_string), "{context}: {row:?}");
        }

        let candles = public_api()
            .candles(&CandlesRequest {
                pair: "XBTUSD".into(),
                interval,
                since: None,
            })
            .await
            .unwrap();
        let list = &candles.candles;

        assert_eq!(candles.pair, "XXBTZUSD");
        assert!(
            !list.is_empty() && list.len() <= 721,
            "{context}: {} candles",
            list.len()
        );
        assert!(
            list[..list.len() - 1].iter().all(|it| it.is_closed),
            "{context}"
        );
        assert!(!list.last().unwrap().is_closed, "{context}");
        // `last` is the last closed candle
        if list.len() > 1 {
            assert_eq!(candles.last, list[list.len() - 2].time, "{context}");
        }

        for pair in list.windows(2) {
            let gap = pair[1].time - pair[0].time;
            assert!(gap > 0 && gap % seconds == 0, "{context}: gap {gap}s");
        }
        for candle in list {
            assert_eq!(candle.time % 60, 0, "{context}: {candle:?}");
            assert!(
                le(candle.low, candle.open) && le(candle.open, candle.high),
                "{context}: {candle:?}"
            );
            assert!(
                le(candle.low, candle.close) && le(candle.close, candle.high),
                "{context}: {candle:?}"
            );
            assert!(
                candle.volume >= Decimal::ZERO && candle.count >= 0,
                "{context}: {candle:?}"
            );
            if candle.volume > Decimal::ZERO {
                assert!(
                    le(candle.low * dec!(0.999999), candle.vwap)
                        && le(candle.vwap, candle.high * dec!(1.000001)),
                    "{context}: vwap outside of the candle {candle:?}"
                );
            }
        }
        // the open candle covers now
        let open = list.last().unwrap();
        assert!(
            open.time <= now_seconds() && now_seconds() < open.time + seconds + 60,
            "{context}: {open:?}"
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn candles_since_a_time_start_at_that_time() {
    let api = public_api();
    let request = CandlesRequest {
        pair: "XBTUSD".into(),
        interval: CandleInterval::Res1h,
        since: None,
    };
    let all = api.candles(&request).await.unwrap().candles;
    let since = all[all.len() - 5].time;

    let recent = api
        .candles(&CandlesRequest {
            since: Some(since),
            ..request
        })
        .await
        .unwrap()
        .candles;

    assert_eq!(recent.first().map(|it| it.time), Some(since));
    assert!(recent.iter().all(|it| it.time >= since));
    assert!((4..=6).contains(&recent.len()), "{} candles", recent.len());
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn candles_of_several_pairs() {
    let api = public_api();
    for pair in ["ETHUSD", "XBTEUR", "SOLUSD", "ADAEUR"] {
        let candles = api
            .candles(&CandlesRequest {
                pair: pair.into(),
                interval: CandleInterval::Res1d,
                since: None,
            })
            .await
            .unwrap();
        assert!(candles.candles.len() > 100, "{pair}");
        assert!(
            candles.candles.iter().all(|it| le(it.low, it.high)),
            "{pair}"
        );
    }
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn maps_the_errors_of_unknown_pairs() {
    let api = public_api();
    let unknown = || KrakenError::Api {
        status: 200,
        body: "EQuery:Unknown asset pair".into(),
    };

    assert_eq!(api.tickers(&["NOPEUSD".into()]).await, Err(unknown()));
    assert_eq!(api.asset_pairs(&["NOPEUSD".into()]).await, Err(unknown()));
    assert_eq!(
        api.candles(&CandlesRequest {
            pair: "NOPEUSD".into(),
            interval: CandleInterval::Res1h,
            since: None
        })
        .await,
        Err(unknown())
    );
    // the answer of the raw client is mapped the same way
    assert_eq!(
        public_client()
            .call_with::<_, Value>("Ticker", Some(&PairParam { pair: "NOPEUSD" }))
            .await,
        Err(unknown())
    );
}

#[tokio::test]
#[ignore = "calls the real Kraken API"]
async fn system_status() {
    let raw: Value = public_client().call("SystemStatus").await.unwrap();
    let status: SystemStatus = decode_strict("SystemStatus", &raw);
    assert_ne!(status.status, SystemState::Unknown, "{raw}");
    assert!(status.timestamp.ends_with('Z'), "{status:?}");
    for event in &status.upcoming_maintenance {
        assert!(
            event.expected_start_utc < event.expected_end_utc,
            "{event:?}"
        );
    }
}
