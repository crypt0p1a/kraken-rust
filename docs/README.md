# Documentation

Each guide holds what to install, configure, build and test for one platform or technology.

| I want to… | Guide |
|------------|-------|
| use the crates from Rust, or work on them (build, tests, live tests, credentials) | [Rust](rust.md) |
| run the trading bot (Docker or cargo), its notifications, its database | [Trading bot](trading-bot.md) |
| use the library from Kotlin: JVM, Android, iOS, macOS, Linux, Windows | [Kotlin Multiplatform](kotlin-multiplatform.md) |
| use the library from Swift: iOS, macOS, Linux | [Swift](swift.md) |
| build the native libraries of `kraken-ffi` (`.so`, `.a`, `.dylib`, `.dll`) | [Native libraries](native-libraries.md) |
| install the toolchains of the targets: Android NDK, Xcode, cross compilers of Linux and Windows | [Toolchains](toolchains.md) |
| know what the CI checks, and its secrets | [CI](ci.md) |
| merge this client with the one of CEX.IO | [Kraken compared to CEX.IO](kraken-vs-cexio.md) |

Every guide starts from a clone of the repository, with [rustup](https://rustup.rs) installed: the
toolchain is the stable one.
