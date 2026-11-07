//! Reads the contracts by compiling the `.capnp` files with our own compiler and walking the
//! `CodeGeneratorRequest` it produces: the same data every code generator gets, including the comments.
//!
//! Pattern: Adapter — see `source.rs`.

mod names;
mod nodes;
mod source;
mod types;

pub use source::CapnpcSource;
