//! `Keywords`: a set of short words (JSContact and JSCalendar `keywords`).
//!
//! Pattern: Value object. Typed as a comma-separated list; each keyword is trimmed, composed (NFC) and lower-cased,
//! the set has no duplicates and is sorted, so equal sets have equal text. At most 64 characters each, 256 keywords.
//! Lower-casing is our rule (provisional): people do not mean "Family" and "family" as two keywords.

use std::collections::BTreeSet;

use fs_interfaces::types_capnp::BasicType;
use unicode_normalization::UnicodeNormalization;

use crate::{BasicValue, TypeProblem};

/// Longest keyword, in characters.
pub const KEYWORD_MAX: usize = 64;
/// Most keywords in one set.
pub const KEYWORDS_MAX: usize = 256;

/// A sorted set of lower-case keywords.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Keywords(BTreeSet<String>);

impl Keywords {
    /// The keywords in order.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// How many keywords.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there is none (a parsed set never is empty).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn one(raw: &str) -> Result<Option<String>, TypeProblem> {
    let k = raw.trim();
    if k.is_empty() {
        return Ok(None);
    }
    let k: String = k.nfc().collect();
    if k.chars().any(crate::value::bad_char) || k.chars().count() > KEYWORD_MAX {
        return Err(TypeProblem::BadKeyword);
    }
    Ok(Some(k.to_lowercase()))
}

impl BasicValue for Keywords {
    const KIND: BasicType = BasicType::Keywords;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let mut set = BTreeSet::new();
        for part in typed.split(',') {
            if let Some(k) = one(part)? {
                set.insert(k);
            }
        }
        if set.is_empty() {
            return Err(TypeProblem::Empty);
        }
        if set.len() > KEYWORDS_MAX {
            return Err(TypeProblem::TooLong);
        }
        Ok(Self(set))
    }

    fn canonical(&self) -> String {
        self.0
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Known keywords that start with the last word typed and are not in the typed list yet; the typed list is kept
    /// in front of each suggestion.
    fn suggest(typed: &str, known: &[&str]) -> Vec<String> {
        let (head, last) = typed.rsplit_once(',').map_or(("", typed), |(h, l)| (h, l));
        let last = last.trim().to_lowercase();
        if last.is_empty() {
            return Vec::new();
        }
        let have: BTreeSet<String> = head
            .split(',')
            .filter_map(|p| one(p).ok().flatten())
            .collect();
        let mut out: Vec<String> = known
            .iter()
            .map(|k| k.to_lowercase())
            .filter(|k| k.starts_with(&last) && *k != last && !have.contains(k))
            .collect();
        out.sort();
        out.dedup();
        let prefix = if head.trim().is_empty() {
            String::new()
        } else {
            format!("{}, ", head.trim())
        };
        out.into_iter().map(|k| format!("{prefix}{k}")).collect()
    }
}
