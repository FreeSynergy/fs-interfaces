//! The generator's interface.
//!
//! Pattern: Strategy (and Template method inside the implementation, see `build`). Trait: [`ToolGenerator`].

use crate::{Catalog, Contract, Error};

/// Turns contracts into a catalog of tools and resource templates. Pure: it runs nothing.
pub trait ToolGenerator {
    /// Generate.
    ///
    /// # Errors
    /// [`Error::DuplicateName`] if two tools would share a name (a bug in the marks, never resolved silently).
    fn generate(&self, contracts: &[Contract]) -> Result<Catalog, Error>;
}
