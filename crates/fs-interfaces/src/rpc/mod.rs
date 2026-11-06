//! The shared server and client side of every contract (feature `rpc`).
//!
//! Every program serves its contract the same way: a Unix socket per program, one two-party RPC
//! system per connection, RFC 9457 problems as RPC errors, cursor pages, a change feed. All of that
//! lives here, once, so a program's server only translates its own methods (ADR 0043 §4). Everything
//! here runs on one thread inside a `tokio::task::LocalSet`, as `capnp-rpc` requires.
//!
//! A consumer ends a subscription by dropping the capability **and** the response it came in: the
//! response keeps its own reference. A `ChangeListener` must not answer `change` with an error: a stream
//! that failed once stays failed, and that ends the feed.
//!
//! The contracts' messages are limited ([`MAX_MESSAGE_BYTES`]); one larger message ends the whole
//! connection with all its capabilities. So a server never answers more than fits: pages are cut by
//! weight ([`Page::of_weighted`]), reads are clamped ([`clamp_read`]).
//!
//! A write of more than one step goes through [`run_to_end`], or a client that goes away leaves it half
//! done.
//!
//! Pattern: Facade (declarations).

mod envelope;
mod feed;
mod finish;
mod ids;
mod options;
mod page;
mod problem;
mod pump;
mod serve;
mod socket;

pub use envelope::{envelope_weight, read_envelope, write_envelope};
pub use feed::{
    ChangeReceiver, ChangeSender, FEED_CAPACITY, FEED_ENDED_PROBLEM_TYPE, FEED_STALL, Notice,
    change_channel, forward, write_change,
};
pub use finish::run_to_end;
pub use ids::{from_millis, read_object_id, read_uuid, to_millis, write_object_id, write_uuid};
pub use options::{LOCAL_OWNER, ServerOptions, default_data_dir, prepare_data_dir};
pub use page::{
    DEFAULT_PAGE_SIZE, MAX_INTAKE_BYTES, MAX_MESSAGE_BYTES, MAX_PAGE_BYTES, Page, check_answer,
    check_intake, clamp_read,
};
pub use problem::{
    INVALID_REQUEST, Problem, TOO_LARGE, failed, invalid_request, is_unimplemented, too_large,
    unsupported,
};
pub use serve::{
    Connection, ReadLimits, connect, connect_with, run_until_signal, serve, serve_with,
};
pub use socket::{BoundSocket, bind_socket, default_socket};
