//! comrak-backed parsing for alt-markdown.
//!
//! [`render_html`] is the spec-compliant CommonMark to HTML path (comrak's own
//! renderer), used while our own AST renderer matures. [`CommonMarkParser`]
//! implements the [`altmd_ast::Parser`] trait by mapping comrak's AST onto the
//! alt-markdown AST, which is the representation the converter and the component
//! layer build on. The hybrid-grammar extensions are layered on this in Phase 2.

pub mod error;

pub use error::ParseError;

use altmd_ast::{AstError, Attrs, Block, Component, ComponentBody, Document, Inline, List, Parser};
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
        NodeValue::CodeBlock(cb) => {
            let name = cb.info.split_whitespace().next().unwrap_or_default();
            if let Some(spec) = registry::lookup(name) {
                if spec.kind == registry::Kind::Fence {
                    let rest = cb.info.get(name.len()..).unwrap_or_default();
                    return Ok(Block::Component(Component {
                        name: name.to_owned(),
                        attrs: parse_attrs(rest),
                        body: ComponentBody::Raw(cb.literal.clone()),
                    }));
                }
            }
            Ok(Block::CodeBlock {
                info: cb.info.clone(),
                literal: cb.literal.clone(),
            })
        }
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

/// The standard-library component registry: which names are components, whether
/// they are written as directives or fences, and whether they render sandboxed.
mod registry {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Kind {
        Directive,
        Fence,
    }

    pub(crate) struct Spec {
        pub(crate) name: &'static str,
        pub(crate) kind: Kind,
        #[allow(
            dead_code,
            reason = "consumed by the renderer/sandbox in a later phase"
        )]
        pub(crate) sandboxed: bool,
    }

    const SPECS: &[Spec] = &[
        Spec {
            name: "chart",
            kind: Kind::Fence,
            sandboxed: false,
        },
        Spec {
            name: "math",
            kind: Kind::Fence,
            sandboxed: false,
        },
        Spec {
            name: "diagram",
            kind: Kind::Fence,
            sandboxed: true,
        },
        Spec {
            name: "callout",
            kind: Kind::Directive,
            sandboxed: false,
        },
        Spec {
            name: "tabs",
            kind: Kind::Directive,
            sandboxed: false,
        },
        Spec {
            name: "accordion",
            kind: Kind::Directive,
            sandboxed: false,
        },
        Spec {
            name: "columns",
            kind: Kind::Directive,
            sandboxed: false,
        },
        Spec {
            name: "embed",
            kind: Kind::Directive,
            sandboxed: true,
        },
        Spec {
            name: "sandbox",
            kind: Kind::Directive,
            sandboxed: true,
        },
    ];

    pub(crate) fn lookup(name: &str) -> Option<&'static Spec> {
        SPECS.iter().find(|spec| spec.name == name)
    }
}

/// Parse an attribute string `{#id .class key=value}` (the braces are optional,
/// as in a fence info string) into [`Attrs`].
fn parse_attrs(input: &str) -> Attrs {
    let trimmed = input.trim();
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(trimmed);
    let mut attrs = Attrs::default();
    for token in tokenize(inner) {
        if let Some(id) = token.strip_prefix('#') {
            attrs.id = Some(id.to_owned());
        } else if let Some(class) = token.strip_prefix('.') {
            attrs.classes.push(class.to_owned());
        } else if let Some((key, value)) = token.split_once('=') {
            attrs.pairs.push((key.to_owned(), value.to_owned()));
        } else if !token.is_empty() {
            attrs.classes.push(token);
        }
    }
    attrs
}

/// Split an attribute string on whitespace, honouring single and double quotes
/// so a quoted value such as `title="a b"` stays one token (quotes removed).
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                } else if ch.is_whitespace() {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                } else {
                    current.push(ch);
                }
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
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

    #[test]
    fn maps_fence_component() {
        let doc = parse("```chart kind=line\njan,1\nfeb,2\n```");
        assert_eq!(
            doc.blocks,
            vec![Block::Component(altmd_ast::Component {
                name: "chart".into(),
                attrs: altmd_ast::Attrs {
                    id: None,
                    classes: vec![],
                    pairs: vec![("kind".into(), "line".into())],
                },
                body: altmd_ast::ComponentBody::Raw("jan,1\nfeb,2\n".into()),
            })]
        );
    }

    #[test]
    fn unregistered_fence_stays_code_block() {
        let doc = parse("```python\nprint(1)\n```");
        assert!(matches!(doc.blocks.first(), Some(Block::CodeBlock { .. })));
    }

    #[test]
    fn parses_attributes() {
        let attrs = super::parse_attrs("{#hero .a .b title=\"a b\" kind=line}");
        assert_eq!(attrs.id.as_deref(), Some("hero"));
        assert_eq!(attrs.classes, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(attrs.get("title"), Some("a b"));
        assert_eq!(attrs.get("kind"), Some("line"));
    }
}
