//! `TypeDescription`: a data type with its fields and commands.
//!
//! Pattern: Value object, built with a Builder-style chain; checked by `validate` before it goes into the registry.

use super::{CommandDescription, FieldDescription};

/// What a program tells the world about one data type it owns. Mirrors `TypeDescription` of `types.capnp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDescription {
    /// Name, unique across programs: `Person`.
    pub name: String,
    /// Owner program (ADR 0050): `fs-people`.
    pub program: String,
    /// The standard it follows: `RFC 9553 Card`.
    pub standard: String,
    /// Translation key of the type.
    pub label: String,
    /// The fields.
    pub fields: Vec<FieldDescription>,
    /// The commands.
    pub commands: Vec<CommandDescription>,
}

impl TypeDescription {
    /// A type with no fields and no commands yet.
    #[must_use]
    pub fn new(name: &str, program: &str, standard: &str, label: &str) -> Self {
        Self {
            name: name.to_owned(),
            program: program.to_owned(),
            standard: standard.to_owned(),
            label: label.to_owned(),
            fields: Vec::new(),
            commands: Vec::new(),
        }
    }

    /// Add a field.
    #[must_use]
    pub fn field(mut self, field: FieldDescription) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a command.
    #[must_use]
    pub fn command(mut self, command: CommandDescription) -> Self {
        self.commands.push(command);
        self
    }

    /// The field called `name`.
    #[must_use]
    pub fn find_field(&self, name: &str) -> Option<&FieldDescription> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The command called `name`.
    #[must_use]
    pub fn find_command(&self, name: &str) -> Option<&CommandDescription> {
        self.commands.iter().find(|c| c.name == name)
    }
}
