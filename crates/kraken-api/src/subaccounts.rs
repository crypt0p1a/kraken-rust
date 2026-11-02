//! Private subaccounts: for the institutional accounts only, called with a key of the master
//! account having the "Funds permissions - Withdraw" permission

use crate::KrakenError;
use crate::market::AssetClass;
use kraken_network::PrivateClient;
use serde::{Deserialize, Serialize};

/// <https://docs.kraken.com/api/docs/rest-api/create-subaccount>
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct NewSubaccount {
    pub username: String,
    pub email: String,
}

/// Whether the subaccount was created
pub(crate) async fn create_subaccount(
    client: &PrivateClient,
    subaccount: &NewSubaccount,
) -> Result<bool, KrakenError> {
    client.call_with("CreateSubaccount", Some(subaccount)).await
}

/// A transfer between the master account and a subaccount
///
/// <https://docs.kraken.com/api/docs/rest-api/account-transfer>
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct AccountTransfer {
    /// e.g. "XBT"
    pub asset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "uniffi", uniffi(default = None))]
    pub asset_class: Option<AssetClass>,
    /// A string, to keep the precision of the caller
    pub amount: String,
    /// Public id of the source account, e.g. "ABCD 1234 EFGH 5678"
    pub from: String,
    /// Public id of the destination account
    pub to: String,
}

/// <https://docs.kraken.com/api/docs/rest-api/account-transfer>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TransferResult {
    pub transfer_id: String,
    /// "pending" or "complete"
    pub status: String,
}

pub(crate) async fn account_transfer(
    client: &PrivateClient,
    transfer: &AccountTransfer,
) -> Result<TransferResult, KrakenError> {
    client.call_with("AccountTransfer", Some(transfer)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_network::params::encode;

    #[test]
    fn encodes_the_new_subaccount() {
        let subaccount = NewSubaccount {
            username: "bot-1".into(),
            email: "bot+1@example.com".into(),
        };
        assert_eq!(
            encode(&subaccount).unwrap(),
            "username=bot-1&email=bot%2B1%40example.com"
        );
    }

    #[test]
    fn encodes_the_transfer() {
        let transfer = AccountTransfer {
            asset: "XBT".into(),
            asset_class: Some(AssetClass::Currency),
            amount: "0.5".into(),
            from: "ABCD 1234 EFGH 5678".into(),
            to: "IJKL 0987 MNOP 6543".into(),
        };
        assert_eq!(
            encode(&transfer).unwrap(),
            "asset=XBT&asset_class=currency&amount=0.5&from=ABCD+1234+EFGH+5678\
             &to=IJKL+0987+MNOP+6543"
        );
    }

    #[test]
    fn decodes_the_transfer() {
        let result: TransferResult =
            serde_json::from_str(r#"{"transfer_id":"TOH3AS2-LPCWR8-JDQGEU","status":"complete"}"#)
                .unwrap();
        assert_eq!(result.status, "complete");
    }
}
