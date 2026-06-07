//! Error types for parsing.

use thiserror::Error;

/// Errors that can arise while parsing alt-markdown source.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The parser could not construct a valid AST.
    #[error("AST construction failed: {0}")]
    Ast(#[from] altmd_ast::AstError),
}
