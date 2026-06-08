//! comrak-backed parsing for alt-markdown.
//!
//! [`render_html`] is the spec-compliant CommonMark to HTML path (comrak's own
//! renderer), used while our own AST renderer matures. [`CommonMarkParser`]
//! implements the [`altmd_ast::Parser`] trait by mapping comrak's AST onto the
//! alt-markdown AST, which is the representation the converter and the component
//! layer build on. The hybrid-grammar extensions are layered on this in Phase 2.

pub mod error;

pub use error::ParseError;

use altmd_ast::{
    Alignment, AstError, Attrs, Block, Component, ComponentBody, Document, Inline, List, ListItem,
    Parser, Table,
};
use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};

/// comrak options with the GFM extensions alt-markdown adopts (tables, task
/// lists, strikethrough, autolinks, footnotes) enabled. This is the production
/// parse configuration; the CommonMark conformance path stays pure (see
/// [`render_html_unsafe`]) so bare-URL autolinking does not perturb the spec
/// fixtures.
fn gfm_options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options
}

/// Maximum nesting depth the parser accepts, for directives and for the block and
/// inline tree alike. Real documents nest a handful of levels deep; tens of
/// thousands of levels is pathological input that would otherwise overflow the
/// stack during the recursive AST walk and abort the process. We reject anything
/// deeper with a defined error instead of crashing, per the engineering baseline
/// (section 2.1: hard, documented resource caps on every parser). The limit sits
/// far above any plausible real document and far below the overflow threshold.
pub const MAX_NESTING_DEPTH: usize = 256;

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
        let segments = split_directives(source)?;
        let blocks = segments_to_blocks(&segments)?;
        Ok(Document { blocks })
    }
}

/// Parse a run of plain CommonMark (between directive fences) into block nodes.
fn parse_commonmark_blocks(source: &str) -> Result<Vec<Block>, AstError> {
    let arena = comrak::Arena::new();
    let options = gfm_options();
    let root = comrak::parse_document(&arena, source, &options);
    root.children().map(|n| map_block(n, 0)).collect()
}

fn map_blocks<'a>(node: &'a AstNode<'a>, depth: usize) -> Result<Vec<Block>, AstError> {
    node.children().map(|n| map_block(n, depth)).collect()
}

fn map_inlines<'a>(node: &'a AstNode<'a>, depth: usize) -> Result<Vec<Inline>, AstError> {
    node.children().map(|n| map_inline(n, depth)).collect()
}

/// The shared depth guard: comrak parses iteratively, but our recursive walk of
/// its tree would overflow the stack on pathologically nested input, so we bound
/// it. See [`MAX_NESTING_DEPTH`].
fn check_depth(depth: usize) -> Result<(), AstError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(AstError::Malformed(format!(
            "nesting exceeds the maximum depth of {MAX_NESTING_DEPTH}"
        )));
    }
    Ok(())
}

