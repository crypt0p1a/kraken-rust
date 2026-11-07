use super::RebaseMultiplier;
use crate::KrakenError;
use crate::lenient;
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// <https://docs.kraken.com/api/docs/rest-api/get-trade-balance>
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradeBalanceRequest {
    /// Asset the balances are expressed in, e.g. "ZUSD": "ZUSD" when missing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// Collateral, margin positions, equity and margin level, in the requested asset
///
/// <https://docs.kraken.com/api/docs/rest-api/get-trade-balance>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TradeBalance {
    /// Combined balance of all the currencies
    #[serde(rename = "eb", deserialize_with = "lenient::decimal")]
    pub equivalent_balance: Decimal,
    /// Combined balance of the equity currencies
    #[serde(rename = "tb", deserialize_with = "lenient::decimal")]
    pub trade_balance: Decimal,
    /// Margin of the open positions
    #[serde(rename = "m", deserialize_with = "lenient::decimal")]
    pub margin: Decimal,
    /// Unrealized net profit / loss of the open positions
    #[serde(rename = "n", deserialize_with = "lenient::decimal")]
    pub unrealized_net: Decimal,
    /// Cost basis of the open positions
    #[serde(rename = "c", deserialize_with = "lenient::decimal")]
    pub cost_basis: Decimal,
    /// Current floating valuation of the open positions
    #[serde(rename = "v", deserialize_with = "lenient::decimal")]
    pub valuation: Decimal,
    /// Trade balance + unrealized net profit / loss
    #[serde(rename = "e", deserialize_with = "lenient::decimal")]
    pub equity: Decimal,
    /// Equity - initial margin: the margin available to new positions
    #[serde(rename = "mf", deserialize_with = "lenient::decimal")]
    pub free_margin: Decimal,
    /// Margin available to new orders
    #[serde(rename = "mfo", default, deserialize_with = "lenient::opt_decimal")]
    pub free_margin_for_orders: Option<Decimal>,
    /// (equity / initial margin) × 100: only with open positions
    #[serde(rename = "ml", default, deserialize_with = "lenient::opt_decimal")]
    pub margin_level: Option<Decimal>,
    /// Value of the unfilled and partially filled orders
    #[serde(rename = "uv", deserialize_with = "lenient::decimal")]
    pub unexecuted_value: Decimal,
}

pub(crate) async fn trade_balance(
    client: &PrivateClient,
    request: &TradeBalanceRequest,
) -> Result<TradeBalance, KrakenError> {
    client.call_with("TradeBalance", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_request() {
        let request = TradeBalanceRequest {
            asset: Some("ZEUR".into()),
            rebase_multiplier: Some(RebaseMultiplier::Base),
        };
        assert_eq!(
            encode(&request).unwrap(),
            "asset=ZEUR&rebase_multiplier=base"
        );
        assert_eq!(encode(&TradeBalanceRequest::default()).unwrap(), "");
    }

    #[test]
    fn decodes_the_trade_balance() {
        let balance: TradeBalance = serde_json::from_str(
            r#"{"eb":"3224744.0162","tb":"3224744.0162","m":"0.0000","n":"0.0000",
            "c":"0.0000","v":"0.0000","e":"3224744.0162","mf":"3224744.0162","uv":"0.0000"}"#,
        )
        .unwrap();
        assert_eq!(balance.equivalent_balance, dec!(3224744.0162));
        assert_eq!(balance.free_margin, dec!(3224744.0162));
        assert_eq!(balance.margin_level, None);
        assert_eq!(balance.free_margin_for_orders, None);
    }

    #[test]
    fn requires_every_documented_value() {
        assert!(
            serde_json::from_str::<TradeBalance>(
                r#"{"eb":"1","tb":"1","m":"0","n":"0","c":"0","v":"0","e":"1","mf":"1"}"#
            )
            .is_err(),
            "uv is required"
        );
    }
}
