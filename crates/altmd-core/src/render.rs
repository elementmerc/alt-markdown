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

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use altmd_ast::{Attrs, Block, Component, ComponentBody, Document, Inline};

/// A document heading, collected up front so that `:::toc` can list every
/// heading and each rendered heading can carry a stable anchor id.
struct Heading {
    level: u8,
    slug: String,
    text: String,
}

/// A labelled, cross-referenceable target collected in the pre-pass: the anchor
/// a `[#label]` reference links to, and the text it displays (a section's heading
/// text, or an auto-numbered name such as "Figure 3").
struct Label {
    anchor: String,
    text: String,
}

/// Render state threaded through the block walk: the headings collected in a
/// pre-pass, a cursor that advances in lockstep with the headings the renderer
/// emits, and the label table every cross-reference resolves against. The
/// pre-pass and the render walk traverse blocks in the same order, so the cursor
/// always names the heading currently being rendered.
struct RenderState {
    headings: Vec<Heading>,
    cursor: usize,
    labels: HashMap<String, Label>,
    /// Per-kind figure counters, advanced as figures are rendered. They count in
    /// the same document order as the pre-pass, so a figure's rendered number
    /// matches the number stored for it in `labels`.
    fig_counters: HashMap<String, u32>,
    /// Bibliography entries, the cited keys in order, and each cited key's number,
    /// for resolving `[@key]` citations and rendering the `:::references` list.
    bib: HashMap<String, String>,
    cited: Vec<String>,
    cite_numbers: HashMap<String, u32>,
}

/// The result of the single pre-pass over the document: the heading list (for
/// `:::toc` and the heading cursor) and the label table every cross-reference
/// resolves against.
#[derive(Default)]
struct Prepass {
    headings: Vec<Heading>,
    labels: HashMap<String, Label>,
    seen_slugs: HashMap<String, u32>,
    fig_counters: HashMap<String, u32>,
    /// Citation keys in first-appearance order, and the set used to dedupe them.
    cite_order: Vec<String>,
    cite_seen: HashSet<String>,
    /// Bibliography entries (`key` to reference text) gathered from `bib` fences.
    bib: HashMap<String, String>,
}

/// Render a [`Document`] to an HTML string.
#[must_use]
pub fn render_document(document: &Document) -> String {
    let mut pre = Prepass::default();
    collect_labels(&document.blocks, &mut pre);
    // Number the citations that have a matching bibliography entry, in the order
    // they first appear. A citation to an undefined key keeps no number and
    // renders as literal text, so plain output stays unchanged.
    let mut cited = Vec::new();
    let mut cite_numbers = HashMap::new();
    for key in &pre.cite_order {
        if pre.bib.contains_key(key) {
            cite_numbers.insert(key.clone(), cited.len() as u32 + 1);
            cited.push(key.clone());
        }
    }
    let mut state = RenderState {
        headings: pre.headings,
        cursor: 0,
        labels: pre.labels,
        fig_counters: HashMap::new(),
        bib: pre.bib,
        cited,
        cite_numbers,
    };
    let mut out = String::new();
    render_blocks(&document.blocks, &mut state, &mut out);
    out
}

