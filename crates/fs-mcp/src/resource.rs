//! A resource template: events and files are read as resources, not called as tools (ADR 0021 extension).
//!
//! Pattern: none (plain data). Trait: none. Only the list exists in stage 1; there is no server behind it.

use crate::{RiskClass, Target};
use serde_json::{Value, json};

use crate::tool::META_KEY;

/// What a resource template stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    /// A change feed behind a `subscribe` method.
    Events,
    /// Bytes behind a capability (`Blob`) or a blob store.
    Blob,
}

/// A template of resource URIs.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceTemplate {
    /// RFC 6570 template.
    pub uri_template: String,
    /// Name, derived from the source.
    pub name: String,
    /// The schema comment.
    pub description: String,
    /// Events or bytes.
    pub kind: ResourceKind,
    /// Class from the marks. Reading still asks (ADR 0054).
    pub risk: RiskClass,
    /// The method this comes from.
    pub source: Target,
    /// No mark was found.
    pub unmarked: bool,
}

impl ResourceTemplate {
    /// The MCP `ResourceTemplate` object, with the same `_meta` key as tools.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "uriTemplate": self.uri_template,
            "name": self.name,
            "description": self.description,
            "_meta": { META_KEY: {
                "risk": self.risk.as_str(),
                "askAlways": self.risk.asks_always(),
                "unmarked": self.unmarked,
                "source": self.source.as_str(),
                "kind": match self.kind { ResourceKind::Events => "events", ResourceKind::Blob => "blob" },
            }},
        })
    }
}
