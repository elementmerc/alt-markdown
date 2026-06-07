//! Error types for AST construction and traversal.

use thiserror::Error;

/// Errors that can arise when building or walking an alt-markdown AST.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AstError {
    /// A parser produced a node that violates a structural invariant.
    #[error("malformed AST node: {0}")]
    Malformed(String),
}
