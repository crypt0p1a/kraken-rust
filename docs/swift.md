# Swift

[`platforms/apple`](../platforms/apple) is a Swift package, `Kraken`: the Swift bindings of
`kraken-ffi` and `KrakenFFI.xcframework` (iOS, iOS simulator, macOS). On Linux, the C module is
the shared library of the host instead of the XCFramework.

In Swift, the C declarations of the 3 crates are a single module, `KrakenFFI` (the
`ffi_module_name` of their `uniffi.toml`), and the Swift sources a single one, `Kraken`.

## Prerequisites

- macOS: Xcode, and the Rust targets of iOS and macOS (see [Toolchains](toolchains.md)):

  ```sh
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
    aarch64-apple-darwin x86_64-apple-darwin
  ```

- Linux: Swift 5.9 or later

## Build

```sh
# macOS: the libraries of iOS and macOS, the Swift sources, the XCFramework
scripts/build.sh ios macos
scripts/bindings.sh swift xcframework

# Linux: the Swift sources, with the library of the host
scripts/bindings.sh swift
```

## Tests

The tests run against a local mocked Kraken (exact decimals both ways, signed private call,
errors, invalid secret, logger implemented in Swift):

```sh
scripts/test-bindings.sh swift     # macOS through the XCFramework, Linux through the .so
scripts/test-bindings.sh ios       # iOS simulator (macOS, Xcode)
```

## Usage

```swift
import Kraken

// share it: Kraken limits the calls per client (Starter tier: 15 calls, +1 every 3 s)
let limiter = KrakenRateLimiter(poolMaximum: 15, tokenIntervalMs: 3000, logger: nil)

let publicApi = try KrakenPublicApi(options: defaultRestOptions(), rateLimiter: limiter)
let ticker = try await publicApi.tickers(pairs: ["XBTUSD"])["XBTUSD"]

let privateApi = try KrakenPrivateApi(
    apiKey: apiKey, apiSecret: base64Secret, options: defaultRestOptions(), rateLimiter: limiter
)
let balances = try await privateApi.extendedBalance()
```

The amounts are `Foundation.Decimal`s (38 digits, read and written with the POSIX locale), see
[`crates/kraken-api/uniffi.toml`](../crates/kraken-api/uniffi.toml).
