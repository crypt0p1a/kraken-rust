#!/usr/bin/env bash
# Generates the bindings of kraken-ffi, and the XCFramework of the Apple platforms
#
#   scripts/bindings.sh kotlin        Kotlin sources in dist/bindings/kotlin (Android, JVM)
#   scripts/bindings.sh swift         Swift sources in platforms/apple/Sources/Kraken, the C
#                                     headers and their module map in dist/bindings/swift/include
#   scripts/bindings.sh xcframework   platforms/apple/KrakenFFI.xcframework, from the static
#                                     libraries of `scripts/build.sh ios macos` (macOS only)
#
# The bindings are generated from the library of the host, built in debug when LIBRARY isn't
# given: they are the same for every platform. The generator is the uniffi-bindgen crate of the
# workspace, so that its version is the one the libraries are built with.
set -euo pipefail

cd "$(dirname "$0")/.."

LIB="kraken_ffi"
BINDINGS="dist/bindings"
APPLE="platforms/apple"

host_library() {
  if [ -n "${LIBRARY:-}" ]; then
    echo "$LIBRARY"
    return
  fi
  cargo build -p kraken-ffi --locked >&2
  case "$(uname -s)" in
    Darwin) echo "target/debug/lib$LIB.dylib" ;;
    MINGW* | MSYS* | CYGWIN*) echo "target/debug/$LIB.dll" ;;
    *) echo "target/debug/lib$LIB.so" ;;
  esac
}

bindgen() {
  cargo run -q -p uniffi-bindgen --locked --bin "$1" -- "${@:2}"
}

kotlin() {
  local library
  library="$(host_library)"
  rm -rf "$BINDINGS/kotlin"
  bindgen uniffi-bindgen generate --library "$library" --language kotlin \
    --out-dir "$BINDINGS/kotlin" --no-format
  find "$BINDINGS/kotlin" -name '*.kt' | sort
}

swift() {
  local library
  library="$(host_library)"
  rm -rf "$BINDINGS/swift" "$APPLE/Sources/Kraken" "$APPLE/Sources/KrakenFFI"
  # the Swift sources of every crate, compiled as a single module: Kraken
  bindgen uniffi-bindgen-swift "$library" "$APPLE/Sources/Kraken" --swift-sources
  # their C declarations, in a single module: KrakenFFI (ffi_module_name of the uniffi.toml)
  bindgen uniffi-bindgen-swift "$library" "$BINDINGS/swift/include" --headers --modulemap \
    --module-name KrakenFFI --modulemap-filename module.modulemap

  # the same module as a system library, for the platforms without XCFramework (Linux): it
  # links the shared library of the host
  mkdir -p "$APPLE/Sources/KrakenFFI"
  cp "$BINDINGS"/swift/include/*.h "$APPLE/Sources/KrakenFFI/"
  sed 's/^\(module KrakenFFI {\)$/\1\n    link "'"$LIB"'"/' "$BINDINGS/swift/include/module.modulemap" \
    > "$APPLE/Sources/KrakenFFI/module.modulemap"

  find "$APPLE/Sources" "$BINDINGS/swift" -type f | sort
}

xcframework() {
  if [ "$(uname -s)" != "Darwin" ]; then
    echo "the XCFramework is built on macOS, with Xcode" >&2
    exit 1
  fi
  [ -f "$BINDINGS/swift/include/module.modulemap" ] || swift

  local work="target/xcframework"
  rm -rf "$work" "$APPLE/KrakenFFI.xcframework"
  mkdir -p "$work/ios-simulator" "$work/macos"
  local static="lib$LIB.a"
  for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin; do
    if [ ! -f "dist/$target/$static" ]; then
      echo "missing dist/$target/$static: run scripts/build.sh ios macos first" >&2
      exit 1
    fi
  done
  # one library per platform: the architectures of the simulator, then of macOS, together
  lipo -create "dist/aarch64-apple-ios-sim/$static" "dist/x86_64-apple-ios/$static" \
    -output "$work/ios-simulator/$static"
  lipo -create "dist/aarch64-apple-darwin/$static" "dist/x86_64-apple-darwin/$static" \
    -output "$work/macos/$static"

  local headers="$BINDINGS/swift/include"
  xcodebuild -create-xcframework \
    -library "dist/aarch64-apple-ios/$static" -headers "$headers" \
    -library "$work/ios-simulator/$static" -headers "$headers" \
    -library "$work/macos/$static" -headers "$headers" \
    -output "$APPLE/KrakenFFI.xcframework"
  find "$APPLE/KrakenFFI.xcframework" -maxdepth 2 | sort
}

if [ "$#" -eq 0 ]; then
  sed -n '2,12p' "$0"
  exit 1
fi

for argument in "$@"; do
  case "$argument" in
    kotlin) kotlin ;;
    swift) swift ;;
    xcframework) xcframework ;;
    *)
      echo "unknown $argument: kotlin, swift or xcframework" >&2
      exit 1
      ;;
  esac
done