/// Walk the document once before rendering to collect every cross-reference
/// target. Each heading gets a unique anchor slug (so `:::toc` and references can
/// link to headings that appear later, and repeated headings get distinct ids)
/// and a label showing its text; each `:::figure` is counted per kind and gets a
/// label showing its auto-number (for example "Figure 3"). The render walk
/// traverses blocks in this same order, so render-time figure numbers match.
fn collect_labels(blocks: &[Block], pre: &mut Prepass) {
    for block in blocks {
        match block {
            Block::Heading { level, content } => {
                let text = inline_text(content);
                let slug = unique_slug(&slugify(&text), &mut pre.seen_slugs);
                pre.labels.insert(
                    slug.clone(),
                    Label {
                        anchor: slug.clone(),
                        text: text.clone(),
                    },
                );
                pre.headings.push(Heading {
                    level: *level,
                    slug,
                    text,
                });
                scan_citations(content, pre);
            }
            Block::Paragraph(content) => scan_citations(content, pre),
            Block::Table(table) => {
                for cell in &table.header {
                    scan_citations(cell, pre);
                }
                for row in &table.rows {
                    for cell in row {
                        scan_citations(cell, pre);
                    }
                }
            }
            Block::Component(component) if component.name == "bib" => {
                if let ComponentBody::Raw(raw) = &component.body {
                    parse_bib_entries(raw, &mut pre.bib);
                }
            }
            Block::Component(component) if component.name == "figure" => {
                let (key, prefix) = figure_kind(&component.attrs);
                let counter = pre.fig_counters.entry(key.to_owned()).or_insert(0);
                *counter += 1;
                if let Some(id) = &component.attrs.id {
                    pre.labels.insert(
                        id.clone(),
                        Label {
                            anchor: id.clone(),
                            text: format!("{prefix} {counter}"),
                        },
                    );
                }
                if let ComponentBody::Children(children) = &component.body {
                    collect_labels(children, pre);
                }
            }
            Block::BlockQuote(blocks) => collect_labels(blocks, pre),
            Block::List(list) => {
                for item in &list.items {
                    collect_labels(&item.blocks, pre);
                }
            }
            Block::FootnoteDefinition { blocks, .. } => collect_labels(blocks, pre),
            Block::Component(component) => {
                if let ComponentBody::Children(blocks) = &component.body {
                    collect_labels(blocks, pre);
                }
            }
            _ => {}
        }
    }
}

/// Map a `:::figure` `kind` attribute to its counter key and caption prefix. An
/// unrecognised kind falls back to a figure.
fn figure_kind(attrs: &Attrs) -> (&'static str, &'static str) {
    match attrs.get("kind") {
        Some("table") => ("table", "Table"),
        Some("listing") => ("listing", "Listing"),
        _ => ("figure", "Figure"),
    }
}

/// Record any `[@key]` citations in an inline sequence in first-appearance order,
/// recursing into the formatted spans that can hold them.
fn scan_citations(inlines: &[Inline], pre: &mut Prepass) {
    for inline in inlines {
        match inline {
            Inline::Citation { key } if !pre.cite_seen.contains(key) => {
                pre.cite_seen.insert(key.clone());
                pre.cite_order.push(key.clone());
            }
            Inline::Emphasis(content)
            | Inline::Strong(content)
            | Inline::Strikethrough(content) => scan_citations(content, pre),
            Inline::Link { content, .. } => scan_citations(content, pre),
            _ => {}
        }
    }
}

/// Parse a `bib` fence body into `key` to reference-text entries, one per line in
/// the form `key: reference text`. The first `: ` separates the key (which has no
/// spaces) from the human-readable reference; the first definition of a key wins.
fn parse_bib_entries(raw: &str, into: &mut HashMap<String, String>) {
    for line in raw.lines() {
        let line = line.trim();
        if let Some((key, text)) = line.split_once(": ") {
            let key = key.trim();
            let valid = key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-'));
            if valid {
                into.entry(key.to_owned())
                    .or_insert_with(|| text.trim().to_owned());
            }
        }
    }
}

/// Reduce heading text to a URL-friendly anchor slug: lowercase, alphanumerics
/// kept, every run of other characters collapsed to a single hyphen.
pub(crate) fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut pending_hyphen = false;
    for ch in text.chars() {
        // Unicode-aware so non-English headings get a real anchor, not "section".
        if ch.is_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(ch.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
    }
    if slug.is_empty() {
        slug.push_str("section");
    }
    slug
}

/// Disambiguate a slug against those already used in this document, appending
/// `-1`, `-2`, ... on collision (the convention GitHub and markdown-it follow).
fn unique_slug(base: &str, seen: &mut HashMap<String, u32>) -> String {
    let count = seen.entry(base.to_owned()).or_insert(0);
    let slug = if *count == 0 {
        base.to_owned()
    } else {
        format!("{base}-{count}")
    };
    *count += 1;
    slug
}

fn render_blocks(blocks: &[Block], state: &mut RenderState, out: &mut String) {
    for block in blocks {
        render_block(block, state, out);
    }
}

