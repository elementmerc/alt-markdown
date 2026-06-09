//! Resolve `:::include` directives by splicing the referenced files in place.
//!
//! This is native only: it needs filesystem access, so it runs in the CLI, not
//! in the WebAssembly core. The browser path leaves an include as a component
//! that renders a link to the source instead.
//!
//! Every include is held to three hard limits so a document cannot be turned into
//! a tool for reading arbitrary files or for looping forever:
//!
//! - **Confinement.** A `src` is resolved relative to the including file and must
//!   canonicalise to a path inside the jail (the top-level document's directory).
//!   `..` traversal, absolute paths, and symlinks that point outside the jail are
//!   all rejected, because canonicalisation resolves them before the check.
//! - **No cycles.** The canonical paths currently being resolved are tracked; a
//!   file that includes itself, directly or transitively, is an error, not a loop.
//! - **Bounded depth.** Include nesting is capped.
//!
//! Any violation fails loud with the offending path, never a silent empty splice.

use std::fs;
use std::path::{Path, PathBuf};

use altmd_core::{Block, ComponentBody, Document};
use anyhow::{Context, Result, bail};

/// Maximum include nesting depth. Real documents nest a handful deep; this sits
/// far above any plausible structure and bounds the work a hostile include graph
/// can demand.
const MAX_INCLUDE_DEPTH: usize = 16;

/// Resolve every `:::include` in `document`, reading files relative to `file`'s
/// directory and confining them to it. `file` is the path the document was read
/// from.
pub fn resolve_document(document: Document, file: &Path) -> Result<Document> {
    let canon_file = file
        .canonicalize()
        .with_context(|| format!("resolving {}", file.display()))?;
    let jail = canon_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    // Seed the stack with the top-level file so a document that includes itself
    // is caught as a cycle.
    let mut stack = vec![canon_file];
    let blocks = resolve(document.blocks, &jail, &jail, &mut stack, 0)?;
    Ok(Document { blocks })
}

/// Resolve includes in a block sequence. `current_dir` is the directory relative
/// includes resolve against; `jail` is the root no include may escape; `stack`
/// holds the canonical paths currently being resolved; `depth` bounds nesting.
fn resolve(
    blocks: Vec<Block>,
    current_dir: &Path,
    jail: &Path,
    stack: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<Vec<Block>> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            Block::Component(component) if component.name == "include" => {
                let src = component
                    .attrs
                    .get("src")
                    .context("an :::include needs a src attribute")?;
                out.extend(resolve_one(src, current_dir, jail, stack, depth)?);
            }
            Block::Component(mut component) => {
                if let ComponentBody::Children(children) = &mut component.body {
                    let taken = std::mem::take(children);
                    *children = resolve(taken, current_dir, jail, stack, depth)?;
                }
                out.push(Block::Component(component));
            }
            Block::BlockQuote(children) => {
                out.push(Block::BlockQuote(resolve(
                    children,
                    current_dir,
                    jail,
                    stack,
                    depth,
                )?));
            }
            Block::List(mut list) => {
                for item in &mut list.items {
                    let taken = std::mem::take(&mut item.blocks);
                    item.blocks = resolve(taken, current_dir, jail, stack, depth)?;
                }
                out.push(Block::List(list));
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

/// Read, validate, parse, and recursively resolve a single included file,
/// returning its blocks to splice in place of the include directive.
fn resolve_one(
    src: &str,
    current_dir: &Path,
    jail: &Path,
    stack: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<Vec<Block>> {
    if depth >= MAX_INCLUDE_DEPTH {
        bail!("include nesting exceeds the maximum depth of {MAX_INCLUDE_DEPTH}: {src}");
    }
    if Path::new(src).is_absolute() {
        bail!("include path must be relative, not absolute: {src}");
    }
    let canon = current_dir
        .join(src)
        .canonicalize()
        .with_context(|| format!("included file not found: {src}"))?;
    if !canon.starts_with(jail) {
        bail!(
            "include escapes the document directory ({}): {src}",
            jail.display()
        );
    }
    if stack.contains(&canon) {
        bail!("include cycle detected at {}", canon.display());
    }
    let text = fs::read_to_string(&canon)
        .with_context(|| format!("reading included file {}", canon.display()))?;
    let document =
        altmd_core::parse(&text).with_context(|| format!("parsing {}", canon.display()))?;
    let child_dir = canon.parent().unwrap_or(jail).to_path_buf();
    stack.push(canon);
    let resolved = resolve(document.blocks, &child_dir, jail, stack, depth + 1)?;
    stack.pop();
    Ok(resolved)
}
