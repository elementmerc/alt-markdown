//! Public facade for the alt-markdown core.
//!
//! The single entry point the CLI and the WASM bindings depend on. [`to_html`]
//! is the spec-compliant CommonMark render path; [`parse`] produces the
//! alt-markdown [`Document`] via the [`CommonMarkParser`]. Later phases route
//! HTML through the AST and add the sanitiser and a component render plan behind
//! this same API.

pub mod error;

pub use error::CoreError;

// One facade: downstream crates depend on these re-exports, not the sub-crates.
pub use altmd_ast::{Block, Document, Inline, List, Parser, Serializer, Span};
pub use altmd_parser::CommonMarkParser;

/// Render alt-markdown `source` to an HTML string.
#[must_use]
pub fn to_html(source: &str) -> String {
    altmd_parser::render_html(source)
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
