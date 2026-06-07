# alt-markdown extension fixtures

This directory holds conformance fixtures for the alt-markdown extensions: input
source paired with the expected AST and the expected HTML output.

Fixtures land alongside the hybrid-grammar parser in Phase 2 of the v0.1 plan.
They also assert the superset invariant: plain CommonMark input must produce the
same result as the base parser, with no extension applied.
