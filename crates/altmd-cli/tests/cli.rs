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

/// Create a fresh temp directory and write each `(relative path, contents)` into
/// it (creating subdirectories as needed). Returns the directory path.
fn temp_tree(name: &str, files: &[(&str, &str)]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("altmd-inc-{name}"));
    let _ = fs::remove_dir_all(&dir);
    for (relative, contents) in files {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    Ok(dir)
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
fn include_splices_the_referenced_file() -> TestResult {
    let dir = temp_tree(
        "ok",
        &[
            ("book.alt", "# Book\n\n:::include{src=\"ch1.alt\"}\n:::\n"),
            ("ch1.alt", "## Chapter One\n\nHello from the chapter.\n"),
        ],
    )?;
    let out = bin().arg("render").arg(dir.join("book.alt")).output()?;
    assert!(out.status.success());
    let html = String::from_utf8_lossy(&out.stdout);
    assert!(html.contains("Chapter One"), "include not spliced: {html}");
    assert!(html.contains("Hello from the chapter"), "{html}");
    Ok(())
}

#[test]
fn include_resolves_from_a_subdirectory() -> TestResult {
    // A relative include inside a subdirectory resolves against that
    // subdirectory, and the result stays inside the document's directory.
    let dir = temp_tree(
        "subdir",
        &[
            (
                "book.alt",
                "# Book\n\n:::include{src=\"parts/intro.alt\"}\n:::\n",
            ),
            ("parts/intro.alt", "Intro text.\n"),
        ],
    )?;
    let out = bin().arg("render").arg(dir.join("book.alt")).output()?;
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Intro text"));
    Ok(())
}

#[test]
fn include_rejects_path_traversal() -> TestResult {
    // A real file that exists but sits outside the document's directory: the
    // document is in project/, the secret is its sibling. `..` reaches a file
    // that exists on every platform, so the rejection comes from the jail check,
    // not from the file simply being absent.
    let dir = temp_tree(
        "traversal",
        &[
            (
                "project/doc.alt",
                "# X\n\n:::include{src=\"../secret.txt\"}\n:::\n",
            ),
            ("secret.txt", "TOP SECRET, outside the project\n"),
        ],
    )?;
    let out = bin()
        .arg("render")
        .arg(dir.join("project/doc.alt"))
        .output()?;
    assert!(
        !out.status.success(),
        "escaping the document directory must fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("escapes the document directory"),
        "{stderr}"
    );
    // The escaped file's contents must never reach the output.
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("TOP SECRET"),
        "the escaped file leaked into the output"
    );
    Ok(())
}

#[test]
fn include_rejects_an_absolute_path() -> TestResult {
    // Build a path that is genuinely absolute on the running platform (a leading
    // slash is not absolute on Windows, which needs a drive prefix), so the
    // absolute-path guard is what rejects it.
    let dir = std::env::temp_dir().join("altmd-inc-absolute");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    let target = std::env::temp_dir().join("altmd-abs-target.alt");
    let doc = dir.join("doc.alt");
    fs::write(
        &doc,
        format!("# X\n\n:::include{{src=\"{}\"}}\n:::\n", target.display()),
    )?;
    let out = bin().arg("render").arg(&doc).output()?;
    assert!(!out.status.success(), "absolute path must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("must be relative"), "{stderr}");
    Ok(())
}

#[test]
fn include_reports_a_missing_file() -> TestResult {
    let dir = temp_tree(
        "missing",
        &[("doc.alt", "# X\n\n:::include{src=\"nope.alt\"}\n:::\n")],
    )?;
    let out = bin().arg("render").arg(dir.join("doc.alt")).output()?;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not found"));
    Ok(())
}

#[test]
fn include_detects_a_cycle() -> TestResult {
    let dir = temp_tree(
        "cycle",
        &[
            ("a.alt", "# A\n\n:::include{src=\"b.alt\"}\n:::\n"),
            ("b.alt", "# B\n\n:::include{src=\"a.alt\"}\n:::\n"),
        ],
    )?;
    let out = bin().arg("render").arg(dir.join("a.alt")).output()?;
    assert!(!out.status.success(), "an include cycle must fail");
    assert!(String::from_utf8_lossy(&out.stderr).contains("cycle"));
    Ok(())
}

#[test]
fn policy_prints_json_and_guards_sections() -> TestResult {
    let file = temp(
        "policy",
        ":::ai-policy{model=any}\n- Introduction: read-only\n- Draft: editable\n:::\n\n# Introduction\n\nx\n",
    )?;

    // The whole policy as JSON.
    let json = bin().arg("policy").arg(&file).output()?;
    assert!(json.status.success());
    let text = String::from_utf8_lossy(&json.stdout);
    assert!(text.contains("\"introduction\": \"read-only\""), "{text}");

    // A read-only section exits non-zero (the guard a host calls).
    let locked = bin()
        .args(["policy", "--section", "Introduction"])
        .arg(&file)
        .output()?;
    assert!(!locked.status.success(), "read-only section must fail");
    assert!(String::from_utf8_lossy(&locked.stdout).contains("read-only"));

    // An editable section exits zero.
    let open = bin()
        .args(["policy", "--section", "Draft"])
        .arg(&file)
        .output()?;
    assert!(open.status.success(), "editable section must pass");
    assert!(String::from_utf8_lossy(&open.stdout).contains("editable"));
    Ok(())
}

#[test]
fn spec_version_prints_the_version() -> TestResult {
    let out = bin().arg("--spec-version").output()?;
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim(), "1.0", "{text}");
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
