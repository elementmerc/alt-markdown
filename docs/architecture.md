# alt-markdown architecture

This document explains how alt-markdown is put together, for anyone who wants to
read the code, fix a bug, or add a component. It is the map; [docs/spec.md](spec.md)
is the grammar, and [CONTRIBUTING.md](../CONTRIBUTING.md) is the build and test
guide.

## The one big idea

There is exactly **one** parser and **one** syntax tree, written in Rust. That
single core compiles two ways, and a thin JavaScript runtime sits on top of it in
the browser:

```
            source text (.alt / .md)
                      |
        +-------------v--------------+
        |   Rust core (one codebase) |
        |   parse -> AST -> render   |
        +------+--------------+------+
               |              |
     wasm32    |              |   native
   (browser)   |              |   (CLI)
        +------v-----+   +----v--------+
        | JS runtime |   | altmd binary|
        | upgrades   |   | render/fmt  |
        | components |   | ast/check   |
        +------------+   +-------------+
```

Because the parser, the renderer, and the serializer all live in the same Rust
core, a document behaves identically in the browser, in the command-line tool,
and in any future host. There is no second parser to drift out of sync. The
browser does not re-implement anything; it loads the same core compiled to
WebAssembly and adds only the parts that must be JavaScript (the DOM and the
component libraries).

## The layers

The system is five Rust crates plus one JavaScript package. Each crate has a
single responsibility, and they depend on each other in one direction only.

```
altmd-ast        the syntax tree + the Parser/Serializer traits (the contract)
   ^
altmd-parser     comrak-backed parser, the hybrid component grammar, serializer
   ^
altmd-sanitize   the raw-HTML allowlist sanitiser
   ^
altmd-core       the public facade: parse, render to HTML, serialise, normalise
   ^                                   |
altmd-wasm  ------+                    +------ altmd-cli (the `altmd` binary)
   |
js/packages/runtime  loads the wasm, renders, upgrades components in the page
```

### altmd-ast: the contract

`altmd-ast` defines the `Document`, `Block`, and `Inline` types and the two
traits every other layer depends on:

- `Parser`: text to `Document`.
- `Serializer`: `Document` back to text.

This is the trait boundary. Nothing above it knows or cares which concrete parser
is in use. The AST is alt-markdown's own type, not the underlying library's, so
the engine can be replaced later without touching the renderer, the components,
or the CLI. Every node also reserves a `Span` (a byte range into the source) for
a future lossless concrete-syntax-tree, so adding byte-exact round-trip later
does not reshape the tree.

### altmd-parser: text to tree

The concrete parser is built on **comrak**, a CommonMark and GitHub-Flavored
Markdown library, chosen because it produces a full in-memory tree (the basis of
round-trip) and compiles cleanly to `wasm32`. It sits behind the `Parser` trait,
so it is an implementation detail, not a commitment.

The parser does three things:

1. **Maps comrak's tree to our AST.** Standard CommonMark and GFM nodes (tables,
   task lists, strikethrough, autolinks, footnotes) become our `Block` and
   `Inline` variants.
2. **Parses the hybrid component grammar** (see below) on top, turning directives
   and recognised fences into `Block::Component` nodes.
3. **Bounds recursion.** The walk of comrak's tree is recursive, so it is capped
   at a fixed nesting depth. Content nested past the cap is replaced with a
   visible marker rather than crashing the process, so one hostile section cannot
   blank an otherwise readable page.

### altmd-sanitize: the allowlist

`altmd-sanitize` takes a string of raw HTML (only the untrusted raw-HTML nodes in
a document) and returns a safe subset: it strips scripts, event handlers, and
dangerous URLs against an allowlist. It is applied only where untrusted HTML
enters; the renderer's own generated tags never pass through it, because they are
safe by construction.

### altmd-core: the facade

`altmd-core` is the public Rust API and the renderer. It exposes `parse`,
`render` (component-aware HTML), `to_commonmark_html` (the pure conformance
path), `to_source`, and `normalise`. The renderer (`render.rs`) walks the AST and
emits HTML, with each component wrapped in an `alt-<name>` custom element around
its static fallback.

### altmd-wasm and altmd-cli: the two hosts

- `altmd-wasm` is a thin `wasm-bindgen` shim exposing `render` (and friends) to
  JavaScript. It carries no logic of its own.
- `altmd-cli` is the `altmd` binary: `render` (with `--standalone` and
  `--commonmark` variants), `fmt`, `ast`, and `check`. `--standalone` wraps the
  rendered HTML with an inlined theme and a small enhancement script, producing a
  single portable file.

### js/packages/runtime: the browser layer

The runtime is the only part that must be JavaScript, because charts, maths,
diagrams, and the DOM are browser APIs. On page load it:

1. loads the wasm core and renders the document (or takes already-rendered HTML),
2. registers the `alt-<name>` custom elements,
3. lets the browser upgrade each element in place, reading data from its own
   static fallback.

