//! Private funding, the Funding (Beta) API of Kraken: the deposits and the withdrawals, their
//! methods, networks, fees, limits and addresses
//!
//! Called on `/funding/v1/*` by the [`FundingClient`](kraken_network::FundingClient): signed GET,
//! POST, PUT and DELETE, JSON answers. The legacy funding endpoints of `/0/private`, deprecated by
//! Kraken, aren't part of the library.
//!
//! `account_id` targets a wallet of the account (see `wallet_accounts`): the default wallet when
//! missing.

use crate::lenient;
use crate::market::AssetClass;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub(crate) mod assets;
pub(crate) mod deposits;
pub(crate) mod fees;
pub(crate) mod limits;
pub(crate) mod methods;
pub(crate) mod networks;

pub use assets::{FundingAssets, FundingDirection, NamedAsset};
pub use deposits::{AddressDetails, FiatAccount};
pub use fees::{FeeDetails, FundingFee, FundingFeeRequest};
pub use limits::{
    DepositLimits, FundingLimit, FundingLimitsRequest, LimitAmounts, LimitValue, TimedLimit,
    WithdrawalLimit, WithdrawalLimits,
};
pub use methods::{
    AddressGeneration, AddressGenerationStatus, DepositMethodDetails, FundingMethodInfo,
    FundingMethods, FundingMethodsRequest, MethodFees, MethodNetwork,
};
pub use networks::{FundingNetwork, FundingNetworks, NetworkGroup};

/// An asset of the funding API: its class and its name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingAsset {
    pub class: AssetClass,
    /// e.g. "USDC"
    pub name: String,
}

/// An exact amount of an asset
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingAmount {
    pub asset: FundingAsset,
    #[serde(deserialize_with = "lenient::decimal")]
    pub amount: Decimal,
}
