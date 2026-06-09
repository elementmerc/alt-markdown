//! alt-markdown abstract syntax tree and the parser/serializer trait boundary.
//!
//! This crate defines the data model that every other layer depends on, so the
//! concrete parser (comrak today, a bespoke engine later) can be swapped without
//! touching the components or the converter. Phase 1 covers the CommonMark node
//! set; component nodes for the hybrid grammar arrive in Phase 2.

pub mod component;
pub mod error;

pub use component::{Attrs, Component, ComponentBody};
pub use error::AstError;

/// A byte range into the original source, available for a future lossless
/// concrete-syntax-tree and byte-identical round-trip. Not yet populated on
/// every node; reserved so adding it later does not reshape the AST.
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

/// Block-level content. Component nodes arrive in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Block {
    /// A heading, level 1 to 6, with inline content.
    Heading {
        /// Heading level, 1 to 6.
        level: u8,
        /// Inline content of the heading.
        content: Vec<Inline>,
    },
    /// A paragraph of inline content.
    Paragraph(Vec<Inline>),
    /// A block quote containing block-level children.
    BlockQuote(Vec<Block>),
    /// An ordered or unordered list.
    List(List),
    /// A code block with an info string and literal contents.
    CodeBlock {
        /// The fenced info string (may be empty), for example `rust`.
        info: String,
        /// The literal code contents, including the trailing newline.
        literal: String,
    },
    /// A thematic break (horizontal rule).
    ThematicBreak,
    /// A GFM pipe table.
    Table(Table),
    /// A GFM footnote definition, referenced by [`Inline::FootnoteReference`].
    FootnoteDefinition {
        /// The footnote label (without the `^`).
        name: String,
        /// The block content of the footnote.
        blocks: Vec<Block>,
    },
    /// A raw HTML block. Sanitised before rendering (Phase 3).
    HtmlBlock(String),
    /// A standard-library component (hybrid-grammar extension).
    Component(Component),
}

/// An ordered or unordered list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct List {
    /// Whether the list is ordered (numbered) rather than bulleted.
    pub ordered: bool,
    /// The starting number for an ordered list (1 for unordered).
    pub start: u32,
    /// Whether the list is tight: a tight list renders its items' paragraphs
    /// without enclosing `<p>` tags, the way CommonMark specifies. A loose list
    /// (items separated by blank lines) keeps the paragraphs.
    pub tight: bool,
    /// The list items.
    pub items: Vec<ListItem>,
}

/// A single list item. Carries optional GFM task-list state so the renderer can
/// emit a checkbox and a future editor can toggle it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ListItem {
    /// `None` for an ordinary item; `Some(checked)` for a GFM task-list item.
    pub task: Option<bool>,
    /// The block-level content of the item.
    pub blocks: Vec<Block>,
}

impl ListItem {
    /// An ordinary (non-task) list item wrapping the given blocks.
    #[must_use]
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { task: None, blocks }
    }
}

/// A GFM pipe table: a header row, body rows, and a per-column alignment.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Table {
    /// Per-column alignment, in column order.
    pub alignments: Vec<Alignment>,
    /// The header row: one cell of inline content per column.
    pub header: Vec<Vec<Inline>>,
    /// The body rows: each a list of cells of inline content.
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// Column alignment for a [`Table`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Alignment {
    /// No explicit alignment.
    None,
    /// Left-aligned.
    Left,
    /// Centre-aligned.
    Center,
    /// Right-aligned.
    Right,
}

/// Inline-level content. Component nodes arrive in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Inline {
    /// Literal text.
    Text(String),
    /// Emphasised (italic) content.
    Emphasis(Vec<Inline>),
    /// Strong (bold) content.
    Strong(Vec<Inline>),
    /// Inline code.
    Code(String),
    /// GFM strikethrough (`~~text~~`).
    Strikethrough(Vec<Inline>),
    /// A GFM footnote reference, resolved against a [`Block::FootnoteDefinition`].
    FootnoteReference {
        /// The footnote label (without the `^`).
        name: String,
    },
    /// A cross-reference to a labelled element elsewhere in the document, written
    /// `[#label]`. The renderer resolves it to a link whose text is the target's
    /// auto-numbered name (for example "Figure 3") or, for a section, its heading
    /// text. An unresolved target renders as the literal text, never dropped.
    CrossRef {
        /// The target label (the text between `[#` and `]`).
        target: String,
    },
    /// A citation to a bibliography entry, written `[@key]`. The renderer
    /// resolves it to a numbered link (`[1]`) into the reference list when a
    /// matching entry exists; an unresolved key renders as the literal text.
    Citation {
        /// The citation key (the text between `[@` and `]`).
        key: String,
    },
    /// A hyperlink.
    Link {
        /// The destination URL.
        url: String,
        /// The optional title (may be empty).
        title: String,
        /// The inline content of the link text.
        content: Vec<Inline>,
    },
    /// An image.
    Image {
        /// The image source URL.
        url: String,
        /// The optional title (may be empty).
        title: String,
        /// The alt-text content.
        alt: Vec<Inline>,
    },
    /// A soft line break (rendered as a space or newline).
    SoftBreak,
    /// A hard line break.
    HardBreak,
    /// Raw inline HTML. Sanitised before rendering (Phase 3).
    HtmlInline(String),
}

/// Produces an alt-markdown [`Document`] from source text.
///
/// This is the trait boundary: callers depend on this, not on the concrete
/// parser implementation.
pub trait Parser {
    /// Parse `source` into a [`Document`].
    ///
    /// # Errors
    /// Returns [`AstError`] if the source contains a construct that cannot be
    /// represented in the current AST.
    fn parse(&self, source: &str) -> Result<Document, AstError>;
}

/// Serialises a [`Document`] back to alt-markdown source text.
pub trait Serializer {
    /// Render `document` to source text. Normalising in v0.1.
    fn to_source(&self, document: &Document) -> String;
}