fn map_block<'a>(node: &'a AstNode<'a>, depth: usize) -> Result<Block, AstError> {
    check_depth(depth)?;
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Heading(h) => Ok(Block::Heading {
            level: h.level,
            content: map_inlines(node, depth + 1)?,
        }),
        NodeValue::Paragraph => Ok(Block::Paragraph(map_inlines(node, depth + 1)?)),
        NodeValue::BlockQuote => Ok(Block::BlockQuote(map_blocks(node, depth + 1)?)),
        NodeValue::List(list) => {
            let ordered = matches!(list.list_type, ListType::Ordered);
            let start = if ordered {
                u32::try_from(list.start).unwrap_or(1)
            } else {
                1
            };
            let items = node
                .children()
                .map(|n| map_list_item(n, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Block::List(List {
                ordered,
                start,
                items,
            }))
        }
        NodeValue::Table(table) => Ok(Block::Table(map_table(node, table, depth + 1)?)),
        NodeValue::FootnoteDefinition(def) => Ok(Block::FootnoteDefinition {
            name: def.name.clone(),
            blocks: map_blocks(node, depth + 1)?,
        }),
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

fn map_inline<'a>(node: &'a AstNode<'a>, depth: usize) -> Result<Inline, AstError> {
    check_depth(depth)?;
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(text) => Ok(Inline::Text(text.clone())),
        NodeValue::Emph => Ok(Inline::Emphasis(map_inlines(node, depth + 1)?)),
        NodeValue::Strong => Ok(Inline::Strong(map_inlines(node, depth + 1)?)),
        NodeValue::Strikethrough => Ok(Inline::Strikethrough(map_inlines(node, depth + 1)?)),
        NodeValue::FootnoteReference(fref) => Ok(Inline::FootnoteReference {
            name: fref.name.clone(),
        }),
        NodeValue::Code(code) => Ok(Inline::Code(code.literal.clone())),
        NodeValue::Link(link) => Ok(Inline::Link {
            url: link.url.clone(),
            title: link.title.clone(),
            content: map_inlines(node, depth + 1)?,
        }),
        NodeValue::Image(link) => Ok(Inline::Image {
            url: link.url.clone(),
            title: link.title.clone(),
            alt: map_inlines(node, depth + 1)?,
        }),
        NodeValue::SoftBreak => Ok(Inline::SoftBreak),
        NodeValue::LineBreak => Ok(Inline::HardBreak),
        NodeValue::HtmlInline(html) => Ok(Inline::HtmlInline(html.clone())),
        other => Err(AstError::Malformed(format!(
            "unsupported inline node: {other:?}"
        ))),
    }
}

/// Map one comrak list-item node, capturing GFM task-list checkbox state. A
/// task item is a `TaskItem` node (it replaces the plain `Item`); its block
/// children are mapped the same way either way.
fn map_list_item<'a>(node: &'a AstNode<'a>, depth: usize) -> Result<ListItem, AstError> {
    check_depth(depth)?;
    let task = match node.data.borrow().value {
        NodeValue::TaskItem(symbol) => Some(symbol.is_some()),
        _ => None,
    };
    Ok(ListItem {
        task,
        blocks: map_blocks(node, depth + 1)?,
    })
}

