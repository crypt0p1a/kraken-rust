# Kotlin Multiplatform

[`platforms/kotlin-multiplatform`](../platforms/kotlin-multiplatform) is the Kotlin library of
`kraken-ffi`, a single one for every platform: JVM, Android, iOS, macOS, Linux and Windows. It is
built on macOS, the only host of the Apple targets, the Linux and Windows libraries being
cross-compiled.

It uses the Gradle plugin `ch.ubique.uniffi.plugin` of
[UbiqueInnovation/uniffi-kotlin-multiplatform-bindings](https://github.com/UbiqueInnovation/uniffi-kotlin-multiplatform-bindings)
(uniffi 0.32.0, Gradle ≥ 9.6.1): it builds `kraken-ffi` with Cargo for every target and generates
the bindings of the 3 crates (`generateBindingsForExternalCrates`), in the packages
`uniffi.kraken_ffi`, `uniffi.kraken_spot_api` and `uniffi.kraken_network`.

## Prerequisites

On macOS:

- a JDK 21 (`brew install openjdk@21`): Gradle itself comes with the wrapper `./gradlew`
- every Rust target and the cross compilers of Linux and Windows, see [Toolchains](toolchains.md):

  ```sh
  rustup target add aarch64-apple-darwin x86_64-apple-darwin \
    aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios \
    aarch64-linux-android armv7-linux-androideabi x86_64-linux-android \
    aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
  brew tap messense/macos-cross-toolchains
  brew install x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu mingw-w64
  ```

- Xcode, for the Apple targets
- the Android SDK with an NDK, in `ANDROID_HOME` or `sdk.dir` of `local.properties`

Outside of macOS, the Apple tasks are disabled: Linux can still build and test its own targets.

## Build

```sh
./gradlew :kraken-multiplatform:assemble -PreleaseBuild=true
```

`-PreleaseBuild=true` builds the release libraries of every target:

| Target | Output (`platforms/kotlin-multiplatform/build/`) | Native libraries |
|--------|--------------------------------------------------|------------------|
| JVM | `libs/kraken-multiplatform-jvm-<version>.jar` | `darwin-aarch64`, `darwin-x86-64`, `linux-aarch64`, `linux-x86-64`, `win32-x86-64` |
| Android | `outputs/aar/kraken-multiplatform.aar` | `arm64-v8a`, `armeabi-v7a`, `x86_64` (no 32-bit `x86` with the plugin) |
| iOS, macOS, Linux, Windows | `classes/kotlin/<target>/main/klib` | `iosArm64`, `iosSimulatorArm64`, `iosX64`, `macosArm64`, `linuxX64`, `linuxArm64`, `mingwX64` (no `macosX64`: neither the plugin nor its runtime support it) |

Without it, a debug build only holds the libraries of the host (`-PandroidAbis=...` picks the
Android ABIs).

## Tests

```sh
./gradlew :kraken-multiplatform:jvmTest                                                    # JVM
./gradlew :kraken-multiplatform:iosSimulatorArm64Test :kraken-multiplatform:macosArm64Test  # macOS
./gradlew :kraken-multiplatform:linuxX64Test                                               # Linux
./gradlew :kraken-multiplatform:connectedAndroidDeviceTest       # Android device / emulator
```

The common tests run on every target, Android included on a device (options, exact decimals
through the converter, invalid secret, transport error, logger implemented in Kotlin); the calls
to a mocked Kraken run on the JVM. `scripts/test-bindings.sh kotlin` and
`scripts/test-bindings.sh android` run the ones of the JVM and of Android.

## Usage

```kotlin
import uniffi.kraken_ffi.KrakenPrivateApi
import uniffi.kraken_ffi.KrakenPublicApi
import uniffi.kraken_ffi.KrakenRateLimiter
import uniffi.kraken_ffi.defaultRestOptions

// share it: Kraken limits the calls per client (Starter tier: 15 calls, +1 every 3 s)
val limiter = KrakenRateLimiter(15u, 3000uL, null)

val publicApi = KrakenPublicApi(defaultRestOptions(), limiter)
val ticker = publicApi.tickers(listOf("XBTUSD")).getValue("XBTUSD")

val privateApi = KrakenPrivateApi(apiKey, base64Secret, defaultRestOptions(), limiter)
val balances = privateApi.extendedBalance()
```

The calls are `suspend` functions, the errors `KrakenException` (`Api`, `Network`, ...). A
`KrakenLogger` implemented in Kotlin receives the logs of the rate limiter.

The amounts are the `BigDecimal` of
[`com.ionspin.kotlin:bignum`](https://github.com/ionspin/kotlin-multiplatform-bignum), available on
every target: top-level `[custom_types.Decimal]` of
[`crates/kraken-spot-api/uniffi.toml`](../crates/kraken-spot-api/uniffi.toml), read by this generator only.
On Android, the keep rules of JNA and of the bindings are published with the AAR
(`consumer-rules.pro`).

## Gradle

The Gradle build lives at the root of the repository (`settings.gradle`, `build.gradle`, the
wrapper `./gradlew`, Gradle 9.7.0) and includes `platforms/kotlin-multiplatform` as
`:kraken-multiplatform`. It is based on
[the-inkwell/gradle-tools](https://github.com/the-inkwell/gradle-tools), like the other projects:

- its commit is `GRADLE_EXTENDED_VERSION_USED` in `gradle.properties`
- its catalog `additionals` (Kotlin, AGP, the UniFFI plugin, SDK levels, test libraries) and its
  `kt-plugins` are downloaded in `gradle/extended`, ignored by git
- the dependencies missing from that catalog are in `gradle/libs.versions.toml` (`libs`)
- the group and the version are in `gradle/versions.gradle` (`GROUP_ID`, `SUFFIX`)
