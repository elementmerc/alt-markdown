//! Component-aware AST to HTML renderer.
//!
//! Renders an [`altmd_ast::Document`] to HTML. Standard CommonMark nodes become
//! their usual tags; component nodes become `alt-<name>` custom elements wrapping
//! a static fallback, so a plain browser shows the fallback and the JS runtime
//! upgrades the element into the rich form (static-fallback-first).
//!
//! Safety: this renderer escapes all text, filters URL schemes on generated
//! links, and runs the sanitiser only on untrusted raw-HTML nodes. Our own
//! generated tags are emitted directly (and are therefore preserved), so the
//! output is safe by construction without a blanket sanitiser pass.

use altmd_ast::{Attrs, Block, Component, ComponentBody, Document, Inline};

/// Render a [`Document`] to an HTML string.
#[must_use]
pub fn render_document(document: &Document) -> String {
    let mut out = String::new();
    render_blocks(&document.blocks, &mut out);
    out
}

fn render_blocks(blocks: &[Block], out: &mut String) {
    for block in blocks {
        render_block(block, out);
    }
}

fn render_block(block: &Block, out: &mut String) {
    match block {
        Block::Heading { level, content } => {
            let level = (*level).clamp(1, 6);
            out.push_str(&format!("<h{level}>"));
            render_inlines(content, out);
            out.push_str(&format!("</h{level}>\n"));
        }
        Block::Paragraph(content) => {
            out.push_str("<p>");
            render_inlines(content, out);
            out.push_str("</p>\n");
        }
        Block::BlockQuote(blocks) => {
            out.push_str("<blockquote>\n");
            render_blocks(blocks, out);
            out.push_str("</blockquote>\n");
        }
        Block::List(list) => {
            let tag = if list.ordered { "ol" } else { "ul" };
            if list.ordered && list.start != 1 {
                out.push_str(&format!("<ol start=\"{}\">\n", list.start));
            } else {
                out.push_str(&format!("<{tag}>\n"));
            }
            for item in &list.items {
                out.push_str("<li>");
                render_blocks(item, out);
                out.push_str("</li>\n");
            }
            out.push_str(&format!("</{tag}>\n"));
        }
        Block::CodeBlock { info, literal } => {
            let lang = info.split_whitespace().next().unwrap_or_default();
            if lang.is_empty() {
                out.push_str("<pre><code>");
            } else {
                out.push_str(&format!(
                    "<pre><code class=\"language-{}\">",
                    escape_attr(lang)
                ));
            }
            out.push_str(&escape_html(literal));
            out.push_str("</code></pre>\n");
        }
        Block::ThematicBreak => out.push_str("<hr />\n"),
        Block::HtmlBlock(html) => out.push_str(&altmd_sanitize::sanitize(html)),
        Block::Component(component) => render_component(component, out),
        _ => {}
    }
}

fn render_component(component: &Component, out: &mut String) {
    // Component names come from the registry, so they are already safe element
    // identifiers; guard anyway against an unexpected name.
    if !is_safe_name(&component.name) {
        return;
    }
    let tag = format!("alt-{}", component.name);
    out.push('<');
    out.push_str(&tag);
    render_attrs(&component.attrs, out);
    out.push('>');
    match &component.body {
        ComponentBody::Children(blocks) => {
            out.push('\n');
            render_blocks(blocks, out);
        }
        ComponentBody::Raw(raw) => {
            out.push_str("<pre>");
            out.push_str(&escape_html(raw));
            out.push_str("</pre>");
        }
        _ => {}
    }
    out.push_str(&format!("</{tag}>\n"));
}

fn render_attrs(attrs: &Attrs, out: &mut String) {
    if let Some(id) = &attrs.id {
        out.push_str(&format!(" id=\"{}\"", escape_attr(id)));
    }
    if !attrs.classes.is_empty() {
        out.push_str(&format!(
            " class=\"{}\"",
            escape_attr(&attrs.classes.join(" "))
        ));
    }
    for (key, value) in &attrs.pairs {
        if is_safe_name(key) {
            out.push_str(&format!(" data-{key}=\"{}\"", escape_attr(value)));
        }
    }
}

fn render_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        render_inline(inline, out);
    }
}

