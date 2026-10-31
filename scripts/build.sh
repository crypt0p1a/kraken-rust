#!/usr/bin/env bash
# Builds the libraries of kraken-ffi (cdylib + staticlib) and gathers them in dist/<target>/
#
#   scripts/build.sh android|ios|macos|linux|windows   every target of a platform
#   scripts/build.sh <target triple>...                 given targets
#   PROFILE=debug scripts/build.sh ...                  release by default
#
# Requirements per platform:
#   android  cargo-ndk and an NDK (ANDROID_NDK_HOME, or the latest of $ANDROID_HOME/ndk)
#   ios      macOS with Xcode
#   macos    macOS
#   linux    a C cross compiler for the targets other than the host (ring), see the README
#   windows  MSVC on Windows (x86_64 / aarch64-pc-windows-msvc), or mingw-w64 (x86_64-pc-windows-gnu)
set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="${PROFILE:-release}"
CRATE="kraken-ffi"
LIB="kraken_ffi"

platform_targets() {
  case "$1" in
    android) echo "aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android" ;;
    ios) echo "aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios" ;;
    macos) echo "aarch64-apple-darwin x86_64-apple-darwin" ;;
    linux) echo "x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu" ;;
    windows) echo "x86_64-pc-windows-msvc aarch64-pc-windows-msvc" ;;
    *) echo "$1" ;;
  esac
}

# the files of the libraries of a target
artifacts() {
  case "$1" in
    *-windows-msvc) echo "$LIB.dll $LIB.dll.lib $LIB.lib" ;;
    *-windows-gnu) echo "$LIB.dll lib$LIB.dll.a lib$LIB.a" ;;
    *-apple-*) echo "lib$LIB.dylib lib$LIB.a" ;;
    *) echo "lib$LIB.so lib$LIB.a" ;;
  esac
}

android_ndk() {
  if [ -n "${ANDROID_NDK_HOME:-}" ]; then return; fi
  local sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
  if [ -d "$sdk/ndk" ]; then
    ANDROID_NDK_HOME="$sdk/ndk/$(ls "$sdk/ndk" | sort -V | tail -1)"
    export ANDROID_NDK_HOME
  fi
}

build() {
  local target="$1"
  local profile_flag=()
  [ "$PROFILE" = "release" ] && profile_flag=(--release)

  rustup target add "$target" >/dev/null 2>&1 || true
  echo "==> $target ($PROFILE)"

  case "$target" in
    *-android*)
      android_ndk
      # minimum API level of the Android libraries
      cargo ndk --platform "${ANDROID_API:-24}" --target "$target" build -p "$CRATE" --locked ${profile_flag[@]+"${profile_flag[@]}"}
      ;;
    *)
      cargo build -p "$CRATE" --locked --target "$target" ${profile_flag[@]+"${profile_flag[@]}"}
      ;;
  esac

  local out="target/$target/$PROFILE"
  mkdir -p "dist/$target"
  for file in $(artifacts "$target"); do
    if [ -f "$out/$file" ]; then
      cp "$out/$file" "dist/$target/"
    else
      echo "missing $out/$file" >&2
      exit 1
    fi
  done
  ls -l "dist/$target"
}

if [ "$#" -eq 0 ]; then
  sed -n '2,15p' "$0"
  exit 1
fi

for argument in "$@"; do
  for target in $(platform_targets "$argument"); do
    build "$target"
  done
done
