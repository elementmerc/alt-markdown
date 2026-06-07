//! WebAssembly bindings for the alt-markdown core.
//!
//! The first WASM crate in the fleet. Built for `wasm32-unknown-unknown` to power
//! the zero-build CDN runtime; it also compiles for the host so `cargo test`
//! exercises the bindings.

use wasm_bindgen::prelude::wasm_bindgen;

/// Render alt-markdown `source` to HTML. Exposed to JavaScript as `render_html`.
#[wasm_bindgen]
#[must_use]
pub fn render_html(source: &str) -> String {
    altmd_core::to_html(source)
}
