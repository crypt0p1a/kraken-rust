//! `uniffi-bindgen generate --library <libkraken_ffi> --language kotlin --out-dir <dir>`
//!
//! The bindings generator of the uniffi version the libraries are built with: a generator of
//! another version produces bindings refused at load time by the checksums. See
//! `scripts/bindings.sh`.

fn main() {
    uniffi::uniffi_bindgen_main()
}
