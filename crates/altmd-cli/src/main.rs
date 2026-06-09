//! alt-markdown command-line tools.

mod include;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

/// alt-markdown command-line tools.
#[derive(Parser)]
#[command(name = "altmd", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render an alt-markdown file to HTML (written to stdout).
    Render {
        /// Path to the .alt (or .md) source file.
        file: PathBuf,
        /// Emit pure CommonMark HTML (spec-conformance path) instead of the
        /// component-aware render. Raw HTML is passed through unsanitised, so
        /// this is for conformance measurement, not untrusted input.
        #[arg(long)]
        commonmark: bool,
        /// Emit a single self-contained HTML document: the rendered content, an
        /// inlined default theme, and a small script that enhances the
        /// components. The file reads correctly with no runtime; the rich
        /// graphics light up when the runtime is reachable.
        #[arg(long)]
        standalone: bool,
        /// URL of the runtime module that enhances components in a standalone
        /// file. Defaults to the published CDN build.
        #[arg(long, default_value = DEFAULT_RUNTIME_URL)]
        runtime_url: String,
    },
    /// Print the parsed AST as JSON (written to stdout).
    Ast {
        /// Path to the source file.
        file: PathBuf,
    },
    /// Normalise a document: parse it and serialise it back to canonical source.
    Fmt {
        /// Path to the source file.
        file: PathBuf,
    },
    /// Validate a document and report any parse error with a diagnostic.
    Check {
        /// Path to the source file.
        file: PathBuf,
    },
}

fn read(file: &Path) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))
}

/// The default theme, embedded so a standalone export needs no external CSS.
const DEFAULT_CSS: &str = include_str!("../assets/default.css");

/// Pinned CDN locations for the runtime and the heavy graphics libraries a
/// standalone document enhances with. Versions match the runtime's peers.
const DEFAULT_RUNTIME_URL: &str = "https://cdn.jsdelivr.net/npm/@altmd/runtime@0.1.0/dist/index.js";
const UPLOT_URL: &str = "https://cdn.jsdelivr.net/npm/uplot@1.6.31/dist/uPlot.esm.js";
const KATEX_URL: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.mjs";
const MERMAID_URL: &str = "https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.esm.min.mjs";
const UPLOT_CSS: &str = "https://cdn.jsdelivr.net/npm/uplot@1.6.31/dist/uPlot.min.css";
const KATEX_CSS: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css";

/// Derive a document title from the first level-1 heading, falling back to a
/// generic name. Cosmetic only (the `<title>` and tab label).
fn document_title(source: &str) -> String {
    for line in source.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("# ") {
            let title = rest.trim().trim_end_matches('#').trim();
            if !title.is_empty() {
                return title.to_owned();
            }
        }
    }
    "alt-markdown document".to_owned()
}

fn escape_title(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Wrap rendered content in a single self-contained HTML document. The body
/// reads correctly with no script (every component ships a static fallback); the
/// trailing module script upgrades the components when the runtime is reachable.
fn build_standalone(title: &str, body_html: &str, runtime_url: &str) -> String {
    let title = escape_title(title);
    format!(
        "<!doctype html>\n\
<html lang=\"en\">\n\
<head>\n\
<meta charset=\"utf-8\" />\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
<title>{title}</title>\n\
<link rel=\"stylesheet\" href=\"{UPLOT_CSS}\" />\n\
<link rel=\"stylesheet\" href=\"{KATEX_CSS}\" />\n\
<style>\n{DEFAULT_CSS}</style>\n\
<script type=\"importmap\">\n\
{{\n  \"imports\": {{\n    \"uplot\": \"{UPLOT_URL}\",\n    \"katex\": \"{KATEX_URL}\",\n    \"mermaid\": \"{MERMAID_URL}\"\n  }}\n}}\n\
</script>\n\
</head>\n\
<body>\n\
<main class=\"altmd-doc\">\n{body_html}</main>\n\
<script type=\"module\">\n\
// The document above reads correctly with no script. This enhances the\n\
// components (charts, maths, diagrams) once the runtime is reachable.\n\
import {{ bootstrap }} from \"{runtime_url}\";\n\
bootstrap();\n\
</script>\n\
</body>\n\
</html>\n"
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render {
            file,
            commonmark,
            standalone,
            runtime_url,
        } => {
            let source = read(&file)?;
            if commonmark && standalone {
                anyhow::bail!("--commonmark and --standalone cannot be combined");
            }
            if standalone
                && (runtime_url.contains('"') || runtime_url.contains(char::is_whitespace))
            {
                anyhow::bail!("--runtime-url must be a plain URL with no quotes or whitespace");
            }
            let html = if commonmark {
                altmd_core::to_commonmark_html(&source)
            } else {
                let document = altmd_core::parse(&source)
                    .with_context(|| format!("parsing {}", file.display()))?;
                let resolved = include::resolve_document(document, &file)
                    .with_context(|| format!("resolving includes in {}", file.display()))?;
                altmd_core::render_document(&resolved)
            };
            if standalone {
                print!(
                    "{}",
                    build_standalone(&document_title(&source), &html, &runtime_url)
                );
            } else {
                print!("{html}");
            }
            Ok(())
        }
        Command::Ast { file } => {
            let source = read(&file)?;
            let document = altmd_core::parse(&source)
                .with_context(|| format!("parsing {}", file.display()))?;
            let json =
                serde_json::to_string_pretty(&document).context("serialising the AST to JSON")?;
            println!("{json}");
            Ok(())
        }
        Command::Fmt { file } => {
            let source = read(&file)?;
            let normalised = altmd_core::normalise(&source)
                .with_context(|| format!("normalising {}", file.display()))?;
            print!("{normalised}");
            Ok(())
        }
        Command::Check { file } => {
            let source = read(&file)?;
            let document = match altmd_core::parse(&source) {
                Ok(document) => document,
                Err(error) => anyhow::bail!("invalid document {}: {error}", file.display()),
            };
            // Validate the include graph as well: missing files, traversal out of
            // the document directory, and cycles are all reported here.
            match include::resolve_document(document, &file) {
                Ok(resolved) => {
                    println!("ok: {} top-level blocks", resolved.blocks.len());
                    Ok(())
                }
                Err(error) => anyhow::bail!("invalid include in {}: {error:#}", file.display()),
            }
        }
    }
}
