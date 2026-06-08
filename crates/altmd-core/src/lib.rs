//! Public facade for the alt-markdown core.
//!
//! The single entry point the CLI and the WASM bindings depend on. [`to_html`]
//! is the spec-compliant CommonMark render path; [`parse`] produces the
//! alt-markdown [`Document`] via the [`CommonMarkParser`]. Later phases route
//! HTML through the AST and add the sanitiser and a component render plan behind
//! this same API.

pub mod error;
pub mod render;

pub use error::CoreError;
pub use render::render_document;

// One facade: downstream crates depend on these re-exports, not the sub-crates.
pub use altmd_ast::{Block, Document, Inline, List, Parser, Serializer, Span};
pub use altmd_parser::{CommonMarkParser, MarkdownSerializer};

/// Render alt-markdown `source` to safe HTML: full CommonMark rendering with raw
/// HTML passed through, then sanitised to a safe subset (scripts, event handlers,
/// and dangerous URL schemes removed).
#[must_use]
pub fn to_html(source: &str) -> String {
    altmd_sanitize::sanitize(&altmd_parser::render_html_unsafe(source))
}

/// Parse alt-markdown `source` into a [`Document`].
///
/// # Errors
/// Returns [`CoreError`] if the source contains a construct the AST cannot yet
/// represent.
pub fn parse(source: &str) -> Result<Document, CoreError> {
    use altmd_ast::Parser as _;
    CommonMarkParser::new()
        .parse(source)
        .map_err(CoreError::from)
}

/// Render alt-markdown `source` to component-aware HTML: parse to the AST, then
/// render components as `alt-<name>` custom elements wrapping their static
/// fallback. This is the output the runtime upgrades in the browser.
///
/// # Errors
/// Returns [`CoreError`] if the source contains an invalid directive.
pub fn render(source: &str) -> Result<String, CoreError> {
    Ok(render_document(&parse(source)?))
}

/// Serialise a [`Document`] back to alt-markdown source text. Normalising: the
/// output re-parses to an equal AST but is not guaranteed byte-identical to the
/// original source. This is the second half of the edit round-trip.
#[must_use]
pub fn to_source(document: &Document) -> String {
    use altmd_ast::Serializer as _;
    MarkdownSerializer::new().to_source(document)
}

/// Normalise alt-markdown `source`: parse it and serialise it back. Useful as a
/// formatter and as the round-trip used by editing consumers.
///
/// # Errors
/// Returns [`CoreError`] if the source cannot be parsed.
pub fn normalise(source: &str) -> Result<String, CoreError> {
    Ok(to_source(&parse(source)?))
}

#[cfg(test)]
mod tests {
    use super::{parse, to_html};

    #[test]
    fn facade_renders() {
        assert!(to_html("# Title").contains("<h1>"));
    }

    #[test]
    fn facade_parses() {
        let doc = parse("# Title").expect("parse");
        assert_eq!(doc.blocks.len(), 1);
    }
}
