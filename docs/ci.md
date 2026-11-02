# CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml), on every push and pull request:

- **secrets**: [gitleaks](https://github.com/gitleaks/gitleaks) on the whole history and on the
  working tree, with its default rules plus the entropy rules of [`.gitleaks.toml`](../.gitleaks.toml)
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
- **build**: the [native libraries](native-libraries.md) of every platform, as artifacts
- **Kotlin Multiplatform**: the library of every target built on macOS (the JVM jar and the AAR as
  artifacts), tested on the JVM (macOS and Linux), macOS and the iOS simulator; the tests of
  Linux on Linux. The common tests run on the JVM target, not on an Android emulator; the generator of
  the Kotlin bindings (`build/uniffi/bindgen`) is cached, rebuilt only with a new plugin version
- **Swift**: tested on Linux, on macOS and on the iOS simulator (XCFramework); the Swift package as
  an artifact
- **audit**: `cargo audit` of the dependencies
- **docker**: the image of the bot on amd64 and arm64; the cache mounts of the Dockerfile (cargo
  registry and target) and its layers are kept in the cache of GitHub between the runs

The runs of a pull request are cancelled by a new push to it; the runs of `main` always go to the
end, to save the caches the pull requests start from.

Daily and on demand, the live contracts run against the real Kraken API, the private ones with the
`KRAKEN_API_KEY` / `KRAKEN_API_SECRET` secrets of the repository (Settings → Secrets and variables
→ Actions): `live_market` alone, then `live_account`, `live_trading`, `live_algo`, `live_earn` and
`live_funding`, which never change the account (see [Rust](rust.md#live-contracts)).
