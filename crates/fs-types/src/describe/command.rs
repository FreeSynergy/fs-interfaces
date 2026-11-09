//! `CommandDescription`: one thing that can be done with a data type.
//!
//! Pattern: Builder, as [`super::FieldDescription`]. A command is described, not executed: the program carries it
//! out through its own contract (`people.capnp`, `tasks.capnp` …), the description only lets a frontend offer it.

use fs_interfaces::types_capnp::CommandKind;

use super::FieldDescription;

/// A command: name, kind, input and result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDescription {
    /// Machine name, unique in its type: `create`, `patch`, `find` …
    pub name: String,
    /// What it does to the data (a frontend picks the button and the confirmation from it).
    pub kind: CommandKind,
    /// Translation key of the label.
    pub label: String,
    /// The input, in the order of the call.
    pub input: Vec<FieldDescription>,
    /// The result; empty = nothing but success.
    pub result: Vec<FieldDescription>,
}

impl CommandDescription {
    /// A command with no input and no result yet.
    #[must_use]
    pub fn new(name: &str, kind: CommandKind, label: &str) -> Self {
        Self {
            name: name.to_owned(),
            kind,
            label: label.to_owned(),
            input: Vec::new(),
            result: Vec::new(),
        }
    }

    /// Add an input field.
    #[must_use]
    pub fn input(mut self, field: FieldDescription) -> Self {
        self.input.push(field);
        self
    }

    /// Add a result field.
    #[must_use]
    pub fn result(mut self, field: FieldDescription) -> Self {
        self.result.push(field);
        self
    }
}
