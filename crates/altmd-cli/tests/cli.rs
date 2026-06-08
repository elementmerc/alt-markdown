//! Integration tests for the `altmd` binary, exercising each subcommand against
//! a temporary file via the path Cargo provides as `CARGO_BIN_EXE_altmd`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_altmd"))
}

/// Write `source` to a uniquely named temp file and return its path.
fn temp(name: &str, source: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("altmd-cli-test-{name}.alt"));
    fs::write(&path, source)?;
    Ok(path)
}

#[test]
fn render_emits_component_html() -> TestResult {
    let file = temp("render", ":::callout{type=warning}\nhi\n:::\n")?;
    let out = bin().arg("render").arg(&file).output()?;
    assert!(out.status.success());
    let html = String::from_utf8_lossy(&out.stdout);
    assert!(html.contains("<alt-callout"), "{html}");
    Ok(())
}

#[test]
fn render_commonmark_passes_raw_html_through() -> TestResult {
    let file = temp("cm", "# H\n\n<div>raw</div>\n")?;
    let out = bin().args(["render", "--commonmark"]).arg(&file).output()?;
    assert!(out.status.success());
    let html = String::from_utf8_lossy(&out.stdout);
    assert!(html.contains("<div>raw</div>"), "{html}");
    Ok(())
}

#[test]
fn fmt_round_trips_through_the_parser() -> TestResult {
    let file = temp("fmt", "#    Title\n\n\n- a\n-  b\n")?;
    let out = bin().arg("fmt").arg(&file).output()?;
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("# Title"), "{text}");
    assert!(text.contains("- a\n- b"), "{text}");
    Ok(())
}

#[test]
fn ast_emits_json() -> TestResult {
    let file = temp("ast", "# Title\n")?;
    let out = bin().arg("ast").arg(&file).output()?;
    assert!(out.status.success());
    let json = String::from_utf8_lossy(&out.stdout);
    assert!(json.contains("\"Heading\""), "{json}");
    Ok(())
}

#[test]
fn check_reports_ok_and_errors() -> TestResult {
    let good = temp("check-ok", "# fine\n")?;
    let ok = bin().arg("check").arg(&good).output()?;
    assert!(ok.status.success());

    let bad = temp("check-bad", ":::bogus\nx\n:::\n")?;
    let err = bin().arg("check").arg(&bad).output()?;
    assert!(!err.status.success(), "unknown directive must fail check");
    let stderr = String::from_utf8_lossy(&err.stderr);
    assert!(stderr.contains("unknown directive"), "{stderr}");
    Ok(())
}
