//! `Language`: a BCP 47 language tag (RFC 5646).
//!
//! Pattern: Adapter over the free crate `oxilangtag` (well-formedness and case normalisation per RFC 5646); the
//! value object around it is ours. Well-formed only: whether the subtags are registered at IANA is not checked
//! (`Unsupported`, DESIGN.md).

use fs_interfaces::types_capnp::BasicType;
use oxilangtag::LanguageTag;

use crate::{BasicValue, TypeProblem};

/// A well-formed language tag in the case RFC 5646 recommends (`de`, `pt-BR`, `zh-Hant-TW`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Language(String);

impl BasicValue for Language {
    const KIND: BasicType = BasicType::Language;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(TypeProblem::Empty);
        }
        LanguageTag::parse_and_normalize(t)
            .map(|tag| Self(tag.into_inner()))
            .map_err(|_| TypeProblem::BadLanguage)
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }

    /// Known tags that start with what is typed (case does not matter), shortest first.
    fn suggest(typed: &str, known: &[&str]) -> Vec<String> {
        let t = typed.trim().to_lowercase();
        if t.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<String> = known
            .iter()
            .filter(|k| k.to_lowercase().starts_with(&t) && k.len() > t.len())
            .map(|k| (*k).to_owned())
            .collect();
        out.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
        out.dedup();
        out
    }
}
