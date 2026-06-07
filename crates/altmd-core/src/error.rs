//! Error types for the core facade.

use thiserror::Error;

/// Errors surfaced by the alt-markdown core.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// Parsing failed.
    #[error(transparent)]
    Parse(#[from] altmd_parser::ParseError),
}
