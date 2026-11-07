//! The thread that owns the Cap'n Proto connection.
//!
//! Pattern: Command (see the module above) — a current-thread runtime and a `LocalSet` on a thread of
//! its own, a queue of commands in, the guard [`Reply`](super::reply::Reply) out. Dropping the
//! [`Worker`] closes the queue, lets the commands under way finish, closes the connection and joins.

use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::path::Path;

use crate::Capabilities;

use super::{Fault, Remote, Timeouts};

/// The thread of one connection, and the way to reach it. Any thread may call; calls overlap.
pub struct Worker<R: Remote> {
    server: &'static str,
    timeouts: Timeouts,
    remote: PhantomData<fn(R)>,
}

impl<R: Remote> Worker<R> {
    /// Start the thread, connect to the server on `socket`, run [`Remote::open`] and wait for it.
    /// `server` names the server in every error (`"people"` gives `people server: ...` and the
    /// thread `fs-people-client`).
    ///
    /// Blocks the calling thread until the server has answered; not for an async task of a
    /// single-threaded runtime.
    ///
    /// # Errors
    /// Nothing listens there, the server runs as another user, it does not answer the contract, or
    /// it stays silent for `timeouts.connect` (`TimedOut`, the text names the deadline).
    pub fn start(
        server: &'static str,
        socket: &Path,
        timeouts: Timeouts,
    ) -> io::Result<(Self, Capabilities)> {
        let _ = (server, socket, timeouts);
        todo!("interface only")
    }

    /// The deadlines of this connection.
    #[must_use]
    pub fn timeouts(&self) -> Timeouts {
        self.timeouts
    }

    /// Run `work` on the thread of the connection and wait for what it returns, for at most
    /// `timeouts.call`. The timer is that thread's, so the caller's runtime needs no clock.
    ///
    /// # Errors
    /// [`Fault`]: the thread is gone, the work ended without an answer, or the deadline ran out.
    pub async fn call<T, F, Fut>(&self, work: F) -> Result<T, Fault>
    where
        T: Send + 'static,
        F: FnOnce(R) -> Fut + Send + 'static,
        Fut: Future<Output = T> + 'static,
    {
        let _ = (self.server, work);
        todo!("interface only")
    }
}
