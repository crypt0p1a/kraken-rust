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

/// The dead man's switch
///
/// <https://docs.kraken.com/api/docs/rest-api/cancel-all-orders-after>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CancelAllAfter {
    /// RFC 3339: when the request was received
    #[serde(rename = "currentTime")]
    pub current_time: String,
    /// RFC 3339: when every open order will be cancelled, unless the timer is extended or
    /// disabled. "0" once disabled
    #[serde(rename = "triggerTime")]
    pub trigger_time: String,
}

#[derive(Serialize)]
struct Timeout {
    timeout: u32,
}

/// Cancels every open order in `timeout` seconds (less than 86400), unless called again: 0
/// disables the timer
pub(crate) async fn cancel_all_orders_after(
    client: &PrivateClient,
    timeout: u32,
) -> Result<CancelAllAfter, KrakenError> {
    client
        .call_with("CancelAllOrdersAfter", Some(&Timeout { timeout }))
        .await
}

/// The orders of a batch: txids and userrefs in `orders`, client identifiers in `cl_ord_ids`
#[derive(Serialize)]
struct Batch {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    orders: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cl_ord_ids: Vec<String>,
}

impl Batch {
    fn new(references: &[OrderReference]) -> Self {
        let mut batch = Batch {
            orders: vec![],
            cl_ord_ids: vec![],
        };
        for reference in references {
            match reference {
                OrderReference::Txid { txid } => batch.orders.push(txid.as_str().into()),
                OrderReference::Userref { userref } => batch.orders.push((*userref).into()),
                OrderReference::ClOrdId { cl_ord_id } => batch.cl_ord_ids.push(cl_ord_id.clone()),
            }
        }
        batch
    }
}

/// Up to 50 orders, sent in JSON: a list of identifiers as in the example of the documentation,
/// whose schema describes a list of objects instead
pub(crate) async fn cancel_order_batch(
    client: &PrivateClient,
    orders: &[OrderReference],
) -> Result<CancelledOrders, KrakenError> {
    client
        .call_json("CancelOrderBatch", &Batch::new(orders))
        .await
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
    fn encodes_the_timeout() {
        assert_eq!(encode(&Timeout { timeout: 60 }).unwrap(), "timeout=60");
    }

    #[test]
    fn decodes_the_timer() {
        let timer: CancelAllAfter = serde_json::from_str(
            r#"{"currentTime":"2023-03-24T17:41:56Z","triggerTime":"2023-03-24T17:42:56Z"}"#,
        )
        .unwrap();
        assert_eq!(timer.trigger_time, "2023-03-24T17:42:56Z");
    }

    #[test]
    fn splits_the_batch() {
        let batch = Batch::new(&[
            OrderReference::Txid {
                txid: "OG5V2Y-RYKVL-DT3V3B".into(),
            },
            OrderReference::Userref { userref: 42 },
            OrderReference::ClOrdId {
                cl_ord_id: "mgr1_buy_1".into(),
            },
        ]);
        assert_eq!(
            serde_json::to_value(&batch).unwrap(),
            serde_json::json!({"orders": ["OG5V2Y-RYKVL-DT3V3B", 42], "cl_ord_ids": ["mgr1_buy_1"]})
        );
        let only_txids = Batch::new(&[OrderReference::Txid { txid: "A".into() }]);
        assert_eq!(
            serde_json::to_value(&only_txids).unwrap(),
            serde_json::json!({"orders": ["A"]})
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
