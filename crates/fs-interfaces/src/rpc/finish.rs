//! Writes that finish, whatever the client does.
//!
//! `capnp-rpc` drops a method's future when the client drops the call or the connection, so a write of
//! several steps could stop halfway without anyone being told. Such a method reads its parameters,
//! then hands the work to [`run_to_end`]: it runs on its own task, and a panic in it answers an error
//! instead of ending the connection with all its capabilities.
//!
//! The same holds when the program stops: dropping the `LocalSet` would cancel that task too. So the
//! tasks of this thread are tracked (one tracker per thread, as everything here runs on one thread) and
//! [`finish_underway`] waits for them. While it waits no new work begins: a client that sends more is
//! told the server is going away, instead of a write that the end of the wait could cut in half.
//!
//! Pattern: Command (detached execution) + Registry (the work under way on this thread).

use std::future::Future;
use std::io;
use std::time::Duration;

use tokio_util::task::TaskTracker;

use super::problem::failed;

/// How long `run_until_signal` waits for work under way after a signal. Below the 90 s after which
/// systemd kills a service by default, so the program can still say what did not finish.
pub const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

thread_local! {
    static UNDERWAY: TaskTracker = TaskTracker::new();
}

/// Run `work` on its own local task to its end, even if the caller goes away.
///
/// # Errors
/// `work` panicked, or the program is stopping and `work` did not begin (kind `disconnected`).
pub async fn run_to_end<T: 'static>(work: impl Future<Output = T> + 'static) -> capnp::Result<T> {
    let task =
        UNDERWAY.with(|underway| (!underway.is_closed()).then(|| underway.spawn_local(work)));
    let task = task.ok_or_else(|| capnp::Error::disconnected("the server is stopping".into()))?;
    task.await
        .map_err(|e| failed(format!("the operation did not finish: {e}")))
}

/// Wait up to `limit` for all [`run_to_end`] work of this thread. Refuses new work meanwhile. For a
/// program that does not stop through `run_until_signal`.
///
/// # Errors
/// `TimedOut`, naming how many did not finish: they end when the `LocalSet` is dropped.
pub async fn finish_underway(limit: Duration) -> io::Result<()> {
    let underway = UNDERWAY.with(TaskTracker::clone);
    underway.close();
    let waited = tokio::time::timeout(limit, underway.wait()).await;
    let left = underway.len();
    underway.reopen();
    waited.map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{left} writes did not finish within {limit:?}"),
        )
    })
}
