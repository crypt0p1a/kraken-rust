use super::OrderFlag;
use crate::market::AssetClass;
use crate::{KrakenError, lenient};
use kraken_network::PrivateClient;
use kraken_network::params::opt_comma_separated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Replaces an open order by a new one: prefer [`super::AmendOrder`], which keeps the order
///
/// <https://docs.kraken.com/api/docs/rest-api/edit-order>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EditOrder {
    /// Kraken identifier of the order, or its userref
    pub txid: String,
    /// e.g. "XBTUSD"
    pub pair: String,
    /// User reference of the new order
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
    /// New volume, in the base asset
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub volume: Option<String>,
    /// New volume shown in the book, for the iceberg orders
    #[serde(rename = "displayvol", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub display_volume: Option<String>,
    /// [`AssetClass::TokenizedAsset`] for the xStocks
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// New limit or trigger price
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price: Option<String>,
    /// New secondary price
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price2: Option<String>,
    /// Only `post` can be changed
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub oflags: Option<Vec<OrderFlag>>,
    /// RFC 3339: the new order is rejected after it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub deadline: Option<String>,
    /// Answers as soon as the replacement is pending
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cancel_response: Option<bool>,
    /// Only validates the inputs: nothing is replaced
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub validate: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EditedOrderDescription {
    /// e.g. "buy 0.00030000 XXBTZGBP @ limit 19500.0"
    pub order: String,
}

/// <https://docs.kraken.com/api/docs/rest-api/edit-order>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct EditedOrder {
    /// "ok" or "err"
    pub status: String,
    /// The new order, when placed
    #[serde(default)]
    pub txid: Option<String>,
    /// The replaced order
    #[serde(rename = "originaltxid", default)]
    pub original_txid: Option<String>,
    #[serde(
        rename = "newuserref",
        default,
        deserialize_with = "lenient::opt_string"
    )]
    pub new_userref: Option<String>,
    #[serde(
        rename = "olduserref",
        default,
        deserialize_with = "lenient::opt_string"
    )]
    pub old_userref: Option<String>,
    /// 0 or 1
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub orders_cancelled: Option<i32>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub volume: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub price: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub price2: Option<Decimal>,
    #[serde(rename = "descr", default)]
    pub description: Option<EditedOrderDescription>,
    #[serde(default)]
    pub error_message: Option<String>,
}

pub(crate) async fn edit_order(
    client: &PrivateClient,
    request: &EditOrder,
) -> Result<EditedOrder, KrakenError> {
    client.call_with("EditOrder", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_edit() {
        let request = EditOrder {
            txid: "OHYO67-6LP66-HMQ437".into(),
            pair: "XBTGBP".into(),
            volume: Some("0.0003".into()),
            price: Some("19500.0".into()),
            price2: Some("32500.0".into()),
            oflags: Some(vec![OrderFlag::Post]),
            validate: Some(true),
            ..Default::default()
        };
        assert_eq!(
            encode(&request).unwrap(),
            "txid=OHYO67-6LP66-HMQ437&pair=XBTGBP&volume=0.0003&price=19500.0&price2=32500.0\
             &oflags=post&validate=true"
        );
    }

    #[test]
    fn decodes_the_edited_order() {
        // example of the documentation
        let edited: EditedOrder = serde_json::from_str(
            r#"{"status":"ok","txid":"OFVXHJ-KPQ3B-VS7ELA","originaltxid":"OHYO67-6LP66-HMQ437",
            "volume":"0.00030000","price":"19500.0","price2":"32500.0","orders_cancelled":1,
            "descr":{"order":"buy 0.00030000 XXBTZGBP @ limit 19500.0"}}"#,
        )
        .unwrap();
        assert_eq!(edited.txid.as_deref(), Some("OFVXHJ-KPQ3B-VS7ELA"));
        assert_eq!(edited.orders_cancelled, Some(1));
        assert_eq!(edited.price2, Some(dec!(32500)));
    }

    #[test]
    fn decodes_a_failed_edit() {
        let edited: EditedOrder = serde_json::from_str(
            r#"{"status":"err","originaltxid":"OHYO67-6LP66-HMQ437","olduserref":42,
            "error_message":"Order not found"}"#,
        )
        .unwrap();
        assert_eq!(edited.txid, None);
        assert_eq!(edited.old_userref.as_deref(), Some("42"));
    }
}
