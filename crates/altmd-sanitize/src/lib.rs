//! Raw-HTML allowlist sanitiser for alt-markdown.
//!
//! Phase 3 implements the real allowlist (stdlib custom-elements plus safe HTML,
//! stripping scripts, event handlers and `javascript:` URIs). This scaffold
//! exposes the surface only. It is NOT a production sanitiser yet and is not on
//! any render path; the Phase 0 render path relies on comrak suppressing raw
//! HTML by default.

pub mod error;

pub use error::SanitizeError;

/// Placeholder for the Phase 3 allowlist sanitiser. Returns the input unchanged.
///
/// Do not rely on this for untrusted input until Phase 3 lands the real allowlist.
#[must_use]
pub fn sanitize(html: &str) -> String {
    html.to_owned()
}
