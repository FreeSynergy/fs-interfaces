//! The generator's interface.
//!
//! Pattern: Strategy (the implementation is [`crate::Generator`]). Trait: [`ToolGenerator`].

use crate::{Catalog, Contract, Error};

/// Turns contracts into a catalog. Pure: it runs nothing.
pub trait ToolGenerator {
    /// Generate.
    ///
    /// # Errors
    /// [`Error::DuplicateName`] if two tools would share a name (a bug in the marks, never resolved silently).
    fn generate(&self, contracts: &[Contract]) -> Result<Catalog, Error>;
}
