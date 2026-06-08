# alt-markdown

A strict superset of CommonMark that renders rich, interactive graphics in the
browser with no build step, and still reads as ordinary markdown when the
runtime is not there.

> "Basically an overengineered synthesis of markdown built for the modern era."
>
> — Daniel Iwugo, author

## What it is

alt-markdown takes plain markdown and adds a small, curated set of components:
callouts, tabs, accordions, columns, charts, maths, tables, diagrams, and
embeds. Each component is written in readable, declarative syntax and always
carries a static fallback, so a document degrades to clean semantic HTML
wherever the runtime does not load. Every valid CommonMark document is already a
valid alt-markdown document.

Three things hold at the same time:

- **A true CommonMark superset.** It passes all 652 of the CommonMark spec
  examples, plus the common GitHub extensions: tables, task lists,
  strikethrough, autolinks, and footnotes.
- **Safe by default.** Untrusted input cannot inject a script, an event handler,
  or a dangerous URL. Diagrams render inside a script-disabled, origin-isolated
  iframe.
- **No build step.** Load the runtime as ES modules and the components light up;
  remove it and the document still reads as markdown.

## See it

A live gallery of three articles (a security deep dive, a literary essay, and a
data-heavy piece) lives in [`js/demo`](js/demo). Each one is a plain `.alt` file
rendered in the browser, and each one tries hard to break the format. Serve the
`js` directory and open the gallery:

```
cd js && python3 -m http.server 8000
# then open http://localhost:8000/demo/
```

Turn JavaScript off and every article still reads as ordinary markdown.

## How it compares

Every tool below has some of these properties. The design goal was to hold all
of them at once.

| Property | alt-markdown | markdown-it | marked | MDX | Markdoc |
|---|:--:|:--:|:--:|:--:|:--:|
| True CommonMark superset | yes | yes | partial | no | no |
| Safe by default | yes | yes | no | yes | yes |
| No build step | yes | yes | yes | no | yes |
| Renders rich components | yes | no | no | yes | yes |
| Degrades to plain markdown | yes | yes | yes | no | partial |
| One core, browser and native | yes | no | no | no | no |

Measured against each tool in its default configuration. Numbers and method are
in the project's own benchmark.

## Why a format, and not just HTML?

There is a popular argument that an AI agent should skip markdown and emit
self-contained HTML, because HTML carries richer information: tables, diagrams,
layouts, interactive controls. The expressiveness point is fair, and it is the
exact gap alt-markdown closes. The question is what you give up to close it.

| | Raw HTML | alt-markdown |
|---|:--:|:--:|
| Rich tables, charts, diagrams | yes | yes |
| Reads as plain text | no | yes |
| Safe by default | no | yes |
| Easy for a person to review | no | yes |
| Easy for a model to rewrite | costly | yes |
| Renders with no adoption | yes | needs the runtime |
| Arbitrary bespoke UI | yes | no, by design |

The trade is deliberate. alt-markdown is not an app platform: it will not hand
you a single-use editor with custom controls. It gives you a fixed, safe,
portable vocabulary that a person can read, a reviewer can check, and a model can
rewrite without wading through inline styles and scripts. When a document is
increasingly maintained by a model rather than a person, a compact structured
source is easier to read and to regenerate than a wall of HTML, and it cannot
quietly carry an exploit.

## Use it from the command line

The `altmd` binary renders, formats, inspects, and validates documents.

```
altmd render doc.alt              # component-aware HTML
altmd render --commonmark doc.alt # pure CommonMark HTML
altmd fmt doc.alt                 # normalise the source
altmd ast doc.alt                 # print the parsed AST as JSON
altmd check doc.alt               # validate and report a diagnostic
```

## How it works

A single parser, written in Rust, compiles to WebAssembly for the browser and to
a native binary for the command line, so the same grammar runs everywhere. The
parser is built on comrak behind a trait boundary, which keeps the door open to a
bespoke engine later without changing the rest of the system.

In the browser, the JavaScript runtime loads the WebAssembly core, renders the
document, and upgrades each component in place. Heavy libraries load only when a
document needs them: uPlot for charts, KaTeX for maths, Mermaid for diagrams.
The runtime that ships to the page stays small (about 14 KB compressed); the
graphics libraries are fetched on demand.

The renderer is safe by construction: text is escaped, generated URLs are
scheme-filtered, only untrusted raw HTML is run through an allowlist sanitiser,
and anything that could carry an exploit (diagrams, embeds, the escape hatch)
renders inside a sandboxed iframe with no access to the host page.

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
docs/
  spec.md          the grammar specification
  decisions        the architecture decision records
```

## Documentation

- [docs/spec.md](docs/spec.md): the grammar.
- [docs/decisions](docs/decisions): the recorded architecture decisions.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to build, test, and contribute.

## Licence

MIT. See [LICENSE](LICENSE).
