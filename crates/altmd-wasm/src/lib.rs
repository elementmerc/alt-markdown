//! WebAssembly bindings for the alt-markdown core.
//!
//! The first WASM crate in the fleet. Built for `wasm32-unknown-unknown` to power
//! the zero-build CDN runtime; it also compiles for the host so `cargo test`
//! exercises the bindings.

use wasm_bindgen::prelude::wasm_bindgen;

/// Render alt-markdown `source` to CommonMark-compatible HTML (no components).
/// Exposed to JavaScript as `render_html`.
#[wasm_bindgen]
#[must_use]
pub fn render_html(source: &str) -> String {
    altmd_core::to_html(source)
}

/// Render alt-markdown `source` to component-aware HTML (the runtime upgrades the
/// `alt-<name>` custom elements). Exposed to JavaScript as `render`.
///
/// # Errors
/// Returns a JavaScript error if the source contains an invalid directive.
#[wasm_bindgen]
pub fn render(source: &str) -> Result<String, wasm_bindgen::JsError> {
    altmd_core::render(source).map_err(|err| wasm_bindgen::JsError::new(&err.to_string()))
}

/// Normalise alt-markdown `source`: parse it and serialise it back to canonical
/// source text. This is the round-trip an editing host (Alexandria) uses to
/// read, edit, and write a document. Exposed to JavaScript as `normalise`.
///
/// # Errors
/// Returns a JavaScript error if the source cannot be parsed.
#[wasm_bindgen]
pub fn normalise(source: &str) -> Result<String, wasm_bindgen::JsError> {
    altmd_core::normalise(source).map_err(|err| wasm_bindgen::JsError::new(&err.to_string()))
}
