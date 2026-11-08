//! `ContactValue`: what a form needs of a contact value.
//!
//! Pattern: Value object (the trait of it) — make one from typed text or say why not.

use super::ValueProblem;

/// A value of a contact point, valid by construction.
pub trait ContactValue: Sized {
    /// From what was typed (spaces around it do not matter).
    ///
    /// # Errors
    /// The first rule the text breaks.
    fn parse(typed: &str) -> Result<Self, ValueProblem>;

    /// The canonical text to store and compare.
    fn as_str(&self) -> &str;

    /// Whether `typed` is a value, without making one; what a field asks on every key.
    ///
    /// # Errors
    /// The same as [`ContactValue::parse`].
    fn check(typed: &str) -> Result<(), ValueProblem> {
        Self::parse(typed).map(|_| ())
    }
}
