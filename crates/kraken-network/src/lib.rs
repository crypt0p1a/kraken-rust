//! HTTP layer of the Kraken REST API: knows how to call an endpoint, not what it means
//!
//! - [`PublicClient`]: `GET /0/public/{method}?{params}`
//! - [`PrivateClient`]: `POST /0/private/{method}`, form encoded body starting with an increasing
//!   nonce, signed by `kraken-crypto`
//! - [`FundingClient`]: the funding API, `/funding/*`: signed GET, POST, PUT and DELETE, the
//!   nonce in a header, JSON bodies and answers
//! - [`RateLimiter`]: token bucket shared by the clients, which also sends the calls in order
//! - [`KrakenError`]: every failure of a call, the `{"error": [...]}` answers included

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("kraken_network");

mod client;
mod envelope;
mod error;
mod nonce;
mod options;
pub mod params;
mod rate_limit;

pub use client::{FundingClient, FundingMethod, PrivateClient, PublicClient};
pub use error::KrakenError;
pub use options::RestOptions;
pub use rate_limit::{Logger, RateLimiter};
