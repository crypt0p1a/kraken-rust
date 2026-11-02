use crate::account::RebaseMultiplier;
use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum AmendType {
    /// The values of the order when placed
    Original,
    /// Amended by the user
    User,
    /// Amended by Kraken, e.g. a reduced quantity
    Restated,
    #[serde(other)]
    Unknown,
}

/// A version of an order
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderAmend {
    pub amend_id: String,
    pub amend_type: AmendType,
    /// Quantity, in the base asset
    #[serde(rename = "order_qty", deserialize_with = "lenient::decimal")]
    pub order_quantity: Decimal,
    /// Quantity shown in the book, for the iceberg orders
    #[serde(
        rename = "display_qty",
        default,
        deserialize_with = "lenient::opt_decimal"
    )]
    pub display_quantity: Option<Decimal>,
    /// Quantity not traded yet
    #[serde(
        rename = "remaining_qty",
        default,
        deserialize_with = "lenient::opt_decimal"
    )]
    pub remaining_quantity: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub limit_price: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub trigger_price: Option<Decimal>,
    /// Why the order was amended
    #[serde(default)]
    pub reason: Option<String>,
    /// The order couldn't take liquidity
    #[serde(default)]
    pub post_only: Option<bool>,
    /// UTC timestamp of the amend
    #[serde(deserialize_with = "lenient::number")]
    pub timestamp: i64,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-order-amends>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderAmends {
    /// Versions of the order, the original one included
    #[serde(deserialize_with = "lenient::number")]
    pub count: i64,
    /// Oldest first
    #[serde(default)]
    pub amends: Vec<OrderAmend>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-order-amends>
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct OrderAmendsRequest {
    /// Kraken identifier of the order
    pub order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// An amend in place: the identifiers and the priority in the queue of the order are kept
///
/// <https://docs.kraken.com/api/docs/rest-api/amend-order>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AmendOrder {
    /// Kraken identifier of the order: this or `cl_ord_id`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub txid: Option<String>,
    /// Client identifier of the order: this or `txid`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cl_ord_id: Option<String>,
    /// New quantity, in the base asset
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub order_qty: Option<String>,
    /// New quantity shown in the book, for the iceberg orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub display_qty: Option<String>,
    /// New limit price, absolute or relative ("+1.5%")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub limit_price: Option<String>,
    /// New trigger price, absolute or relative
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trigger_price: Option<String>,
    /// Required for the pairs which aren't crypto, e.g. the xStocks
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub pair: Option<String>,
    /// Rejects a new limit price which would take liquidity
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub post_only: Option<bool>,
    /// RFC 3339: the amend is rejected after it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub deadline: Option<String>,
}

#[derive(Deserialize)]
struct Amended {
    amend_id: String,
}

/// The identifier of the amend
pub(crate) async fn amend_order(
    client: &PrivateClient,
    request: &AmendOrder,
) -> Result<String, KrakenError> {
    if request.txid.is_some() == request.cl_ord_id.is_some() {
        return Err(KrakenError::InvalidArgument {
            reason: "an amend takes either the txid or the cl_ord_id of the order".into(),
        });
    }
    let amended: Amended = client.call_with("AmendOrder", Some(request)).await?;
    Ok(amended.amend_id)
}

pub(crate) async fn order_amends(
    client: &PrivateClient,
    request: &OrderAmendsRequest,
) -> Result<OrderAmends, KrakenError> {
    client.call_with("OrderAmends", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_request() {
        let request = OrderAmendsRequest {
            order_id: "OQCLML-BW3P3-BUCMWZ".into(),
            rebase_multiplier: None,
        };
        assert_eq!(encode(&request).unwrap(), "order_id=OQCLML-BW3P3-BUCMWZ");
    }

    #[test]
    fn encodes_the_amend() {
        let request = AmendOrder {
            txid: Some("OQCLML-BW3P3-BUCMWZ".into()),
            order_qty: Some("1.0".into()),
            limit_price: Some("+1.5%".into()),
            post_only: Some(true),
            deadline: Some("2026-11-02T10:00:00Z".into()),
            ..Default::default()
        };
        assert_eq!(
            encode(&request).unwrap(),
            "txid=OQCLML-BW3P3-BUCMWZ&order_qty=1.0&limit_price=%2B1.5%25&post_only=true\
             &deadline=2026-11-02T10%3A00%3A00Z"
        );
    }

    #[test]
    fn decodes_the_amends() {
        let amends: OrderAmends = serde_json::from_str(
            r#"{"count":2,"amends":[
            {"amend_id":"TM2EBP-HGEUP-36CIRK","amend_type":"original","order_qty":"1.25",
                "remaining_qty":"1.25","limit_price":"30010.0","post_only":false,
                "timestamp":1688666559897},
            {"amend_id":"TJSMEH-AA67V-YUSQ6O","amend_type":"user","order_qty":"1.00",
                "display_qty":"0.5","remaining_qty":"1.00","limit_price":"30000.0",
                "reason":"User requested","post_only":true,"timestamp":1688666600000}]}"#,
        )
        .unwrap();
        assert_eq!(amends.count, 2);
        assert_eq!(amends.amends[0].amend_type, AmendType::Original);
        assert_eq!(amends.amends[1].order_quantity, dec!(1));
        assert_eq!(amends.amends[1].display_quantity, Some(dec!(0.5)));
        assert_eq!(amends.amends[1].trigger_price, None);
    }

    #[test]
    fn decodes_every_type() {
        for (text, amend_type) in [
            ("original", AmendType::Original),
            ("user", AmendType::User),
            ("restated", AmendType::Restated),
            ("system", AmendType::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<AmendType>(text.into()).unwrap(),
                amend_type
            );
        }
    }
}
