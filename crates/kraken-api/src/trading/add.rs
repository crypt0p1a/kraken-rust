use super::{OrderFlag, OrderSide, OrderType, TimeInForce};
use crate::KrakenError;
use kraken_network::PrivateClient;
use kraken_network::params::opt_comma_separated;
use serde::{Deserialize, Serialize};

/// <https://docs.kraken.com/api/docs/rest-api/add-order>
///
/// The amounts and prices are strings, to keep the precision given by the caller
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewOrder {
    /// Alternate name of the pair, e.g. "XBTUSD"
    pub pair: String,
    #[serde(rename = "type")]
    pub side: OrderSide,
    #[serde(rename = "ordertype")]
    pub order_type: OrderType,
    /// Volume, in the base currency (in the quote currency with [`OrderFlag::Viqc`])
    pub volume: String,
    /// Limit price, or trigger price of the stop / take profit orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price: Option<String>,
    /// Limit price of the stop-loss-limit / take-profit-limit orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub price2: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_comma_separated"
    )]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub oflags: Option<Vec<OrderFlag>>,
    /// GTC when missing
    #[serde(rename = "timeinforce", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub time_in_force: Option<TimeInForce>,
    /// "0" (now), "<UTC timestamp in seconds>" or "+<seconds from now>"
    #[serde(rename = "starttm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// Required by GTD: "<UTC timestamp in seconds>" or "+<seconds from now>"
    #[serde(rename = "expiretm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub expire_time: Option<String>,
    /// Client identifier: a UUID, or up to 18 ASCII characters. Exclusive with `userref`
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub cl_ord_id: Option<String>,
    /// Numeric client identifier, shared by a group of orders
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub userref: Option<i32>,
    /// Validates the parameters without placing the order
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub validate: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewOrderDescription {
    /// e.g. "buy 1.25000000 XBTUSD @ limit 27500.0"
    pub order: String,
    #[serde(default)]
    pub close: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewOrderAnswer {
    #[serde(rename = "descr")]
    pub description: NewOrderDescription,
    /// Identifiers of the orders placed, empty when only validated
    #[serde(rename = "txid", default)]
    pub txids: Vec<String>,
}

/// The checks Kraken would fail on, done before calling it
pub(crate) fn check(request: &NewOrder) -> Result<(), KrakenError> {
    let invalid = |reason: &str| {
        Err(KrakenError::InvalidArgument {
            reason: reason.into(),
        })
    };

    if request.order_type == OrderType::Unknown {
        return invalid("OrderType::Unknown can't be sent");
    }

    if request.time_in_force == Some(TimeInForce::Unknown) {
        return invalid("TimeInForce::Unknown can't be sent");
    }

    if request.cl_ord_id.is_some() && request.userref.is_some() {
        return invalid("cl_ord_id and userref can't be both given");
    }

    if request.time_in_force == Some(TimeInForce::Gtd) && request.expire_time.is_none() {
        return invalid("a GTD order requires an expire_time");
    }

    Ok(())
}

pub(crate) async fn add_order(
    client: &PrivateClient,
    request: &NewOrder,
) -> Result<NewOrderAnswer, KrakenError> {
    check(request)?;
    client.call_with("AddOrder", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    fn limit_buy() -> NewOrder {
        NewOrder {
            pair: "XBTUSD".into(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            volume: "1.25".into(),
            price: Some("27500.0".into()),
            price2: None,
            oflags: Some(vec![OrderFlag::Post, OrderFlag::Fciq]),
            time_in_force: Some(TimeInForce::Gtd),
            start_time: None,
            expire_time: Some("+43200".into()),
            cl_ord_id: Some("mgr1_buy_1".into()),
            userref: None,
            validate: None,
        }
    }

    #[test]
    fn encodes_the_new_order() {
        assert_eq!(
            encode(&limit_buy()).unwrap(),
            "pair=XBTUSD&type=buy&ordertype=limit&volume=1.25&price=27500.0&oflags=post%2Cfciq\
             &timeinforce=GTD&expiretm=%2B43200&cl_ord_id=mgr1_buy_1"
        );
    }

    #[test]
    fn checks_the_requests_before_sending_them() {
        assert_eq!(check(&limit_buy()), Ok(()));

        let invalid = [
            NewOrder {
                order_type: OrderType::Unknown,
                ..limit_buy()
            },
            NewOrder {
                time_in_force: Some(TimeInForce::Unknown),
                ..limit_buy()
            },
            NewOrder {
                userref: Some(1),
                ..limit_buy()
            },
            NewOrder {
                expire_time: None,
                ..limit_buy()
            },
        ];
        for request in invalid {
            assert!(
                matches!(check(&request), Err(KrakenError::InvalidArgument { .. })),
                "{request:?}"
            );
        }

        // without time in force, no expiry is required
        let gtc = NewOrder {
            time_in_force: None,
            expire_time: None,
            ..limit_buy()
        };
        assert_eq!(check(&gtc), Ok(()));
    }

    #[test]
    fn decodes_the_answer() {
        let answer: NewOrderAnswer = serde_json::from_str(
            r#"{"descr":{"order":"buy 1.25000000 XBTUSD @ limit 27500.0"},
                "txid":["OU22CG-KLAF2-FWUDD7"]}"#,
        )
        .unwrap();
        assert_eq!(answer.txids, vec!["OU22CG-KLAF2-FWUDD7"]);
        assert_eq!(answer.description.close, None);
    }
}
