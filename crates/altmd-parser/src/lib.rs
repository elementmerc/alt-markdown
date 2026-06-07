//! comrak-backed parsing for alt-markdown.
//!
//! Phase 0 exposes a direct CommonMark-to-HTML render via comrak so the rest of
//! the stack has a working baseline. Phase 1 routes parsing through
//! [`altmd_ast::Document`] via the [`altmd_ast::Parser`] trait; this passthrough
//! is replaced then, not added to.

pub mod error;

pub use error::ParseError;

/// Render CommonMark `source` to HTML using comrak with safe defaults: raw HTML
/// and dangerous links are suppressed (comrak's `unsafe_` option stays off).
///
/// Phase 0 baseline. Re-implemented to route through the alt-markdown AST in
/// Phase 1 while keeping this signature.
#[must_use]
pub fn render_html(source: &str) -> String {
    let options = comrak::Options::default();
    comrak::markdown_to_html(source, &options)
}

#[cfg(test)]
mod tests {
    use super::render_html;

    #[test]
    fn renders_basic_markdown() {
        let html = render_html("# Hi\n\nsome **bold** text");
        assert!(html.contains("<h1>"), "expected a heading: {html}");
        assert!(
            html.contains("<strong>bold</strong>"),
            "expected bold: {html}"
        );
    }

    #[test]
    fn suppresses_raw_html_by_default() {
        let html = render_html("<script>alert(1)</script>");
        assert!(
            !html.contains("<script>"),
            "raw script must not pass: {html}"
        );
    }
}
