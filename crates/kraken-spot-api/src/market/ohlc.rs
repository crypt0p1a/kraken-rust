use crate::KrakenError;
use crate::lenient;
use kraken_network::PublicClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Duration of a candle, sent in minutes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum CandleInterval {
    Res1m,
    Res5m,
    Res15m,
    Res30m,
    Res1h,
    Res4h,
    Res1d,
    Res1w,
    Res15d,
}

impl CandleInterval {
    pub fn minutes(self) -> u32 {
        match self {
            CandleInterval::Res1m => 1,
            CandleInterval::Res5m => 5,
            CandleInterval::Res15m => 15,
            CandleInterval::Res30m => 30,
            CandleInterval::Res1h => 60,
            CandleInterval::Res4h => 240,
            CandleInterval::Res1d => 1440,
            CandleInterval::Res1w => 10080,
            CandleInterval::Res15d => 21600,
        }
    }
}

impl Serialize for CandleInterval {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.minutes())
    }
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ohlc-data>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CandlesRequest {
    /// e.g. "XBTUSD"
    pub pair: String,
    pub interval: CandleInterval,
    /// Only the candles after this UTC timestamp, in seconds. At most the last 720 candles are
    /// returned, whatever this value
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub since: Option<i64>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ohlc-data>
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Candle {
    /// Start of the candle, UTC timestamp in seconds
    pub time: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub vwap: Decimal,
    pub volume: Decimal,
    pub count: i64,
    /// false for the last candle of the answer, still being built
    pub is_closed: bool,
}

/// Candles of a pair, oldest first
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Candles {
    /// Kraken name of the pair, e.g. "XXBTZUSD"
    pub pair: String,
    pub candles: Vec<Candle>,
    /// To give as `since` to only get the candles closed after this answer
    pub last: i64,
}

/// [time, open, high, low, close, vwap, volume, count]
#[derive(Deserialize)]
struct RawCandle(
    #[serde(deserialize_with = "lenient::number")] i64,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::decimal")] Decimal,
    #[serde(deserialize_with = "lenient::number")] i64,
);

/// The answer is `{"<pair>": [[...], ...], "last": 1688671200}`
fn decode(mut result: Map<String, Value>) -> Result<Candles, KrakenError> {
    let last = result
        .remove("last")
        .map(|last| lenient::number::<_, i64>(last).map_err(KrakenError::decode))
        .transpose()?
        .unwrap_or_default();

    let (pair, raw) = result
        .into_iter()
        .next()
        .ok_or_else(|| KrakenError::Decode {
            reason: "no candles in the answer".into(),
        })?;

    let raw: Vec<RawCandle> = serde_json::from_value(raw).map_err(KrakenError::decode)?;
    let count = raw.len();

    let candles = raw
        .into_iter()
        .enumerate()
        .map(
            |(index, RawCandle(time, open, high, low, close, vwap, volume, trades))| Candle {
                time,
                open,
                high,
                low,
                close,
                vwap,
                volume,
                count: trades,
                is_closed: index + 1 < count,
            },
        )
        .collect();

    Ok(Candles {
        pair,
        candles,
        last,
    })
}

pub(crate) async fn candles(
    client: &PublicClient,
    request: &CandlesRequest,
) -> Result<Candles, KrakenError> {
    decode(client.call_with("OHLC", Some(request)).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_request() {
        let request = CandlesRequest {
            pair: "XBTUSD".into(),
            interval: CandleInterval::Res4h,
            since: Some(1688671200),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "pair=XBTUSD&interval=240&since=1688671200"
        );
    }

    #[test]
    fn decodes_the_candles() {
        let result = serde_json::from_str(
            r#"{"XXBTZUSD":[
                [1688671200,"30306.1","30306.2","30305.7","30305.7","30306.1","3.39243896",23],
                [1688671260,"30304.5","30304.5","30300.0","30300.0","30300.7","4.42996871",18]
            ],"last":1688671200}"#,
        )
        .unwrap();

        let candles = decode(result).unwrap();
        assert_eq!(candles.pair, "XXBTZUSD");
        assert_eq!(candles.last, 1688671200);
        assert_eq!(candles.candles.len(), 2);
        assert_eq!(candles.candles[0].close, dec!(30305.7));
        assert_eq!(candles.candles[1].count, 18);
        assert!(candles.candles[0].is_closed);
        assert!(!candles.candles[1].is_closed);
    }

    #[test]
    fn sends_every_interval_in_minutes() {
        for (interval, minutes) in [
            (CandleInterval::Res1m, 1),
            (CandleInterval::Res5m, 5),
            (CandleInterval::Res15m, 15),
            (CandleInterval::Res30m, 30),
            (CandleInterval::Res1h, 60),
            (CandleInterval::Res4h, 240),
            (CandleInterval::Res1d, 1440),
            (CandleInterval::Res1w, 10080),
            (CandleInterval::Res15d, 21600),
        ] {
            assert_eq!(interval.minutes(), minutes);
            assert_eq!(
                serde_json::to_string(&interval).unwrap(),
                minutes.to_string()
            );
        }
    }

    #[test]
    fn omits_the_missing_since() {
        let request = CandlesRequest {
            pair: "XBTUSD".into(),
            interval: CandleInterval::Res1m,
            since: None,
        };
        assert_eq!(encode(&request).unwrap(), "pair=XBTUSD&interval=1");
    }

    #[test]
    fn the_only_candle_is_open() {
        let candles = decode(
            serde_json::from_str(r#"{"XXBTZUSD":[[1,"1","2","0.5","1.5","1.2","3",4]],"last":0}"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(candles.candles.len(), 1);
        assert!(!candles.candles[0].is_closed);
        assert_eq!(candles.candles[0].low, dec!(0.5));
        assert_eq!(candles.candles[0].count, 4);
    }

    #[test]
    fn decodes_the_empty_answers() {
        let candles =
            decode(serde_json::from_str(r#"{"XXBTZUSD":[],"last":12}"#).unwrap()).unwrap();
        assert!(candles.candles.is_empty());
        assert_eq!(candles.last, 12);

        // `last` missing, numbers instead of strings
        let candles =
            decode(serde_json::from_str(r#"{"XXBTZUSD":[[1,1,2,0.5,1.5,1.2,3,4]]}"#).unwrap())
                .unwrap();
        assert_eq!(candles.last, 0);
        assert_eq!(candles.candles[0].high, dec!(2.0));
    }

    #[test]
    fn rejects_the_incomplete_answers() {
        let no_pair = decode(serde_json::from_str(r#"{"last":12}"#).unwrap());
        assert!(matches!(no_pair, Err(KrakenError::Decode { .. })));

        let short_row = decode(
            serde_json::from_str(r#"{"XXBTZUSD":[[1,"1","2","0.5","1.5","1.2","3"]]}"#).unwrap(),
        );
        assert!(matches!(short_row, Err(KrakenError::Decode { .. })));

        let long_row = decode(
            serde_json::from_str(r#"{"XXBTZUSD":[[1,"1","2","0.5","1.5","1.2","3",4,5]]}"#)
                .unwrap(),
        );
        assert!(matches!(long_row, Err(KrakenError::Decode { .. })));

        let not_a_number = decode(
            serde_json::from_str(r#"{"XXBTZUSD":[[1,"x","2","0.5","1.5","1.2","3",4]]}"#).unwrap(),
        );
        assert!(matches!(not_a_number, Err(KrakenError::Decode { .. })));
    }
}
