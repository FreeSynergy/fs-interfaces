//! The type description a program reports for each data type it owns (ADR 0066 point 3).

mod check;
mod command;
mod describes;
mod field;
mod problem;
mod registry;
mod type_description;
mod validate;
mod wire;

#[cfg(feature = "rpc")]
mod server;

pub use check::{RecordIssue, RecordProblem};
pub use command::CommandDescription;
pub use describes::DescribesType;
pub use field::FieldDescription;
pub use fs_interfaces::types_capnp::CommandKind;
pub use problem::DescriptionProblem;
pub use registry::TypeRegistry;
#[cfg(feature = "rpc")]
pub use server::CatalogServer;
pub use type_description::TypeDescription;
