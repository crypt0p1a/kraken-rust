# Rust

The crates of the workspace, used directly from Rust, and how to work on them.

## Prerequisites

- [rustup](https://rustup.rs), stable toolchain
- optional: [cargo-nextest](https://nexte.st) (`cargo install cargo-nextest`) for the JUnit
  reports, [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
  (`cargo install cargo-llvm-cov`) for the coverage
- the hook refusing a commit holding a secret ([gitleaks](https://github.com/gitleaks/gitleaks)):

  ```sh
  git config core.hooksPath .githooks
  ```

## Usage

```rust
use kraken_api::{PrivateApi, PublicApi, RateLimiter, RestOptions};
use std::sync::Arc;

// share it: Kraken limits the calls per client
let limiter = Arc::new(RateLimiter::default());

let public_api = PublicApi::new(RestOptions::default(), limiter.clone())?;
let tickers = public_api.tickers(&["XBTUSD".into()]).await?;

let private_api = PrivateApi::new(key, base64_secret, RestOptions::default(), limiter)?;
let balances = private_api.extended_balance().await?;
```

The libraries never read the credentials: an application gives them to `PrivateApi::new` from its
own secure storage.

## Layout of kraken-api

```
src/
├── market/     public    Time (time.rs), Ticker (ticker.rs), AssetPairs (pairs.rs), OHLC (ohlc.rs)
├── account/    private   BalanceEx (balance.rs), TradeVolume (fees.rs)
├── trading/    private   enums (types.rs), Order (order.rs),
│                         OpenOrders / ClosedOrders / QueryOrders (query.rs), AddOrder (add.rs)
├── public_api.rs         PublicApi: facade of market
├── private_api.rs        PrivateApi: facade of account and trading
├── params.rs             parameters shared by the endpoints
└── lenient.rs            numbers sent as strings
```

A new endpoint goes in its domain (models + `pub(crate) async fn`), then in its facade, then in
`kraken-ffi`.

`kraken-crypto`, `kraken-network` and `kraken-api` don't depend on UniFFI: their `uniffi` feature,
enabled by `kraken-ffi` only, derives the UniFFI traits of their models and errors.

## Models

The amounts, prices, volumes and fees are exact decimals (`kraken_api::Decimal`, from
`rust_decimal`: base 10, 28 significant digits), parsed from the strings of Kraken without going
through a float. A `f64` keeps ~16 significant digits: the volume of a small asset
(`812345678901.12345678` SHIB) would lose its last ones, and `volume × price` would no longer be
the cost. Compare them exactly; a value beyond 28 digits is refused rather than rounded. Only the
timestamps stay `f64` (seconds, exact to the microsecond).

Through UniFFI, a `Decimal` crosses as its text and is rebuilt as a `BigDecimal` in Kotlin and a
`Foundation.Decimal` in Swift, see
[`crates/kraken-api/uniffi.toml`](../crates/kraken-api/uniffi.toml); a `String` in the other
languages. The prices and volumes of the new orders are strings, formatted with the decimals of
the pair (`AssetPair.pair_decimals`, `lot_decimals`).

## Tests

```sh
cargo test --workspace                                                    # offline
cargo nextest run --workspace --profile ci                                # + JUnit report
cargo llvm-cov nextest --workspace --open                                 # coverage
```

| Level | Where | What |
|-------|-------|------|
| unit | `#[cfg(test)]` of each module | every value of the enums, the encoding of every parameter, the arrays of an unexpected size, the required fields missing / null / unreadable, lenient numbers, signature, nonces across threads, rate limiter under concurrency |
| network | `kraken-network/tests/clients.rs` | the clients against a mocked server ([wiremock](https://docs.rs/wiremock)): GET / POST, body, `API-Sign` recomputed independently, increasing nonces, error mapping |
| endpoints | `kraken-api/tests/mock_{market,account,trading}.rs` | every call against a mocked Kraken: path, exact parameters, signature, answer decoded |
| fixtures | `kraken-api/tests/fixtures.rs` | the real shapes of the answers (`tests/fixtures`), decoded **strictly** and served end to end |
| facade | `kraken-ffi/tests/facade.rs` | the UniFFI objects, with a foreign logger |
| live contracts | `kraken-api/tests/live_{market,account,trading}.rs` | the real Kraken API, see below |

### Live contracts

They run against the real Kraken API. The credentials are read from the environment, or else from
a `.env` file at the root of the repository, ignored by git:

```sh
cargo test -p kraken-api --test live_market -- --ignored --test-threads=1   # real public API
cp .env.example .env                                                      # then fill the key and secret
cargo test -p kraken-api --test live_account -- --ignored --test-threads=1
cargo test -p kraken-api --test live_trading -- --ignored --test-threads=1
```

The key needs the Query Funds, Query Open Orders & Trades, Query Closed Orders & Trades and
Create & Modify Orders permissions, never Withdraw Funds. The new orders of the live tests are only
validated by Kraken (`validate=true` is forced): no order is ever placed.

### Strict decoding

`tests/strict/mod.rs` decodes an answer with the model through
[serde_ignored](https://docs.rs/serde_ignored): a field sent by Kraken but ignored by a model fails
the test, like a required field missing or null, or a value that can't be read. The live contracts
apply it to every pair (~1 500), every asset, every interval and the first pages of closed orders,
then check the consistency of the values (books not crossed, OHLC bounds, candle spacing, `since`
/ `start` / `end` filters, pagination, fee tiers, cost = executed volume × average price…). With
`--nocapture`, they print which optional fields Kraken actually sends.
