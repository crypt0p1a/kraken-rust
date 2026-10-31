# Kraken Spot Trading Library 👋
![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Kraken Spot REST client in Rust, exported through UniFFI

Kraken counterpart of the Rust crate of
[kotlin-cex-spot-trading](https://github.com/crypt0p1a/kotlin-cex-spot-trading) (CEX.IO): same
dependencies, same rate limiter, same error variants and the same UniFFI facade, so that both
providers can be merged later.

The endpoints are, for now, the ones `cex-trading-bot` needs from an exchange: tickers, pairs
info, candles, fees, balances, orders and new orders.

## Crates

```
crates/
├── kraken-crypto    API secret, API-Sign signature (HMAC-SHA512)         pure, synchronous
├── kraken-network   HTTP clients, rate limiter, nonce, answers, errors   ← kraken-crypto
├── kraken-api       the endpoints, grouped by domain, and their models   ← kraken-network
└── kraken-ffi       UniFFI export: objects with async methods            ← kraken-api, kraken-network
```

| Crate            | Content                                                                                   |
|------------------|-------------------------------------------------------------------------------------------|
| `kraken-crypto`  | `ApiSecret`: base64 secret, never printed, `sign(path, nonce, body)`                      |
| `kraken-network` | `PublicClient` (`GET /0/public`), `PrivateClient` (signed `POST /0/private`), `RateLimiter`, `RestOptions`, `KrakenError`, form encoding helpers |
| `kraken-api`     | `market`, `account`, `trading` (models + calls), `PublicApi` and `PrivateApi` facades     |
| `kraken-ffi`     | `KrakenPublicApi`, `KrakenPrivateApi`, `KrakenRateLimiter`, `KrakenLogger`                |

`kraken-crypto`, `kraken-network` and `kraken-api` don't depend on UniFFI: their `uniffi` feature,
enabled by `kraken-ffi` only, derives the UniFFI traits of their models and errors.

### kraken-api

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

## Endpoints, compared to CEX.IO

| `cex-trading-bot` (CEX.IO) | Kraken         | `PublicApi` / `PrivateApi`            |
|----------------------------|----------------|---------------------------------------|
| `tickers`                  | `Ticker`       | `tickers`                             |
| `pairsInfo`                | `AssetPairs`   | `asset_pairs`                         |
| `candles`                  | `OHLC`         | `candles`                             |
| `getMyCurrentFee`          | `TradeVolume`  | `trade_volume`                        |
| `getMyAccountStatus`       | `BalanceEx`    | `extended_balance`                    |
| `orders` (open)            | `OpenOrders`   | `open_orders`, `open_orders_for_pair` |
| `orders` (archived)        | `ClosedOrders` | `closed_orders`                       |
| `orders` (by id)           | `QueryOrders`  | `query_orders`, `order`               |
| `newOrder`                 | `AddOrder`     | `new_order`                           |
| `serverTime` (tests)       | `Time`         | `server_time`                         |

### Differences to handle when merging the providers

- **Pairs**: Kraken takes an alternate name (`XBTUSD`) and answers with its own name (`XXBTZUSD`),
  CEX.IO uses `BTC-USD` everywhere. `AssetPair.altname` / `wsname` give the mapping; BTC is `XBT`.
- **Accounts**: no Spot sub-accounts on Kraken, the balances are the ones of the API key.
  `ExtendedBalance::available()` is the equivalent of `balance - balanceOnHold`.
- **Client order ids**: `cl_ord_id` is a UUID or at most 18 ASCII characters (CEX.IO accepts
  longer `clientOrderId`s). `userref` is a numeric alternative.
- **Orders**: the answers are keyed by txid, returned here as lists with `Order.txid` filled.
  Closed orders are paginated by 50 (`offset`) and filtered by seconds, not milliseconds.
- **Units**: Kraken timestamps are in seconds (with decimals for the orders), CEX.IO in
  milliseconds. Fees are percents (0.26 = 0.26%) on both.
- **Candles**: `since` instead of from/to, 720 candles at most, the last one is still open
  (`is_closed = false`).
- **Ticker**: values of today and of the last 24h, no `volumeUSD` / `priceChangePercentage`
  (computable from `open` and `last_trade.price`).
- **Rate limit**: the default `RateLimiter` follows the Starter tier (15 calls, +1 every 3s).
  The order calls have their own, separate limit on Kraken.
- **Nonce**: strictly increasing per API key, created inside the rate limiter queue so that the
  calls leave in order. Don't share an API key between processes.

## Tests

| Level | Where | What |
|-------|-------|------|
| unit | `#[cfg(test)]` of each module | every value of the enums, the encoding of every parameter, the arrays of an unexpected size, the required fields missing / null / unreadable, lenient numbers, signature, nonces across threads, rate limiter under concurrency |
| network | `kraken-network/tests/clients.rs` | the clients against a mocked server ([wiremock](https://docs.rs/wiremock)): GET / POST, body, `API-Sign` recomputed independently, increasing nonces, error mapping |
| endpoints | `kraken-api/tests/mock_{market,account,trading}.rs` | every call against a mocked Kraken: path, exact parameters, signature, answer decoded |
| fixtures | `kraken-api/tests/fixtures.rs` | the real shapes of the answers (`tests/fixtures`), decoded **strictly** and served end to end |
| facade | `kraken-ffi/tests/facade.rs` | the UniFFI objects, with a foreign logger |
| live contracts | `kraken-api/tests/live_{market,account,trading}.rs` | the real Kraken API, see below |

### Strict decoding

`tests/strict/mod.rs` decodes an answer with the model through
[serde_ignored](https://docs.rs/serde_ignored): a field sent by Kraken but ignored by a model fails
the test, like a required field missing or null, or a value that can't be read. The live contracts
apply it to every pair (~1 500), every asset, every interval and the first pages of closed orders,
then check the consistency of the values (books not crossed, OHLC bounds, candle spacing, `since`
/ `start` / `end` filters, pagination, fee tiers, cost = executed volume × average price…). With
`--nocapture`, they print which optional fields Kraken actually sends.

The new orders of the live tests are only validated by Kraken (`validate=true` is forced): no order
is ever placed.

```sh
cargo test --workspace                                                    # offline
cargo test -p kraken-api --test live_market -- --ignored --test-threads=1   # real public API
cp .env.example .env                                                      # then fill the key and secret
cargo test -p kraken-api --test live_account -- --ignored --test-threads=1
cargo test -p kraken-api --test live_trading -- --ignored --test-threads=1
```

The credentials are read from the environment, or else from a `.env` file at the root of the
repository, ignored by git: copy [`.env.example`](.env.example). The key needs the Query Funds,
Query Open Orders & Trades, Query Closed Orders & Trades and Create & Modify Orders permissions,
never Withdraw Funds. The libraries never read the credentials: an application gives them to
`PrivateApi::new` from its own secure storage.

### Models

The amounts are `f64`, like in `cex-spot-trading`: compare them with a tolerance, and give the
prices and volumes of the new orders as strings, formatted with the decimals of the pair
(`AssetPair.pair_decimals`, `lot_decimals`).

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml), on every push and pull request:

- **secrets**: [gitleaks](https://github.com/gitleaks/gitleaks) on the whole history and on the
  working tree, with its default rules plus the entropy rules of [`.gitleaks.toml`](.gitleaks.toml)
  (Kraken private keys and API keys, long high-entropy base64 strings),
  [trufflehog](https://github.com/trufflesecurity/trufflehog) on the history, and no `.env`
  committed
- **lint**: `cargo fmt --check`, clippy with the warnings as errors
- **tests**: the offline suite on Linux and macOS, each crate alone without the `uniffi` feature
- **audit**: `cargo audit` of the dependencies

Daily and on demand, the live contracts run against the real Kraken API, the private ones with the
`KRAKEN_API_KEY` / `KRAKEN_API_SECRET` secrets of the repository (Settings → Secrets and variables
→ Actions).

Locally, `.githooks/pre-commit` refuses a commit holding a secret:

```sh
git config core.hooksPath .githooks
```

## Author

👤 **codlab**

* Twitter: [@codlab](https://twitter.com/codlab)
* Github: [@crypt0p1a](https://github.com/crypt0p1a)

## 📝 License

Copyright © 2026 [crypt0p1a](https://github.com/crypt0p1a).

This project is [MIT](LICENSE) licensed.
