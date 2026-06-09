//! The `[alt.ai]` edit-permission convention (validation spike).
//!
//! A document can carry an `:::ai-policy` block declaring which of its sections an
//! AI agent may edit autonomously. This module parses that block into a typed,
//! deterministic permission map and a `can_edit` guard. It does **not** make any
//! model behave: it is the parseable half of the convention. The enforcing half
//! lives in the consuming host (an editing tool, a hook), which reads this map and
//! refuses writes to protected sections. Whether a model voluntarily respects an
//! unenforced policy is an empirical question, measured separately.
//!
//! Policy keys are heading slugs, the same anchors the renderer assigns, so a
//! policy entry governs that heading's section. A key is slugified on parse, so a
//! policy may name a section by its heading text or its slug interchangeably.

use std::collections::BTreeMap;

use altmd_ast::{Block, ComponentBody, Document, Inline};

use crate::render::slugify;

/// What an AI agent may do to a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// The section must not be changed.
    ReadOnly,
    /// The section may be edited freely.
    Editable,
    /// Content may be added to the section but existing content not changed.
    AppendOnly,
}

impl Permission {
    /// Parse a permission word. Unrecognised words are treated as read-only, the
    /// safe default, so a typo locks rather than unlocks a section.
    fn parse(word: &str) -> Self {
        match word.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "editable" | "edit" | "rw" | "write" => Self::Editable,
            "append-only" | "append" => Self::AppendOnly,
            _ => Self::ReadOnly,
        }
    }

    /// Whether an agent may write to a section with this permission at all
    /// (editable or append-only). Read-only sections return false.
    #[must_use]
    pub fn is_writable(self) -> bool {
        matches!(self, Self::Editable | Self::AppendOnly)
    }
}

/// A parsed `[alt.ai]` edit policy: the models it addresses, the default for any
/// section it does not name, and the per-section permission map (keyed by slug).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AiPolicy {
    /// Which models the policy addresses, verbatim from the `model` attribute
    /// (for example "any"). Informational; the host decides how to apply it.
    pub model: Option<String>,
    /// The permission for any section the policy does not explicitly name.
    pub default: Permission,
    /// Section slug to permission. A `BTreeMap` so the serialised output is
    /// deterministic (sorted), which keeps two runs byte-identical.
    pub sections: BTreeMap<String, Permission>,
}

impl AiPolicy {
    /// The permission governing the section with anchor `slug`.
    #[must_use]
    pub fn permission(&self, slug: &str) -> Permission {
        self.sections.get(slug).copied().unwrap_or(self.default)
    }

    /// Whether an agent may write to the section with anchor `slug`.
    #[must_use]
    pub fn can_edit(&self, slug: &str) -> bool {
        self.permission(slug).is_writable()
    }
}

/// Extract the first `:::ai-policy` block from a document into a typed policy.
/// Returns `None` when the document carries no policy.
#[must_use]
pub fn extract_policy(document: &Document) -> Option<AiPolicy> {
    let component = find_policy_block(&document.blocks)?;
    let model = component.attrs.get("model").map(str::to_owned);
    let default = component
        .attrs
        .get("default")
        .map_or(Permission::Editable, Permission::parse);
    let mut sections = BTreeMap::new();
    if let ComponentBody::Children(blocks) = &component.body {
        collect_entries(blocks, &mut sections);
    }
    Some(AiPolicy {
        model,
        default,
        sections,
    })
}

/// Find the first `ai-policy` component anywhere in the block tree.
fn find_policy_block(blocks: &[Block]) -> Option<&altmd_ast::Component> {
    for block in blocks {
        match block {
            Block::Component(component) if component.name == "ai-policy" => return Some(component),
            Block::Component(component) => {
                if let ComponentBody::Children(children) = &component.body {
                    if let Some(found) = find_policy_block(children) {
                        return Some(found);
                    }
                }
            }
            Block::BlockQuote(children) => {
                if let Some(found) = find_policy_block(children) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Read `slug: permission` entries from a policy body. Entries are list items or
/// plain lines of the form `section: permission`; the section is slugified so it
/// matches the heading's anchor whether written as text or as a slug.
fn collect_entries(blocks: &[Block], into: &mut BTreeMap<String, Permission>) {
    for block in blocks {
        match block {
            Block::List(list) => {
                for item in &list.items {
                    collect_entries(&item.blocks, into);
                }
            }
            Block::Paragraph(inlines) => {
                for line in inline_text(inlines).lines() {
                    if let Some((section, permission)) = line.split_once(':') {
                        let slug = slugify(section.trim());
                        if !slug.is_empty() {
                            into.insert(slug, Permission::parse(permission));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Flatten an inline sequence to plain text (policy entries are plain text).
fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) => text.push_str(value),
            Inline::Emphasis(content) | Inline::Strong(content) => {
                text.push_str(&inline_text(content));
            }
            Inline::SoftBreak | Inline::HardBreak => text.push('\n'),
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{Permission, extract_policy};
    use altmd_ast::Parser;
    use altmd_parser::CommonMarkParser;

    fn policy(source: &str) -> super::AiPolicy {
        let doc = CommonMarkParser::new().parse(source).expect("parse");
        extract_policy(&doc).expect("a policy")
    }

    #[test]
    fn parses_sections_and_default() {
        let p = policy(
            ":::ai-policy{model=any}\n- Introduction: read-only\n- Draft notes: editable\n- Changelog: append-only\n:::",
        );
        assert_eq!(p.model.as_deref(), Some("any"));
        // Default is editable: a section the policy does not name may be edited.
        assert!(p.can_edit("anything-unlisted"));
        // Keys are slugified, so heading text resolves to the heading's anchor.
        assert_eq!(p.permission("introduction"), Permission::ReadOnly);
        assert_eq!(p.permission("draft-notes"), Permission::Editable);
        assert_eq!(p.permission("changelog"), Permission::AppendOnly);
        assert!(!p.can_edit("introduction"));
        assert!(p.can_edit("draft-notes"));
        assert!(p.can_edit("changelog"));
    }

    #[test]
    fn unknown_permission_word_locks_the_section() {
        // A typo must fail safe: lock the section, not unlock it.
        let p = policy(":::ai-policy\n- Secrets: editaable\n:::");
        assert_eq!(p.permission("secrets"), Permission::ReadOnly);
    }

    #[test]
    fn explicit_default_applies_to_unlisted_sections() {
        let p = policy(":::ai-policy{default=read-only}\n- Notes: editable\n:::");
        assert!(!p.can_edit("anything-else"));
        assert!(p.can_edit("notes"));
    }

    #[test]
    fn no_policy_block_yields_none() {
        let doc = CommonMarkParser::new().parse("# Just a heading\n").expect("parse");
        assert!(super::extract_policy(&doc).is_none());
    }
}
