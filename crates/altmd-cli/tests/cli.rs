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
fn render_standalone_is_self_contained() -> TestResult {
    let file = temp(
        "standalone",
        "# My Spec\n\n```chart kind=bar\nx,y\na,1\n```\n\n<script>alert(1)</script>\n",
    )?;
    let out = bin().args(["render", "--standalone"]).arg(&file).output()?;
    assert!(out.status.success());
    let html = String::from_utf8_lossy(&out.stdout);
    // A complete document with the theme and enhancement script inlined.
    assert!(html.starts_with("<!doctype html>"), "{html}");
    assert!(html.contains("<style>"), "theme not inlined: {html}");
    assert!(html.contains("alt-markdown default theme"), "{html}");
    assert!(
        html.contains("<title>My Spec</title>"),
        "title not derived: {html}"
    );
    assert!(
        html.contains("import { bootstrap }"),
        "enhancer missing: {html}"
    );
    assert!(
        html.contains("type=\"importmap\""),
        "import map missing: {html}"
    );
    // The content and its static fallback are present.
    assert!(html.contains("<alt-chart"), "{html}");
    assert!(
        html.contains("<th>x</th>"),
        "chart fallback table missing: {html}"
    );
    // A hostile payload in the source never becomes a live script.
    assert!(!html.contains("<script>alert"), "payload leaked: {html}");
    Ok(())
}

#[test]
fn render_standalone_rejects_commonmark_combo() -> TestResult {
    let file = temp("combo", "# x\n")?;
    let out = bin()
        .args(["render", "--standalone", "--commonmark"])
        .arg(&file)
        .output()?;
    assert!(!out.status.success(), "combining the two flags must fail");
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
