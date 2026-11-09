//! `Rule` and `BasicRules`: reach the rules of a basic type by its kind alone.
//!
//! Pattern: Registry (kind → behaviour, `coding.md` §2) of Strategy objects (`dyn Rule`), filled by an Adapter that
//! lifts every [`BasicValue`] into a `Rule`. This is what a generated frontend and a program's generic `check`
//! command use: they know a field's [`BasicType`] from the type description, not the Rust type. Kinds the registry does
//! not know (a newer schema) are treated as plain text, as `schema/types.capnp` promises.

use std::marker::PhantomData;

use fs_interfaces::types_capnp::BasicType;
use fs_interfaces::values::{EmailAddress, PhoneNumber};

use crate::{
    Address, BasicValue, Choice, Date, DateTime, Image, Key, Keywords, Language, Link,
    MultilineText, Name, Place, Reference, Text, TypeProblem,
};

/// The rules of one basic type, object safe.
pub trait Rule: Send + Sync {
    /// The kind this rule is for.
    fn kind(&self) -> BasicType;

    /// Check `typed` and return the canonical text. `choices` is the list of a `choice` field (ignored by the rest).
    ///
    /// # Errors
    /// The first rule the text breaks.
    fn check(&self, typed: &str, choices: &[String]) -> Result<String, TypeProblem>;

    /// Completions. `known` are canonical values of the same kind already in use; for a `choice` the completions come
    /// from `choices`. Empty when the type has none to offer.
    fn suggest(&self, typed: &str, known: &[&str], choices: &[String]) -> Vec<String>;
}

/// Lifts a [`BasicValue`] to a [`Rule`].
struct Of<T>(PhantomData<fn() -> T>);

impl<T: BasicValue> Rule for Of<T> {
    fn kind(&self) -> BasicType {
        T::KIND
    }

    fn check(&self, typed: &str, _choices: &[String]) -> Result<String, TypeProblem> {
        T::parse(typed).map(|v| v.canonical())
    }

    fn suggest(&self, typed: &str, known: &[&str], _choices: &[String]) -> Vec<String> {
        T::suggest(typed, known)
    }
}

/// The one type whose list comes from the field.
struct ChoiceRule;

impl Rule for ChoiceRule {
    fn kind(&self) -> BasicType {
        BasicType::Choice
    }

    fn check(&self, typed: &str, choices: &[String]) -> Result<String, TypeProblem> {
        Choice::pick(typed, choices).map(|c| c.as_str().to_owned())
    }

    fn suggest(&self, typed: &str, _known: &[&str], choices: &[String]) -> Vec<String> {
        Choice::suggest(typed, choices)
    }
}

/// All rules by kind.
pub struct BasicRules {
    rules: Vec<Box<dyn Rule>>,
}

fn boxed<T: BasicValue + 'static>() -> Box<dyn Rule> {
    Box::new(Of::<T>(PhantomData))
}

impl BasicRules {
    /// The rules of every kind this version knows.
    #[must_use]
    pub fn standard() -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            boxed::<Text>(),
            boxed::<MultilineText>(),
            boxed::<EmailAddress>(),
            boxed::<PhoneNumber>(),
            boxed::<Name>(),
            boxed::<Address>(),
            boxed::<Place>(),
            boxed::<Date>(),
            boxed::<DateTime>(),
            boxed::<Language>(),
            boxed::<Link>(),
            boxed::<Image>(),
            boxed::<Key>(),
            Box::new(ChoiceRule),
            boxed::<Keywords>(),
            boxed::<Reference>(),
        ];
        Self { rules }
    }

    /// The rule for `kind`; `unknown` and kinds from a newer schema get the rule of plain text.
    #[must_use]
    pub fn rule(&self, kind: BasicType) -> &dyn Rule {
        self.rules
            .iter()
            .find(|r| r.kind() == kind)
            .unwrap_or(&self.rules[0])
            .as_ref()
    }

    /// Check `typed` as `kind`; the canonical text.
    ///
    /// # Errors
    /// The first rule the text breaks.
    pub fn check(
        &self,
        kind: BasicType,
        typed: &str,
        choices: &[String],
    ) -> Result<String, TypeProblem> {
        self.rule(kind).check(typed, choices)
    }

    /// Completions of `typed` for `kind`.
    #[must_use]
    pub fn suggest(
        &self,
        kind: BasicType,
        typed: &str,
        known: &[&str],
        choices: &[String],
    ) -> Vec<String> {
        self.rule(kind).suggest(typed, known, choices)
    }
}
