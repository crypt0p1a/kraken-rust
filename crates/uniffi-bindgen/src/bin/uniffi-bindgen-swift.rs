//! `uniffi-bindgen-swift <libkraken_ffi> <dir> --swift-sources --headers --modulemap ...`
//!
//! The Swift generator: one module map for the headers of every crate, as an XCFramework needs.
//! See `scripts/bindings.sh`.

fn main() {
    uniffi::uniffi_bindgen_swift()
}
