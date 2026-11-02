use crate::KrakenError;
use kraken_network::PrivateClient;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum WalletStatus {
    Active,
    Disabled,
    Closed,
    /// Sent as "unknown" by Kraken, or unknown to this version of the library
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    /// The primary account
    Main,
    Spot,
    Pay,
    PropPaper,
    PropReal,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WalletFlags {
    /// Created by the user, false for the wallets created by Kraken
    pub user_defined: bool,
    pub active: bool,
}

/// A wallet of the account
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WalletAccount {
    /// Public identifier of the wallet, e.g. "WX6V-JUKW-KKPB-QE36"
    pub account_id: String,
    pub flags: WalletFlags,
    pub status: WalletStatus,
    #[serde(rename = "type")]
    pub wallet_type: WalletType,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WalletCursor {
    /// None when there is no further page
    #[serde(default)]
    pub next: Option<String>,
}

/// <https://docs.kraken.com/api/docs/rest-api/list-wallet-accounts>
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct WalletAccounts {
    pub accounts: Vec<WalletAccount>,
    pub cursor: WalletCursor,
}

pub(crate) async fn wallet_accounts(client: &PrivateClient) -> Result<WalletAccounts, KrakenError> {
    client.call("ListWalletAccounts").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_wallets() {
        // example of the documentation
        let wallets: WalletAccounts = serde_json::from_str(
            r#"{"accounts":[{"account_id":"WX6V-JUKW-KKPB-QE36",
            "flags":{"user_defined":false,"active":true},"status":"active","type":"main",
            "name":null}],"cursor":{"next":null}}"#,
        )
        .unwrap();
        let main = &wallets.accounts[0];
        assert_eq!(main.wallet_type, WalletType::Main);
        assert_eq!(main.status, WalletStatus::Active);
        assert!(main.flags.active && !main.flags.user_defined);
        assert_eq!(main.name, None);
        assert_eq!(wallets.cursor.next, None);
    }

    #[test]
    fn decodes_every_type_and_status() {
        for (text, wallet_type) in [
            ("main", WalletType::Main),
            ("spot", WalletType::Spot),
            ("pay", WalletType::Pay),
            ("prop_paper", WalletType::PropPaper),
            ("prop_real", WalletType::PropReal),
            ("unknown", WalletType::Unknown),
            ("earn", WalletType::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<WalletType>(text.into()).unwrap(),
                wallet_type
            );
        }
        for (text, status) in [
            ("active", WalletStatus::Active),
            ("disabled", WalletStatus::Disabled),
            ("closed", WalletStatus::Closed),
            ("unknown", WalletStatus::Unknown),
        ] {
            assert_eq!(
                serde_json::from_value::<WalletStatus>(text.into()).unwrap(),
                status
            );
        }
    }
}
