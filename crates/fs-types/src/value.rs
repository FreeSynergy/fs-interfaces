//! `BasicValue`: what every basic type does — check, normalise, suggest.
//!
//! Pattern: Template method over a Value object (`coding.md` §2) — one trait, each type fills in `parse` and
//! `canonical`; `check` and the default `suggest` come with the trait. Trait first (ADR 0043): a program or a form
//! depends on this trait or on the [`crate::Rule`] registry, never on a concrete type.

use fs_interfaces::types_capnp::BasicType;

use crate::TypeProblem;

/// A value of one basic type, valid by construction.
pub trait BasicValue: Sized {
    /// The kind this value belongs to (names the standard in `schema/types.capnp`).
    const KIND: BasicType;

    /// Check and normalise what was typed (spaces around it do not matter).
    ///
    /// # Errors
    /// The first rule the text breaks.
    fn parse(typed: &str) -> Result<Self, TypeProblem>;

    /// The canonical text to store and compare (equal values have equal canonical text).
    fn canonical(&self) -> String;

    /// Whether `typed` is a value, without keeping it; what a field asks on every key.
    ///
    /// # Errors
    /// The same as [`BasicValue::parse`].
    fn check(typed: &str) -> Result<(), TypeProblem> {
        Self::parse(typed).map(|_| ())
    }

    /// Completions of what is typed, from `known` values of the same type (best first).
    ///
    /// The default offers nothing: a type with no sensible completion says so by leaving this out (it does not
    /// invent one). `known` holds canonical texts.
    #[must_use]
    fn suggest(typed: &str, known: &[&str]) -> Vec<String> {
        let _ = (typed, known);
        Vec::new()
    }
}

/// A character no text of ours holds: a control character, a line or paragraph separator (U+2028, U+2029), or a
/// bidirectional override or isolate (U+202A to U+202E, U+2066 to U+2069), which can make a text read differently from
/// what it is. `\n` is not special here; the types that allow it say so.
pub(crate) fn bad_char(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Shared by the one-line types: trim, compose to NFC, refuse bad characters and over-long text.
pub(crate) fn one_line(typed: &str, max_chars: usize) -> Result<String, TypeProblem> {
    use unicode_normalization::UnicodeNormalization;
    let t = typed.trim();
    if t.is_empty() {
        return Err(TypeProblem::Empty);
    }
    if t.chars().any(bad_char) {
        return Err(TypeProblem::Character);
    }
    let t: String = t.nfc().collect();
    if t.chars().count() > max_chars {
        return Err(TypeProblem::TooLong);
    }
    Ok(t)
}
