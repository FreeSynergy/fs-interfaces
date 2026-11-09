//! `Choice`: one entry of a fixed list.
//!
//! Pattern: Value object. It is the one basic type whose rules depend on the field: the allowed keys come from the
//! field description (`FieldDescription.choices`), so it has `pick(typed, allowed)` instead of
//! [`crate::BasicValue::parse`]; the registry hands the list over. Keys are matched exactly after trimming.

use crate::TypeProblem;

/// The chosen key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Choice(String);

impl Choice {
    /// The key if it is one of `allowed`.
    ///
    /// # Errors
    /// [`TypeProblem::Empty`] for nothing, [`TypeProblem::NotInList`] for any other text.
    pub fn pick(typed: &str, allowed: &[String]) -> Result<Self, TypeProblem> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(TypeProblem::Empty);
        }
        if allowed.iter().any(|a| a == t) {
            Ok(Self(t.to_owned()))
        } else {
            Err(TypeProblem::NotInList)
        }
    }

    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The allowed keys that start with what is typed (case does not matter), in list order.
    #[must_use]
    pub fn suggest(typed: &str, allowed: &[String]) -> Vec<String> {
        let t = typed.trim().to_lowercase();
        allowed
            .iter()
            .filter(|a| a.to_lowercase().starts_with(&t) && **a != typed.trim())
            .cloned()
            .collect()
    }
}
