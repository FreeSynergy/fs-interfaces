//! MCP tool generator, stage 1 (ADR 0021 extension, ADR 0054): lists the tools and resource templates
//! that the Cap'n Proto contracts allow, with a risk class each. It runs nothing: no server, no bridge,
//! no call. Eine Liste ist keine Erlaubnis: the list is an offer, the engine decides on every call.
//! Pattern: Strategy (see `DESIGN.md`).
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod blocklist;
mod capnpc_source;
mod catalog;
mod error;
mod generator;
mod jsonschema;
mod marks;
mod model;
mod resource;
mod risk;
mod secrets;
mod source;
mod target;
mod tool;

pub use blocklist::Blocklist;
pub use capnpc_source::CapnpcSource;
pub use catalog::{Catalog, Left, Why};
pub use error::Error;
pub use generator::{Generator, ToolGenerator};
pub use jsonschema::SchemaBuilder;
pub use marks::MarkFiles;
pub use model::{Contract, EnumDef, Field, Index, Interface, Method, StructDef, TypeName, TypeRef};
pub use resource::{ResourceKind, ResourceTemplate};
pub use risk::{Mark, RiskClass, RiskSource};
pub use secrets::{doc_looks_secret, name_looks_secret};
pub use source::ContractSource;
pub use target::Target;
pub use tool::{META_KEY, Tool};