/// Render the blocks of a list item. In a tight list, a direct paragraph child
/// is emitted as bare inline content (no `<p>` wrapper), per CommonMark, so a
/// task-list checkbox sits on the same line as its text; other blocks (and all
/// blocks in a loose list) render normally.
fn render_item_blocks(blocks: &[Block], tight: bool, state: &mut RenderState, out: &mut String) {
    for block in blocks {
        match block {
            Block::Paragraph(content) if tight => render_inlines(content, state, out),
            _ => render_block(block, state, out),
        }
    }
}

fn render_block(block: &Block, state: &mut RenderState, out: &mut String) {
    match block {
        Block::Heading { level, content } => {
            let level = (*level).clamp(1, 6);
            let slug = state.headings.get(state.cursor).map(|h| h.slug.clone());
            state.cursor += 1;
            match slug {
                Some(slug) => out.push_str(&format!("<h{level} id=\"{}\">", escape_attr(&slug))),
                None => out.push_str(&format!("<h{level}>")),
            }
            render_inlines(content, state, out);
            out.push_str(&format!("</h{level}>\n"));
        }
        Block::Paragraph(content) => {
            out.push_str("<p>");
            render_inlines(content, state, out);
            out.push_str("</p>\n");
        }
        Block::BlockQuote(blocks) => {
            out.push_str("<blockquote>\n");
            render_blocks(blocks, state, out);
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
                render_item_blocks(&item.blocks, list.tight, state, out);
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
        Block::Table(table) => render_table(table, state, out),
        Block::FootnoteDefinition { name, blocks } => {
            out.push_str(&format!(
                "<section class=\"footnote\" id=\"fn-{}\">\n",
                escape_attr(name)
            ));
            render_blocks(blocks, state, out);
            out.push_str("</section>\n");
        }
        Block::HtmlBlock(html) => out.push_str(&altmd_sanitize::sanitize(html)),
        Block::Component(component) => render_component(component, state, out),
        _ => {}
    }
}

fn render_table(table: &altmd_ast::Table, state: &mut RenderState, out: &mut String) {
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
        render_inlines(cell, state, out);
        out.push_str("</th>\n");
    }
    out.push_str("</tr>\n</thead>\n");
    if !table.rows.is_empty() {
        out.push_str("<tbody>\n");
        for row in &table.rows {
            out.push_str("<tr>\n");
            for (col, cell) in row.iter().enumerate() {
                out.push_str(&format!("<td{}>", style(col)));
                render_inlines(cell, state, out);
                out.push_str("</td>\n");
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody>\n");
    }
    out.push_str("</table>\n");
}

fn render_component(component: &Component, state: &mut RenderState, out: &mut String) {
    // Component names come from the registry, so they are already safe element
    // identifiers; guard anyway against an unexpected name.
    if !is_safe_name(&component.name) {
        return;
    }
    // A figure is a numbered, captioned wrapper rendered as native semantic HTML
    // (no runtime needed), not an upgradeable custom element.
    if component.name == "figure" {
        render_figure(component, state, out);
        return;
    }
    // A bib fence is bibliography data consumed by the pre-pass; it has no visible
    // output of its own (the formatted list is rendered by :::references).
    if component.name == "bib" {
        return;
    }
    // An ai-policy block is machine-readable metadata (which sections an AI agent
    // may edit). It produces no visible output; a host reads it via the policy API.
    if component.name == "ai-policy" {
        return;
    }
    let tag = format!("alt-{}", component.name);
    out.push('<');
    out.push_str(&tag);
    render_attrs(&component.attrs, out);
    out.push('>');
    render_fallback(component, state, out);
    out.push_str(&format!("</{tag}>\n"));
}

/// Render a `:::figure` as a native `<figure>` with an auto-numbered caption. The
/// number comes from a render-time per-kind counter that advances in the same
/// document order as the pre-pass, so it matches the number a cross-reference to
/// this figure shows.
fn render_figure(component: &Component, state: &mut RenderState, out: &mut String) {
    let (key, prefix) = figure_kind(&component.attrs);
    let counter = state.fig_counters.entry(key.to_owned()).or_insert(0);
    *counter += 1;
    let number = *counter;
    out.push_str("<figure");
    if let Some(id) = &component.attrs.id {
        out.push_str(&format!(" id=\"{}\"", escape_attr(id)));
    }
    out.push_str(">\n");
    render_children(component, state, out);
    out.push_str(&format!(
        "<figcaption><span class=\"alt-figlabel\">{prefix} {number}</span>"
    ));
    if let Some(caption) = component.attrs.get("caption") {
        out.push_str(": ");
        out.push_str(&escape_html(caption));
    }
    out.push_str("</figcaption>\n</figure>\n");
}

/// Render a component's mandatory static fallback: semantic HTML that reads well
/// with no runtime, which the JS layer then enhances in place.
fn render_fallback(component: &Component, state: &mut RenderState, out: &mut String) {
    match component.name.as_str() {
        "callout" => fallback_callout(component, state, out),
        "accordion" => fallback_accordion(component, state, out),
        "tabs" => fallback_wrap(component, state, out, "alt-tabs"),
        "tab" => fallback_tab(component, state, out),
        "columns" | "column" => fallback_wrap(component, state, out, "alt-stack"),
        "chart" | "table" => fallback_data_table(component, out),
        "math" => fallback_math(component, out),
        "embed" => fallback_embed(component, out),
        "toc" => fallback_toc(state, out),
        "references" => fallback_references(state, out),
        "include" => fallback_include(component, out),
        _ => fallback_default(component, state, out),
    }
}

fn render_children(component: &Component, state: &mut RenderState, out: &mut String) {
    if let ComponentBody::Children(blocks) = &component.body {
        render_blocks(blocks, state, out);
    }
}

/// Render a table of contents: a `<nav>` linking to every heading in the
/// document by its anchor slug. Fully static, so it needs no runtime.
fn fallback_toc(state: &RenderState, out: &mut String) {
    out.push_str("<nav class=\"alt-toc\" aria-label=\"Table of contents\">");
    if state.headings.is_empty() {
        out.push_str("</nav>");
        return;
    }
    out.push_str("\n<ul>\n");
    for heading in &state.headings {
        let level = heading.level.clamp(1, 6);
        out.push_str(&format!(
            "<li class=\"alt-toc-l{level}\"><a href=\"#{}\">{}</a></li>\n",
            escape_attr(&heading.slug),
            escape_html(&heading.text)
        ));
    }
    out.push_str("</ul>\n</nav>");
}

/// Render the bibliography: an ordered list of the cited references in citation
/// order, each anchored so a `[@key]` citation links to it. The list numbering
/// matches the numbers the citations show.
fn fallback_references(state: &RenderState, out: &mut String) {
    out.push_str("<ol class=\"alt-references\">");
    for key in &state.cited {
        let text = state.bib.get(key).map_or("", String::as_str);
        out.push_str(&format!(
            "\n<li id=\"ref-{}\">{}</li>",
            escape_attr(key),
            escape_html(text)
        ));
    }
    out.push_str("\n</ol>");
}

fn fallback_callout(component: &Component, state: &mut RenderState, out: &mut String) {
    let kind = component.attrs.get("type").unwrap_or("note");
    out.push_str(&format!(
        "<aside class=\"alt-callout alt-callout-{}\" role=\"note\">\n",
        sanitize_class(kind)
    ));
    render_children(component, state, out);
    out.push_str("</aside>");
}

fn fallback_accordion(component: &Component, state: &mut RenderState, out: &mut String) {
    let title = component.attrs.get("title").unwrap_or("Details");
    out.push_str(&format!(
        "<details><summary>{}</summary>\n",
        escape_html(title)
    ));
    render_children(component, state, out);
    out.push_str("</details>");
}

fn fallback_tab(component: &Component, state: &mut RenderState, out: &mut String) {
    let title = component.attrs.get("title").unwrap_or("Tab");
    out.push_str(&format!(
        "<section class=\"alt-tab\"><h3>{}</h3>\n",
        escape_html(title)
    ));
    render_children(component, state, out);
    out.push_str("</section>");
}

fn fallback_wrap(component: &Component, state: &mut RenderState, out: &mut String, class: &str) {
    out.push_str(&format!("<div class=\"{class}\">\n"));
    render_children(component, state, out);
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

/// The fallback for an unresolved `:::include`: a link to the source file. The
/// CLI splices the included content in place before rendering, so this is what a
/// reader sees only where there is no filesystem (the browser) or where includes
/// were not expanded.
fn fallback_include(component: &Component, out: &mut String) {
    let src = component.attrs.get("src").unwrap_or("");
    if !src.is_empty() {
        out.push_str(&format!(
            "<a class=\"alt-include\" href=\"{}\">{}</a>",
            escape_attr(&safe_url(src)),
            escape_html(src)
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

fn fallback_default(component: &Component, state: &mut RenderState, out: &mut String) {
    match &component.body {
        ComponentBody::Children(blocks) => {
            out.push('\n');
            render_blocks(blocks, state, out);
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

fn render_inlines(inlines: &[Inline], state: &mut RenderState, out: &mut String) {
    for inline in inlines {
        render_inline(inline, state, out);
    }
}

fn render_inline(inline: &Inline, state: &mut RenderState, out: &mut String) {
    match inline {
        Inline::Text(text) => out.push_str(&escape_html(text)),
        Inline::Emphasis(content) => {
            out.push_str("<em>");
            render_inlines(content, state, out);
            out.push_str("</em>");
        }
        Inline::Strong(content) => {
            out.push_str("<strong>");
            render_inlines(content, state, out);
            out.push_str("</strong>");
        }
        Inline::Strikethrough(content) => {
            out.push_str("<del>");
            render_inlines(content, state, out);
            out.push_str("</del>");
        }
        Inline::CrossRef { target } => render_crossref(target, state, out),
        Inline::Citation { key } => render_citation(key, state, out),
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
            render_inlines(content, state, out);
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

/// Resolve a `[#label]` cross-reference. A known target becomes a link showing
/// the target's name; an unknown one renders as the literal text it was written
/// as, so plain CommonMark output is unchanged and `altmd check` reports the
/// dangling reference instead.
fn render_crossref(target: &str, state: &RenderState, out: &mut String) {
    match state.labels.get(target) {
        Some(label) => out.push_str(&format!(
            "<a class=\"alt-xref\" href=\"#{}\">{}</a>",
            escape_attr(&label.anchor),
            escape_html(&label.text)
        )),
        None => out.push_str(&escape_html(&format!("[#{target}]"))),
    }
}

/// Resolve a `[@key]` citation. A key with a bibliography entry becomes a
/// numbered link into the reference list; an unknown key renders as the literal
/// text it was written as, so plain CommonMark output is unchanged.
fn render_citation(key: &str, state: &RenderState, out: &mut String) {
    match state.cite_numbers.get(key) {
        Some(number) => out.push_str(&format!(
            "<a class=\"alt-cite\" href=\"#ref-{}\">[{number}]</a>",
            escape_attr(key)
        )),
        None => out.push_str(&escape_html(&format!("[@{key}]"))),
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
            Inline::CrossRef { target } => text.push_str(target),
            Inline::Citation { key } => text.push_str(key),
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
    strip_bidi_controls(text)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Remove the explicit Unicode bidirectional formatting controls (embeddings and
/// overrides U+202A..U+202E, isolates U+2066..U+2069). Real right-to-left scripts
/// render correctly without them via the Unicode bidi algorithm, but an override
/// in untrusted text can silently reorder the visible characters around it (the
/// "Trojan Source" spoofing class). Zero-width joiners, which emoji sequences and
/// scripts like Persian need, are deliberately left untouched.
fn strip_bidi_controls(text: &str) -> Cow<'_, str> {
    if text.chars().any(is_bidi_control) {
        Cow::Owned(text.chars().filter(|c| !is_bidi_control(*c)).collect())
    } else {
        Cow::Borrowed(text)
    }
}

/// True for the explicit bidirectional formatting controls.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
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
        assert!(html.contains("<h1 id=\"title\">Title</h1>"), "{html}");
        assert!(html.contains("<strong>bold</strong>"), "{html}");
    }

    #[test]
    fn headings_get_unique_anchor_slugs() {
        let html = render("# Hello World\n\n## Hello World\n\n## A & B!");
        assert!(html.contains("<h1 id=\"hello-world\">"), "{html}");
        // A repeated heading is disambiguated, not duplicated.
        assert!(html.contains("<h2 id=\"hello-world-1\">"), "{html}");
        // Punctuation collapses to single hyphens, no leading/trailing hyphen.
        assert!(html.contains("<h2 id=\"a-b\">"), "{html}");
    }

    #[test]
    fn toc_lists_every_heading_with_anchor_links() {
        let html = render(":::toc\n:::\n\n# First\n\n## Second\n\n### Third");
        assert!(html.contains("<alt-toc>"), "toc wrapper missing: {html}");
        assert!(html.contains("<nav class=\"alt-toc\""), "{html}");
        assert!(
            html.contains("<li class=\"alt-toc-l1\"><a href=\"#first\">First</a></li>"),
            "{html}"
        );
        assert!(
            html.contains("<li class=\"alt-toc-l2\"><a href=\"#second\">Second</a></li>"),
            "{html}"
        );
        assert!(
            html.contains("<li class=\"alt-toc-l3\"><a href=\"#third\">Third</a></li>"),
            "{html}"
        );
    }

    #[test]
    fn toc_escapes_heading_text() {
        // A heading containing markup characters must not leak a tag into the nav.
        let html = render(":::toc\n:::\n\n# <script>alert(1)</script>");
        assert!(!html.contains("<script"), "toc leaked a tag: {html}");
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
            html.contains("<input type=\"checkbox\" checked disabled /> done</li>"),
            "checked box not inline with text: {html}"
        );
        assert!(
            html.contains("<input type=\"checkbox\" disabled /> todo</li>"),
            "unchecked box not inline with text: {html}"
        );
    }

    #[test]
    fn tight_lists_drop_the_paragraph_wrapper() {
        // A tight list item must not wrap its text in <p>, so a checkbox or
        // bullet sits on the same line as the text.
        let html = render("- one\n- two");
        assert!(
            html.contains("<li>one</li>"),
            "tight item wrapped in <p>: {html}"
        );
    }

    #[test]
    fn loose_lists_keep_the_paragraph_wrapper() {
        let html = render("- one\n\n- two");
        assert!(
            html.contains("<li><p>one</p>"),
            "loose item lost its <p>: {html}"
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
    fn footnote_resolves_across_a_directive() {
        // A reference whose definition sits past a directive (a different parse
        // run) must still become a link, not leak as literal text.
        let html = render("text[^a]\n\n:::callout\nbox\n:::\n\nmore[^b]\n\n[^a]: one\n[^b]: two");
        assert!(
            html.contains("<a href=\"#fn-a\">a</a>") && html.contains("<a href=\"#fn-b\">b</a>"),
            "refs not resolved: {html}"
        );
        assert!(
            html.contains("id=\"fn-a\"") && html.contains("id=\"fn-b\""),
            "definitions missing: {html}"
        );
        assert!(!html.contains("[^a]"), "ref leaked as text: {html}");
    }

    #[test]
    fn strips_bidi_override_controls() {
        // A right-to-left override in text must not survive: it is the Trojan
        // Source spoofing vector and could reorder surrounding characters.
        let html = render("before \u{202E}reversed\u{202C} after");
        assert!(!html.contains('\u{202E}'), "RLO survived: {html:?}");
        assert!(!html.contains('\u{202C}'), "PDF survived: {html:?}");
        assert!(
            html.contains("reversed") && html.contains("after"),
            "content lost: {html}"
        );
    }

    #[test]
    fn keeps_zero_width_joiner_for_emoji() {
        // The ZWJ that builds a family emoji must be preserved, not stripped.
        let html = render("\u{1F468}\u{200D}\u{1F469}");
        assert!(html.contains('\u{200D}'), "ZWJ stripped, emoji broken: {html:?}");
    }

    #[test]
    fn non_english_heading_gets_a_real_anchor() {
        // A heading with no ASCII letters still gets a meaningful, unique slug.
        let html = render("# 日本語\n\n## café");
        assert!(html.contains("id=\"日本語\""), "{html}");
        assert!(html.contains("id=\"café\""), "{html}");
    }

    #[test]
    fn cross_reference_to_a_heading_resolves() {
        let html = render("# Introduction\n\nback to [#introduction] please");
        assert!(
            html.contains("<a class=\"alt-xref\" href=\"#introduction\">Introduction</a>"),
            "{html}"
        );
    }

    #[test]
    fn cross_reference_resolves_forward() {
        // A reference before its target still resolves, because the label table
        // is built in a pre-pass over the whole document.
        let html = render("jump to [#target] first\n\n# Target");
        assert!(
            html.contains("<a class=\"alt-xref\" href=\"#target\">Target</a>"),
            "{html}"
        );
    }

    #[test]
    fn unresolved_cross_reference_stays_literal_text() {
        // An unknown target renders as the literal text it was written as, so
        // plain CommonMark output is unchanged and nothing is silently dropped.
        let html = render("see [#nope] here");
        assert!(html.contains("[#nope]"), "literal text lost: {html}");
        assert!(!html.contains("alt-xref"), "should not be a link: {html}");
    }

    #[test]
    fn figure_renders_a_native_numbered_caption() {
        let html = render(":::figure{#fig:one caption=\"First plot\"}\n![](a.png)\n:::");
        assert!(html.contains("<figure id=\"fig:one\">"), "{html}");
        assert!(
            html.contains(
                "<figcaption><span class=\"alt-figlabel\">Figure 1</span>: First plot</figcaption>"
            ),
            "{html}"
        );
        // A figure is native semantic HTML, not an upgradeable custom element.
        assert!(!html.contains("<alt-figure"), "{html}");
    }

    #[test]
    fn figures_and_tables_number_independently() {
        let src = concat!(
            ":::figure{caption=\"a\"}\nx\n:::\n\n",
            ":::figure{kind=table caption=\"t\"}\ny\n:::\n\n",
            ":::figure{caption=\"b\"}\nz\n:::",
        );
        let html = render(src);
        assert!(html.contains("Figure 1</span>: a"), "{html}");
        assert!(html.contains("Table 1</span>: t"), "{html}");
        assert!(html.contains("Figure 2</span>: b"), "{html}");
    }

    #[test]
    fn cross_reference_to_a_figure_shows_its_number() {
        // Forward reference to a figure resolves to its auto-number.
        let html = render("see [#fig:plot]\n\n:::figure{#fig:plot caption=\"P\"}\nx\n:::");
        assert!(
            html.contains("<a class=\"alt-xref\" href=\"#fig:plot\">Figure 1</a>"),
            "{html}"
        );
    }

    #[test]
    fn citations_number_in_order_and_list_only_cited_entries() {
        let src = concat!(
            "First [@b], then [@a], then [@b] again.\n\n",
            "```bib\n",
            "a: Author A. Title A. 2020.\n",
            "b: Author B. Title B. 2021.\n",
            "c: Author C. Never cited. 2022.\n",
            "```\n\n",
            ":::references\n:::",
        );
        let html = render(src);
        // Numbered by first appearance: b is 1, a is 2; the repeat of b stays 1.
        assert!(
            html.contains("<a class=\"alt-cite\" href=\"#ref-b\">[1]</a>"),
            "{html}"
        );
        assert!(
            html.contains("<a class=\"alt-cite\" href=\"#ref-a\">[2]</a>"),
            "{html}"
        );
        // The bibliography lists only cited entries, in citation order.
        assert!(html.contains("<li id=\"ref-b\">Author B. Title B. 2021.</li>"), "{html}");
        assert!(html.contains("<li id=\"ref-a\">Author A. Title A. 2020.</li>"), "{html}");
        assert!(!html.contains("Never cited"), "uncited entry leaked: {html}");
        let b_pos = html.find("ref-b\">Author B").expect("b in list");
        let a_pos = html.find("ref-a\">Author A").expect("a in list");
        assert!(b_pos < a_pos, "references not in citation order: {html}");
    }

    #[test]
    fn undefined_citation_stays_literal_text() {
        let html = render("a stray [@nobody] citation");
        assert!(html.contains("[@nobody]"), "literal text lost: {html}");
        assert!(!html.contains("alt-cite"), "should not be a link: {html}");
    }

    #[test]
    fn bib_fence_has_no_visible_output() {
        let html = render("```bib\na: Author A. Title. 2020.\n```");
        assert!(!html.contains("alt-bib"), "bib rendered an element: {html}");
        assert!(!html.contains("Author A"), "bib source leaked: {html}");
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
