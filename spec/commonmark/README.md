# CommonMark conformance fixtures

This directory holds the CommonMark specification test suite, used to prove that
alt-markdown is a faithful superset: every CommonMark document must parse and
render the same way.

The suite is vendored and wired as a test harness in Phase 1 of the v0.1 plan.
Any deliberate divergence from CommonMark (for example dropping a redundant,
ambiguous construct) is documented here with its rationale.

## Vendored file

`spec.json` is the official CommonMark test suite, version 0.31.2, from
https://spec.commonmark.org/0.31.2/spec.json. The CommonMark specification is
copyright John MacFarlane and contributors, licensed under CC-BY-SA 4.0. The file
is vendored unchanged so the conformance test is reproducible offline.

Current result: 652 of 652 examples pass (100%).

## Deliberate divergence: raw HTML

Conformance is measured with raw HTML passed through, as the spec requires.
Production rendering suppresses raw HTML by default and sanitises it through the
Phase 3 allowlist instead. This is a safety choice, not a parsing limitation: the
parser handles every CommonMark construct.
