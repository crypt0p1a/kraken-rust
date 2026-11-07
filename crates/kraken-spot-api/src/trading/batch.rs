use super::{NewOrder, NewOrderDescription};
use crate::KrakenError;
use kraken_network::PrivateClient;
use serde::Deserialize;
use serde_json::{Map, Value};

/// The parameters of a batch, not of its orders: they must be the same for every order
const BATCH_FIELDS: [&str; 5] = ["pair", "asset_class", "deadline", "validate", "broker"];

/// The JSON body of a batch: `{"pair", ..., "orders": [{...}, ...]}`, the parameters of the
/// batch being taken out of its orders
fn batch_body(orders: &[NewOrder]) -> Result<Map<String, Value>, KrakenError> {
    if !(2..=15).contains(&orders.len()) {
        return Err(KrakenError::InvalidArgument {
            reason: format!("a batch holds 2 to 15 orders, not {}", orders.len()),
        });
    }
    let mut body = Map::new();
    let mut listed = Vec::with_capacity(orders.len());
    for (index, order) in orders.iter().enumerate() {
        let Value::Object(mut order) = serde_json::to_value(order).map_err(KrakenError::decode)?
        else {
            unreachable!("an order serializes to an object")
        };
        for field in BATCH_FIELDS {
            let value = order.remove(field);
            if index == 0 {
                if let Some(value) = value {
                    body.insert(field.into(), value);
                }
            } else if value.as_ref() != body.get(field) {
                return Err(KrakenError::InvalidArgument {
                    reason: format!("the orders of a batch share their {field}"),
                });
            }
        }
        listed.push(Value::Object(order));
    }
    body.insert("orders".into(), Value::Array(listed));
    Ok(body)
}

/// An order of the batch, in the order of the request
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BatchOrderAnswer {
    #[serde(rename = "descr", default)]
    pub description: Option<NewOrderDescription>,
    /// Kraken identifier, when the order was placed
    #[serde(default)]
    pub txid: Option<String>,
    /// Why this order failed
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct BatchAnswer {
    orders: Vec<BatchOrderAnswer>,
}

/// Sent in JSON, the only way to send a list of orders
pub(crate) async fn add_order_batch(
    client: &PrivateClient,
    orders: &[NewOrder],
) -> Result<Vec<BatchOrderAnswer>, KrakenError> {
    let answer: BatchAnswer = client
        .call_json("AddOrderBatch", &batch_body(orders)?)
        .await?;
    Ok(answer.orders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::{
        OrderFlag, OrderSide, OrderType, SelfTradePrevention, TimeInForce, Trigger,
    };
    use serde_json::json;

    fn limit(side: OrderSide, price: &str) -> NewOrder {
        NewOrder {
            pair: "XBTUSD".into(),
            side,
            order_type: OrderType::Limit,
            volume: "1.2".into(),
            price: Some(price.into()),
            price2: None,
            oflags: None,
            time_in_force: None,
            start_time: None,
            expire_time: None,
            cl_ord_id: None,
            userref: None,
            display_volume: None,
            asset_class: None,
            trigger: None,
            leverage: None,
            reduce_only: None,
            self_trade_prevention: None,
            deadline: Some("2026-11-02T10:00:00Z".into()),
            broker: None,
            validate: Some(true),
        }
    }

    #[test]
    fn takes_the_parameters_of_the_batch_out_of_its_orders() {
        let orders = [
            NewOrder {
                oflags: Some(vec![OrderFlag::Post, OrderFlag::Fciq]),
                time_in_force: Some(TimeInForce::Gtd),
                expire_time: Some("+3600".into()),
                cl_ord_id: Some("mgr1_buy_1".into()),
                ..limit(OrderSide::Buy, "28300.0")
            },
            NewOrder {
                order_type: OrderType::StopLossLimit,
                price2: Some("27000.0".into()),
                trigger: Some(Trigger::Index),
                self_trade_prevention: Some(SelfTradePrevention::CancelBoth),
                reduce_only: Some(false),
                ..limit(OrderSide::Sell, "27100.0")
            },
        ];
        assert_eq!(
            Value::Object(batch_body(&orders).unwrap()),
            json!({"pair": "XBTUSD", "deadline": "2026-11-02T10:00:00Z", "validate": true,
            "orders": [
                {"type": "buy", "ordertype": "limit", "volume": "1.2", "price": "28300.0",
                    "oflags": "post,fciq", "timeinforce": "GTD", "expiretm": "+3600",
                    "cl_ord_id": "mgr1_buy_1"},
                {"type": "sell", "ordertype": "stop-loss-limit", "volume": "1.2",
                    "price": "27100.0", "price2": "27000.0", "trigger": "index",
                    "reduce_only": false, "stptype": "cancel-both"}
            ]})
        );
    }

    #[test]
    fn refuses_the_orders_of_different_batches() {
        let other_pair = NewOrder {
            pair: "ETHUSD".into(),
            ..limit(OrderSide::Sell, "1")
        };
        assert!(batch_body(&[limit(OrderSide::Buy, "1"), other_pair]).is_err());
        let not_validated = NewOrder {
            validate: None,
            ..limit(OrderSide::Sell, "1")
        };
        assert!(batch_body(&[limit(OrderSide::Buy, "1"), not_validated]).is_err());
        assert!(
            batch_body(&[limit(OrderSide::Buy, "1")]).is_err(),
            "a single order"
        );
    }

    #[test]
    fn decodes_the_answers_of_each_order() {
        // example of the documentation, plus an order failing
        let answer: BatchAnswer = serde_json::from_str(
            r#"{"orders":[
            {"txid":"O5OR23-ADFAD-Y2G61C","descr":{"order":"buy 0.80300000 XBTUSD @ limit 28300.0",
                "close":"close position @ stop loss 27000.0 -> limit 26000.0"}},
            {"error":"EOrder:Insufficient funds"}]}"#,
        )
        .unwrap();
        assert_eq!(
            answer.orders[0].txid.as_deref(),
            Some("O5OR23-ADFAD-Y2G61C")
        );
        assert!(
            answer.orders[0]
                .description
                .as_ref()
                .unwrap()
                .close
                .is_some()
        );
        assert_eq!(
            answer.orders[1].error.as_deref(),
            Some("EOrder:Insufficient funds")
        );
        assert_eq!(answer.orders[1].txid, None);
    }
}
