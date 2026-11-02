use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

/// An open order, or the open orders of a user reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum OrderReference {
    /// Kraken identifier of the order
    Txid { txid: String },
    /// Every open order of this user reference
    Userref { userref: i32 },
    /// Client identifier of the order
    ClOrdId { cl_ord_id: String },
}

/// Kraken takes the txid and the userref in the same `txid` parameter
impl Serialize for OrderReference {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            OrderReference::Txid { txid } => map.serialize_entry("txid", txid)?,
            OrderReference::Userref { userref } => map.serialize_entry("txid", userref)?,
            OrderReference::ClOrdId { cl_ord_id } => map.serialize_entry("cl_ord_id", cl_ord_id)?,
        }
        map.end()
    }
}

/// <https://docs.kraken.com/api/docs/rest-api/cancel-order>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CancelledOrders {
    /// Orders cancelled
    #[serde(deserialize_with = "lenient::number")]
    pub count: i32,
    /// The orders are pending cancellation
    #[serde(default)]
    pub pending: Option<bool>,
}

pub(crate) async fn cancel_order(
    client: &PrivateClient,
    order: &OrderReference,
) -> Result<CancelledOrders, KrakenError> {
    client.call_with("CancelOrder", Some(order)).await
}

/// Every open order of the account
pub(crate) async fn cancel_all(client: &PrivateClient) -> Result<CancelledOrders, KrakenError> {
    client.call("CancelAll").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_every_reference() {
        assert_eq!(
            encode(&OrderReference::Txid {
                txid: "OYVGEW-VYV5B-UUEXSK".into()
            })
            .unwrap(),
            "txid=OYVGEW-VYV5B-UUEXSK"
        );
        assert_eq!(
            encode(&OrderReference::Userref { userref: 42 }).unwrap(),
            "txid=42"
        );
        assert_eq!(
            encode(&OrderReference::ClOrdId {
                cl_ord_id: "mgr1 buy".into()
            })
            .unwrap(),
            "cl_ord_id=mgr1+buy"
        );
    }

    #[test]
    fn decodes_the_cancellation() {
        let cancelled: CancelledOrders = serde_json::from_str(r#"{"count":1}"#).unwrap();
        assert_eq!(cancelled.count, 1);
        assert_eq!(cancelled.pending, None);
        let pending: CancelledOrders =
            serde_json::from_str(r#"{"count":0,"pending":true}"#).unwrap();
        assert_eq!(pending.pending, Some(true));
    }
}
