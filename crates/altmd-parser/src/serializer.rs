//! Serialise an alt-markdown [`Document`] back to source text.
//!
//! This is the inverse of [`crate::CommonMarkParser`] and the second half of the
//! round-trip that editing consumers (Alexandria) need: parse to an AST, edit
//! the AST, serialise back to a `.alt` file. It is **normalising**, not
//! byte-identical: whitespace, fence widths, and emphasis markers are
//! regularised, but re-parsing the output yields an AST equal to the input. The
//! byte-lossless concrete-syntax-tree is future work (the AST carries [`Span`]s
//! for it); this serializer is the v0.1 contract.
//!
//! Text is escaped following the same rules CommonMark's own formatter uses, so
//! markdown-significant characters in content survive the round-trip as literals.

use altmd_ast::{Alignment, Block, Component, ComponentBody, Document, Inline, List, Serializer};

/// The normalising alt-markdown serializer.
#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownSerializer;

impl MarkdownSerializer {
    /// Create a new serializer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Serializer for MarkdownSerializer {
    fn to_source(&self, document: &Document) -> String {
        let mut out = serialize_blocks(&document.blocks);
        out.push('\n');
        out
    }
}

/// Serialise a block sequence, one block per entry, separated by a blank line.
fn serialize_blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(serialize_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn serialize_block(block: &Block) -> String {
    match block {
        Block::Heading { level, content } => {
            let hashes = "#".repeat((*level).clamp(1, 6) as usize);
            format!("{hashes} {}", serialize_inlines(content, true))
        }
        Block::Paragraph(content) => serialize_inlines(content, true),
        Block::BlockQuote(blocks) => prefix_lines(&serialize_blocks(blocks), "> ", "> "),
        Block::List(list) => serialize_list(list),
        Block::CodeBlock { info, literal } => serialize_code_block(info, literal),
        Block::ThematicBreak => "---".to_owned(),
        Block::Table(table) => serialize_table(table),
        Block::FootnoteDefinition { name, blocks } => {
            let body = serialize_blocks(blocks);
            // Continuation lines are indented by four spaces, per GFM footnotes.
            prefix_lines(&body, &format!("[^{name}]: "), "    ")
        }
        Block::HtmlBlock(html) => html.trim_end_matches('\n').to_owned(),
        Block::Component(component) => serialize_component(component),
        _ => String::new(),
    }
}

fn serialize_list(list: &List) -> String {
    let mut lines = Vec::new();
    for (i, item) in list.items.iter().enumerate() {
        let marker = match (list.ordered, item.task) {
            (true, _) => format!("{}. ", list.start as usize + i),
            (false, Some(true)) => "- [x] ".to_owned(),
            (false, Some(false)) => "- [ ] ".to_owned(),
            (false, None) => "- ".to_owned(),
        };
        let indent = " ".repeat(marker.len());
        lines.push(prefix_lines(
            &serialize_blocks(&item.blocks),
            &marker,
            &indent,
        ));
    }
    lines.join("\n")
}

fn serialize_code_block(info: &str, literal: &str) -> String {
    // Use a fence long enough to contain any backtick run in the body.
    let fence = "`".repeat(longest_backtick_run(literal).max(2) + 1);
    let body = if literal.is_empty() || literal.ends_with('\n') {
        literal.to_owned()
    } else {
        format!("{literal}\n")
    };
    format!("{fence}{info}\n{body}{fence}")
}

fn serialize_component(component: &Component) -> String {
    let attrs = serialize_attrs(component);
    match &component.body {
        ComponentBody::Raw(body) => {
            // Fence component: ```name attrs\nbody```. The name and attrs are
            // space-separated so the info string parses the name back out.
            let fence = "`".repeat(longest_backtick_run(body).max(2) + 1);
            let head = if attrs.is_empty() {
                component.name.clone()
            } else {
                format!("{} {attrs}", component.name)
            };
            let body = if body.is_empty() || body.ends_with('\n') {
                body.clone()
            } else {
                format!("{body}\n")
            };
            format!("{fence}{head}\n{body}{fence}")
        }
        ComponentBody::Children(blocks) => {
            // Container directive: :::name{attrs}\n...\n:::
            let head = format!(":::{}{attrs}", component.name);
            let inner = serialize_blocks(blocks);
            if inner.is_empty() {
                format!("{head}\n:::")
            } else {
                format!("{head}\n{inner}\n:::")
            }
        }
        _ => String::new(),
    }
}

/// Serialise a component's attribute block as `{#id .class key=value}`. Values
/// containing whitespace are double-quoted. Empty attributes yield an empty
/// string (no braces).
fn serialize_attrs(component: &Component) -> String {
    let attrs = &component.attrs;
    let mut parts = Vec::new();
    if let Some(id) = &attrs.id {
        parts.push(format!("#{id}"));
    }
    for class in &attrs.classes {
        parts.push(format!(".{class}"));
    }
    for (key, value) in &attrs.pairs {
        if value.chars().any(char::is_whitespace) || value.is_empty() {
            parts.push(format!("{key}=\"{value}\""));
        } else {
            parts.push(format!("{key}={value}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", parts.join(" "))
    }
}

fn serialize_table(table: &altmd_ast::Table) -> String {
    let mut lines = Vec::new();
    let cell = |inlines: &[Inline]| serialize_inlines_in_table(inlines);
    lines.push(row(&table
        .header
        .iter()
        .map(|c| cell(c))
        .collect::<Vec<_>>()));
    lines.push(delimiter_row(&table.alignments, table.header.len()));
    for r in &table.rows {
        lines.push(row(&r.iter().map(|c| cell(c)).collect::<Vec<_>>()));
    }
    lines.join("\n")
}

fn row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

fn delimiter_row(alignments: &[Alignment], columns: usize) -> String {
    let cells: Vec<String> = (0..columns)
        .map(|i| match alignments.get(i) {
            Some(Alignment::Left) => ":---".to_owned(),
            Some(Alignment::Center) => ":---:".to_owned(),
            Some(Alignment::Right) => "---:".to_owned(),
            _ => "---".to_owned(),
        })
        .collect();
    format!("| {} |", cells.join(" | "))
}

fn serialize_inlines(inlines: &[Inline], begin_content: bool) -> String {
    let mut out = String::new();
    let mut begin = begin_content;
    for inline in inlines {
        serialize_inline(inline, &mut begin, false, &mut out);
    }
    out
}

fn serialize_inlines_in_table(inlines: &[Inline]) -> String {
    let mut out = String::new();
    let mut begin = false;
    for inline in inlines {
        serialize_inline(inline, &mut begin, true, &mut out);
    }
    out
}

fn serialize_inline(inline: &Inline, begin: &mut bool, in_table: bool, out: &mut String) {
    match inline {
        Inline::Text(text) => escape_text(text, begin, in_table, out),
        Inline::Emphasis(content) => wrap(content, "*", begin, in_table, out),
        Inline::Strong(content) => wrap(content, "**", begin, in_table, out),
        Inline::Strikethrough(content) => wrap(content, "~~", begin, in_table, out),
        Inline::Code(code) => {
            out.push_str(&serialize_inline_code(code));
            *begin = false;
        }
        Inline::Link {
            url,
            title,
            content,
        } => {
            out.push('[');
            let mut inner = false;
            for c in content {
                serialize_inline(c, &mut inner, in_table, out);
            }
            out.push_str("](");
            out.push_str(&serialize_url(url));
            if !title.is_empty() {
                out.push_str(&format!(" \"{}\"", title.replace('"', "\\\"")));
            }
            out.push(')');
            *begin = false;
        }
        Inline::Image { url, title, alt } => {
            out.push_str("![");
            let mut inner = false;
            for c in alt {
                serialize_inline(c, &mut inner, in_table, out);
            }
            out.push_str("](");
            out.push_str(&serialize_url(url));
            if !title.is_empty() {
                out.push_str(&format!(" \"{}\"", title.replace('"', "\\\"")));
            }
            out.push(')');
            *begin = false;
        }
        Inline::SoftBreak => {
            out.push('\n');
            *begin = true;
        }
        Inline::HardBreak => {
            out.push_str("\\\n");
            *begin = true;
        }
        Inline::HtmlInline(html) => {
            out.push_str(html);
            *begin = false;
        }
        Inline::FootnoteReference { name } => {
            out.push_str(&format!("[^{name}]"));
            *begin = false;
        }
        _ => {}
    }
}

fn wrap(content: &[Inline], marker: &str, begin: &mut bool, in_table: bool, out: &mut String) {
    out.push_str(marker);
    let mut inner_begin = false;
    for c in content {
        serialize_inline(c, &mut inner_begin, in_table, out);
    }
    out.push_str(marker);
    *begin = false;
}

/// Escape markdown-significant characters in text so it round-trips as literal
/// content. Mirrors CommonMark's normal-text escaping. `begin` tracks whether we
/// are at the start of a line, where `- + = . )` can start a block.
fn escape_text(text: &str, begin: &mut bool, in_table: bool, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied().unwrap_or('\0');
        let follows_digit = i > 0 && chars[i - 1].is_ascii_digit();
        let escape = matches!(
            c,
            '*' | '_' | '[' | ']' | '#' | '<' | '>' | '\\' | '`' | '!'
        ) || (c == '&' && next.is_ascii_alphabetic())
            || (in_table && c == '|')
            || (*begin && matches!(c, '-' | '+' | '=') && !follows_digit)
            || (*begin
                && matches!(c, '.' | ')')
                && follows_digit
                && (next == '\0' || next.is_whitespace()));
        if escape {
            out.push('\\');
        }
        *begin = c == '\n';
        out.push(c);
    }
}

/// Wrap inline code in a backtick fence long enough to contain it, padding with
/// a space when the content starts or ends with a backtick.
fn serialize_inline_code(code: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(code) + 1);
    let pad = code.starts_with('`')
        || code.ends_with('`')
        || code.starts_with(' ') && code.ends_with(' ');
    if pad {
        format!("{fence} {code} {fence}")
    } else {
        format!("{fence}{code}{fence}")
    }
}

/// Render a URL for a link destination, wrapping in angle brackets if it
/// contains spaces or control characters.
fn serialize_url(url: &str) -> String {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        format!("<{}>", url.replace('>', "%3E"))
    } else {
        url.to_owned()
    }
}

fn longest_backtick_run(text: &str) -> usize {
    let mut max = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            max = max.max(run);
        } else {
            run = 0;
        }
    }
    max
}

/// Prefix the first line of `text` with `first` and every later line with
/// `rest`. Blank continuation lines get the trimmed prefix so the structure
/// re-parses without trailing whitespace.
fn prefix_lines(text: &str, first: &str, rest: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let prefix = if i == 0 { first } else { rest };
        if line.is_empty() {
            out.push_str(prefix.trim_end());
        } else {
            out.push_str(prefix);
            out.push_str(line);
        }
    }
    out
}
