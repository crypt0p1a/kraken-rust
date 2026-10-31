use crate::KrakenError;
use crate::lenient;
use kraken_network::PrivateClient;
use serde::Deserialize;
use std::collections::HashMap;

/// <https://docs.kraken.com/api/docs/rest-api/get-extended-balance>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ExtendedBalance {
    #[serde(deserialize_with = "lenient::number")]
    pub balance: f64,
    /// Held by the open orders
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub hold_trade: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub credit: Option<f64>,
    #[serde(default, deserialize_with = "lenient::opt_number")]
    pub credit_used: Option<f64>,
}

impl ExtendedBalance {
    /// What can be used by a new order: balance + credit - credit_used - hold_trade
    pub fn available(&self) -> f64 {
        self.balance + self.credit.unwrap_or_default()
            - self.credit_used.unwrap_or_default()
            - self.hold_trade.unwrap_or_default()
    }
}

/// Keyed by asset, e.g. "XXBT", "ZUSD"
pub(crate) async fn extended_balance(
    client: &PrivateClient,
) -> Result<HashMap<String, ExtendedBalance>, KrakenError> {
    client.call("BalanceEx").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_the_available_balance() {
        let balances: HashMap<String, ExtendedBalance> = serde_json::from_str(
            r#"{"ZUSD":{"balance":"25435.21","hold_trade":"8249.76","credit":"0.0000",
                "credit_used":"0.0000"},"XXBT":{"balance":"1.2"}}"#,
        )
        .unwrap();

        assert!((balances["ZUSD"].available() - 17185.45).abs() < 1e-9);
        assert_eq!(balances["XXBT"].available(), 1.2);
    }

    #[test]
    fn counts_the_credit_in_the_available_balance() {
        let balance: ExtendedBalance = serde_json::from_str(
            r#"{"balance":"100","hold_trade":"30","credit":"50","credit_used":"20"}"#,
        )
        .unwrap();
        assert_eq!(balance.available(), 100.0);
    }

    #[test]
    fn requires_the_balance() {
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"hold_trade":"1"}"#).is_err());
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"balance":null}"#).is_err());
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"balance":"abc"}"#).is_err());
    }
}
