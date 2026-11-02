//! Private account data: balances, 30 days volume and fees
//!
//! Kraken has no Spot sub-accounts: the balances are the ones of the account of the API key.

pub(crate) mod api_key;
pub(crate) mod balance;
pub(crate) mod credit;
pub(crate) mod fees;
pub(crate) mod ledgers;
pub(crate) mod trade_balance;
pub(crate) mod wallets;

pub use api_key::{ApiKeyInfo, ApiKeyInfoRequest};
pub use balance::{BalanceRequest, ExtendedBalance, RebaseMultiplier};
pub use credit::{CreditLimits, CreditLineAsset, CreditLines};
pub use fees::{FeeInfo, TradeVolume, VolumeInputs};
pub use ledgers::{LedgerEntry, LedgerFilter, LedgerType, Ledgers, LedgersRequest};
pub use trade_balance::{TradeBalance, TradeBalanceRequest};
pub use wallets::{WalletAccount, WalletAccounts, WalletFlags, WalletStatus, WalletType};
