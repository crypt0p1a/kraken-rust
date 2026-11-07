use super::Order;
use super::order::with_txids;
use crate::KrakenError;
use kraken_network::PrivateClient;
use kraken_network::params::comma_separated;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// <https://docs.kraken.com/api/docs/rest-api/get-open-orders>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OpenOrdersRequest {
    /// Includes the identifiers of the trades of each order
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trades: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cl_ord_id: Option<String>,
}

/// Which time `start` and `end` are compared to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum CloseTime {
    Open,
    Close,
    Both,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-closed-orders>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClosedOrdersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trades: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cl_ord_id: Option<String>,
    /// UTC timestamp in seconds (exclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start: Option<i64>,
    /// UTC timestamp in seconds (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub end: Option<i64>,
    /// Offset of the page: 50 orders are returned at most
    #[serde(rename = "ofs", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub offset: Option<i32>,
    /// Both when missing
    #[serde(rename = "closetime", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub close_time: Option<CloseTime>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-orders-info>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct QueryOrdersRequest {
    /// Up to 50 identifiers of orders
    #[serde(rename = "txid", serialize_with = "comma_separated")]
    pub txids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trades: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
}

/// A page of closed orders
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClosedOrders {
    /// Most recent first
    pub orders: Vec<Order>,
    /// Count of the orders matching the request, all pages included
    pub count: i64,
}

#[derive(Deserialize)]
struct OpenAnswer {
    open: HashMap<String, Order>,
}

#[derive(Deserialize)]
struct ClosedAnswer {
    closed: HashMap<String, Order>,
    #[serde(default)]
    count: i64,
}

pub(crate) async fn open_orders(
    client: &PrivateClient,
    request: &OpenOrdersRequest,
) -> Result<Vec<Order>, KrakenError> {
    let answer: OpenAnswer = client.call_with("OpenOrders", Some(request)).await?;
    Ok(with_txids(answer.open))
}

pub(crate) async fn closed_orders(
    client: &PrivateClient,
    request: &ClosedOrdersRequest,
) -> Result<ClosedOrders, KrakenError> {
    let answer: ClosedAnswer = client.call_with("ClosedOrders", Some(request)).await?;
    Ok(ClosedOrders {
        orders: with_txids(answer.closed),
        count: answer.count,
    })
}

pub(crate) async fn query_orders(
    client: &PrivateClient,
    request: &QueryOrdersRequest,
) -> Result<Vec<Order>, KrakenError> {
    let answer: HashMap<String, Order> = client.call_with("QueryOrders", Some(request)).await?;
    Ok(with_txids(answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_requests() {
        assert_eq!(encode(&OpenOrdersRequest::default()).unwrap(), "");
        assert_eq!(
            encode(&ClosedOrdersRequest {
                cl_ord_id: Some("mgr1_buy_1".into()),
                start: Some(1688000000),
                close_time: Some(CloseTime::Close),
                ..Default::default()
            })
            .unwrap(),
            "cl_ord_id=mgr1_buy_1&start=1688000000&closetime=close"
        );
        assert_eq!(
            encode(&QueryOrdersRequest {
                txids: vec!["A".into(), "B".into()],
                trades: Some(true),
                userref: None,
            })
            .unwrap(),
            "txid=A%2CB&trades=true"
        );
    }

    #[test]
    fn decodes_the_closed_orders() {
        let answer: ClosedAnswer = serde_json::from_str(&format!(
            r#"{{"closed":{},"count":2}}"#,
            super::super::order::SAMPLE
        ))
        .unwrap();
        assert_eq!(answer.closed.len(), 2);
        assert_eq!(answer.count, 2);
    }

    #[test]
    fn encodes_every_parameter_of_the_open_orders() {
        assert_eq!(
            encode(&OpenOrdersRequest {
                trades: Some(false),
                userref: Some(-12),
                cl_ord_id: Some("a b&c".into()),
            })
            .unwrap(),
            "trades=false&userref=-12&cl_ord_id=a+b%26c"
        );
    }

    #[test]
    fn encodes_every_parameter_of_the_closed_orders() {
        assert_eq!(
            encode(&ClosedOrdersRequest {
                trades: Some(true),
                userref: Some(7),
                cl_ord_id: None,
                start: Some(1),
                end: Some(2),
                offset: Some(100),
                close_time: Some(CloseTime::Both),
            })
            .unwrap(),
            "trades=true&userref=7&start=1&end=2&ofs=100&closetime=both"
        );
        assert_eq!(encode(&ClosedOrdersRequest::default()).unwrap(), "");
        for (close_time, text) in [
            (CloseTime::Open, "open"),
            (CloseTime::Close, "close"),
            (CloseTime::Both, "both"),
        ] {
            assert_eq!(
                serde_json::to_string(&close_time).unwrap(),
                format!("\"{text}\"")
            );
        }
    }

    #[test]
    fn encodes_a_single_txid() {
        assert_eq!(
            encode(&QueryOrdersRequest {
                txids: vec!["OQCLML-BW3P3-BUCMWZ".into()],
                trades: None,
                userref: Some(3),
            })
            .unwrap(),
            "txid=OQCLML-BW3P3-BUCMWZ&userref=3"
        );
    }

    #[test]
    fn decodes_empty_answers() {
        let open: OpenAnswer = serde_json::from_str(r#"{"open":{}}"#).unwrap();
        assert!(with_txids(open.open).is_empty());
        let closed: ClosedAnswer = serde_json::from_str(r#"{"closed":{}}"#).unwrap();
        assert_eq!(closed.count, 0);
        assert!(serde_json::from_str::<OpenAnswer>(r#"{}"#).is_err());
    }
}
