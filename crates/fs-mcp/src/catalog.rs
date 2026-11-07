//! The result of one generator run: what exists, and, as important, what was left out and why.
//!
//! Pattern: Value object (a report). Trait: none. Every omission is listed, so a missing tool is never
//! unexplained and a forgotten mark is never silent.

use crate::{ResourceTemplate, Target, Tool};
use serde_json::{Value, json};

/// Why something is not offered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// On the fixed block list of the generator (text says why).
    Blocked(&'static str),
    /// A mark file excludes it.
    Marked(String),
    /// A name or comment looks like a secret and nobody released it. `path` says where.
    SecretSuspect {
        /// The suspicious name, such as `AccountSetup.secret`.
        path: String,
    },
    /// Free-form content (`Data`, or a field named json, config, proof, payload, blob, data) that nobody
    /// released in writing as free of secrets.
    FreeForm {
        /// Where, such as `files/Upload.write.data`.
        path: String,
    },
    /// Something the generator cannot judge: a structure it does not know, or an `AnyPointer`.
    Unclear {
        /// Where.
        path: String,
    },
    /// Not a call: a listener or a stream. Events become resources.
    NotACall(&'static str),
}

/// A target left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    /// The method, member or interface.
    pub target: Target,
    /// Why.
    pub why: Why,
}

/// Everything one run produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    /// Tools, sorted by name.
    pub tools: Vec<Tool>,
    /// Resource templates.
    pub resources: Vec<ResourceTemplate>,
    /// Everything left out, with reasons.
    pub left_out: Vec<Left>,
    /// Targets that got no mark (they are tools with class `unknown`).
    pub unmarked: Vec<Target>,
    /// Marks or releases whose target does not exist in the contracts (a rename leaves these behind).
    pub stale: Vec<Target>,
}

impl Catalog {
    /// `tools/list` and `resources/templates/list` in one document.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "tools": self.tools.iter().map(Tool::to_json).collect::<Vec<_>>(),
            "resourceTemplates": self.resources.iter().map(ResourceTemplate::to_json).collect::<Vec<_>>(),
        })
    }
}
