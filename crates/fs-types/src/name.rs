//! `Name`: the name of a person, JSContact `Name` (RFC 9553 §2.2.1.1).
//!
//! Pattern: Value object, built by `Name::new` (the check) or parsed from one line (the full name only: splitting
//! "Ada Lovelace" into given and surname is a guess that is wrong for many names, so it is not done). The
//! component kinds are the wire enum [`NameKind`].

use fs_interfaces::types_capnp::{BasicType, NameKind};
use unicode_normalization::UnicodeNormalization;

use crate::value::bad_char;
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
    /// [`TypeProblem::NoParts`] with neither full text nor component; [`TypeProblem::Character`] for control characters (a line end at the edges is no problem, white space is collapsed);
    /// [`TypeProblem::TooLong`] above 1024 characters in a part.
    pub fn new(full: &str, components: &[(NameKind, &str)]) -> Result<Self, TypeProblem> {
        // A separator is literal (RFC 9553 section 2.2.1.2): neither trimmed nor collapsed, only dropped when empty.
        let components: Vec<(NameKind, String)> = components
            .iter()
            .map(|(k, v)| {
                let text = if *k == NameKind::Separator {
                    (*v).nfc().collect()
                } else {
                    tidy(v)
                };
                (*k, text)
            })
            .filter(|(_, v)| !v.is_empty())
            .collect();
        let full = tidy(full);
        if full.is_empty() && components.is_empty() {
            return Err(TypeProblem::NoParts);
        }
        let texts = components
            .iter()
            .map(|c| c.1.as_str())
            .chain([full.as_str()]);
        if texts.clone().any(|t| t.chars().any(bad_char)) {
            return Err(TypeProblem::Character);
        }
        if texts.map(|t| t.chars().count()).max().unwrap_or(0) > 1024 {
            return Err(TypeProblem::TooLong);
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
        let mut after_separator = true;
        for (kind, v) in &self.components {
            let separator = *kind == NameKind::Separator;
            if !separator && !after_separator {
                out.push(' ');
            }
            out.push_str(v);
            after_separator = separator;
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
