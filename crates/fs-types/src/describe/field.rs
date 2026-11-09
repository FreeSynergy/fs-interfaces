//! `FieldDescription`: one field of a data type, or one input or result of a command.
//!
//! Pattern: Builder (`coding.md` §2) — `FieldDescription::new("emails", BasicType::Email).multiple().label("person.emails")`.

use fs_interfaces::types_capnp::BasicType;

/// One field. Mirrors `FieldDescription` of `types.capnp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescription {
    /// Machine name, lower camel case, unique in its type or command.
    pub name: String,
    /// The basic type of each value.
    pub basic: BasicType,
    /// A value is needed.
    pub required: bool,
    /// More than one value is allowed.
    pub multiple: bool,
    /// Translation key (`coding.md` §2a), never a sentence.
    pub label: String,
    /// For a `reference`: the data type referred to; empty = any.
    pub target: String,
    /// For a `choice`: the allowed keys.
    pub choices: Vec<String>,
}

impl FieldDescription {
    /// A field that is optional, single, with no label yet.
    #[must_use]
    pub fn new(name: &str, basic: BasicType) -> Self {
        Self {
            name: name.to_owned(),
            basic,
            required: false,
            multiple: false,
            label: String::new(),
            target: String::new(),
            choices: Vec::new(),
        }
    }

    /// A value is needed.
    #[must_use]
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Several values are allowed.
    #[must_use]
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    /// The translation key of the label.
    #[must_use]
    pub fn label(mut self, key: &str) -> Self {
        key.clone_into(&mut self.label);
        self
    }

    /// The data type a `reference` points to.
    #[must_use]
    pub fn target(mut self, data_type: &str) -> Self {
        data_type.clone_into(&mut self.target);
        self
    }

    /// The allowed keys of a `choice`.
    #[must_use]
    pub fn choices(mut self, keys: &[&str]) -> Self {
        self.choices = keys.iter().map(|k| (*k).to_owned()).collect();
        self
    }
}
