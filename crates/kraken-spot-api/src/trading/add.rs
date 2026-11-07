use super::{OrderFlag, OrderSide, OrderType, SelfTradePrevention, TimeInForce, Trigger};
use crate::KrakenError;
use crate::market::AssetClass;
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
    /// `0` (now), `<UTC timestamp in seconds>` or `+<seconds from now>`
    #[serde(rename = "starttm", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub start_time: Option<String>,
    /// Required by GTD: `<UTC timestamp in seconds>` or `+<seconds from now>`
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
    /// Volume shown in the book, for the iceberg orders
    #[serde(rename = "displayvol", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub display_volume: Option<String>,
    /// [`AssetClass::TokenizedAsset`] for the xStocks
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// Price signal of the stop-loss and take-profit orders: the last trade when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub trigger: Option<Trigger>,
    /// e.g. "2:1": none when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub leverage: Option<String>,
    /// Only reduces an open margin position
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub reduce_only: Option<bool>,
    #[serde(rename = "stptype", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub self_trade_prevention: Option<SelfTradePrevention>,
    /// RFC 3339: the order is rejected after it
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub deadline: Option<String>,
    /// IIBAN of the broker (partner) of the order
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub broker: Option<String>,
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
            display_volume: None,
            asset_class: None,
            trigger: None,
            leverage: None,
            reduce_only: None,
            self_trade_prevention: None,
            deadline: None,
            broker: None,
        }
    }

    #[test]
    fn encodes_the_parameters_of_the_margin_iceberg_and_triggered_orders() {
        let order = NewOrder {
            pair: "AAPLxUSD".into(),
            side: OrderSide::Sell,
            order_type: OrderType::StopLossLimit,
            volume: "2".into(),
            price: Some("180".into()),
            price2: Some("179.5".into()),
            oflags: None,
            time_in_force: Some(TimeInForce::Fok),
            start_time: None,
            expire_time: None,
            cl_ord_id: None,
            userref: None,
            display_volume: Some("0.5".into()),
            asset_class: Some(AssetClass::TokenizedAsset),
            trigger: Some(Trigger::Index),
            leverage: Some("2:1".into()),
            reduce_only: Some(true),
            self_trade_prevention: Some(SelfTradePrevention::CancelOldest),
            deadline: Some("2026-11-02T10:00:00Z".into()),
            broker: Some("AA88 N84G WOAK NMOI".into()),
            validate: Some(true),
        };
        assert_eq!(
            encode(&order).unwrap(),
            "pair=AAPLxUSD&type=sell&ordertype=stop-loss-limit&volume=2&price=180&price2=179.5\
             &timeinforce=FOK&displayvol=0.5&asset_class=tokenized_asset&trigger=index\
             &leverage=2%3A1&reduce_only=true&stptype=cancel-oldest\
             &deadline=2026-11-02T10%3A00%3A00Z&broker=AA88+N84G+WOAK+NMOI&validate=true"
        );
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

    #[test]
    fn encodes_every_parameter_of_a_new_order() {
        let order = NewOrder {
            pair: "XBTUSD".into(),
            side: OrderSide::Sell,
            order_type: OrderType::StopLossLimit,
            volume: "0.5".into(),
            price: Some("25000".into()),
            price2: Some("24900".into()),
            oflags: Some(vec![OrderFlag::Fcib, OrderFlag::Nompp]),
            time_in_force: Some(TimeInForce::Ioc),
            start_time: Some("0".into()),
            expire_time: Some("1700000000".into()),
            cl_ord_id: None,
            userref: Some(42),
            validate: Some(true),
            display_volume: None,
            asset_class: None,
            trigger: None,
            leverage: None,
            reduce_only: None,
            self_trade_prevention: None,
            deadline: None,
            broker: None,
        };
        assert_eq!(
            encode(&order).unwrap(),
            "pair=XBTUSD&type=sell&ordertype=stop-loss-limit&volume=0.5&price=25000\
             &price2=24900&oflags=fcib%2Cnompp&timeinforce=IOC&starttm=0\
             &expiretm=1700000000&userref=42&validate=true"
        );
    }

    #[test]
    fn encodes_the_minimal_market_order() {
        let order = NewOrder {
            order_type: OrderType::Market,
            price: None,
            oflags: Some(vec![OrderFlag::Viqc]),
            time_in_force: None,
            expire_time: None,
            cl_ord_id: None,
            ..limit_buy()
        };
        assert_eq!(
            encode(&order).unwrap(),
            "pair=XBTUSD&type=buy&ordertype=market&volume=1.25&oflags=viqc"
        );
    }

    #[test]
    fn escapes_the_client_order_ids() {
        let order = NewOrder {
            cl_ord_id: Some("a=b&c d/é".into()),
            ..limit_buy()
        };
        assert!(
            encode(&order)
                .unwrap()
                .contains("cl_ord_id=a%3Db%26c+d%2F%C3%A9")
        );
    }

    #[test]
    fn decodes_the_answer_of_a_validated_order() {
        let answer: NewOrderAnswer =
            serde_json::from_str(r#"{"descr":{"order":"buy 0.00010 XBTUSD @ limit 38000.0"}}"#)
                .unwrap();
        assert!(answer.txids.is_empty());
        assert!(serde_json::from_str::<NewOrderAnswer>(r#"{"txid":[]}"#).is_err());
    }
}
