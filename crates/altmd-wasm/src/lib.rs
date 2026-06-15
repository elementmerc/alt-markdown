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

/// Render alt-markdown `source` to component-aware HTML with source positions:
/// each top-level block carries a `data-line="N"` attribute naming the 1-based
/// source line it began on. An editor uses these to map a rendered block back to
/// its source for scroll-sync and click-to-source. Exposed to JavaScript as
/// `render_with_positions`.
///
/// # Errors
/// Returns a JavaScript error if the source contains an invalid directive.
#[wasm_bindgen]
pub fn render_with_positions(source: &str) -> Result<String, wasm_bindgen::JsError> {
    altmd_core::render_with_positions(source)
        .map_err(|err| wasm_bindgen::JsError::new(&err.to_string()))
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

/// Read the document's AI edit policy (the `:::ai-policy` block) as a JSON string,
/// or `null` when the document carries none. A browser host reads this to learn
/// which sections an AI agent may edit. Exposed to JavaScript as `policy`.
///
/// # Errors
/// Returns a JavaScript error if the source cannot be parsed.
#[wasm_bindgen]
pub fn policy(source: &str) -> Result<String, wasm_bindgen::JsError> {
    let document =
        altmd_core::parse(source).map_err(|err| wasm_bindgen::JsError::new(&err.to_string()))?;
    let policy = altmd_core::extract_policy(&document);
    serde_json::to_string(&policy).map_err(|err| wasm_bindgen::JsError::new(&err.to_string()))
}
