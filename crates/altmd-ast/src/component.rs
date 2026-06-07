//! Component nodes and their attributes: the hybrid-grammar extensions.
//!
//! A [`Component`] is the AST representation of a standard-library component,
//! whether written as a colon directive (block children) or a fenced block (a
//! raw data or diagram payload). The renderer maps each component name to its
//! Web Component and its mandatory static fallback.

use serde::{Deserialize, Serialize};

/// Attributes shared across all three component syntaxes, written as
/// `{#id .class key=value}` (the braces are optional in fence info strings).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attrs {
    /// The `#id` attribute, if present.
    pub id: Option<String>,
    /// The `.class` attributes, in source order.
    pub classes: Vec<String>,
    /// The `key=value` pairs, in source order.
    pub pairs: Vec<(String, String)>,
}

impl Attrs {
    /// Look up the first value for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A standard-library component node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// The component name, for example `callout` or `chart`.
    pub name: String,
    /// The component's attributes.
    pub attrs: Attrs,
    /// The component's body.
    pub body: ComponentBody,
}

/// The body of a component: block children (directive) or a raw payload (fence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ComponentBody {
    /// Block-level children, from a container directive.
    Children(Vec<crate::Block>),
    /// A raw text payload, from a fenced component (chart data, diagram source).
    Raw(String),
}
