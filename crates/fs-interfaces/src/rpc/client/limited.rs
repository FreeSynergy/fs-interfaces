//! One deadline for any call on any connection, the same for every program.
//!
//! Pattern: Decorator (a function that wraps a future) — a call on a bare [`Connection`](crate::rpc::Connection)
//! (the way `fs-files` talks to its server after `connect_matching`) waits for ever if the server hangs, and the
//! window freezes. [`call_limited`] puts the deadline around it. The [`Worker`](super::Worker) uses the very same
//! function for each of its calls, so there is one place that decides what a timeout is.
//!
//! A call that ran out of time is dropped; `capnp-rpc` then tells the server it is finished, and the connection stays
//! usable for the next call. A write may still have happened (see [`CALL_TIMEOUT`](super::CALL_TIMEOUT)).

use std::future::Future;
use std::time::Duration;

use super::Fault;

/// Run `call` (a request's `send().promise`, or any future that talks to the server) for at most `limit`.
/// The timer is the one of the runtime that polls this future, so that runtime needs `enable_time`.
///
/// Long calls (a transfer) pass a longer `limit` ([`Timeouts::call`](super::Timeouts) is the default), or
/// call again for each step.
///
/// # Errors
/// [`Fault::TimedOut`] naming `server` and `limit`.
pub async fn call_limited<T>(
    server: &'static str,
    limit: Duration,
    call: impl Future<Output = T>,
) -> Result<T, Fault> {
    tokio::time::timeout(limit, call)
        .await
        .map_err(|_| Fault::TimedOut { server, limit })
}
