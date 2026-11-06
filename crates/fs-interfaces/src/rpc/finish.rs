//! Writes that finish, whatever the client does.
//!
//! `capnp-rpc` drops a method's future when the client drops the call or the connection, so a write of
//! several steps could stop halfway without anyone being told. Such a method reads its parameters,
//! then hands the work to [`run_to_end`]: it runs on its own task, and a panic in it answers an error
//! instead of ending the connection with all its capabilities.
//!
//! Pattern: Command (detached execution).

use std::future::Future;

/// Run `work` on its own local task to its end, even if the caller goes away.
///
/// # Errors
/// `work` panicked.
pub async fn run_to_end<T: 'static>(work: impl Future<Output = T> + 'static) -> capnp::Result<T> {
    tokio::task::spawn_local(work)
        .await
        .map_err(|e| capnp::Error::failed(format!("the operation did not finish: {e}")))
}
