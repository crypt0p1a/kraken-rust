# Toolchains

What to install to build `kraken-ffi` for each target. The [native libraries](native-libraries.md)
and the [Kotlin Multiplatform library](kotlin-multiplatform.md) both rely on it.

## Rust targets

```sh
rustup target add <target>...
```

| Platform | Targets | Host |
|----------|---------|------|
| Android | `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` (`i686-linux-android`: `scripts/build.sh` only) | any, with the NDK |
| iOS | `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` | macOS |
| macOS | `aarch64-apple-darwin`, `x86_64-apple-darwin` | macOS |
| Linux | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | Linux, or macOS with the cross compilers |
| Windows | `x86_64-pc-windows-gnu` | Windows, or macOS / Linux with mingw-w64 |
| Windows (MSVC) | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | Windows |

## Android

- the Android SDK with an NDK (r28 or later, for the 16 KB pages): Android Studio → SDK Manager →
  *NDK (Side by side)*, or `sdkmanager "ndk;28.2.13676358"`
- `ANDROID_HOME` (or `sdk.dir` in `local.properties` at the root, for Gradle)
- for `scripts/build.sh`: [cargo-ndk](https://github.com/bbqsrc/cargo-ndk)
  (`cargo install cargo-ndk`), and the NDK in `ANDROID_NDK_HOME`, else the latest of
  `$ANDROID_HOME/ndk`; API level 24 by default (`ANDROID_API`). The Kotlin Multiplatform library
  doesn't need cargo-ndk: its Gradle plugin drives the NDK itself (`ANDROID_NDK_ROOT` on the CI)

## iOS and macOS

macOS with Xcode (and its command line tools: `xcode-select --install`). The iOS simulator runs the
tests of the iOS targets.

## Linux and Windows from macOS

`ring`, under the TLS of `reqwest`, compiles C: each target needs its C cross compiler, and Linux
its linker.

```sh
brew tap messense/macos-cross-toolchains
brew trust --tap messense/macos-cross-toolchains   # Homebrew 7 and later
brew install aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu mingw-w64
```

Then, in `~/.cargo/config.toml`:

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

The formulas install each compiler under two names (`x86_64-unknown-linux-gnu-gcc` and
`x86_64-linux-gnu-gcc`): either works. The same can be given through the environment
(`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=...`), as the CI does. It is kept out of the
repository: a `.cargo/config.toml` there would also apply to the native builds of Linux and to the
Docker image.

## Windows (MSVC)

On Windows, with the Visual Studio build tools (and clang for `aarch64`).
