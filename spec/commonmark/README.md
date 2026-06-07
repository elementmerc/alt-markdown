# CommonMark conformance fixtures

This directory holds the CommonMark specification test suite, used to prove that
alt-markdown is a faithful superset: every CommonMark document must parse and
render the same way.

The suite is vendored and wired as a test harness in Phase 1 of the v0.1 plan.
Any deliberate divergence from CommonMark (for example dropping a redundant,
ambiguous construct) is documented here with its rationale.
