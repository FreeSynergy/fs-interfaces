//! Where contracts come from.
//!
//! Pattern: Strategy — the generator asks a [`ContractSource`] and does not care whether the contracts
//! were compiled from `.capnp` files ([`crate::CapnpcSource`]) or built by a test. Trait: [`ContractSource`].

use crate::{Contract, Error};

/// A provider of the contracts in the generator's own model.
pub trait ContractSource {
    /// All contracts, one per schema file, with the comments of the schema.
    ///
    /// # Errors
    /// If the schemas cannot be compiled or read; never a partial result.
    fn contracts(&self) -> Result<Vec<Contract>, Error>;
}
