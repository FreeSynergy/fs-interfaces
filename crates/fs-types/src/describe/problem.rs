//! `DescriptionProblem`: why a type description cannot be accepted.
//!
//! Pattern: Value object — a code with the name it is about; for the author of the description (a developer), so the
//! name travels with it and no translation is needed.

/// A defect in a type description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptionProblem {
    /// The type, field or command has no name.
    EmptyName,
    /// A name that is not lower camel case (fields, commands) or upper camel case (types).
    BadName(String),
    /// The type names no owner program.
    EmptyProgram,
    /// A label that is empty or not a translation key (`a.b-c`, lower case).
    BadLabel(String),
    /// Two fields (or inputs, results) with one name, or two commands.
    Duplicate(String),
    /// A `choice` field with no keys.
    NoChoices(String),
    /// Keys on a field that is not a `choice`.
    ChoicesOnNonChoice(String),
    /// A choice key that is not a lower-case key (`a`, `in-process`) and so can neither be typed back after trimming nor
    /// carry a label key `<label>.<key>`.
    BadChoice(String),
    /// Two equal keys in one list of choices.
    DuplicateChoice(String),
    /// A `target` on a field that is not a `reference`.
    TargetOnNonReference(String),
    /// A command of kind `unknown`.
    UnknownCommandKind(String),
    /// A type of this name is already registered.
    TypeExists(String),
}

impl DescriptionProblem {
    /// A stable code for logs and tests.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyName => "empty-name",
            Self::BadName(_) => "bad-name",
            Self::EmptyProgram => "empty-program",
            Self::BadLabel(_) => "bad-label",
            Self::Duplicate(_) => "duplicate",
            Self::NoChoices(_) => "no-choices",
            Self::ChoicesOnNonChoice(_) => "choices-on-non-choice",
            Self::BadChoice(_) => "bad-choice",
            Self::DuplicateChoice(_) => "duplicate-choice",
            Self::TargetOnNonReference(_) => "target-on-non-reference",
            Self::UnknownCommandKind(_) => "unknown-command-kind",
            Self::TypeExists(_) => "type-exists",
        }
    }
}

impl std::fmt::Display for DescriptionProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName | Self::EmptyProgram => write!(f, "{}", self.code()),
            Self::BadName(n)
            | Self::BadLabel(n)
            | Self::Duplicate(n)
            | Self::NoChoices(n)
            | Self::ChoicesOnNonChoice(n)
            | Self::BadChoice(n)
            | Self::DuplicateChoice(n)
            | Self::TargetOnNonReference(n)
            | Self::UnknownCommandKind(n)
            | Self::TypeExists(n) => write!(f, "{}: {n}", self.code()),
        }
    }
}

impl std::error::Error for DescriptionProblem {}
