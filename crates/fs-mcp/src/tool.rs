//! One MCP tool as data, and its wire form.
//!
//! Pattern: none (plain data) with a fixed projection (`to_json`). The projection cannot say "read only":
//! there is no field for it, `readOnlyHint` is always written as `false`. Trait: none.

use crate::{RiskClass, Target};
use serde_json::{Value, json};

/// Namespace of our keys inside `_meta`.
pub const META_KEY: &str = "net.freesynergy/mcp";

/// A tool of the generated catalog. Nothing here runs anything (stage 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    /// Capability id such as `message.send`, else `file.interface.method[.member]`.
    pub name: String,
    /// The schema comment; says so if there is none.
    pub description: String,
    /// JSON Schema of the arguments.
    pub input_schema: Value,
    /// JSON Schema of the result.
    pub output_schema: Value,
    /// Risk class from the marks; `Unknown` without one.
    pub risk: RiskClass,
    /// The method or union member this comes from.
    pub source: Target,
    /// The interface whose handle the first argument names; `None` for a root interface.
    pub handle_of: Option<String>,
    /// True if no mark was found.
    pub unmarked: bool,
}

impl Tool {
    /// The tool as an MCP `Tool` object. `readOnlyHint` is `false` for every tool: reading is not harmless
    /// (ADR 0054), and hints are untrusted anyway. `destructiveHint` is `true` for every tool for the same
    /// reason. The real class travels in `_meta`, where the engine, not the caller, reads it.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
            "outputSchema": self.output_schema,
            "annotations": { "readOnlyHint": false, "destructiveHint": true, "openWorldHint": true },
            "_meta": { META_KEY: {
                "risk": self.risk.as_str(),
                "askAlways": self.risk.asks_always(),
                "unmarked": self.unmarked,
                "source": self.source.as_str(),
                "handleOf": self.handle_of,
            }},
        })
    }
}
