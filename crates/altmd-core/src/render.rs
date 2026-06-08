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
                match item.task {
                    Some(true) => {
                        out.push_str("<li class=\"task-list-item\">");
                        out.push_str("<input type=\"checkbox\" checked disabled /> ");
                    }
                    Some(false) => {
                        out.push_str("<li class=\"task-list-item\">");
                        out.push_str("<input type=\"checkbox\" disabled /> ");
                    }
                    None => out.push_str("<li>"),
                }
                render_blocks(&item.blocks, out);
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
        Block::Table(table) => render_table(table, out),
        Block::FootnoteDefinition { name, blocks } => {
            out.push_str(&format!(
                "<section class=\"footnote\" id=\"fn-{}\">\n",
                escape_attr(name)
            ));
            render_blocks(blocks, out);
            out.push_str("</section>\n");
        }
        Block::HtmlBlock(html) => out.push_str(&altmd_sanitize::sanitize(html)),
        Block::Component(component) => render_component(component, out),
        _ => {}
    }
}

fn render_table(table: &altmd_ast::Table, out: &mut String) {
    use altmd_ast::Alignment;
    let style = |col: usize| match table.alignments.get(col) {
        Some(Alignment::Left) => " style=\"text-align:left\"",
        Some(Alignment::Center) => " style=\"text-align:center\"",
        Some(Alignment::Right) => " style=\"text-align:right\"",
        _ => "",
    };
    out.push_str("<table>\n<thead>\n<tr>\n");
    for (col, cell) in table.header.iter().enumerate() {
        out.push_str(&format!("<th{}>", style(col)));
        render_inlines(cell, out);
        out.push_str("</th>\n");
    }
    out.push_str("</tr>\n</thead>\n");
    if !table.rows.is_empty() {
        out.push_str("<tbody>\n");
        for row in &table.rows {
            out.push_str("<tr>\n");
            for (col, cell) in row.iter().enumerate() {
                out.push_str(&format!("<td{}>", style(col)));
                render_inlines(cell, out);
                out.push_str("</td>\n");
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody>\n");
    }
    out.push_str("</table>\n");
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
    render_fallback(component, out);
    out.push_str(&format!("</{tag}>\n"));
}

/// Render a component's mandatory static fallback: semantic HTML that reads well
/// with no runtime, which the JS layer then enhances in place.
fn render_fallback(component: &Component, out: &mut String) {
    match component.name.as_str() {
        "callout" => fallback_callout(component, out),
        "accordion" => fallback_accordion(component, out),
        "tabs" => fallback_wrap(component, out, "alt-tabs"),
        "tab" => fallback_tab(component, out),
        "columns" | "column" => fallback_wrap(component, out, "alt-stack"),
        "chart" | "table" => fallback_data_table(component, out),
        "math" => fallback_math(component, out),
        "embed" => fallback_embed(component, out),
        _ => fallback_default(component, out),
    }
}

fn render_children(component: &Component, out: &mut String) {
    if let ComponentBody::Children(blocks) = &component.body {
        render_blocks(blocks, out);
    }
}

fn fallback_callout(component: &Component, out: &mut String) {
    let kind = component.attrs.get("type").unwrap_or("note");
    out.push_str(&format!(
        "<aside class=\"alt-callout alt-callout-{}\" role=\"note\">\n",
        sanitize_class(kind)
    ));
    render_children(component, out);
    out.push_str("</aside>");
}

fn fallback_accordion(component: &Component, out: &mut String) {
    let title = component.attrs.get("title").unwrap_or("Details");
    out.push_str(&format!(
        "<details><summary>{}</summary>\n",
        escape_html(title)
    ));
    render_children(component, out);
    out.push_str("</details>");
}

fn fallback_tab(component: &Component, out: &mut String) {
    let title = component.attrs.get("title").unwrap_or("Tab");
    out.push_str(&format!(
        "<section class=\"alt-tab\"><h3>{}</h3>\n",
        escape_html(title)
    ));
    render_children(component, out);
    out.push_str("</section>");
}

fn fallback_wrap(component: &Component, out: &mut String, class: &str) {
    out.push_str(&format!("<div class=\"{class}\">\n"));
    render_children(component, out);
    out.push_str("</div>");
}

fn fallback_math(component: &Component, out: &mut String) {
    if let ComponentBody::Raw(raw) = &component.body {
        out.push_str(&format!(
            "<code class=\"alt-math\">{}</code>",
            escape_html(raw.trim_end())
        ));
    }
}

fn fallback_embed(component: &Component, out: &mut String) {
    let src = safe_url(component.attrs.get("src").unwrap_or(""));
    if !src.is_empty() {
        out.push_str(&format!(
            "<a href=\"{}\" rel=\"noopener noreferrer\">{}</a>",
            escape_attr(&src),
            escape_html(&src)
        ));
    }
}

fn fallback_data_table(component: &Component, out: &mut String) {
    if let ComponentBody::Raw(raw) = &component.body {
        render_csv_table(raw, out);
    }
}

fn fallback_default(component: &Component, out: &mut String) {
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
}

/// Render comma-separated data as an HTML table (the static fallback for charts
/// and enhanced tables). The first non-empty row is treated as the header.
fn render_csv_table(raw: &str, out: &mut String) {
    let mut rows = raw.lines().filter(|line| !line.trim().is_empty());
    out.push_str("<table>");
    if let Some(header) = rows.next() {
        out.push_str("<thead><tr>");
        for cell in header.split(',') {
            out.push_str(&format!("<th>{}</th>", escape_html(cell.trim())));
        }
        out.push_str("</tr></thead>");
    }
    out.push_str("<tbody>");
    for row in rows {
        out.push_str("<tr>");
        for cell in row.split(',') {
            out.push_str(&format!("<td>{}</td>", escape_html(cell.trim())));
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table>");
}

/// Reduce a string to a safe CSS class suffix: lowercase letters, digits, hyphens.
fn sanitize_class(value: &str) -> String {
    let cleaned: String = value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();
    if cleaned.is_empty() {
        "default".to_owned()
    } else {
        cleaned
    }
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
        Inline::Strikethrough(content) => {
            out.push_str("<del>");
            render_inlines(content, out);
            out.push_str("</del>");
        }
        Inline::FootnoteReference { name } => {
            out.push_str(&format!(
                "<sup class=\"footnote-ref\"><a href=\"#fn-{}\">{}</a></sup>",
                escape_attr(name),
                escape_html(name)
            ));
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
    fn chart_fallback_is_a_data_table() {
        let html = render("```chart kind=line\nmonth,value\njan,1\n```");
        assert!(html.contains("<alt-chart data-kind=\"line\">"), "{html}");
        assert!(html.contains("<th>month</th>"), "header missing: {html}");
        assert!(html.contains("<td>jan</td>"), "row missing: {html}");
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
    fn renders_gfm_table_with_alignment() {
        let html = render("| a | b |\n|:--|--:|\n| 1 | 2 |");
        assert!(html.contains("<table>"), "{html}");
        assert!(
            html.contains("<th style=\"text-align:left\">a</th>"),
            "{html}"
        );
        assert!(
            html.contains("<th style=\"text-align:right\">b</th>"),
            "{html}"
        );
        assert!(
            html.contains("<td style=\"text-align:left\">1</td>"),
            "{html}"
        );
    }

    #[test]
    fn renders_gfm_task_list_checkboxes() {
        let html = render("- [x] done\n- [ ] todo");
        assert!(
            html.contains("<input type=\"checkbox\" checked disabled />"),
            "checked box missing: {html}"
        );
        assert!(
            html.contains("<input type=\"checkbox\" disabled />"),
            "unchecked box missing: {html}"
        );
    }

    #[test]
    fn renders_gfm_strikethrough_and_autolink() {
        let html = render("~~gone~~ and https://example.com");
        assert!(html.contains("<del>gone</del>"), "{html}");
        assert!(
            html.contains("<a href=\"https://example.com\">https://example.com</a>"),
            "{html}"
        );
    }

    #[test]
    fn renders_gfm_footnotes() {
        let html = render("text[^a]\n\n[^a]: note");
        assert!(
            html.contains("<sup class=\"footnote-ref\"><a href=\"#fn-a\">a</a></sup>"),
            "ref missing: {html}"
        );
        assert!(
            html.contains("<section class=\"footnote\" id=\"fn-a\">"),
            "definition missing: {html}"
        );
    }

    #[test]
    fn table_cells_neutralise_hostile_content() {
        // Raw HTML in a cell is sanitised (the script tag is stripped, not
        // executed); a literal `<` typed as text is escaped. Either way no live
        // script tag can reach the output.
        let html = render("| h | k |\n|---|---|\n| <script>alert(1)</script> | 1 < 2 |\n");
        assert!(
            !html.contains("<script"),
            "script leaked from table cell: {html}"
        );
        assert!(html.contains("&lt; 2"), "literal lt not escaped: {html}");
    }

    #[test]
    fn neutralises_hostile_content_inside_components() {
        // Hostile payloads inside component fences and bodies must not produce a
        // live tag or event handler: a raw component body is escaped, a markdown
        // body is sanitised, and a component attribute is escaped.
        let cases = [
            "```chart\nm,<script>alert(1)</script>\n```",
            ":::callout\n<img src=x onerror=alert(1)>\n:::",
            "::::tabs\n:::tab{title=\"x\\\"><script>alert(1)</script>\"}\nhi\n:::\n::::",
            "```diagram\n<script>alert(1)</script>\n```",
        ];
        for case in cases {
            let html = render(case);
            assert!(
                !html.contains("<script"),
                "script leaked from: {case}\n{html}"
            );
            let handler = regex_lite_event_handler(&html);
            assert!(!handler, "event handler leaked from: {case}\n{html}");
        }
    }

    /// True if the HTML contains an inline event handler (`<tag ... on*=`).
    /// A small hand-rolled check to avoid pulling in a regex dependency.
    fn regex_lite_event_handler(html: &str) -> bool {
        let lower = html.to_ascii_lowercase();
        let mut in_tag = false;
        let bytes = lower.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'<' => in_tag = true,
                b'>' => in_tag = false,
                b'o' if in_tag => {
                    let rest = &lower[i..];
                    if rest.starts_with("on")
                        && rest[2..]
                            .find('=')
                            .map(|eq| {
                                lower[i + 2..i + 2 + eq]
                                    .chars()
                                    .all(|c| c.is_ascii_alphabetic())
                            })
                            .unwrap_or(false)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    #[test]
    fn sanitises_raw_html_blocks() {
        let html = render("<div onclick=\"steal()\">hi</div>");
        assert!(!html.to_lowercase().contains("onclick"), "{html}");
    }

    #[test]
    fn callout_fallback_is_an_aside() {
        let html = render(":::callout{type=warning}\nx\n:::");
        assert!(
            html.contains("<aside class=\"alt-callout alt-callout-warning\" role=\"note\">"),
            "{html}"
        );
    }

    #[test]
    fn accordion_fallback_is_native_details() {
        let html = render(":::accordion{title=\"More\"}\nbody\n:::");
        assert!(html.contains("<details><summary>More</summary>"), "{html}");
        assert!(html.contains("body"), "{html}");
    }

    #[test]
    fn tabs_fallback_is_headed_sections() {
        let html = render("::::tabs\n:::tab{title=\"A\"}\nalpha\n:::\n::::");
        assert!(
            html.contains("<section class=\"alt-tab\"><h3>A</h3>"),
            "{html}"
        );
        assert!(html.contains("alpha"), "{html}");
    }

    #[test]
    fn math_fallback_is_code() {
        let html = render("```math\nE=mc^2\n```");
        assert!(
            html.contains("<code class=\"alt-math\">E=mc^2</code>"),
            "{html}"
        );
    }

    #[test]
    fn embed_fallback_is_a_link() {
        let html = render(":::embed{src=\"https://example.com/v\"}\n:::");
        assert!(html.contains("<a href=\"https://example.com/v\""), "{html}");
    }
}
