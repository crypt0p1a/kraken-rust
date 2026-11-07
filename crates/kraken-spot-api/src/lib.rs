//! Calls of the Kraken Spot REST API, grouped by domain
//!
//! | Module           | Endpoints                                         |
//! |------------------|---------------------------------------------------|
//! | [`funding`]      | private, Funding (Beta): /funding/v1/assets, /funding/v1/methods, /funding/v1/networks, /funding/v1/fees, /funding/v1/limits, /funding/v1/deposit/address, /funding/v2/deposit/addresses, /funding/v1/deposits, /funding/v1/addresses (GET, POST, PUT, DELETE), /funding/v1/withdrawals (GET, POST) |
//! | [`market`]       | public: Time, SystemStatus, MaintenanceSchedule, Assets, AssetPairs, Ticker, OHLC, Depth, GroupedBook, Trades, Spread; private: Level3 |
//! | [`transparency`] | public: PreTrade, PostTrade                       |
//! | [`account`]      | private: Balance, BalanceEx, CreditLines, TradeBalance, TradeVolume, Ledgers, QueryLedgers, AddExport, ExportStatus, RetrieveExport, RemoveExport, GetApiKeyInfo, ListWalletAccounts |
//! | [`algo`]         | private: AddStrategy, CancelStrategy, GetOpenStrategies, GetClosedStrategies, QueryStrategies |
//! | [`earn`]         | private: Earn/Strategies, Earn/Allocations, Earn/Allocate, Earn/AllocateStatus, Earn/Deallocate, Earn/DeallocateStatus |
//! | [`subaccounts`]  | private: CreateSubaccount, AccountTransfer        |
//! | [`trading`]      | private: OpenOrders, ClosedOrders, QueryOrders, OrderAmends, TradesHistory, QueryTrades, OpenPositions, AddOrder, AddOrderBatch, AmendOrder, EditOrder, CancelOrder, CancelAll, CancelAllOrdersAfter, CancelOrderBatch, GetWebSocketsToken |
//!
//! Each domain holds its models and its calls, [`PublicApi`] and [`PrivateApi`] being the facades
//! composing them over the clients of `kraken-network`.
//!
//! The amounts and prices are exact [`Decimal`]s, parsed from the strings of Kraken: a float
//! loses the last digits of the large volumes of the small assets (`123456789.12345678`). Through
//! UniFFI, they are strings, to read as `BigDecimal` / `Decimal` by the foreign languages.
//!
//! ```no_run
//! use kraken_spot_api::{PublicApi, RateLimiter, RestOptions};
//! use std::sync::Arc;
//!
//! # async fn run() -> Result<(), kraken_spot_api::KrakenError> {
//! // share it: Kraken limits the calls per client
//! let limiter = Arc::new(RateLimiter::default());
//! let api = PublicApi::new(RestOptions::default(), limiter)?;
//! let tickers = api.tickers(&["XBTUSD".into()]).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("kraken_spot_api");

// the exact text of the decimal: "0.00058823", never a float
#[cfg(feature = "uniffi")]
uniffi::custom_type!(Decimal, String, {
    remote,
    lower: |decimal| decimal.to_string(),
    try_lift: |text| Ok(lenient::parse_decimal(&text).map_err(uniffi::deps::anyhow::Error::msg)?),
});

pub mod account;
pub mod algo;
mod cursor;
pub mod earn;
pub mod funding;
mod keyed;
mod lenient;
pub mod market;
mod params;
mod private_api;
mod public_api;
pub mod subaccounts;
pub mod trading;
pub mod transparency;

pub use cursor::Cursor;
pub use kraken_network::{KrakenError, Logger, RateLimiter, RestOptions};
pub use private_api::PrivateApi;
pub use public_api::PublicApi;
pub use rust_decimal::Decimal;
