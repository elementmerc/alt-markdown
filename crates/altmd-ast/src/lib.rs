//! alt-markdown abstract syntax tree and the parser/serializer trait boundary.
//!
//! This crate defines the data model that every other layer depends on, so the
//! concrete parser (comrak today, a bespoke engine later) can be swapped without
//! touching the components or the converter. The node set is intentionally small
//! in this scaffold and is fleshed out in Phase 1.

pub mod error;

pub use error::AstError;

/// A byte range into the original source, carried on every node so a future
/// lossless concrete-syntax-tree and byte-identical round-trip can land without
/// reshaping the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// A parsed alt-markdown document: a sequence of block-level nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Document {
    /// Top-level block nodes in document order.
    pub blocks: Vec<Block>,
}

/// Block-level content. Extended in Phase 1; component nodes arrive in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Block {
    /// A paragraph of inline content.
    Paragraph(Vec<Inline>),
    /// A heading, level 1 to 6.
    Heading {
        /// Heading level, 1 to 6.
        level: u8,
        /// Inline content of the heading.
        content: Vec<Inline>,
    },
}

/// Inline-level content. Extended in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Inline {
    /// Literal text.
    Text(String),
}

/// Produces an alt-markdown [`Document`] from source text.
///
/// This is the trait boundary: callers depend on this, not on the concrete
/// parser implementation.
pub trait Parser {
    /// Parse `source` into a [`Document`].
    ///
    /// # Errors
    /// Returns [`AstError`] if the source cannot be represented as a valid AST.
    fn parse(&self, source: &str) -> Result<Document, AstError>;
}

/// Serialises a [`Document`] back to alt-markdown source text.
pub trait Serializer {
    /// Render `document` to source text. Normalising in v0.1.
    fn to_source(&self, document: &Document) -> String;
}
