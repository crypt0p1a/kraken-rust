#!/usr/bin/env bash
# Generates the bindings of kraken-ffi, then runs their tests against a local mocked Kraken
#
#   scripts/test-bindings.sh kotlin    Kotlin Multiplatform on the JVM, with the library of the host
#   scripts/test-bindings.sh swift     Swift with the library of the host: swift test (macOS:
#                                      through the XCFramework, Linux: the shared library)
#   scripts/test-bindings.sh ios       Swift on an iOS simulator, through the XCFramework (macOS)
#   scripts/test-bindings.sh android   the common tests of Kotlin Multiplatform on an Android device
#                                      or emulator (adb, Android SDK, NDK, rustup Android targets)
#
# GRADLE=<command> to use another Gradle than the wrapper of the repository.
set -euo pipefail

cd "$(dirname "$0")/.."

GRADLE="${GRADLE:-./gradlew}"
APPLE="platforms/apple"

apple_libraries() {
  scripts/build.sh ios macos
  scripts/bindings.sh swift xcframework
}

kotlin() {
  # the uniffi plugin builds kraken-ffi for the host and generates the bindings
  "$GRADLE" :kraken-multiplatform:jvmTest --console=plain
}

swift() {
  if [ "$(uname -s)" = "Darwin" ]; then
    apple_libraries
    (cd "$APPLE" && command swift test)
  else
    scripts/bindings.sh swift
    local library
    library="$(pwd)/target/debug"
    (cd "$APPLE" && LD_LIBRARY_PATH="$library${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
      command swift test -Xlinker -L"$library")
  fi
}

ios() {
  apple_libraries
  # the first iPhone simulator available
  local simulator
  simulator="$(xcrun simctl list devices available -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next(device["udid"] for runtime, list in sorted(devices.items(), reverse=True)
           if "iOS" in runtime for device in list if device["name"].startswith("iPhone")))
')"
  (cd "$APPLE" && xcodebuild test -scheme Kraken -destination "id=$simulator" -quiet)
}

android() {
  # the uniffi plugin builds kraken-ffi for the ABIs of the device and generates the bindings
  "$GRADLE" :kraken-multiplatform:connectedAndroidDeviceTest --console=plain
}

if [ "$#" -eq 0 ]; then
  sed -n '2,12p' "$0"
  exit 1
fi

for argument in "$@"; do
  case "$argument" in
    kotlin) kotlin ;;
    swift) swift ;;
    ios) ios ;;
    android) android ;;
    *)
      echo "unknown $argument: kotlin, swift, ios or android" >&2
      exit 1
      ;;
  esac
done
