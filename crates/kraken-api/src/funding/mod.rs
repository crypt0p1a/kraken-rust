//! Private funding, the Funding (Beta) API of Kraken: the deposits and the withdrawals, their
//! methods, networks, fees, limits and addresses
//!
//! Called on `/funding/v1/*` by the [`FundingClient`](kraken_network::FundingClient): signed GET,
//! POST, PUT and DELETE, JSON answers. The legacy funding endpoints of `/0/private`, deprecated by
//! Kraken, aren't part of the library.
//!
//! `account_id` targets a wallet of the account (see `wallet_accounts`): the default wallet when
//! missing.

pub(crate) mod assets;

pub use assets::{FundingAssets, FundingDirection, NamedAsset};
