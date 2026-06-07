//! alt-markdown command-line tools.

use std::fs;
use std::path::PathBuf;

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
    },
    /// Print the parsed AST as JSON. Lands in Phase 1.
    Ast {
        /// Path to the source file.
        file: PathBuf,
    },
    /// Validate a document (unknown directives, missing fallbacks). Lands in Phase 6.
    Check {
        /// Path to the source file.
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render { file } => {
            let source =
                fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            print!("{}", altmd_core::to_html(&source));
            Ok(())
        }
        Command::Ast { .. } | Command::Check { .. } => {
            anyhow::bail!(
                "not yet implemented in v0.1 Phase 0; see private/plans/v0.1-plan.md for the phase map"
            )
        }
    }
}
