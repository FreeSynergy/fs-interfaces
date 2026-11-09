//! E-mail address and phone number as basic types.
//!
//! Pattern: Adapter — the value objects live in `fs-interfaces` (`values`, shared by all programs since before this
//! crate); here they are made to follow [`BasicValue`], so the registry reaches them like every other type. Nothing is
//! re-implemented.

use std::collections::HashMap;

use fs_interfaces::types_capnp::BasicType;
use fs_interfaces::values::{ContactValue, EmailAddress, PhoneNumber};

use crate::{BasicValue, TypeProblem};

impl BasicValue for EmailAddress {
    const KIND: BasicType = BasicType::Email;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        <Self as ContactValue>::parse(typed).map_err(TypeProblem::from)
    }

    fn canonical(&self) -> String {
        self.as_str().to_owned()
    }

    /// Completes the domain from the domains of `known` addresses, the most frequent first. With no `@` typed it offers
    /// `typed@domain` for each of them (the person has typed the local part).
    fn suggest(typed: &str, known: &[&str]) -> Vec<String> {
        let typed = typed.trim();
        if typed.is_empty() || typed.matches('@').count() > 1 {
            return Vec::new();
        }
        let (local, prefix) = typed.split_once('@').unwrap_or((typed, ""));
        if local.is_empty() {
            return Vec::new();
        }
        let prefix = prefix.to_lowercase();
        let mut count: HashMap<&str, usize> = HashMap::new();
        for k in known {
            if let Some((_, d)) = k.rsplit_once('@').filter(|(_, d)| d.starts_with(&prefix)) {
                *count.entry(d).or_default() += 1;
            }
        }
        let mut domains: Vec<(&str, usize)> = count.into_iter().collect();
        domains.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        domains
            .into_iter()
            .map(|(d, _)| format!("{local}@{d}"))
            .filter(|s| <EmailAddress as ContactValue>::check(s).is_ok())
            .collect()
    }
}

impl BasicValue for PhoneNumber {
    const KIND: BasicType = BasicType::Phone;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        <Self as ContactValue>::parse(typed).map_err(TypeProblem::from)
    }

    fn canonical(&self) -> String {
        self.as_str().to_owned()
    }
}
