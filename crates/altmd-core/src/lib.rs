//! Public facade for the alt-markdown core.
//!
//! The single entry point the CLI and the WASM bindings depend on. In v0.1
//! Phase 0 it renders CommonMark to HTML via [`altmd_parser`]; later phases route
//! through the AST and add the sanitiser and a component render plan behind this
//! same API.

pub mod error;

pub use error::CoreError;

// Re-export the AST surface so downstream crates depend on one facade.
pub use altmd_ast::{Block, Document, Inline, Parser, Serializer, Span};

/// Render alt-markdown `source` to an HTML string.
#[must_use]
pub fn to_html(source: &str) -> String {
    altmd_parser::render_html(source)
}

#[cfg(test)]
mod tests {
    use super::to_html;

    #[test]
    fn facade_renders() {
        assert!(to_html("# Title").contains("<h1>"));
    }
}
