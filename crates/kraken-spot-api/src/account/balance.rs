use crate::KrakenError;
use crate::lenient::{self, ExactDecimal};
use kraken_network::PrivateClient;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How the amounts of the tokenized assets (xStocks) are shown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "lowercase")]
pub enum RebaseMultiplier {
    /// In units of the underlying equity
    Rebased,
    /// In units of the token
    Base,
}

/// The parameters of the balances
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct BalanceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub rebase_multiplier: Option<RebaseMultiplier>,
}

/// <https://docs.kraken.com/api/docs/rest-api/get-extended-balance>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ExtendedBalance {
    #[serde(deserialize_with = "lenient::decimal")]
    pub balance: Decimal,
    /// Held by the open orders
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub hold_trade: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub credit: Option<Decimal>,
    #[serde(default, deserialize_with = "lenient::opt_decimal")]
    pub credit_used: Option<Decimal>,
}

impl ExtendedBalance {
    /// What can be used by a new order: balance + credit - credit_used - hold_trade
    pub fn available(&self) -> Decimal {
        self.balance + self.credit.unwrap_or_default()
            - self.credit_used.unwrap_or_default()
            - self.hold_trade.unwrap_or_default()
    }
}

/// The cash balances, net of the pending withdrawals, keyed by asset, e.g. "XXBT", "ZUSD"
///
/// <https://docs.kraken.com/api/docs/rest-api/get-account-balance>
pub(crate) async fn balance(
    client: &PrivateClient,
    request: &BalanceRequest,
) -> Result<HashMap<String, Decimal>, KrakenError> {
    let balances: HashMap<String, ExactDecimal> =
        client.call_with("Balance", Some(request)).await?;
    Ok(balances
        .into_iter()
        .map(|(asset, amount)| (asset, amount.0))
        .collect())
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
    use kraken_network::params::encode;
    use rust_decimal_macros::dec;

    #[test]
    fn encodes_the_balance_request() {
        let request = BalanceRequest {
            rebase_multiplier: Some(RebaseMultiplier::Rebased),
        };
        assert_eq!(encode(&request).unwrap(), "rebase_multiplier=rebased");
        let request = BalanceRequest {
            rebase_multiplier: Some(RebaseMultiplier::Base),
        };
        assert_eq!(encode(&request).unwrap(), "rebase_multiplier=base");
        assert_eq!(encode(&BalanceRequest::default()).unwrap(), "");
    }

    #[test]
    fn decodes_the_balances_exactly() {
        let balances: HashMap<String, ExactDecimal> =
            serde_json::from_str(r#"{"ZUSD":"2970172.7962","XXBT":"0.0000000001","XETH":"0"}"#)
                .unwrap();
        assert_eq!(balances["ZUSD"].0, dec!(2970172.7962));
        assert_eq!(balances["XXBT"].0.to_string(), "0.0000000001");
        assert!(serde_json::from_str::<HashMap<String, ExactDecimal>>(r#"{"A":null}"#).is_err());
    }

    #[test]
    fn computes_the_available_balance() {
        let balances: HashMap<String, ExtendedBalance> = serde_json::from_str(
            r#"{"ZUSD":{"balance":"25435.21","hold_trade":"8249.76","credit":"0.0000",
                "credit_used":"0.0000"},"XXBT":{"balance":"1.2"}}"#,
        )
        .unwrap();

        assert_eq!(balances["ZUSD"].available(), dec!(17185.45));
        assert_eq!(balances["XXBT"].available(), dec!(1.2));
    }

    #[test]
    fn counts_the_credit_in_the_available_balance() {
        let balance: ExtendedBalance = serde_json::from_str(
            r#"{"balance":"100","hold_trade":"30","credit":"50","credit_used":"20"}"#,
        )
        .unwrap();
        assert_eq!(balance.available(), dec!(100.0));
    }

    #[test]
    fn requires_the_balance() {
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"hold_trade":"1"}"#).is_err());
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"balance":null}"#).is_err());
        assert!(serde_json::from_str::<ExtendedBalance>(r#"{"balance":"abc"}"#).is_err());
    }
}
