# alt-markdown specification

alt-markdown is a strict superset of CommonMark: every valid CommonMark document
is already a valid alt-markdown document, and a plain markdown reader still shows
all of its prose. The extensions add a curated set of components on top, written
in a readable, declarative syntax, and every component degrades to clean static
HTML when the runtime is absent.

This document describes the architecture, the extension grammar, and the safety
model. The exact component vocabulary is small and grows deliberately; the
conformance suites under `spec/` are the executable definition.

## Architecture

One parser and one syntax tree, written in Rust, are the single canonical
implementation. The same core compiles two ways:

- to WebAssembly, for the browser runtime, and
- to a native binary, for the `altmd` command-line tool.

Because there is one core, a document parses and renders the same way everywhere.
The parser sits behind a trait boundary (the syntax tree is alt-markdown's own
type, not the underlying library's), so the engine can be replaced later without
disturbing the renderer, the serializer, or the components.

Three layers sit on that core:

1. **Parser** — text to syntax tree and back.
2. **Renderer** — syntax tree to HTML, with each component carrying a static
   fallback.
3. **Runtime** — a small JavaScript and Web Components package that upgrades the
   fallbacks into rich, interactive components in the browser.

## The hybrid grammar

Components are written in three complementary forms, each for the job it suits.

1. **Colon directives** for structural components, for example a callout, tabs,
   columns, an accordion, or a table of contents:

   ```
   :::callout{type=warning}
   Heads up: this is a note.
   :::
   ```

2. **Fenced code blocks with an info string** for data and diagram payloads,
   where the content stays readable as plain text:

   ````
   ```chart kind=bar
   tier,requests
   Free,60
   Pro,600
   ```
   ````

3. **Raw custom elements** only as an explicitly marked, sandboxed escape hatch
   for arbitrary interactivity.

Shared attribute syntax `{#id .class key=value}` applies to directives and
fences alike. An unknown directive is a defined error, not a silent omission, so
a typo surfaces instead of disappearing.

## The component contract

Every component declares a mandatory static fallback. A chart falls back to a
data table, a diagram to its source, maths to a code span, an embed to a link. A
plain CommonMark reader, and any lossy export target, still shows something
sensible. This keeps the format an additive superset: a plain markdown file is
already valid, and an alt-markdown file degrades gracefully wherever markdown
renders.

The current vocabulary covers callouts, tabs, accordions, columns, charts,
maths, tables, diagrams, embeds, and a table of contents, plus the GitHub
extensions (tables, task lists, strikethrough, autolinks, footnotes).

## Safety model

The renderer is safe by construction:

- Text is escaped, and generated link and image URLs are scheme-filtered, so
  `javascript:`, `vbscript:`, and dangerous `data:` URLs cannot survive.
- The vetted standard-library components render directly in the page. They accept
  declarative attributes and data only; they never receive a code string to run.
- Anything that could carry an exploit (diagrams, embeds, the raw-HTML escape
  hatch) renders inside a sandboxed iframe with no access to the host page.
- Raw HTML from the document is run through an allowlist sanitiser; component
  output is never injected into the page unsanitised.

The static fallback is the default render and interactivity is an explicit,
sandboxed opt-in, which makes the safe path the default path. Client-side rich
rendering is a known stored-XSS sink, and confining the dangerous surface to the
single sandboxed path is the mitigation.

## Conformance

Two suites under `spec/` define correctness:

- `spec/commonmark` is the upstream CommonMark suite. Passing it in full is the
  superset guarantee: plain markdown keeps parsing exactly as it did.
- `spec/altmd` holds the extension fixtures: input mapped to expected syntax tree
  and HTML for each component.

A change is correct only when both suites stay green.
