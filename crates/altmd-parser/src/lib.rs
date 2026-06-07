//! comrak-backed parsing for alt-markdown.
//!
//! [`render_html`] is the spec-compliant CommonMark to HTML path (comrak's own
//! renderer), used while our own AST renderer matures. [`CommonMarkParser`]
//! implements the [`altmd_ast::Parser`] trait by mapping comrak's AST onto the
//! alt-markdown AST, which is the representation the converter and the component
//! layer build on. The hybrid-grammar extensions are layered on this in Phase 2.

pub mod error;

pub use error::ParseError;

use altmd_ast::{AstError, Block, Document, Inline, List, Parser};
use comrak::nodes::{AstNode, ListType, NodeValue};

/// Render CommonMark `source` to HTML using comrak with safe defaults: raw HTML
/// and dangerous links are suppressed (comrak's `unsafe_` option stays off).
#[must_use]
pub fn render_html(source: &str) -> String {
    let options = comrak::Options::default();
    comrak::markdown_to_html(source, &options)
}

/// Render CommonMark `source` to HTML with raw HTML passed through verbatim
/// (comrak's `unsafe_` option on). This is the spec-compliant rendering used to
/// measure CommonMark conformance.
///
/// It is NOT safe for untrusted input on its own: it must be paired with the
/// Phase 3 allowlist sanitiser before reaching a browser. The safe default for
/// production is [`render_html`].
#[must_use]
pub fn render_html_unsafe(source: &str) -> String {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    comrak::markdown_to_html(source, &options)
}

/// The CommonMark parser: maps comrak's AST onto the alt-markdown AST.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommonMarkParser;

impl CommonMarkParser {
    /// Create a new parser.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Parser for CommonMarkParser {
    fn parse(&self, source: &str) -> Result<Document, AstError> {
        let arena = comrak::Arena::new();
        let options = comrak::Options::default();
        let root = comrak::parse_document(&arena, source, &options);
        let blocks = root
            .children()
            .map(map_block)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Document { blocks })
    }
}

fn map_blocks<'a>(node: &'a AstNode<'a>) -> Result<Vec<Block>, AstError> {
    node.children().map(map_block).collect()
}

fn map_inlines<'a>(node: &'a AstNode<'a>) -> Result<Vec<Inline>, AstError> {
    node.children().map(map_inline).collect()
}

fn map_block<'a>(node: &'a AstNode<'a>) -> Result<Block, AstError> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Heading(h) => Ok(Block::Heading {
            level: h.level,
            content: map_inlines(node)?,
        }),
        NodeValue::Paragraph => Ok(Block::Paragraph(map_inlines(node)?)),
        NodeValue::BlockQuote => Ok(Block::BlockQuote(map_blocks(node)?)),
        NodeValue::List(list) => {
            let ordered = matches!(list.list_type, ListType::Ordered);
            let start = if ordered {
                u32::try_from(list.start).unwrap_or(1)
            } else {
                1
            };
            let items = node
                .children()
                .map(map_blocks)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Block::List(List {
                ordered,
                start,
                items,
            }))
        }
        NodeValue::CodeBlock(cb) => Ok(Block::CodeBlock {
            info: cb.info.clone(),
            literal: cb.literal.clone(),
        }),
        NodeValue::ThematicBreak => Ok(Block::ThematicBreak),
        NodeValue::HtmlBlock(hb) => Ok(Block::HtmlBlock(hb.literal.clone())),
        other => Err(AstError::Malformed(format!(
            "unsupported block node: {other:?}"
        ))),
    }
}

fn map_inline<'a>(node: &'a AstNode<'a>) -> Result<Inline, AstError> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(text) => Ok(Inline::Text(text.clone())),
        NodeValue::Emph => Ok(Inline::Emphasis(map_inlines(node)?)),
        NodeValue::Strong => Ok(Inline::Strong(map_inlines(node)?)),
        NodeValue::Code(code) => Ok(Inline::Code(code.literal.clone())),
        NodeValue::Link(link) => Ok(Inline::Link {
            url: link.url.clone(),
            title: link.title.clone(),
            content: map_inlines(node)?,
        }),
        NodeValue::Image(link) => Ok(Inline::Image {
            url: link.url.clone(),
            title: link.title.clone(),
            alt: map_inlines(node)?,
        }),
        NodeValue::SoftBreak => Ok(Inline::SoftBreak),
        NodeValue::LineBreak => Ok(Inline::HardBreak),
        NodeValue::HtmlInline(html) => Ok(Inline::HtmlInline(html.clone())),
        other => Err(AstError::Malformed(format!(
            "unsupported inline node: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{CommonMarkParser, render_html};
    use altmd_ast::{Block, Document, Inline, List, Parser};

    fn parse(source: &str) -> Document {
        CommonMarkParser::new().parse(source).expect("parse")
    }

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

    #[test]
    fn maps_heading_paragraph_emphasis_and_code() {
        let doc = parse("# Title\n\nsome *emph* and `code`");
        assert_eq!(
            doc.blocks,
            vec![
                Block::Heading {
                    level: 1,
                    content: vec![Inline::Text("Title".into())],
                },
                Block::Paragraph(vec![
                    Inline::Text("some ".into()),
                    Inline::Emphasis(vec![Inline::Text("emph".into())]),
                    Inline::Text(" and ".into()),
                    Inline::Code("code".into()),
                ]),
            ]
        );
    }

    #[test]
    fn maps_bullet_list() {
        let doc = parse("- a\n- b");
        assert_eq!(
            doc.blocks,
            vec![Block::List(List {
                ordered: false,
                start: 1,
                items: vec![
                    vec![Block::Paragraph(vec![Inline::Text("a".into())])],
                    vec![Block::Paragraph(vec![Inline::Text("b".into())])],
                ],
            })]
        );
    }

    #[test]
    fn maps_ordered_list_start() {
        let doc = parse("3. c\n4. d");
        let Some(Block::List(list)) = doc.blocks.first() else {
            unreachable!("expected a list")
        };
        assert!(list.ordered);
        assert_eq!(list.start, 3);
        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn maps_blockquote_and_link() {
        let doc = parse("> see [here](https://example.com \"t\")");
        assert_eq!(
            doc.blocks,
            vec![Block::BlockQuote(vec![Block::Paragraph(vec![
                Inline::Text("see ".into()),
                Inline::Link {
                    url: "https://example.com".into(),
                    title: "t".into(),
                    content: vec![Inline::Text("here".into())],
                },
            ])])]
        );
    }

    #[test]
    fn maps_fenced_code_block() {
        let doc = parse("```rust\nfn main() {}\n```");
        assert_eq!(
            doc.blocks,
            vec![Block::CodeBlock {
                info: "rust".into(),
                literal: "fn main() {}\n".into(),
            }]
        );
    }
}
