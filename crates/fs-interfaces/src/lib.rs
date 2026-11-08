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
//! * [`groups_capnp`] - groups, and directories that mirror them (owner `fs-groups`)
//! * [`files_capnp`] - files, and backends that keep the bytes (owner `fs-files`)
//! * [`events_capnp`] - events (owner `fs-calendar`)
//! * [`places_capnp`] - places, and sources of map tiles (owner `fs-maps`)
//! * [`texts_capnp`] - texts (owner `fs-wiki`)
//! * [`surveys_capnp`] - surveys and decisions (owner `fs-surveys`)
//! * [`history_capnp`] - what happened, across programs (owner `fs-history`)
//! * [`resources_capnp`] - things, capacity, needs and offers (owner `fs-resources`)
//! * [`whiteboard_capnp`] - boards and their elements (owner `fs-whiteboard`)
//! * [`streaming_capnp`] - live streams and where they go (owner `fs-streaming`)
//! * [`store_capnp`] - install and update programs from Git sources (owner `fs-store`)
//!
//! [`provider`] holds the Rust side of the provider rule: [`Unsupported`] and [`Capabilities`].
//!
//! [`rpc`] (feature `rpc`) is the shared server and client side of every contract, so that each
//! program's server only translates its own methods.
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
pub mod events_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod files_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod groups_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod history_capnp;
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
pub mod places_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod resources_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod streaming_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod store_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod surveys_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod tasks_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod whiteboard_capnp;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
pub mod texts_capnp;

pub mod provider;
#[cfg(feature = "rpc")]
pub mod rpc;

pub use provider::{Capabilities, Support, UNSUPPORTED_PROBLEM_TYPE, Unsupported};
