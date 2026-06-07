# alt-markdown specification

alt-markdown is a strict superset of CommonMark: every valid CommonMark document
is already a valid alt-markdown document, and a plain markdown reader still shows
all of its prose. The extensions add a curated set of components on top, expressed
in a readable, declarative syntax.

This document will hold the frozen, unambiguous grammar for those extensions. An
unambiguous spec with a conformance test suite is a precondition for the format.
The grammar is frozen here before the extension parser is built (Phase 2 of the
v0.1 plan).

## Status

Placeholder. The locked architecture and the shape of the grammar are recorded in
[docs/decisions/0001-core-architecture-and-grammar.md](decisions/0001-core-architecture-and-grammar.md).

## The hybrid grammar (summary)

Three complementary forms, each for the job it suits:

1. Colon directives for structural components, for example a note callout.
2. Fenced code blocks with an info string for data and diagram payloads.
3. Raw custom elements, only as an explicitly marked, sandboxed escape hatch.

Every component declares a mandatory static fallback, so the document degrades
gracefully wherever markdown renders.
