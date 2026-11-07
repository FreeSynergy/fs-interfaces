//! Tool generation: the trait, the generator and the checks every candidate passes.
//!
//! Pattern: see `run.rs`.

mod api;
mod facts;
mod resources;
mod run;
mod scan;
mod tools;
mod variant;
mod vet;

pub use api::ToolGenerator;
pub use run::Generator;
