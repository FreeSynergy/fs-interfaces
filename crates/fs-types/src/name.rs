//! `Name`: the name of a person, JSContact `Name` (RFC 9553 §2.2.1.1).
//!
//! Pattern: Value object, built by `Name::new` (the check) or parsed from one line (the full name only: splitting
//! "Ada Lovelace" into given and surname is a guess that is wrong for many names, so it is not done). The
//! component kinds are the wire enum [`NameKind`].

use fs_interfaces::types_capnp::{BasicType, NameKind};
use unicode_normalization::UnicodeNormalization;

use crate::{BasicValue, TypeProblem};

/// A name: a full text, components, or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    full: String,
    components: Vec<(NameKind, String)>,
}

fn tidy(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .nfc()
        .collect()
}

impl Name {
    /// Make a name; texts are trimmed, white space collapsed, empty components dropped.
    ///
    /// # Errors
    /// [`TypeProblem::NoParts`] with neither full text nor component; [`TypeProblem::Character`] for control characters;
    /// [`TypeProblem::TooLong`] above 1024 characters in a part.
    pub fn new(full: &str, components: &[(NameKind, &str)]) -> Result<Self, TypeProblem> {
        let all = std::iter::once(full).chain(components.iter().map(|c| c.1));
        for t in all {
            if t.chars().any(char::is_control) {
                return Err(TypeProblem::Character);
            }
            if t.chars().count() > 1024 {
                return Err(TypeProblem::TooLong);
            }
        }
        let components: Vec<(NameKind, String)> = components
            .iter()
            .map(|(k, v)| (*k, tidy(v)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        let full = tidy(full);
        if full.is_empty() && components.is_empty() {
            return Err(TypeProblem::NoParts);
        }
        Ok(Self { full, components })
    }

    /// The components in order.
    #[must_use]
    pub fn components(&self) -> &[(NameKind, String)] {
        &self.components
    }

    /// The text for display: the full name, or the components put together (separators attach, others get a space).
    #[must_use]
    pub fn display(&self) -> String {
        if !self.full.is_empty() {
            return self.full.clone();
        }
        let mut out = String::new();
        for (kind, v) in &self.components {
            if *kind != NameKind::Separator
                && !out.is_empty()
                && !out.ends_with(char::is_whitespace)
            {
                out.push(' ');
            }
            out.push_str(v);
        }
        out
    }

    /// Fill `full` from the components when it is empty (the suggestion a form shows).
    #[must_use]
    pub fn with_full(mut self) -> Self {
        if self.full.is_empty() {
            self.full = self.display();
        }
        self
    }
}

impl BasicValue for Name {
    const KIND: BasicType = BasicType::Name;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        if typed.trim().is_empty() {
            return Err(TypeProblem::Empty);
        }
        Self::new(typed, &[])
    }

    fn canonical(&self) -> String {
        self.display()
    }
}
