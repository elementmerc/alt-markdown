//! alt-markdown command-line tools.

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render { file, commonmark } => {
            let source = read(&file)?;
            let html = if commonmark {
                altmd_core::to_commonmark_html(&source)
            } else {
                altmd_core::render(&source)
                    .with_context(|| format!("rendering {}", file.display()))?
            };
            print!("{html}");
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
            match altmd_core::parse(&source) {
                Ok(document) => {
                    println!("ok: {} top-level blocks", document.blocks.len());
                    Ok(())
                }
                Err(error) => {
                    anyhow::bail!("invalid document {}: {error}", file.display())
                }
            }
        }
    }
}
