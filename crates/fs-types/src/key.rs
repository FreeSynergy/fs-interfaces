//! `Key`: an OpenPGP fingerprint.
//!
//! Pattern: Value object. RFC 9580: a version 4 fingerprint has 40 hexadecimal digits (SHA-1), a version 6 one has 64
//! (SHA-256). Spaces, colons and a leading `openpgp4fpr:` are accepted when typed; the canonical form is the bare
//! digits in upper case. Other key kinds (SSH, age, X.509) are `Unsupported` (DESIGN.md).

use fs_interfaces::types_capnp::BasicType;

use crate::{BasicValue, TypeProblem};

/// An OpenPGP fingerprint, upper-case hexadecimal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(String);

impl BasicValue for Key {
    const KIND: BasicType = BasicType::Key;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(TypeProblem::Empty);
        }
        let t = t.strip_prefix("openpgp4fpr:").unwrap_or(t);
        let digits: String = t.chars().filter(|c| !matches!(c, ' ' | ':')).collect();
        if !matches!(digits.len(), 40 | 64) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TypeProblem::BadFingerprint);
        }
        Ok(Self(digits.to_ascii_uppercase()))
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }
}

impl Key {
    /// The fingerprint in the usual groups of four digits, for display.
    #[must_use]
    pub fn grouped(&self) -> String {
        self.0
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
