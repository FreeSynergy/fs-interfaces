//! `CommandDescription`: one thing that can be done with a data type.
//!
//! Pattern: Builder, as [`super::FieldDescription`]. A command is described, not executed: the program carries it
//! out through its own contract (`people.capnp`, `tasks.capnp` …), the description only lets a frontend offer it.

use fs_interfaces::types_capnp::{BasicType, CommandKind};

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
    /// Whether the program can do it now; `false` = listed but not there yet (a frontend shows it locked).
    pub supported: bool,
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
            supported: true,
        }
    }

    /// Report it as not there yet: listed so a frontend can show it locked, until the core has it.
    #[must_use]
    pub fn unsupported(mut self) -> Self {
        self.supported = false;
        self
    }

    /// The standard `find`: input `term` (empty = everything, which is the list), result a list of references to
    /// `data_type`. Paging with a `Cursor` is part of every `find` call (rule in `types.capnp`), not a field here.
    #[must_use]
    pub fn standard_find(label: &str, data_type: &str) -> Self {
        Self::new("find", CommandKind::Find, label)
            .input(FieldDescription::new("term", BasicType::Text).label("find.term"))
            .result(
                FieldDescription::new("items", BasicType::Reference)
                    .multiple()
                    .label("find.items")
                    .target(data_type),
            )
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
