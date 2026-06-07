//! Error types for sanitisation.

use thiserror::Error;

/// Errors that can arise during sanitisation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SanitizeError {
    /// Input could not be parsed as HTML for sanitisation.
    #[error("could not sanitise input: {0}")]
    Invalid(String),
}
