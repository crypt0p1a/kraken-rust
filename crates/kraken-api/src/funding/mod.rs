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

pub(crate) mod addresses;
pub(crate) mod assets;
pub(crate) mod deposits;
pub(crate) mod fees;
pub(crate) mod limits;
pub(crate) mod methods;
pub(crate) mod networks;

pub use addresses::{
    Beneficiary, NewWithdrawalAddress, SavedAddress, WithdrawalAccount, WithdrawalAddress,
    WithdrawalAddressDetails, WithdrawalAddresses, WithdrawalAddressesRequest,
};
pub use assets::{FundingAssets, FundingDirection, NamedAsset};
pub use deposits::{
    AddressDetails, ClaimedAddress, ClaimedAddresses, ClaimedAddressesRequest, Deposit, Deposits,
    DepositsRequest, FiatAccount, TransferStatus,
};
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

/// What a funding request applies to: a method, a network or a group of networks, sent and
/// answered as an object of a single key, e.g. `{"network_id": "..."}`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(untagged)]
pub enum FundingScope {
    MethodId { method_id: String },
    NetworkId { network_id: String },
    NetworkGroupId { network_group_id: String },
}

/// The query of a scope, a deep object: `scope[method_id]`, `scope[network_id]` or
/// `scope[network_group_id]`, flattened into the query of the request
#[derive(Serialize, Default)]
pub(crate) struct ScopeQuery<'a> {
    #[serde(rename = "scope[method_id]", skip_serializing_if = "Option::is_none")]
    method_id: Option<&'a str>,
    #[serde(rename = "scope[network_id]", skip_serializing_if = "Option::is_none")]
    network_id: Option<&'a str>,
    #[serde(
        rename = "scope[network_group_id]",
        skip_serializing_if = "Option::is_none"
    )]
    network_group_id: Option<&'a str>,
}

impl<'a> ScopeQuery<'a> {
    pub(crate) fn new(scope: Option<&'a FundingScope>) -> Self {
        match scope {
            None => ScopeQuery::default(),
            Some(FundingScope::MethodId { method_id }) => ScopeQuery {
                method_id: Some(method_id),
                ..Default::default()
            },
            Some(FundingScope::NetworkId { network_id }) => ScopeQuery {
                network_id: Some(network_id),
                ..Default::default()
            },
            Some(FundingScope::NetworkGroupId { network_group_id }) => ScopeQuery {
                network_group_id: Some(network_group_id),
                ..Default::default()
            },
        }
    }
}

/// The asset filter of a query, a deep object: `asset[class]` and `asset[name]`, the name
/// requiring the class
#[derive(Serialize, Default)]
pub(crate) struct AssetFilterQuery<'a> {
    #[serde(rename = "asset[class]", skip_serializing_if = "Option::is_none")]
    class: Option<AssetClass>,
    #[serde(rename = "asset[name]", skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

impl<'a> AssetFilterQuery<'a> {
    pub(crate) fn new(
        class: Option<AssetClass>,
        name: Option<&'a str>,
    ) -> Result<Self, crate::KrakenError> {
        if name.is_some() && class.is_none() {
            return Err(crate::KrakenError::InvalidArgument {
                reason: "the asset filter requires its class".into(),
            });
        }
        Ok(AssetFilterQuery { class, name })
    }
}

/// An exact amount of an asset
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingAmount {
    pub asset: FundingAsset,
    #[serde(deserialize_with = "lenient::decimal")]
    pub amount: Decimal,
}
