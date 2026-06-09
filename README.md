# alt-markdown

> "Basically an overengineered synthesis of markdown built for the modern era."

A strict superset of CommonMark that renders rich, interactive graphics in the
browser with no build step, and still reads as ordinary markdown when the
runtime is not there.


## What it is

Plain markdown, plus a small curated set of components (callouts, tabs,
accordions, charts, maths, tables, diagrams, a table of contents). Each
component is declarative and carries a static fallback, so three things hold at
once:

- **A true CommonMark superset.** All 652 spec examples pass, plus the GitHub
  extensions (tables, task lists, strikethrough, autolinks, footnotes).
- **Safe by default.** Untrusted input cannot inject a script, an event handler,
  or a dangerous URL. Diagrams render in a script-disabled, origin-isolated iframe.
- **No build step.** Load the runtime and the components light up; remove it and
  the document still reads as markdown.

## How it compares

| Property | alt-markdown | markdown-it | marked | MDX | Markdoc |
|---|:--:|:--:|:--:|:--:|:--:|
| True CommonMark superset | 🟩 | 🟩 | 🟨 | 🟥 | 🟥 |
| Safe by default | 🟩 | 🟩 | 🟥 | 🟩 | 🟩 |
| No build step | 🟩 | 🟩 | 🟩 | 🟥 | 🟩 |
| Renders rich components | 🟩 | 🟥 | 🟥 | 🟩 | 🟩 |
| Degrades to plain markdown | 🟩 | 🟩 | 🟩 | 🟥 | 🟨 |
| One core, browser and native | 🟩 | 🟥 | 🟥 | 🟥 | 🟥 |

🟩 yes · 🟨 partial · 🟥 no. Each tool in its default configuration.

## The cost of richness

The same one-page spec, three ways, measured in real Claude tokens:

| Form | Tokens | Content (vs scaffolding) | Charts, diagrams, maths | Safe by default |
|---|--:|:--:|:--:|:--:|
| Plain Markdown | 606 | 77% | 🟥 | 🟨 |
| **alt-markdown** | **637** | **70%** | 🟩 | 🟩 |
| Self-contained HTML | 3,247 | 15% | 🟩 | 🟥 |

alt-markdown costs about 5% more tokens than plain markdown but renders what
markdown cannot. The equivalent self-contained HTML costs about 5x the tokens,
is only 15% reading content, and turns any ingested string into a possible
exploit.

## Why a format, and not just HTML?

| | Plain Markdown | Raw HTML | alt-markdown |
|---|:--:|:--:|:--:|
| Rich tables, charts, diagrams | 🟨 | 🟩 | 🟩 |
| Reads as plain text | 🟩 | 🟥 | 🟩 |
| Safe by default | 🟨 | 🟥 | 🟩 |
| Easy for a person to review | 🟩 | 🟥 | 🟩 |
| Easy for a model to rewrite | 🟩 | 🟨 | 🟩 |
| Renders with no adoption | 🟥 | 🟩 | 🟨 |
| Arbitrary bespoke UI | 🟥 | 🟩 | 🟥 |

alt-markdown keeps plain markdown's readability and review-ability (the middle
rows), adds the richness of HTML, and stays safe by default. The one trade is
that, like a plain `.md` file, a `.alt` file needs its runtime to render; the
last row is a deliberate scope choice, not a missing feature.

## See it

A live gallery of three articles (a security deep dive, a literary essay, and a
data-heavy piece) lives in [`js/demo`](js/demo). Each is a plain `.alt` file that
tries hard to break the format, and each ships in a light and a Notion-style dark
theme.

```
cd js && python3 -m http.server 8000   # then open http://localhost:8000/demo/
```

Turn JavaScript off and every article still reads as ordinary markdown.

## Use it from the command line

```
altmd render doc.alt              # component-aware HTML
altmd render --standalone doc.alt # one self-contained HTML file, themed
altmd render --commonmark doc.alt # pure CommonMark HTML
altmd fmt doc.alt                 # normalise the source
altmd ast doc.alt                 # print the parsed AST as JSON
altmd check doc.alt               # validate and report a diagnostic
```

`--standalone` writes a single portable file: the document reads correctly with
no runtime, and the graphics light up when the runtime is reachable.

## How it works

One Rust core compiles to WebAssembly for the browser and to a native binary for
the CLI, so the same grammar runs everywhere. In the browser, a small runtime
(about 14 KB compressed) loads the core, renders the document, and upgrades each
component in place; the heavy graphics libraries (uPlot, KaTeX, Mermaid) load
only when a document needs them. The renderer is safe by construction: text is
escaped, URLs are scheme-filtered, raw HTML is sanitised against an allowlist,
and anything that could carry an exploit renders in a sandboxed iframe.

## Project layout

```
crates/
  altmd-ast        the syntax tree and the parser/serializer traits
  altmd-parser     comrak-backed parser, the component grammar, the serializer
  altmd-sanitize   the raw-HTML allowlist sanitiser
  altmd-core       the public facade: parse, render, serialise
  altmd-wasm       the WebAssembly bindings
  altmd-cli        the altmd command-line tool
js/
  packages/runtime the browser runtime that upgrades components
  demo             the live gallery
docs/              the spec and the architecture guide
```

## Documentation

- [docs/spec.md](docs/spec.md): the grammar and safety model.
- [docs/architecture.md](docs/architecture.md): how the pieces fit together.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to build, test, and contribute.

## Licence

MIT. See [LICENSE](LICENSE).
