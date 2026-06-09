# Changelog

All notable changes to alt-markdown are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0/).

## [Unreleased]

### Format

- A strict CommonMark superset: every CommonMark document is valid alt-markdown,
  and all 652 examples in the spec suite pass.
- A curated component set, each with a static fallback so a plain markdown reader
  still shows something sensible: callouts, tabs, accordions, columns, charts,
  maths, tables, diagrams, embeds, and a table of contents.
- The GitHub extensions: tables, task lists, strikethrough, autolinks, and
  footnotes.
- Cross-references (`[#label]`) that resolve to a link showing the target's
  heading text or its figure number.
- Numbered, captioned figures (`:::figure`), with figures, tables, and listings
  counted independently.
- Numeric citations (`[@key]`) with a bibliography and a rendered reference list.
- File includes (`:::include`) that splice one document into another, confined to
  the document's directory and checked for cycles.
- Heading anchors with Unicode-aware slugs, so a heading in any script gets a real
  anchor.
- A grammar spec frozen at version 1.0; additions stay backwards compatible within
  the 1.x line.

### Runtime

- A browser runtime (about 14 KB compressed) that upgrades the static fallbacks
  into rich, interactive components, loading the heavy graphics libraries only
  when a document needs them.
- A live playground: edit alt-markdown in the browser, watch it render, and share
  what you make with a link.
- A light theme and a Notion-style dark theme.

### Command line

- `altmd render` (component-aware HTML, plus a `--standalone` single-file export),
  `ast`, `fmt`, and `check`.
- `altmd policy` reads a document's AI edit policy and guards section writes.
- `altmd --spec-version` reports the grammar version a build conforms to.

### Security

- Safe by construction: text is escaped, URLs are scheme-filtered, raw HTML is
  sanitised against an allowlist, and diagrams and embeds render in a sandboxed
  iframe that cannot reach the page.
- Unicode bidirectional overrides, the Trojan Source spoofing class, are stripped
  from rendered text.

### AI collaboration

- An `:::ai-policy` block that declares which sections an AI agent may edit,
  enforced by the host through the policy guard.

### Documentation

- A README, an architecture guide, the grammar specification, and a contributing
  guide.
