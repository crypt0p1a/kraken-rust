//! Private account data: balances, 30 days volume and fees
//!
//! Kraken has no Spot sub-accounts: the balances are the ones of the account of the API key.

pub(crate) mod balance;
pub(crate) mod fees;

pub use balance::{BalanceRequest, ExtendedBalance, RebaseMultiplier};
pub use fees::{FeeInfo, TradeVolume, VolumeInputs};
