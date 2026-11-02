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
pub(crate) mod withdrawals;

pub use addresses::{
    NewWithdrawalAddress, SavedAddress, WithdrawalAddress, WithdrawalAddresses,
    WithdrawalAddressesRequest,
};
pub use assets::{FundingAssets, FundingDirection, NamedAsset};
pub use deposits::{
    ClaimedAddress, ClaimedAddresses, ClaimedAddressesRequest, Deposit, Deposits, DepositsRequest,
    TransferStatus,
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
pub use withdrawals::{
    CreatedWithdrawal, NewWithdrawal, Withdrawal, WithdrawalAmount, WithdrawalFee,
    WithdrawalStatus, Withdrawals, WithdrawalsRequest,
};

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

/// An address of a deposit or a withdrawal: `{"crypto": {...}}` or `{"fiat": {...}}`
// a decoded answer, not moved around: no Box, which the bindings can't carry
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "snake_case")]
pub enum AddressDetails {
    Crypto {
        address: String,
        /// Destination tag, for the networks requiring one
        #[serde(default)]
        tag: Option<String>,
        #[serde(default)]
        memo: Option<String>,
        /// The withdrawal addresses only
        #[serde(default)]
        beneficiary: Option<Beneficiary>,
    },
    Fiat {
        #[serde(flatten)]
        account: FiatAccount,
    },
}

/// The beneficiary of a withdrawal address (travel rule)
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Beneficiary {
    /// "sender" (the user) or "other"
    #[serde(default)]
    pub recipient: Option<String>,
    /// "individual" or "business"
    #[serde(default)]
    pub typ: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub state_province: Option<String>,
    #[serde(default)]
    pub address_1: Option<String>,
    #[serde(default)]
    pub address_2: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    /// The virtual asset service provider of the counterparty
    #[serde(default)]
    pub counterparty_vasp_name: Option<String>,
}

/// A bank account, a card or a payment account of a fiat method: the fields of the deposits
/// and of the withdrawals
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FiatAccount {
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub account_type: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub bank: Option<String>,
    #[serde(default)]
    pub bank_code: Option<String>,
    #[serde(default)]
    pub bic: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub branch_code: Option<String>,
    #[serde(default)]
    pub bank_address: Option<String>,
    #[serde(default)]
    pub bsb: Option<String>,
    #[serde(default)]
    pub beneficiary: Option<Beneficiary>,
    #[serde(default)]
    pub card_number: Option<String>,
    #[serde(default)]
    pub card_number_last: Option<String>,
    #[serde(default)]
    pub card_type: Option<String>,
    #[serde(default)]
    pub cvc: Option<String>,
    #[serde(default)]
    pub expire_month: Option<String>,
    #[serde(default)]
    pub expire_year: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub masked_iban: Option<String>,
    #[serde(default)]
    pub intermediary_address: Option<String>,
    #[serde(default)]
    pub intermediary_bank: Option<String>,
    #[serde(default)]
    pub intermediary_branch: Option<String>,
    #[serde(default)]
    pub intermediary_routing: Option<String>,
    #[serde(default)]
    pub intermediary_swift: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub merchant_reference: Option<String>,
    #[serde(default)]
    pub name_on_account: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub routing: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub swift: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub third_party_email: Option<String>,
    #[serde(default)]
    pub transaction_id: Option<String>,
    #[serde(default)]
    pub transit: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    /// Proves the ownership of the address
    #[serde(default)]
    pub signature: Option<String>,
}

/// An exact amount of an asset
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FundingAmount {
    pub asset: FundingAsset,
    #[serde(deserialize_with = "lenient::decimal")]
    pub amount: Decimal,
}
