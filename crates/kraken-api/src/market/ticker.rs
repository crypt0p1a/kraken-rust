use crate::KrakenError;
use crate::lenient;
use crate::params::Pairs;
use kraken_network::PublicClient;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// Best ask or bid
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceLevel {
    pub price: f64,
    pub whole_lot_volume: f64,
    pub lot_volume: f64,
}

/// Last trade closed
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeLevel {
    pub price: f64,
    pub lot_volume: f64,
}

/// Value of today (since 00:00 UTC) and of the last 24 hours
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub today: f64,
    pub last_24h: f64,
}

/// Count of today (since 00:00 UTC) and of the last 24 hours
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowCount {
    pub today: i64,
    pub last_24h: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-ticker-information>
///
/// Kraken sends the values as arrays (`"a": ["35000.1", "1", "1.000"]`), named here
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    #[serde(rename = "o", deserialize_with = "lenient::number")]
    pub open: f64,
}

#[derive(Deserialize)]
struct LenientF64(#[serde(deserialize_with = "lenient::number")] f64);

#[derive(Deserialize)]
struct LenientI64(#[serde(deserialize_with = "lenient::number")] i64);

fn price_level<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PriceLevel, D::Error> {
    let (price, whole_lot_volume, lot_volume) =
        <(LenientF64, LenientF64, LenientF64)>::deserialize(deserializer)?;
    Ok(PriceLevel {
        price: price.0,
        whole_lot_volume: whole_lot_volume.0,
        lot_volume: lot_volume.0,
    })
}

fn trade_level<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TradeLevel, D::Error> {
    let (price, lot_volume) = <(LenientF64, LenientF64)>::deserialize(deserializer)?;
    Ok(TradeLevel {
        price: price.0,
        lot_volume: lot_volume.0,
    })
}

fn window<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Window, D::Error> {
    let (today, last_24h) = <(LenientF64, LenientF64)>::deserialize(deserializer)?;
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
        assert_eq!(ticker.ask.price, 30300.1);
        assert_eq!(ticker.bid.lot_volume, 1.0);
        assert_eq!(ticker.last_trade.lot_volume, 0.00067643);
        assert_eq!(ticker.volume.last_24h, 4412.73601799);
        assert_eq!(ticker.trades.today, 34619);
        assert_eq!(ticker.open, 30502.8);
    }
}
