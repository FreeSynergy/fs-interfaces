//! A named place in a contract that can carry a mark: an interface, a method, a union member, a field.
//!
//! Pattern: Newtype — `Target` is a validated-by-construction string key, so marks, releases and the block
//! list cannot be mixed up with free text. Trait: none (plain value); it keys [`crate::RiskSource`].

use std::fmt;

/// `file/Owner` for an interface or struct, `file/Owner.name` for a method, union member or field,
/// and `file/Interface.method.param` for a method parameter. `file` is the schema file stem.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Target(String);

impl Target {
    /// An interface or a struct as a whole, such as `messenger/Guard`.
    #[must_use]
    pub fn owner(file: &str, owner: &str) -> Self {
        Self(format!("{file}/{owner}"))
    }

    /// A method, union member or field of `owner`, such as `messenger/MessageAction.decide`.
    #[must_use]
    pub fn item(file: &str, owner: &str, name: &str) -> Self {
        Self(format!("{file}/{owner}.{name}"))
    }

    /// The key as written in the mark files and in reports.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
