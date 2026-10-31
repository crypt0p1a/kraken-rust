//! Calls of the Kraken Spot REST API, grouped by domain
//!
//! | Module        | Endpoints                                         |
//! |---------------|---------------------------------------------------|
//! | [`market`]    | public: Time, Ticker, AssetPairs, OHLC            |
//! | [`account`]   | private: BalanceEx, TradeVolume                   |
//! | [`trading`]   | private: OpenOrders, ClosedOrders, QueryOrders, AddOrder |
//!
//! Each domain holds its models and its calls, [`PublicApi`] and [`PrivateApi`] being the facades
//! composing them over the clients of `kraken-network`.
//!
//! ```no_run
//! use kraken_api::{PublicApi, RateLimiter, RestOptions};
//! use std::sync::Arc;
//!
//! # async fn run() -> Result<(), kraken_api::KrakenError> {
//! // share it: Kraken limits the calls per client
//! let limiter = Arc::new(RateLimiter::default());
//! let api = PublicApi::new(RestOptions::default(), limiter)?;
//! let tickers = api.tickers(&["XBTUSD".into()]).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("kraken_api");

pub mod account;
mod lenient;
pub mod market;
mod params;
mod private_api;
mod public_api;
pub mod trading;

pub use kraken_network::{KrakenError, Logger, RateLimiter, RestOptions};
pub use private_api::PrivateApi;
pub use public_api::PublicApi;
