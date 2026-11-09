//! `Address`: a postal address, JSContact `Address` (RFC 9553 §2.5.1.1).
//!
//! Pattern: Value object, built by `Address::new`. The order of the parts is the country's business, so a text made
//! from components is only a list of the values (`, ` between them) and `full` stays what the person wrote. The country
//! code is checked for shape (two letters); whether it is an assigned ISO 3166-1 code is `Unsupported` (DESIGN.md).

use fs_interfaces::types_capnp::{AddressKind, BasicType};
use unicode_normalization::UnicodeNormalization;

use crate::{BasicValue, TypeProblem};

/// A postal address: components, country code, full text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    components: Vec<(AddressKind, String)>,
    country_code: String,
    full: String,
}

fn tidy(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .nfc()
        .collect()
}

impl Address {
    /// Make an address; texts are trimmed, white space collapsed, empty components dropped, the country code upper-cased.
    ///
    /// # Errors
    /// [`TypeProblem::NoParts`] with no full text and no component; [`TypeProblem::BadCountryCode`] unless empty or two
    /// letters; [`TypeProblem::Character`], [`TypeProblem::TooLong`] as for a name.
    pub fn new(
        components: &[(AddressKind, &str)],
        country_code: &str,
        full: &str,
    ) -> Result<Self, TypeProblem> {
        for t in std::iter::once(full).chain(components.iter().map(|c| c.1)) {
            if t.chars().any(char::is_control) {
                return Err(TypeProblem::Character);
            }
            if t.chars().count() > 1024 {
                return Err(TypeProblem::TooLong);
            }
        }
        let country_code = country_code.trim().to_ascii_uppercase();
        if !country_code.is_empty()
            && (country_code.len() != 2 || !country_code.bytes().all(|b| b.is_ascii_uppercase()))
        {
            return Err(TypeProblem::BadCountryCode);
        }
        let components: Vec<(AddressKind, String)> = components
            .iter()
            .map(|(k, v)| (*k, tidy(v)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        let full = tidy(full);
        if full.is_empty() && components.is_empty() {
            return Err(TypeProblem::NoParts);
        }
        Ok(Self {
            components,
            country_code,
            full,
        })
    }

    /// The components in order.
    #[must_use]
    pub fn components(&self) -> &[(AddressKind, String)] {
        &self.components
    }

    /// ISO 3166-1 alpha-2 code in upper case, empty when not given.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.country_code
    }

    /// The text for display: `full`, or the component values with `, ` between them.
    #[must_use]
    pub fn display(&self) -> String {
        if self.full.is_empty() {
            self.components
                .iter()
                .map(|c| c.1.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            self.full.clone()
        }
    }
}

impl BasicValue for Address {
    const KIND: BasicType = BasicType::Address;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        if typed.trim().is_empty() {
            return Err(TypeProblem::Empty);
        }
        Self::new(&[], "", typed)
    }

    fn canonical(&self) -> String {
        self.display()
    }
}
