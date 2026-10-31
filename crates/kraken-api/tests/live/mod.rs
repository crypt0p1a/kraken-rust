//! Helpers of the live tests: credentials, clients on the real Kraken API, strict decoding
//!
//! The contract checks decode the raw answers of Kraken with the models of the crate and fail when
//! a field sent by Kraken is ignored by a model (it should be mapped, or listed as deliberately
//! ignored), when a field required by a model is missing or null, or when a value can't be read.

#![allow(dead_code)]

use kraken_api::{PrivateApi, PublicApi, RateLimiter, RestOptions};
use kraken_network::{PrivateClient, PublicClient};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

pub use crate::strict::decode_strict;

/// Shared like in an application: Kraken limits the calls per IP and per key
static RATE_LIMITER: LazyLock<Arc<RateLimiter>> = LazyLock::new(Default::default);

/// The variable from the environment, or else from the `.env` file at the root of the repository
pub fn env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .or_else(|| from_dotenv(name))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("{name} is required by the live private tests, see .env.example"))
}

fn from_dotenv(name: &str) -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
    let content = std::fs::read_to_string(path).ok()?;

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == name)
        .map(|(_, value)| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
}

pub fn public_api() -> PublicApi {
    PublicApi::new(RestOptions::default(), RATE_LIMITER.clone()).unwrap()
}

pub fn private_api() -> PrivateApi {
    PrivateApi::new(
        env("KRAKEN_API_KEY"),
        env("KRAKEN_API_SECRET"),
        RestOptions::default(),
        RATE_LIMITER.clone(),
    )
    .unwrap()
}

/// Raw answers of the public endpoints
pub fn public_client() -> PublicClient {
    PublicClient::new(RestOptions::default(), RATE_LIMITER.clone()).unwrap()
}

/// Raw answers of the private endpoints
pub fn private_client() -> PrivateClient {
    PrivateClient::new(
        env("KRAKEN_API_KEY"),
        &env("KRAKEN_API_SECRET"),
        RestOptions::default(),
        RATE_LIMITER.clone(),
    )
    .unwrap()
}

pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Counts how often each key is present in a set of objects: prints which optional fields Kraken
/// actually sends (run with `--nocapture`)
#[derive(Default)]
pub struct Presence {
    total: usize,
    keys: BTreeMap<String, usize>,
    nulls: BTreeMap<String, usize>,
}

impl Presence {
    pub fn record(&mut self, object: &Value) {
        self.total += 1;
        for (key, value) in object.as_object().expect("an object") {
            *self.keys.entry(key.clone()).or_default() += 1;
            if value.is_null() {
                *self.nulls.entry(key.clone()).or_default() += 1;
            }
        }
    }

    pub fn total(&self) -> usize {
        self.total
    }

    /// The keys of `required` must be in every object, and never null
    pub fn assert_always(&self, context: &str, required: &[&str]) {
        for key in required {
            assert_eq!(
                self.keys.get(*key).copied().unwrap_or_default(),
                self.total,
                "{context}: `{key}` is missing from some answers"
            );
            assert_eq!(
                self.nulls.get(*key).copied().unwrap_or_default(),
                0,
                "{context}: `{key}` is null in some answers"
            );
        }
    }

    pub fn print(&self, context: &str) {
        println!("{context}: {} objects", self.total);
        for (key, count) in &self.keys {
            let nulls = self.nulls.get(key).copied().unwrap_or_default();
            println!(
                "  {key:<28} present {count:>5}/{}  null {nulls}",
                self.total
            );
        }
    }
}

/// Every element of a JSON object (an answer keyed by pair, asset or txid)
pub fn entries(value: &Value) -> impl Iterator<Item = (&String, &Value)> {
    value.as_object().expect("an object keyed by id").iter()
}

/// `a <= b`, with a margin for the rounding of the decimals sent as strings
pub fn le(a: f64, b: f64) -> bool {
    a <= b + b.abs() * 1e-9 + 1e-12
}
