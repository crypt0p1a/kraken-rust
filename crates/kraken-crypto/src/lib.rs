//! Cryptography of the Kraken private REST calls
//!
//! Pure and synchronous: no network, no clock. The nonce is given by the caller.
//!
//! ```
//! use kraken_crypto::ApiSecret;
//!
//! let secret = ApiSecret::from_base64("c2VjcmV0").unwrap();
//! let signature = secret.sign("/0/private/BalanceEx", 1, "nonce=1");
//! assert_eq!(signature.len(), 88); // base64 of a SHA-512
//! ```
//!
//! <https://docs.kraken.com/api/docs/guides/spot-rest-auth>

mod secret;
mod signature;

pub use secret::{ApiSecret, SecretError};
