# alt-markdown specification

**Spec version: 1.0**

alt-markdown is a strict superset of CommonMark: every valid CommonMark document
is already a valid alt-markdown document, and a plain markdown reader still shows
all of its prose. The extensions add a curated set of components on top, written
in a readable, declarative syntax, and every component degrades to clean static
HTML when the runtime is absent.

This document describes the architecture, the extension grammar, the authoring
features, the component vocabulary, and the safety model. The conformance suites
under `spec/` are the executable definition; this prose explains them.

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

1. **Parser**: text to syntax tree and back.
2. **Renderer**: syntax tree to HTML, with each component carrying a static
   fallback.
3. **Runtime**: a small JavaScript and Web Components package that upgrades the
   fallbacks into rich, interactive components in the browser.

## The hybrid grammar

Components are written in three complementary forms, each for the job it suits.

### Colon directives

A directive is for a structural component (a callout, tabs, columns, an accordion,
a figure, a table of contents). It opens with three or more colons and a name,
and closes with a line of the same number of colons:

```
:::callout{type=warning}
Heads up: this is a note.
:::
```

Grammar:

```
directive   = open-line  body  close-line
open-line   = colon-run name [ attributes ]
close-line  = colon-run
colon-run   = ":" ":" ":" { ":" }        ; three or more colons
name        = letter { letter | digit | "-" | "_" }
```

A close line must repeat the **same number of colons** as its open line. To nest
one directive inside another, give the outer directive **more** colons than the
inner one, so the inner close does not also close the outer:

```
::::tabs
:::tab{title=Overview}
content
:::
::::
```

An unknown directive name is a defined error, not a silent omission, so a typo
surfaces instead of disappearing.

### Fenced components

A fenced code block whose info string starts with a known component name is that
component; the body is the data payload, which stays readable as plain text:

````
```chart kind=bar
tier,requests
Free,60
Pro,600
```
````

Grammar:

