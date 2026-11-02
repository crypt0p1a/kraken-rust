use super::BalanceRequest;
use crate::KrakenError;
use crate::lenient;
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;

/// Balance and credit line of an asset
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CreditLineAsset {
    #[serde(deserialize_with = "lenient::decimal")]
    pub balance: Decimal,
    /// Held by the open orders
    #[serde(deserialize_with = "lenient::decimal")]
    pub hold_trade: Decimal,
    /// Credit limit: only for the assets with a credit line
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub credit: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub credit_used: Option<Decimal>,
    /// Collateral factor, 0 to 1: only for the eligible assets
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub collateral_value: Option<Decimal>,
    /// Rate of the drawn credit, in percent per rollover period
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub rollover_fees: Option<Decimal>,
    /// Rate of the undrawn credit, in percent per rollover period
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub reserve_fees: Option<Decimal>,
}

/// Summary of the credit over every asset, in USD
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CreditLimits {
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub total_credit_usd: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub total_credit_used_usd: Option<Decimal>,
    /// Sum of the balances × their collateral factor
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub total_collateral_value_usd: Option<Decimal>,
    /// Total collateral - total credit
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub equity_usd: Option<Decimal>,
    /// Total collateral / total credit
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub ongoing_balance: Option<Decimal>,
    /// Total credit used / equity
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub debt_to_equity: Option<Decimal>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-credit-lines>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct CreditLines {
    /// Keyed by asset, e.g. "ZUSD"
    pub asset_details: HashMap<String, CreditLineAsset>,
    pub limits_monitor: CreditLimits,
}

/// Only for the VIP accounts with credit lines
pub(crate) async fn credit_lines(
    client: &PrivateClient,
    request: &BalanceRequest,
) -> Result<CreditLines, KrakenError> {
    client.call_with("CreditLines", Some(request)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn decodes_the_credit_lines() {
        let lines: CreditLines = serde_json::from_str(
            r#"{"asset_details":{
                "ZUSD":{"balance":"1000.5000","hold_trade":"0.0000","credit":"50000.0000",
                    "credit_used":"12500.0000","rollover_fees":"0.0100","reserve_fees":"0.0050"},
                "XXBT":{"balance":"2.5","hold_trade":"0.1","collateral_value":0.95}},
            "limits_monitor":{"total_credit_usd":"50000.00","total_credit_used_usd":"12500.00",
                "total_collateral_value_usd":"160000.00","equity_usd":"110000.00",
                "ongoing_balance":"3.2000","debt_to_equity":"0.1136"}}"#,
        )
        .unwrap();

        let usd = &lines.asset_details["ZUSD"];
        assert_eq!(usd.credit_used, Some(dec!(12500)));
        assert_eq!(usd.collateral_value, None);
        assert_eq!(
            lines.asset_details["XXBT"].collateral_value,
            Some(dec!(0.95))
        );
        assert_eq!(lines.limits_monitor.debt_to_equity, Some(dec!(0.1136)));
    }

    #[test]
    fn requires_the_balances_and_the_monitor() {
        assert!(serde_json::from_str::<CreditLines>(r#"{"asset_details":{}}"#).is_err());
        assert!(
            serde_json::from_str::<CreditLineAsset>(r#"{"balance":"1"}"#).is_err(),
            "hold_trade is required"
        );
    }
}
