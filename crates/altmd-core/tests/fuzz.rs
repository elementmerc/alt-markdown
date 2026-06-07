//! Property tests for the public render surface.
//!
//! These assert two invariants that must hold for *any* input, hostile or not:
//!   1. No input panics or aborts the process (parse, render, to_html all return).
//!   2. No input produces an executable `<script>` or `<iframe>` tag in the
//!      output: text is escaped and raw HTML is sanitised, so the dangerous
//!      substrings can never reach a browser as live tags.
//!
//! The structured generator builds "directive soup" from the real grammar
//! vocabulary (colon fences, blockquotes, lists, attribute blocks, fence
//! components) so the parser's own code paths are exercised, not just random
//! bytes. The deep-nesting overflow that prompted MAX_NESTING_DEPTH is the kind
//! of defect this guards against returning.

use proptest::collection::vec;
use proptest::prelude::*;

/// The output must never contain a live script or iframe tag, case-insensitive.
///
/// These are sound invariants: a literal `<` in document text is always escaped
/// to `&lt;`, and raw HTML is sanitised, so the `<script` / `<iframe` substrings
/// can never appear as live tags whatever the input. (A bare `javascript:`
/// substring is deliberately not checked here: it appears legitimately as escaped
/// text inside code spans and is harmless there; URL-scheme stripping is covered
/// by the deterministic render-path tests instead.)
fn assert_no_executable_tag(html: &str) {
    let lower = html.to_ascii_lowercase();
    assert!(!lower.contains("<script"), "script tag leaked: {html}");
    assert!(!lower.contains("<iframe"), "iframe tag leaked: {html}");
}

/// A vocabulary of grammar-significant fragments. Joined with newlines, random
/// sequences of these produce well-formed and malformed directives, nesting,
/// attribute blocks, and CommonMark, which is far more likely to hit parser
/// branches than uniformly random bytes.
fn fragment() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just(":::callout"),
        Just("::::tabs"),
        Just(":::tab{title=x}"),
        Just(":::"),
        Just("::::"),
        Just("```chart kind=bar"),
        Just("```"),
        Just("> quote"),
        Just("- item"),
        Just("# heading"),
        Just("{#id .a key=\"v v\"}"),
        Just("[link](javascript:alert(1))"),
        Just("<script>alert(1)</script>"),
        Just("<img src=x onerror=alert(1)>"),
        Just("plain text *em* `code`"),
        Just(""),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(800))]

    /// Arbitrary bytes (as lossy UTF-8) must not panic and must stay safe.
    #[test]
    fn arbitrary_input_never_panics_or_leaks(raw in vec(any::<u8>(), 0..4096)) {
        let source = String::from_utf8_lossy(&raw);
        let html = altmd_core::to_html(&source);
        assert_no_executable_tag(&html);
        if let Ok(rendered) = altmd_core::render(&source) {
            assert_no_executable_tag(&rendered);
        }
    }

    /// Structured grammar soup must not panic and must stay safe.
    #[test]
    fn directive_soup_never_panics_or_leaks(parts in vec(fragment(), 0..120)) {
        let source = parts.join("\n");
        let html = altmd_core::to_html(&source);
        assert_no_executable_tag(&html);
        if let Ok(rendered) = altmd_core::render(&source) {
            assert_no_executable_tag(&rendered);
        }
    }
}
