# Kraken Spot Trading Library 👋
![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Kraken Spot REST client in Rust, exported through UniFFI, and a trading bot running on it

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
├── kraken-ffi       UniFFI export: objects with async methods            ← kraken-api, kraken-network
├── kraken-trading-bot  the trading bot (executable, Docker)             ← kraken-api, trading-notify
├── trading-notify   notifications of a trading bot through ntfy, i18n   ← ntfy
└── uniffi-bindgen   the generator of the Kotlin and Swift bindings      ← uniffi (cli)
```

| Crate            | Content                                                                                   |
|------------------|-------------------------------------------------------------------------------------------|
| `kraken-crypto`  | `ApiSecret`: base64 secret, never printed, `sign(path, nonce, body)`                      |
| `kraken-network` | `PublicClient` (`GET /0/public`), `PrivateClient` (signed `POST /0/private`), `RateLimiter`, `RestOptions`, `KrakenError`, form encoding helpers |
| `kraken-api`     | `market`, `account`, `trading` (models + calls), `PublicApi` and `PrivateApi` facades     |
| `kraken-ffi`     | `KrakenPublicApi`, `KrakenPrivateApi`, `KrakenRateLimiter`, `KrakenLogger`                |
| `kraken-trading-bot` | the managers of [the trading bot](#trading-bot), port of `cex-trading-bot`            |
| `trading-notify` | the [notifications](#notifications) of the bot: languages, texts, sending through ntfy |
| `uniffi-bindgen` | `uniffi-bindgen`, `uniffi-bindgen-swift`: the [bindings](#bindings-android-ios-macos-jvm) of `kraken-ffi` |

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

## Trading bot

`kraken-trading-bot` is the port of `cex-trading-bot` on Kraken. Each **manager** trades its own
budget on one pair, with **limit orders only**, in a loop:

1. buys with everything it has at **-`buy_below`%** of the last price (GTD order: not filled
   after `buy_order_timeout`, it expires and is placed again from the new price)
2. sells everything it bought for **what it cost + `sell_above`%**, fees included (GTC order)
3. starts again with what the sale brought: every cycle ends with more than it started with

Its orders carry its id as Kraken **`userref`**: Kraken knows which order belongs to which
manager, whatever happens to the database. The fees are taken in the quote currency (`fciq`). A
manager only spends its own money, and only what the account has.

### Configuration

The managers are in a YAML file ([example](managers.example.yaml)),
the credentials in the environment:

```yaml
dry_run: true              # orders only validated by Kraken, never placed: false to trade
tick_interval: 5m
buy_order_timeout: 12h
managers:
  - id: 1                  # userref of its orders: 1 to 2147483647, unique, never reused
    pair: XBTUSD           # or XBT/USD, XXBTZUSD
    budget: 30             # in the quote currency (USD)
    buy_below: 1.0         # buys at -1% of the last price
    sell_above: 1.5        # sells for its cost + 1.5%, fees included
  - id: 2
    pair: ETHUSD
    budget: 30
    buy_below: 0.8
    sell_above: 1.2
    mode: closing          # active (default) | closing: sells what it holds, then stops
```

- To stop a manager, set `mode: closing`, and remove it once it holds nothing: the open orders of
  a userref no manager has are reported in the logs and left untouched.
- Changing the budget or the percentages of a manager applies from its next order.
- The configuration is checked at start: unique ids, known pairs, valid percentages.

| Variable | Default | |
|----------|---------|-|
| `KRAKEN_API_KEY`, `KRAKEN_API_SECRET` | | Query Funds, Query Open/Closed Orders, Create & Modify Orders; never Withdraw Funds |
| `KRAKEN_BOT_CONFIG` | `managers.yaml` | the YAML file |
| `KRAKEN_BOT_DATABASE` | `kraken-bot.sqlite` | the SQLite database |
| `KRAKEN_BOT_NTFY_TOPIC`, `KRAKEN_BOT_NTFY_TOKEN` | | the [notifications](#notifications) |
| `RUST_LOG` | `info` | |

### Running it

```sh
cp managers.example.yaml managers.yaml   # then edit it
cp .env.example .env                                               # then fill the key and secret
docker compose up -d --build
docker compose logs -f
```

The image holds only the binary (distroless, non-root, ~70 MB): `managers.yaml` is mounted
read-only in `/config`, the database lives in the `kraken-bot-data` volume (`/data`). `docker
compose stop` lets the tick in progress end.

Without Docker: `cargo run --release -p kraken-trading-bot` (or `-- --once` for a single tick,
`-- --check` to only check the configuration and the pairs).

### Notifications

The bot notifies the phone through [ntfy](https://ntfy.sh) (the `trading-notify` crate, on the
[`ntfy`](https://crates.io/crates/ntfy) crate):

| Event | Notification |
|-------|--------------|
| `started` | the bot started, dry run or live |
| `placed` | a buy or a sell placed: volume, price, total, expiry |
| `filled` | a buy or a sell filled, entirely or partially; the gain of each sale, fees included |
| `expired` | an order expired or canceled, partially filled or not |
| `failed` | an order refused, a manager or a tick failing: once, until it works again |
| `daily_summary` | every day after `daily_summary_hour` (UTC): the value of each manager, its gain, its sales |

1. install the ntfy app (Android, iOS) and subscribe to a long random topic
   (`openssl rand -hex 16`): whoever knows it reads the notifications
2. put it in `.env`: `KRAKEN_BOT_NTFY_TOPIC=...` (and `KRAKEN_BOT_NTFY_TOKEN` for a protected
   topic)
3. add to `managers.yaml`:

```yaml
notifications:
  ntfy:
    server: https://ntfy.sh   # or a self-hosted server
  language: en                # en (default) or fr
  events: [started, placed, filled, expired, failed, daily_summary]   # all by default
  dry_run: false              # true: the orders of a dry run too (one per tick and manager)
  daily_summary_hour: 20      # UTC
```

The fills are found when the orders are synchronized: each change of an order is notified once,
restarts included, and the history found at the first start isn't. A notification which can't be
sent is logged, the bot goes on. Adding a language: a catalog in `trading-notify/src/i18n.rs`.

### Database

The SQLite database (`/data/kraken-bot.sqlite`) is there to work with what the bot did:

| Table | Content |
|-------|---------|
| `orders` | the orders of the managers, synchronized from Kraken (status, volumes, cost, fees, prices, times) |
| `managers` | the current state of each manager: configuration, balances, entry price, value |
| `manager_snapshots` | the state of each manager at each tick |
| `actions` | the decisions: orders placed or validated (dry run), idle reasons, errors |
| `tickers` | the prices of the pairs of the managers, at each tick |
| `sync_state` | where the synchronization of the orders is |

The amounts are stored as text, to stay exact.

```sh
docker run --rm -v kraken-rust_kraken-bot-data:/data -it keinos/sqlite3 sqlite3 /data/kraken-bot.sqlite \
  "SELECT id, pair, budget, value, quote_available, base_available FROM managers"
```

### Tests of the bot

- the decisions (planner, ledger, strategy) with the values of the tests of `cex-trading-bot`,
  including 500 random cycles which must all make money
- the database, the configuration, the synchronization
- `scenarios.rs`: the bot tick after tick on a simulated Kraken (fills, expiry, partial fills,
  closing managers, missing funds, orphans, restart without database, refused orders, dry run,
  600 ticks of a random market where every cycle makes money, only limit orders)
- `kraken_wiring.rs`: the requests sent to a mocked Kraken
- `live_dry_run.rs`: one tick on the real Kraken, as a dry run

## Platforms

`scripts/build.sh` builds the libraries of `kraken-ffi` (cdylib and staticlib) in release and
gathers them in `dist/<target>/`:

| Platform | Targets | Libraries |
|----------|---------|-----------|
| `android` | `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android` | `.so`, `.a` |
| `ios` | `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` | `.a`, `.dylib` |
| `macos` | `aarch64-apple-darwin`, `x86_64-apple-darwin` | `.dylib`, `.a` |
| `linux` | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `.so`, `.a` |
| `windows` | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` (`x86_64-pc-windows-gnu` with mingw-w64) | `.dll`, `.lib` |

```sh
scripts/build.sh android ios macos      # platforms
scripts/build.sh x86_64-pc-windows-gnu  # or targets
PROFILE=debug scripts/build.sh linux    # release by default
```

- **Android**: [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) (`cargo install cargo-ndk`) and
  an NDK (`ANDROID_NDK_HOME`, else the latest of `$ANDROID_HOME/ndk`); API level 24 by default
  (`ANDROID_API`)
- **iOS / macOS**: macOS with Xcode
- **Linux / Windows from macOS**: the C cross compilers used by `ring`, declared in
  `~/.cargo/config.toml`:

  ```sh
  brew tap messense/macos-cross-toolchains
  brew install aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu mingw-w64
  ```

  ```toml
  [target.aarch64-unknown-linux-gnu]
  linker = "aarch64-unknown-linux-gnu-gcc"
  [target.x86_64-unknown-linux-gnu]
  linker = "x86_64-unknown-linux-gnu-gcc"
  [target.x86_64-pc-windows-gnu]
  linker = "x86_64-w64-mingw32-gcc"

  [env]
  CC_aarch64_unknown_linux_gnu = "aarch64-unknown-linux-gnu-gcc"
  AR_aarch64_unknown_linux_gnu = "aarch64-unknown-linux-gnu-ar"
  CC_x86_64_unknown_linux_gnu = "x86_64-unknown-linux-gnu-gcc"
  AR_x86_64_unknown_linux_gnu = "x86_64-unknown-linux-gnu-ar"
  CC_x86_64_pc_windows_gnu = "x86_64-w64-mingw32-gcc"
  AR_x86_64_pc_windows_gnu = "x86_64-w64-mingw32-ar"
  ```

- **Windows (MSVC)**: on Windows, with the Visual Studio build tools (and clang for `aarch64`)

The CI builds every platform on its native runner and publishes the libraries as artifacts.

### Bindings: Android, iOS, macOS, JVM

The bindings are generated by the `uniffi-bindgen` crate of the workspace (same uniffi version as
the libraries), from the library of the host: they are the same for every platform.

| Directory | What | Built with |
|-----------|------|------------|
| [`platforms/android`](platforms/android) | Android library (AAR): Kotlin bindings, `libkraken_ffi.so` of the 4 ABIs, JNA | Gradle, Android SDK |
| [`platforms/apple`](platforms/apple) | Swift package: Swift bindings, `KrakenFFI.xcframework` (iOS, iOS simulator, macOS) | SwiftPM / Xcode |
| [`platforms/kotlin`](platforms/kotlin) | the tests of the Kotlin bindings on the JVM, shared with Android | Gradle |
| [`platforms/kotlin-multiplatform`](platforms/kotlin-multiplatform) | Kotlin Multiplatform library: JVM, Android, iOS, macOS, Linux | Gradle ≥ 9.6.1, [uniffi KMP plugin](https://github.com/UbiqueInnovation/uniffi-kotlin-multiplatform-bindings) |

```sh
scripts/bindings.sh kotlin              # dist/bindings/kotlin
scripts/bindings.sh swift               # platforms/apple/Sources, dist/bindings/swift/include
scripts/bindings.sh xcframework         # platforms/apple/KrakenFFI.xcframework (after build.sh ios macos)

scripts/build.sh android && scripts/bindings.sh kotlin
gradle -p platforms/android assembleRelease   # build/outputs/aar/kraken-android-release.aar
```

#### Kotlin Multiplatform

[`platforms/kotlin-multiplatform`](platforms/kotlin-multiplatform) is built with the Gradle plugin
`ch.ubique.uniffi.plugin` of
[UbiqueInnovation/uniffi-kotlin-multiplatform-bindings](https://github.com/UbiqueInnovation/uniffi-kotlin-multiplatform-bindings)
(uniffi 0.32.0, Gradle ≥ 9.6.1, Kotlin 2.4, AGP 9): it builds `kraken-ffi` with Cargo for every
target and generates the bindings of the 3 crates (`generateBindingsForExternalCrates`), in the
same packages as the Kotlin ones (`uniffi.kraken_api`, ...). The `Decimal`s are the `BigDecimal` of
[`com.ionspin.kotlin:bignum`](https://github.com/ionspin/kotlin-multiplatform-bignum), available on
every target (top-level `[custom_types.Decimal]` of `crates/kraken-api/uniffi.toml`, read by this
generator only).

```sh
cd platforms/kotlin-multiplatform
gradle jvmTest linuxX64Test             # Linux
gradle iosSimulatorArm64Test macosArm64Test   # macOS
gradle assemble                         # the libraries of every target, the AAR (Android SDK, NDK)
```

The common tests run on every target (options, exact decimals through the converter, invalid
secret, transport error, logger implemented in Kotlin); the calls to a mocked Kraken run on the JVM.

In Swift, the C declarations of the 3 crates are a single module, `KrakenFFI` (the
`ffi_module_name` of their `uniffi.toml`), and the Swift sources a single one, `Kraken`.

The bindings are tested on every platform against a local mocked Kraken (exact decimals both
ways, signed private call, errors, invalid secret, logger implemented in the foreign language):

```sh
scripts/test-bindings.sh kotlin    # JVM, library of the host
scripts/test-bindings.sh swift     # macOS through the XCFramework, Linux through the .so
scripts/test-bindings.sh ios       # iOS simulator (macOS, Xcode)
scripts/test-bindings.sh android   # Android device or emulator (adb, Android SDK, cargo-ndk)
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
- **Amounts**: exact `Decimal`s here, `f64` in `cex-spot-trading`: to align on the decimals when
  merging.
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
cargo nextest run --workspace --profile ci                                # + JUnit report
cargo llvm-cov nextest --workspace --open                                 # coverage
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

The amounts, prices, volumes and fees are exact decimals (`kraken_api::Decimal`, from
`rust_decimal`: base 10, 28 significant digits), parsed from the strings of Kraken without going
through a float. A `f64` keeps ~16 significant digits: the volume of a small asset
(`812345678901.12345678` SHIB) would lose its last ones, and `volume × price` would no longer be
the cost. Compare them exactly; a value beyond 28 digits is refused rather than rounded. Only the
timestamps stay `f64` (seconds, exact to the microsecond).

Through UniFFI, a `Decimal` crosses as its text and is rebuilt as a `java.math.BigDecimal` in
Kotlin and a `Foundation.Decimal` in Swift (38 digits, read and written with the POSIX locale),
see [`crates/kraken-api/uniffi.toml`](crates/kraken-api/uniffi.toml); a `String` in the other
languages. The prices and volumes of the new orders are strings, formatted with the decimals of
the pair (`AssetPair.pair_decimals`, `lot_decimals`).

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml), on every push and pull request:

- **secrets**: [gitleaks](https://github.com/gitleaks/gitleaks) on the whole history and on the
  working tree, with its default rules plus the entropy rules of [`.gitleaks.toml`](.gitleaks.toml)
  (Kraken private keys and API keys, long high-entropy base64 strings),
  [trufflehog](https://github.com/trufflesecurity/trufflehog) on the history, and no `.env`
  committed
- **lint**: `cargo fmt --check`, clippy with the warnings as errors
- **tests**: the offline suite with [cargo-nextest](https://nexte.st) on Linux x86_64 and
  aarch64, macOS and Windows, each crate alone without the `uniffi` feature
- **test report**: the JUnit reports of every platform (`.config/nextest.toml`, profile `ci`)
  published in a *Test results* check
- **coverage**: [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), summary in the job,
  HTML and lcov reports as artifacts, at least 95% of the lines; sent to Codecov when the
  `CODECOV_TOKEN` secret is set
- **build**: the libraries of every platform (see [Platforms](#platforms)), as artifacts
- **bindings**: generated and tested on the JVM, on Linux (Swift), on macOS and on the iOS
  simulator (XCFramework), and on an Android emulator (AAR); the AAR, the Swift package and the
  Kotlin sources as artifacts
- **Kotlin Multiplatform**: tested on the JVM, Linux, macOS and the iOS simulator; the libraries of
  every target, Android included, as artifacts
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
