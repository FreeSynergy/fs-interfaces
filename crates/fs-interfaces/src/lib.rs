//! FreeSynergy interface contracts (ADR 0045, 0050).
//!
//! The Cap'n Proto schemas live in `schema/`; the Rust code generated from them is committed
//! (ADR 0040) and exposed as one module per schema, so a normal build needs no schema compiler.
//!
//! * [`common_capnp`] - shared types (ids, envelope, change feed, provider-rule types)
//! * [`tasks_capnp`] - tasks (owner `fs-tasks`)
//! * [`messenger_capnp`] - messages (owner `fs-messenger`)
//! * [`hub_capnp`] - hub (owner `fs-hub`)
//! * [`people_capnp`] - people and roles (owner `fs-people`)
//!
//! [`provider`] holds the Rust side of the provider rule: [`Unsupported`] and [`Capabilities`].
//!
//! Pattern: Adapter, the generated modules are the wire form; Null Object, a default method that
//! answers `Unsupported` is the explicit "not possible".

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod common_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod hub_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod messenger_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod people_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod tasks_capnp;

pub mod provider;

pub use provider::{Capabilities, Support, UNSUPPORTED_PROBLEM_TYPE, Unsupported};
