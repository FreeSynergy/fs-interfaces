//! FreeSynergy basic types and type descriptions (ADR 0066, schema `types.capnp` in `fs-interfaces`).
//!
//! * Basic types: [`BasicValue`] with one value object per kind ([`Text`], [`EmailAddress`], [`Name`], [`Address`],
//!   [`Place`], [`Date`], [`Language`], [`Link`], [`Image`], [`Key`], [`Choice`], [`Keywords`], [`Reference`] …): they
//!   check, normalise and suggest. [`BasicRules`] reaches them by [`BasicType`] alone.
//! * Type description: [`TypeDescription`] (fields, commands), the trait [`DescribesType`] a program implements for each
//!   data type it owns, and [`TypeRegistry`] that collects them and answers the `TypeCatalog` contract.
//!
//! Pattern: Registry (rules by kind, descriptions by name), Value object, Adapter (to the free crates and to the wire
//! form). Where it lives and why: `DESIGN.md`.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod address;
mod choice;
mod contact;
mod datetime;
pub mod describe;
mod image;
mod iri;
mod key;
mod keywords;
mod language;
mod name;
mod place;
mod problem;
mod rule;
mod text;
mod value;

pub use address::Address;
pub use choice::Choice;
pub use datetime::{Date, DateTime};
pub use describe::{
    CommandDescription, CommandKind, DescribesType, DescriptionProblem, FieldDescription,
    RecordIssue, RecordProblem, TypeDescription, TypeRegistry,
};
pub use fs_interfaces::types_capnp::{AddressKind, BasicType, NameKind};
pub use fs_interfaces::values::{EmailAddress, PhoneNumber};
pub use image::Image;
pub use iri::{Link, Reference};
pub use key::Key;
pub use keywords::Keywords;
pub use language::Language;
pub use name::Name;
pub use place::Place;
pub use problem::TypeProblem;
pub use rule::{BasicRules, Rule};
pub use text::{MultilineText, Text};
pub use value::BasicValue;
