//! The shared server and client side of every contract (feature `rpc`).
//!
//! Every program serves its contract the same way: a Unix socket per program, one two-party RPC
//! system per connection, RFC 9457 problems as RPC errors, cursor pages, a change feed. All of that
//! lives here, once, so a program's server only translates its own methods (ADR 0043 §4). Everything
//! here runs on one thread inside a `tokio::task::LocalSet`, as `capnp-rpc` requires.
//!
//! Pattern: Facade (declarations).

mod envelope;
mod feed;
mod ids;
mod page;
mod problem;
mod serve;
mod socket;

pub use envelope::{read_envelope, write_envelope};
pub use feed::{Notice, forward, write_change};
pub use ids::{from_millis, read_object_id, read_uuid, to_millis, write_object_id, write_uuid};
pub use page::{DEFAULT_PAGE_SIZE, Page};
pub use problem::{Problem, failed, is_unimplemented, unsupported};
pub use serve::{connect, run_until_signal, serve};
pub use socket::{bind_socket, default_socket};
