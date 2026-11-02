# Kraken Spot Trading Library 👋
![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?cacheSeconds=2592000)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Kraken Spot REST client in Rust, exported through UniFFI, and a trading bot running on it

Kraken counterpart of the Rust crate of
[kotlin-cex-spot-trading](https://github.com/crypt0p1a/kotlin-cex-spot-trading) (CEX.IO): same
dependencies, same rate limiter, same error variants and the same UniFFI facade, so that both
providers can be merged later.

Every endpoint of the Kraken Spot REST API is covered: market data, transparency, account data,
trading, algorithmic trading (TWAP), Earn, subaccounts and funding (the Funding Beta API, the
legacy funding endpoints being deprecated by Kraken). See [the endpoints](docs/rust.md#endpoints),
and the ones `cex-trading-bot` uses [compared to CEX.IO](docs/kraken-vs-cexio.md).

## Getting started

Pick the guide of the platform or technology to use: each one lists what to install, configure,
build and test.

| I want to… | Guide |
|------------|-------|
| use the crates from Rust, or work on them | [Rust](docs/rust.md) |
| run the trading bot | [Trading bot](docs/trading-bot.md) |
| use the library from Kotlin: JVM, Android, iOS, macOS, Linux, Windows | [Kotlin Multiplatform](docs/kotlin-multiplatform.md) |
| use the library from Swift: iOS, macOS, Linux | [Swift](docs/swift.md) |
| build the native libraries (`.so`, `.a`, `.dylib`, `.dll`) | [Native libraries](docs/native-libraries.md) |
| install the toolchains: Android NDK, Xcode, cross compilers | [Toolchains](docs/toolchains.md) |
| know what the CI checks | [CI](docs/ci.md) |

All the guides: [`docs/`](docs/README.md).

### Quick start

```sh
cargo test --workspace                                     # Rust, offline
./gradlew :kraken-multiplatform:jvmTest                    # Kotlin, on the JVM
cp managers.example.yaml managers.yaml && cp .env.example .env   # the bot: edit them, then
docker compose up -d --build
```

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
| `kraken-trading-bot` | the managers of [the trading bot](docs/trading-bot.md), port of `cex-trading-bot`     |
| `trading-notify` | the [notifications](docs/trading-bot.md#notifications) of the bot: languages, texts, sending through ntfy |
| `uniffi-bindgen` | `uniffi-bindgen`, `uniffi-bindgen-swift`: the [bindings](docs/native-libraries.md#bindings) of `kraken-ffi` |

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

The same from [Kotlin](docs/kotlin-multiplatform.md#usage) and [Swift](docs/swift.md#usage). The
amounts are exact decimals everywhere: `Decimal` in Rust, `BigDecimal` in Kotlin,
`Foundation.Decimal` in Swift ([why](docs/rust.md#models)).

## Author

👤 **codlab**

* Twitter: [@codlab](https://twitter.com/codlab)
* Github: [@crypt0p1a](https://github.com/crypt0p1a)

## 📝 License

Copyright © 2026 [crypt0p1a](https://github.com/crypt0p1a).

This project is [MIT](LICENSE) licensed.
