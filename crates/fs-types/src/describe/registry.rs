//! `TypeRegistry`: the data types a program owns, by name.
//!
//! Pattern: Registry (`coding.md` §2) — name → [`TypeDescription`], filled once at start-up through the trait
//! [`DescribesType`] and answered over the `TypeCatalog` contract. A description is validated when it goes in, so
//! everything in the registry is sound.

use std::collections::BTreeMap;

use super::validate::Mode;
use super::{DescribesType, DescriptionProblem, TypeDescription};

/// Descriptions by type name, in name order.
#[derive(Clone, Debug, Default)]
pub struct TypeRegistry {
    types: BTreeMap<String, TypeDescription>,
}

impl TypeRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the description of `T`.
    ///
    /// # Errors
    /// The defect of the description, or [`DescriptionProblem::TypeExists`].
    pub fn register<T: DescribesType>(&mut self) -> Result<&mut Self, DescriptionProblem> {
        self.add(T::describe())
    }

    /// Register a description.
    ///
    /// # Errors
    /// The defect of the description, or [`DescriptionProblem::TypeExists`].
    pub fn add(&mut self, description: TypeDescription) -> Result<&mut Self, DescriptionProblem> {
        self.insert(description, Mode::Strict)
    }

    /// Add a description that was read from the wire: as `add`, but a kind this version does not know is let through
    /// (it may be newer than we are; a frontend shows such a field as plain text).
    pub(crate) fn add_read(
        &mut self,
        description: TypeDescription,
    ) -> Result<&mut Self, DescriptionProblem> {
        self.insert(description, Mode::Lenient)
    }

    fn insert(
        &mut self,
        description: TypeDescription,
        mode: Mode,
    ) -> Result<&mut Self, DescriptionProblem> {
        description.validate_in(mode)?;
        if self.types.contains_key(&description.name) {
            return Err(DescriptionProblem::TypeExists(description.name));
        }
        self.types.insert(description.name.clone(), description);
        Ok(self)
    }

    /// The description of a type.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&TypeDescription> {
        self.types.get(name)
    }

    /// All descriptions in name order.
    pub fn iter(&self) -> impl Iterator<Item = &TypeDescription> {
        self.types.values()
    }

    /// How many types.
    #[must_use]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether there is none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// `(type, field, target)` of every reference whose target type is not in this registry. The target may live in
    /// another program, so this is a list to look at, not an error.
    #[must_use]
    pub fn unresolved_targets(&self) -> Vec<(String, String, String)> {
        self.types
            .values()
            .flat_map(|t| {
                t.fields
                    .iter()
                    .filter(|f| !f.target.is_empty() && !self.types.contains_key(&f.target))
                    .map(|f| (t.name.clone(), f.name.clone(), f.target.clone()))
            })
            .collect()
    }
}