```
fenced      = fence-line  body  fence-line
fence-line  = "```" [ name [ space info ] ]
```

A fence whose name is not a registered component is an ordinary code block, so
existing fenced code is unaffected.

### Raw custom elements

A raw custom element is allowed only as an explicitly marked, sandboxed escape
hatch (`:::sandbox`) for arbitrary interactivity. It always renders inside an
isolated iframe (see the safety model).

### Attributes

The shared attribute syntax applies to directives and fences alike:

```
attributes  = "{" attribute { space attribute } "}"
attribute   = "#" id | "." class | key "=" value | key "=" quoted
quoted      = '"' { any-but-quote } '"'
```

For example, `{#fig:hist .wide caption="Daily active users"}` sets an id, a class,
and a quoted value containing spaces.

## Authoring features

These features make a document hold together as a single, navigable work. Each is
a strict superset: it degrades to readable text in a plain markdown viewer.

### Headings and anchors

Every heading is given a stable, unique anchor id derived from its text (lowercase,
non-alphanumeric runs collapsed to single hyphens). The slug is Unicode aware, so
a heading in any script gets a real anchor. Repeated heading text is disambiguated
with a numeric suffix.

### Table of contents

`:::toc` renders a navigation list linking to every heading in the document,
including headings that appear after it.

### Cross-references

`[#label]` is a cross-reference to a labelled element. It resolves to a link whose
text is the target's name: a section shows its heading text; a numbered element
(a figure or table) shows its number, for example "Figure 3". A reference may
point forwards to a target that appears later. An unresolved reference renders as
the literal text it was written as, so a `[#label]` that names nothing is shown
verbatim, exactly as plain CommonMark would render it.

### Figures and numbering

`:::figure` wraps any content (an image, chart, diagram, table, or code) in a
captioned, auto-numbered figure:

```
:::figure{#fig:growth caption="Daily active users"}
```chart kind=line
month,users
jan,10
feb,40
```
:::
```

The `kind` attribute selects the counter and the caption prefix, so figures,
tables, and listings number independently:

| `kind`    | Prefix    |
|-----------|-----------|
| `figure`  | Figure    |
| `table`   | Table     |
| `listing` | Listing   |

A `[#fig:growth]` cross-reference to a figure shows its number and links to it.

### Citations and bibliography

An inline `[@key]` citation resolves to a numbered link into a reference list,
numbered by order of first appearance, with repeats reusing the same number.
Entries are authored in a `bib` fence, one `key: reference text` per line, and
`:::references` marks where the formatted list of cited entries renders:

````
Markdown is compact [@thariq2026].

```bib
thariq2026: Shihipar, T. The Unreasonable Effectiveness of HTML. 2026.
```

:::references
:::
````

A `bib` fence produces no output of its own; an entry that is never cited is left
out of the list; and a citation to an undefined key renders as literal text.
Numeric style only in 1.0.

### Includes

`:::include{src="chapter-1.alt"}` splices another document in place. Resolution is
native (the `altmd` CLI reads and inlines the file before rendering), so a work
can be split across files. A `src` is resolved relative to the including file and
must stay inside the document's own directory: traversal out of that directory,
absolute paths, and symlink escapes are all refused, the include graph is checked
for cycles, and nesting depth is capped. In the browser, where there is no
filesystem, an include renders a link to its source instead.

## Component vocabulary

Every component declares a mandatory static fallback, so a plain CommonMark reader
and any lossy export still show something sensible.

| Component    | Form              | Static fallback         | Since |
|--------------|-------------------|-------------------------|-------|
| `callout`    | directive         | aside                   | 1.0   |
| `tabs` / `tab` | directive       | headed sections         | 1.0   |
| `accordion`  | directive         | details / summary       | 1.0   |
| `columns` / `column` | directive | stacked blocks          | 1.0   |
| `toc`        | directive         | navigation list         | 1.0   |
| `figure`     | directive         | figure with caption     | 1.0   |
| `references` | directive         | ordered list            | 1.0   |
| `include`    | directive         | link to the source      | 1.0   |
| `chart`      | fence             | data table              | 1.0   |
| `table`      | fence             | data table              | 1.0   |
| `math`       | fence             | code span               | 1.0   |
| `diagram`    | fence, sandboxed  | source in an iframe     | 1.0   |
| `bib`        | fence             | none (data only)        | 1.0   |
| `embed`      | directive, sandboxed | link                 | 1.0   |
| `sandbox`    | directive, sandboxed | escape hatch          | 1.0   |

Inline extensions: cross-references (`[#label]`), citations (`[@key]`), and the
GitHub extensions (tables, task lists, strikethrough, autolinks, footnotes).

## Safety model

The renderer is safe by construction:

- Text is escaped, and generated link and image URLs are scheme-filtered, so
  `javascript:`, `vbscript:`, and dangerous `data:` URLs cannot survive.
- Unicode bidirectional formatting controls are stripped from text, so a
  right-to-left override cannot reorder the visible characters (the Trojan Source
  spoof). Zero-width joiners, which emoji and scripts like Persian need, are kept.
- The vetted standard-library components render directly in the page. They accept
  declarative attributes and data only; they never receive a code string to run.
- Anything that could carry an exploit (diagrams, embeds, the raw-HTML escape
  hatch) renders inside a sandboxed iframe with no access to the host page.
- Raw HTML from the document is run through an allowlist sanitiser.

The static fallback is the default render, and interactivity is an explicit,
sandboxed opt-in, which makes the safe path the default path. Client-side rich
rendering is a known stored-XSS sink, and confining the dangerous surface to the
single sandboxed path is the mitigation.

## Versioning and stability

The grammar is frozen at version 1.0. Within the 1.x line it is additive: new
components and attributes may be added, but no construct is removed or changed in
a way that stops a 1.0 document parsing. A document that is valid today stays
valid. The `altmd --spec-version` command reports the version a build conforms to.

## Conformance

Two suites under `spec/` define correctness:

- `spec/commonmark` is the upstream CommonMark suite. Passing it in full is the
  superset guarantee: plain markdown keeps parsing exactly as it did.
- `spec/altmd/cases.json` holds the extension fixtures: each input document mapped
  to the output it must produce, or marked as an input the parser must reject.

A change is correct only when both suites stay green.