/// Map a comrak table node into a [`Table`]. Children are `TableRow` nodes (the
/// header row carries `header == true`); each cell's children are inlines.
fn map_table<'a>(
    node: &'a AstNode<'a>,
    table: &comrak::nodes::NodeTable,
    depth: usize,
) -> Result<Table, AstError> {
    let alignments = table
        .alignments
        .iter()
        .copied()
        .map(map_alignment)
        .collect();
    let mut header = Vec::new();
    let mut rows = Vec::new();
    for row in node.children() {
        let is_header = matches!(row.data.borrow().value, NodeValue::TableRow(true));
        let cells = row
            .children()
            .map(|cell| map_inlines(cell, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        if is_header {
            header = cells;
        } else {
            rows.push(cells);
        }
    }
    Ok(Table {
        alignments,
        header,
        rows,
    })
}

fn map_alignment(alignment: TableAlignment) -> Alignment {
    match alignment {
        TableAlignment::None => Alignment::None,
        TableAlignment::Left => Alignment::Left,
        TableAlignment::Center => Alignment::Center,
        TableAlignment::Right => Alignment::Right,
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
            name: "table",
            kind: Kind::Fence,
            sandboxed: false,
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
            name: "tab",
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
            name: "column",
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

/// A segment of a document: either a run of plain markdown text or a container
/// directive with its own (recursively segmented) children.
enum Segment {
    Text(String),
    Directive {
        name: String,
        attrs: Attrs,
        children: Vec<Segment>,
    },
}

/// A directive frame on the scanner stack. The root frame has `colons == 0`.
struct Frame {
    name: String,
    attrs: Attrs,
    colons: usize,
    segments: Vec<Segment>,
    text: Vec<String>,
}

impl Frame {
    fn root() -> Self {
        Self {
            name: String::new(),
            attrs: Attrs::default(),
            colons: 0,
            segments: Vec::new(),
            text: Vec::new(),
        }
    }

    fn new(name: String, attrs: Attrs, colons: usize) -> Self {
        Self {
            name,
            attrs,
            colons,
            segments: Vec::new(),
            text: Vec::new(),
        }
    }

    fn flush_text(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text).join("\n");
        self.segments.push(Segment::Text(text));
    }
}

/// Recognise a container-directive opening line: three or more colons followed by
/// a name and an optional attribute block, for example `:::callout{type=warn}`.
fn parse_open(line: &str) -> Option<(usize, String, Attrs)> {
    let trimmed = line.trim_end();
    let colons = trimmed.chars().take_while(|&c| c == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = trimmed.get(colons..)?.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if !name.chars().next().is_some_and(char::is_alphabetic) {
        return None;
    }
    let after = rest.get(name.len()..).unwrap_or_default().trim();
    Some((colons, name, parse_attrs(after)))
}

/// Recognise a container-directive closing line: a line of three or more colons
/// and nothing else.
fn parse_close(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    if trimmed.len() >= 3 && trimmed.chars().all(|c| c == ':') {
        Some(trimmed.len())
    } else {
        None
    }
}

/// Split `source` into a tree of text and directive segments. A directive is
/// closed by a colon run of the same length as its opener (innermost first), so
/// directives nest. An unclosed directive is an error.
fn split_directives(source: &str) -> Result<Vec<Segment>, AstError> {
    let mut stack: Vec<Frame> = vec![Frame::root()];

    for line in source.lines() {
        if let Some((colons, name, attrs)) = parse_open(line) {
            // The root frame occupies one slot, so the live directive depth is
            // stack.len() - 1; cap it before pushing to keep segments_to_blocks'
            // recursion (and the renderer's) bounded. See MAX_NESTING_DEPTH.
            if stack.len() > MAX_NESTING_DEPTH {
                return Err(AstError::Malformed(format!(
                    "directive nesting exceeds the maximum depth of {MAX_NESTING_DEPTH}"
                )));
            }
            if let Some(top) = stack.last_mut() {
                top.flush_text();
            }
            stack.push(Frame::new(name, attrs, colons));
            continue;
        }

        if let Some(colons) = parse_close(line) {
            let matches_top = stack
                .last()
                .is_some_and(|frame| frame.colons == colons && frame.colons != 0);
            if matches_top {
                if let Some(mut frame) = stack.pop() {
                    frame.flush_text();
                    let segment = Segment::Directive {
                        name: frame.name,
                        attrs: frame.attrs,
                        children: frame.segments,
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.segments.push(segment);
                    }
                }
                continue;
            }
        }

        if let Some(top) = stack.last_mut() {
            top.text.push(line.to_owned());
        }
    }

    if stack.len() != 1 {
        return Err(AstError::Malformed("unclosed directive".to_owned()));
    }
    let Some(mut root) = stack.pop() else {
        return Err(AstError::Malformed(
            "internal: empty directive stack".to_owned(),
        ));
    };
    root.flush_text();
    Ok(root.segments)
}

/// Turn a segment tree into block nodes, validating directive names against the
/// registry. An unknown or non-directive name is a defined error, never silent.
fn segments_to_blocks(segments: &[Segment]) -> Result<Vec<Block>, AstError> {
    let mut blocks = Vec::new();
    for segment in segments {
        match segment {
            Segment::Text(text) => blocks.extend(parse_commonmark_blocks(text)?),
            Segment::Directive {
                name,
                attrs,
                children,
            } => {
                let spec = registry::lookup(name)
                    .ok_or_else(|| AstError::Malformed(format!("unknown directive: {name}")))?;
                if spec.kind != registry::Kind::Directive {
                    return Err(AstError::Malformed(format!("'{name}' is not a directive")));
                }
                blocks.push(Block::Component(Component {
                    name: name.clone(),
                    attrs: attrs.clone(),
                    body: ComponentBody::Children(segments_to_blocks(children)?),
                }));
            }
        }
    }
    Ok(blocks)
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
                    altmd_ast::ListItem::new(vec![Block::Paragraph(vec![Inline::Text(
                        "a".into()
                    )])]),
                    altmd_ast::ListItem::new(vec![Block::Paragraph(vec![Inline::Text(
                        "b".into()
                    )])]),
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

    #[test]
    fn maps_callout_directive() {
        let doc = parse(":::callout{type=warning}\nHeads up.\n:::");
        assert_eq!(
            doc.blocks,
            vec![Block::Component(altmd_ast::Component {
                name: "callout".into(),
                attrs: altmd_ast::Attrs {
                    id: None,
                    classes: vec![],
                    pairs: vec![("type".into(), "warning".into())],
                },
                body: altmd_ast::ComponentBody::Children(vec![Block::Paragraph(vec![
                    Inline::Text("Heads up.".into()),
                ])]),
            })]
        );
    }

    #[test]
    fn nests_directives() {
        let doc = parse(":::tabs\n:::callout\nhi\n:::\n:::");
        let Some(Block::Component(tabs)) = doc.blocks.first() else {
            unreachable!("expected a tabs component")
        };
        assert_eq!(tabs.name, "tabs");
        let altmd_ast::ComponentBody::Children(children) = &tabs.body else {
            unreachable!("expected children")
        };
        assert!(matches!(children.first(), Some(Block::Component(c)) if c.name == "callout"));
    }

    #[test]
    fn orders_text_around_directives() {
        let doc = parse("before\n\n:::callout\nin\n:::\n\nafter");
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[0], Block::Paragraph(_)));
        assert!(matches!(doc.blocks[1], Block::Component(_)));
        assert!(matches!(doc.blocks[2], Block::Paragraph(_)));
    }

    #[test]
    fn unknown_directive_is_an_error() {
        assert!(CommonMarkParser::new().parse(":::bogus\nx\n:::").is_err());
    }

    #[test]
    fn fence_name_used_as_directive_is_an_error() {
        assert!(CommonMarkParser::new().parse(":::chart\nx\n:::").is_err());
    }

    #[test]
    fn unclosed_directive_is_an_error() {
        assert!(CommonMarkParser::new().parse(":::callout\nx").is_err());
    }

    #[test]
    fn deeply_nested_directives_error_not_crash() {
        let depth = super::MAX_NESTING_DEPTH + 50;
        let src = ":::callout\n".repeat(depth) + "x\n" + &":::\n".repeat(depth);
        let err = CommonMarkParser::new()
            .parse(&src)
            .expect_err("deep nesting must be a defined error");
        assert!(format!("{err}").contains("depth"), "{err}");
    }

    #[test]
    fn deeply_nested_blockquotes_error_not_crash() {
        let src = "> ".repeat(super::MAX_NESTING_DEPTH + 50) + "x\n";
        assert!(CommonMarkParser::new().parse(&src).is_err());
    }

    #[test]
    fn deeply_nested_emphasis_errors_not_crash() {
        let n = super::MAX_NESTING_DEPTH + 50;
        let src = "*".repeat(n) + "x" + &"*".repeat(n);
        // comrak may or may not nest this deeply; if it does, we must not crash.
        let _ = CommonMarkParser::new().parse(&src);
    }

    #[test]
    fn nesting_within_the_limit_is_accepted() {
        let depth = 40;
        let src = ":::callout\n".repeat(depth) + "x\n" + &":::\n".repeat(depth);
        let doc = CommonMarkParser::new().parse(&src).expect("parse");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn plain_markdown_is_unchanged_by_the_splitter() {
        let src = "# H\n\npara with *em*\n\n- a\n- b";
        let direct = super::parse_commonmark_blocks(src).expect("direct parse");
        assert_eq!(parse(src).blocks, direct);
    }
}
