use crate::KrakenError;
use crate::lenient::{self, ExactDecimal};
use crate::params::Pairs;
use kraken_network::PublicClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// Best ask or bid
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PriceLevel {
    pub price: Decimal,
    pub whole_lot_volume: Decimal,
    pub lot_volume: Decimal,
}

/// Last trade closed
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradeLevel {
    pub price: Decimal,
    pub lot_volume: Decimal,
}

/// Value of today (since 00:00 UTC) and of the last 24 hours
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Window {
    pub today: Decimal,
    pub last_24h: Decimal,
}

/// Count of today (since 00:00 UTC) and of the last 24 hours
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WindowCount {
    pub today: i64,
    pub last_24h: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ticker-information>
///
/// Kraken sends the values as arrays (`"a": ["35000.1", "1", "1.000"]`), named here
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Ticker {
    #[serde(rename = "a", deserialize_with = "price_level")]
    pub ask: PriceLevel,
    #[serde(rename = "b", deserialize_with = "price_level")]
    pub bid: PriceLevel,
    #[serde(rename = "c", deserialize_with = "trade_level")]
    pub last_trade: TradeLevel,
    #[serde(rename = "v", deserialize_with = "window")]
    pub volume: Window,
    /// Volume weighted average price
    #[serde(rename = "p", deserialize_with = "window")]
    pub vwap: Window,
    #[serde(rename = "t", deserialize_with = "window_count")]
    pub trades: WindowCount,
    #[serde(rename = "l", deserialize_with = "window")]
    pub low: Window,
    #[serde(rename = "h", deserialize_with = "window")]
    pub high: Window,
    /// Today's opening price
    #[serde(rename = "o", deserialize_with = "lenient::decimal")]
    pub open: Decimal,
}

#[derive(Deserialize)]
struct LenientI64(#[serde(deserialize_with = "lenient::number")] i64);

fn price_level<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PriceLevel, D::Error> {
    let (price, whole_lot_volume, lot_volume) =
        <(ExactDecimal, ExactDecimal, ExactDecimal)>::deserialize(deserializer)?;
    Ok(PriceLevel {
        price: price.0,
        whole_lot_volume: whole_lot_volume.0,
        lot_volume: lot_volume.0,
    })
}

fn trade_level<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TradeLevel, D::Error> {
    let (price, lot_volume) = <(ExactDecimal, ExactDecimal)>::deserialize(deserializer)?;
    Ok(TradeLevel {
        price: price.0,
        lot_volume: lot_volume.0,
    })
}

fn window<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Window, D::Error> {
    let (today, last_24h) = <(ExactDecimal, ExactDecimal)>::deserialize(deserializer)?;
    Ok(Window {
        today: today.0,
        last_24h: last_24h.0,
    })
}

fn window_count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<WindowCount, D::Error> {
    let (today, last_24h) = <(LenientI64, LenientI64)>::deserialize(deserializer)?;
    Ok(WindowCount {
        today: today.0,
        last_24h: last_24h.0,
    })
}

/// Keyed by the Kraken name of the pair, which may differ from the requested one
/// (`XBTUSD` → `XXBTZUSD`). All the pairs when `pairs` is empty
pub(crate) async fn tickers(
    client: &PublicClient,
    pairs: &[String],
) -> Result<HashMap<String, Ticker>, KrakenError> {
    client
        .call_with("Ticker", Some(&Pairs { pair: pairs }))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn decodes_the_tickers() {
        let tickers: HashMap<String, Ticker> = serde_json::from_str(
            r#"{"XXBTZUSD":{"a":["30300.10000","1","1.000"],"b":["30300.00000","1","1.000"],
                "c":["30303.20000","0.00067643"],"v":["4083.67001100","4412.73601799"],
                "p":["30706.77771","30689.13205"],"t":[34619,38907],
                "l":["29868.30000","29868.30000"],"h":["31631.00000","31631.00000"],
                "o":"30502.80000"}}"#,
        )
        .unwrap();

        let ticker = &tickers["XXBTZUSD"];
        assert_eq!(ticker.ask.price, dec!(30300.1));
        assert_eq!(ticker.bid.lot_volume, dec!(1.0));
        assert_eq!(ticker.last_trade.lot_volume, dec!(0.00067643));
        assert_eq!(ticker.volume.last_24h, dec!(4412.73601799));
        assert_eq!(ticker.trades.today, 34619);
        assert_eq!(ticker.open, dec!(30502.8));
    }

    const TICKER: &str = r#"{"a":["1","2","3"],"b":["0.5","1","1"],"c":["0.75","0.1"],
        "v":["10","20"],"p":["0.7","0.8"],"t":[1,2],"l":["0.4","0.3"],"h":["1.1","1.2"],
        "o":"0.6"}"#;

    #[test]
    fn names_every_value_of_the_arrays() {
        let ticker: Ticker = serde_json::from_str(TICKER).unwrap();
        assert_eq!(
            ticker.ask,
            PriceLevel {
                price: dec!(1.0),
                whole_lot_volume: dec!(2.0),
                lot_volume: dec!(3.0)
            }
        );
        assert_eq!(
            ticker.last_trade,
            TradeLevel {
                price: dec!(0.75),
                lot_volume: dec!(0.1)
            }
        );
        assert_eq!(
            ticker.volume,
            Window {
                today: dec!(10.0),
                last_24h: dec!(20.0)
            }
        );
        assert_eq!(
            ticker.vwap,
            Window {
                today: dec!(0.7),
                last_24h: dec!(0.8)
            }
        );
        assert_eq!(
            ticker.trades,
            WindowCount {
                today: 1,
                last_24h: 2
            }
        );
        assert_eq!(
            ticker.low,
            Window {
                today: dec!(0.4),
                last_24h: dec!(0.3)
            }
        );
        assert_eq!(
            ticker.high,
            Window {
                today: dec!(1.1),
                last_24h: dec!(1.2)
            }
        );
        assert_eq!(ticker.open, dec!(0.6));
    }

    #[test]
    fn rejects_the_arrays_of_another_size() {
        for (from, to) in [
            (r#""a":["1","2","3"]"#, r#""a":["1","2"]"#),
            (r#""c":["0.75","0.1"]"#, r#""c":["0.75","0.1","9"]"#),
            (r#""t":[1,2]"#, r#""t":[1]"#),
            (r#""o":"0.6""#, r#""o":["0.6"]"#),
        ] {
            let changed = TICKER.replace(from, to);
            assert!(serde_json::from_str::<Ticker>(&changed).is_err(), "{to}");
        }
    }

    #[test]
    fn requires_every_value() {
        let without_open = TICKER.replace(r#""o":"0.6""#, r#""x":"0.6""#);
        assert_ne!(without_open, TICKER);
        assert!(serde_json::from_str::<Ticker>(&without_open).is_err());
    }
}
