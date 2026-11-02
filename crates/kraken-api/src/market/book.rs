use super::AssetClass;
use super::keyed::single_pair;
use crate::KrakenError;
use crate::lenient::{self, ExactDecimal};
use kraken_network::{PrivateClient, PublicClient};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An aggregated price level of the order book
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BookLevel {
    pub price: Decimal,
    pub volume: Decimal,
    /// Last update of the level, UTC timestamp in seconds
    pub timestamp: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-order-book>
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderBook {
    /// Kraken name of the pair, e.g. "XXBTZUSD"
    pub pair: String,
    /// Lowest first
    pub asks: Vec<BookLevel>,
    /// Highest first
    pub bids: Vec<BookLevel>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-order-book>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderBookRequest {
    /// e.g. "XBTUSD"
    pub pair: String,
    /// Levels per side, 1 to 500: 100 when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub count: Option<u32>,
    /// [`AssetClass::TokenizedAsset`] for the pairs of the tokenized assets
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
}

/// [price, volume, timestamp]
#[derive(Deserialize)]
struct RawLevel(
    ExactDecimal,
    ExactDecimal,
    #[serde(deserialize_with = "lenient::number")] i64,
);

#[derive(Deserialize)]
struct RawBook {
    asks: Vec<RawLevel>,
    bids: Vec<RawLevel>,
}

fn levels(raw: Vec<RawLevel>) -> Vec<BookLevel> {
    raw.into_iter()
        .map(|RawLevel(price, volume, timestamp)| BookLevel {
            price: price.0,
            volume: volume.0,
            timestamp,
        })
        .collect()
}

fn decode_book(result: Map<String, Value>) -> Result<OrderBook, KrakenError> {
    let (pair, book, _) = single_pair(result)?;
    let book: RawBook = serde_json::from_value(book).map_err(KrakenError::decode)?;
    Ok(OrderBook {
        pair,
        asks: levels(book.asks),
        bids: levels(book.bids),
    })
}

pub(crate) async fn order_book(
    client: &PublicClient,
    request: &OrderBookRequest,
) -> Result<OrderBook, KrakenError> {
    decode_book(client.call_with("Depth", Some(request)).await?)
}

/// A level of the grouped order book
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GroupedLevel {
    #[serde(deserialize_with = "lenient::decimal")]
    pub price: Decimal,
    #[serde(rename = "qty", deserialize_with = "lenient::decimal")]
    pub quantity: Decimal,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-grouped-order-book>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GroupedOrderBook {
    /// The pair as requested, e.g. "XBTUSD"
    pub pair: String,
    /// Ticks per level
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub grouping: Option<u32>,
    /// Highest first
    pub bids: Vec<GroupedLevel>,
    /// Lowest first
    pub asks: Vec<GroupedLevel>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-grouped-order-book>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct GroupedOrderBookRequest {
    pub pair: String,
    /// Levels per side: 10, 25, 100, 250 or 1000; 10 when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub depth: Option<u32>,
    /// Ticks per level: 1, 5, 10, 25, 50, 100, 250, 500 or 1000; 1 when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub grouping: Option<u32>,
}

pub(crate) async fn grouped_order_book(
    client: &PublicClient,
    request: &GroupedOrderBookRequest,
) -> Result<GroupedOrderBook, KrakenError> {
    client.call_with("GroupedBook", Some(request)).await
}

/// An order of the level 3 order book
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BookOrder {
    #[serde(deserialize_with = "lenient::decimal")]
    pub price: Decimal,
    #[serde(rename = "qty", deserialize_with = "lenient::decimal")]
    pub quantity: Decimal,
    pub order_id: String,
    /// UTC timestamp, in nanoseconds
    #[serde(deserialize_with = "lenient::number")]
    pub timestamp: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-level-3-order-book>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Level3OrderBook {
    /// e.g. "BTC/USD"
    pub pair: String,
    pub bids: Vec<BookOrder>,
    pub asks: Vec<BookOrder>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-level-3-order-book>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Level3OrderBookRequest {
    pub pair: String,
    /// Levels per side: 0 (the whole book), 10, 25, 100, 250 or 1000
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub depth: Option<u32>,
}

/// Private: requires the "Query open orders & trades" permission
pub(crate) async fn level3_order_book(
    client: &PrivateClient,
    request: &Level3OrderBookRequest,
) -> Result<Level3OrderBook, KrakenError> {
    client.call_with("Level3", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_order_book_request() {
        let request = OrderBookRequest {
            pair: "XBTUSD".into(),
            count: Some(3),
            asset_class: Some(AssetClass::TokenizedAsset),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "pair=XBTUSD&count=3&asset_class=tokenized_asset"
        );
    }

    #[test]
    fn encodes_the_grouped_order_book_request() {
        let request = GroupedOrderBookRequest {
            pair: "XBTUSD".into(),
            depth: Some(25),
            grouping: None,
        };
        assert_eq!(encode(&request).unwrap(), "pair=XBTUSD&depth=25");
    }

    #[test]
    fn encodes_the_level3_order_book_request() {
        let request = Level3OrderBookRequest {
            pair: "XBTUSD".into(),
            depth: Some(0),
        };
        assert_eq!(encode(&request).unwrap(), "pair=XBTUSD&depth=0");
    }

    #[test]
    fn decodes_the_order_book() {
        let book = decode_book(
            serde_json::from_str(
                r#"{"XXBTZUSD":{"asks":[["77934.10000","4.317",1793612207]],
                "bids":[["77934.00000","0.001",1793612198],["77933.9","1",1793612181]]}}"#,
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(book.pair, "XXBTZUSD");
        assert_eq!(book.asks[0].price, dec!(77934.1));
        assert_eq!(book.asks[0].volume, dec!(4.317));
        assert_eq!(book.bids[1].timestamp, 1793612181);
        assert_eq!(book.bids.len(), 2);
    }

    #[test]
    fn rejects_the_incomplete_levels() {
        let short = decode_book(
            serde_json::from_str(r#"{"XXBTZUSD":{"asks":[["1","2"]],"bids":[]}}"#).unwrap(),
        );
        assert!(matches!(short, Err(KrakenError::Decode { .. })));
    }

    #[test]
    fn decodes_the_grouped_order_book() {
        let grouped: GroupedOrderBook = serde_json::from_str(
            r#"{"pair":"XBTUSD","grouping":100,"bids":[{"price":"77930.00000","qty":"0.0038893"}],
            "asks":[{"price":"77940.00000","qty":"17.27504303"}]}"#,
        )
        .unwrap();
        assert_eq!(grouped.grouping, Some(100));
        assert_eq!(grouped.asks[0].quantity, dec!(17.27504303));
    }

    #[test]
    fn decodes_the_level3_order_book() {
        let level3: Level3OrderBook = serde_json::from_str(
            r#"{"pair":"YFI/EUR","bids":[{"price":"3062.00000","qty":"0.29665800",
            "order_id":"O5KJU4-IEQTM-NDMS6W","timestamp":1765622008594292000}],"asks":[]}"#,
        )
        .unwrap();
        assert_eq!(level3.bids[0].order_id, "O5KJU4-IEQTM-NDMS6W");
        assert_eq!(level3.bids[0].timestamp, 1765622008594292000);
        assert_eq!(level3.bids[0].price, dec!(3062));
    }
}
