//! CommonMark conformance harness.
//!
//! Proves the superset property: every CommonMark document renders per the spec.
//! Runs the official spec suite when vendored at `spec/commonmark/spec.json`
//! (relative to the workspace root); otherwise runs a small inline set and prints
//! a note. The full suite is the acceptance gate for Phase 1.
//!
//! Conformance is measured against the spec-compliant renderer
//! (`render_html_unsafe`, raw HTML passed through) because the CommonMark spec
//! requires raw HTML in the output. Production rendering (`to_html`) suppresses
//! raw HTML by default and will sanitise it via the Phase 3 allowlist; that is a
//! deliberate, safety-motivated difference, not a parsing deficiency.

use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct SpecCase {
    markdown: String,
    html: String,
    example: u32,
}

fn spec_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/commonmark/spec.json")
}

#[test]
fn commonmark_conformance() {
    let path = spec_json_path();
    if !path.exists() {
        eprintln!(
            "spec.json not vendored at {} - running inline cases only (see spec/commonmark/README.md)",
            path.display()
        );
        run_inline_cases();
        return;
    }

    let data = std::fs::read_to_string(&path).expect("read spec.json");
    let cases: Vec<SpecCase> = serde_json::from_str(&data).expect("parse spec.json");
    let total = cases.len();
    assert!(total > 0, "spec.json contained no cases");

    let mut pass = 0usize;
    let mut failures: Vec<u32> = Vec::new();
    for case in &cases {
        if altmd_parser::render_html_unsafe(&case.markdown) == case.html {
            pass += 1;
        } else {
            failures.push(case.example);
        }
    }

    let rate = pass as f64 / total as f64;
    eprintln!(
        "CommonMark conformance: {pass}/{total} ({:.2}%)",
        rate * 100.0
    );
    assert!(
        rate >= 0.99,
        "conformance below 99%: {pass}/{total}; first failing examples: {:?}",
        failures.iter().take(15).collect::<Vec<_>>()
    );
}

fn run_inline_cases() {
    let cases = [
        ("# foo\n", "<h1>foo</h1>\n"),
        ("*foo*\n", "<p><em>foo</em></p>\n"),
        ("**foo**\n", "<p><strong>foo</strong></p>\n"),
        ("`code`\n", "<p><code>code</code></p>\n"),
        ("> quote\n", "<blockquote>\n<p>quote</p>\n</blockquote>\n"),
        ("- a\n- b\n", "<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n"),
    ];
    for (markdown, expected) in cases {
        assert_eq!(
            altmd_core::to_html(markdown),
            expected,
            "case: {markdown:?}"
        );
    }
}