The heavy graphics libraries (uPlot for charts, KaTeX for maths, Mermaid for
diagrams) are lazy imports, loaded only when a document actually uses them, so
the runtime that ships to the page stays small (about 14 KB compressed).

## Data flow, end to end

```
  .alt source
      |  parse            (altmd-parser, behind the Parser trait)
      v
  Document (AST)
      |  render           (altmd-core/render.rs)
      v
  HTML: <alt-chart> ... static fallback (a data table) ... </alt-chart>
      |
      +--- no runtime --> the fallback is the final render (readable, safe)
      |
      +--- runtime loads --> registerComponents() --> the browser upgrades
                             <alt-chart> into a live uPlot canvas in place
```

The key property is **static-fallback-first**. The core renders a complete,
readable, safe document with no JavaScript. The runtime is pure enhancement: it
upgrades elements that are already there. Turn the runtime off and the document
degrades to clean semantic HTML, which is what keeps it a true superset of
markdown.

## The hybrid grammar

Components are written three ways, each for the job it suits (the full grammar is
in [docs/spec.md](spec.md)):

- **Colon directives** `:::name{...} ... :::` for structural components
  (callouts, tabs, accordions, columns, a table of contents).
- **Fenced code blocks** with an info string (` ```chart `, ` ```math `,
  ` ```diagram `) for data and diagram payloads, which stay readable as plain
  text.
- **Raw custom elements**, only as an explicitly marked, sandboxed escape hatch.

A small registry names the known components and whether each is a directive or a
fence. An unknown directive is a defined error, never a silent omission, so a
typo surfaces in `altmd check` instead of disappearing.

## The component contract

Every component obeys the same contract, which is what makes the system safe and
portable:

1. **A mandatory static fallback.** A chart falls back to a data table, a diagram
   to its source, maths to a code span, an embed to a link. The fallback is
   semantic HTML that reads correctly with no runtime.
2. **Declarative input only.** A component receives attributes and data, never a
   code string to evaluate. This is the load-bearing safety rule.
3. **Upgrade in place.** The runtime enhances the fallback element; it never
   replaces the document or trusts the component's own input.

Adding a component means: register its name, render its fallback in the core, and
write the runtime upgrade. It never means giving the document a way to run
arbitrary code.

## Safety model

The renderer is safe by construction, so there is no blanket sanitiser pass over
trusted output:

- **Text is escaped** at every boundary.
- **Generated URLs are scheme-filtered**: `javascript:`, `vbscript:`, and
  dangerous `data:` URLs collapse to an empty attribute (case-insensitive,
  whitespace-trimmed). Safe `data:image` and ordinary links pass.
- **Raw HTML is sanitised** against an allowlist, and only where untrusted HTML
  enters.
- **Anything that could carry an exploit** (diagrams, embeds, the escape hatch)
  renders inside a sandboxed iframe with no `allow-scripts` and no
  `allow-same-origin`, so it has no access to the host page.

Client-side rich rendering is a known stored-XSS sink. Confining the dangerous
surface to the single sandboxed path, and making the safe static fallback the
default render, is the mitigation. Diagrams are the worked example: Mermaid runs
to produce an SVG, but that SVG is displayed in a locked iframe, so a diagram
that tried to smuggle a script through its own labels is contained.

## Build targets

One core, two outputs:

- **WebAssembly** (`wasm32-unknown-unknown`, via `wasm-bindgen --target web`) for
  the browser. Built by `js/scripts/build-wasm.sh` into `js/wasm/web`.
- **Native binary** for the CLI, built like any Rust crate.

The same parser and renderer run in both, so the only behaviour difference
between the command line and the browser is that the browser adds the live
component upgrades.

## Extensibility and scale

The architecture is built to outlast its current implementation:

- **The Parser trait is the seam.** A bespoke, lossless engine can replace comrak
  later (for byte-exact round-trip) without changing anything above the trait.
  The AST already carries spans for that day.
- **Components lazy-load**, so the vocabulary can grow without growing the
  baseline payload.
- **Rendering streams down the block list** and does not accumulate unbounded
  state, so a large document does not force everything into memory at once.

## Testing strategy

Correctness is pinned by several layers, all run in CI:

- **CommonMark conformance** (`spec/commonmark`): all 652 examples pass through
  the pure path, which is the superset guarantee.
- **Component and safety unit tests** in `altmd-core`, including an XSS corpus
  whose payloads must render inert.
- **Property tests** (proptest): the render surface never panics or leaks a live
  tag on arbitrary input, and the serializer's normal form is a stable fixpoint.
- **CLI integration tests** for each subcommand, including `--standalone`.
- **Browser end-to-end tests** (Playwright): the gallery articles render their
  charts and diagrams, the diagrams sit in locked iframes, task-list checkboxes
  are interactive, and no payload in the cybersecurity article executes.

A change is ready only when every layer is green.