fn render_inline(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(text) => out.push_str(&escape_html(text)),
        Inline::Emphasis(content) => {
            out.push_str("<em>");
            render_inlines(content, out);
            out.push_str("</em>");
        }
        Inline::Strong(content) => {
            out.push_str("<strong>");
            render_inlines(content, out);
            out.push_str("</strong>");
        }
        Inline::Code(text) => {
            out.push_str("<code>");
            out.push_str(&escape_html(text));
            out.push_str("</code>");
        }
        Inline::Link {
            url,
            title,
            content,
        } => {
            out.push_str(&format!("<a href=\"{}\"", escape_attr(&safe_url(url))));
            if !title.is_empty() {
                out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
            }
            out.push('>');
            render_inlines(content, out);
            out.push_str("</a>");
        }
        Inline::Image { url, title, alt } => {
            out.push_str(&format!(
                "<img src=\"{}\" alt=\"{}\"",
                escape_attr(&safe_url(url)),
                escape_attr(&inline_text(alt))
            ));
            if !title.is_empty() {
                out.push_str(&format!(" title=\"{}\"", escape_attr(title)));
            }
            out.push_str(" />");
        }
        Inline::SoftBreak => out.push('\n'),
        Inline::HardBreak => out.push_str("<br />\n"),
        Inline::HtmlInline(html) => out.push_str(&altmd_sanitize::sanitize(html)),
        _ => {}
    }
}

/// Collect the plain text of an inline sequence (for image alt text).
fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) => text.push_str(value),
            Inline::Emphasis(content) | Inline::Strong(content) => {
                text.push_str(&inline_text(content));
            }
            Inline::Link { content, .. } => text.push_str(&inline_text(content)),
            Inline::Image { alt, .. } => text.push_str(&inline_text(alt)),
            Inline::SoftBreak | Inline::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text
}

/// A safe custom-element name or attribute key: lowercase ASCII letters, digits,
/// and hyphens, starting with a letter.
fn is_safe_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Drop dangerous URL schemes from generated links, keeping image data URIs.
fn safe_url(url: &str) -> String {
    let lower = url.trim().to_ascii_lowercase();
    let dangerous = lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || (lower.starts_with("data:") && !lower.starts_with("data:image/"));
    if dangerous {
        String::new()
    } else {
        url.to_string()
    }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(text: &str) -> String {
    escape_html(text).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::render_document;
    use altmd_ast::Parser;
    use altmd_parser::CommonMarkParser;

    fn render(source: &str) -> String {
        let doc = CommonMarkParser::new().parse(source).expect("parse");
        render_document(&doc)
    }

    #[test]
    fn renders_basic_blocks() {
        let html = render("# Title\n\nsome **bold** text");
        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
    }

    #[test]
    fn renders_directive_as_custom_element() {
        let html = render(":::callout{type=warning}\nHeads up.\n:::");
        assert!(
            html.contains("<alt-callout data-type=\"warning\">"),
            "{html}"
        );
        assert!(html.contains("Heads up."), "fallback missing: {html}");
        assert!(html.contains("</alt-callout>"), "{html}");
    }

    #[test]
    fn renders_fence_component_with_pre_fallback() {
        let html = render("```chart kind=line\njan,1\n```");
        assert!(html.contains("<alt-chart data-kind=\"line\">"), "{html}");
        assert!(html.contains("<pre>jan,1\n</pre>"), "{html}");
    }

    #[test]
    fn strips_raw_inline_script() {
        let html = render("a <script>alert(1)</script> b");
        assert!(!html.contains("<script"), "script survived: {html}");
    }

    #[test]
    fn escapes_literal_special_characters() {
        let html = render("1 < 2 & 3");
        assert!(html.contains("&lt;"), "lt not escaped: {html}");
        assert!(html.contains("&amp;"), "amp not escaped: {html}");
    }

    #[test]
    fn filters_javascript_urls() {
        let html = render("[x](javascript:alert(1))");
        assert!(!html.contains("javascript:"), "js url survived: {html}");
        assert!(html.contains("<a href=\"\">x</a>"), "{html}");
    }

    #[test]
    fn sanitises_raw_html_blocks() {
        let html = render("<div onclick=\"steal()\">hi</div>");
        assert!(!html.to_lowercase().contains("onclick"), "{html}");
    }
}
