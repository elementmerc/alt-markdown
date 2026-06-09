//! alt-markdown extension conformance harness.
//!
//! The executable definition of the extension grammar. Each case maps an input
//! document to substrings its component-aware render must contain, or marks it as
//! an input the parser must reject. Together with the CommonMark suite (which
//! guarantees the superset property), this pins the behaviour `docs/spec.md`
//! describes. The fixtures live at `spec/altmd/cases.json`.

use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    input: String,
    #[serde(default)]
    contains: Vec<String>,
    #[serde(default)]
    lacks: Vec<String>,
    #[serde(default)]
    error: bool,
}

fn cases_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/altmd/cases.json")
}

#[test]
fn altmd_extension_conformance() {
    let path = cases_path();
    let data = std::fs::read_to_string(&path).expect("read spec/altmd/cases.json");
    let cases: Vec<Case> = serde_json::from_str(&data).expect("parse cases.json");
    assert!(!cases.is_empty(), "spec/altmd/cases.json had no cases");

    for case in &cases {
        let rendered = altmd_core::render(&case.input);
        if case.error {
            assert!(
                rendered.is_err(),
                "case {}: expected a parse error, got output",
                case.name
            );
            continue;
        }
        assert!(
            rendered.is_ok(),
            "case {}: render failed: {:?}",
            case.name,
            rendered.as_ref().err()
        );
        let html = rendered.unwrap_or_default();
        for needle in &case.contains {
            assert!(
                html.contains(needle.as_str()),
                "case {}: expected {needle:?} in:\n{html}",
                case.name
            );
        }
        for needle in &case.lacks {
            assert!(
                !html.contains(needle.as_str()),
                "case {}: did not expect {needle:?} in:\n{html}",
                case.name
            );
        }
    }
    eprintln!("alt-markdown extension conformance: {} cases", cases.len());
}
