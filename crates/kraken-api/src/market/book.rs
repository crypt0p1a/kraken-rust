use super::AssetClass;
use super::keyed::single_pair;
use crate::KrakenError;
use crate::lenient::{self, ExactDecimal};
use kraken_network::PublicClient;
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
}
