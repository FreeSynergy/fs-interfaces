//! The answer of one command, which cannot be lost without a word.
//!
//! Pattern: RAII / Guard — made when the command starts running. If it is dropped after that without
//! having answered (the command panicked, or the thread was shut down under it), `Drop` answers
//! [`Fault::Interrupted`], instead of the caller seeing a silent "connection gone".

use tokio::sync::oneshot::Sender;

use super::Fault;

/// The way back to the caller of a command that is running.
pub(super) struct Reply<T> {
    to: Option<Sender<Result<T, Fault>>>,
    server: &'static str,
}

#[allow(dead_code, clippy::needless_pass_by_value)] // interface only, until the implementation
impl<T> Reply<T> {
    /// The command has started.
    pub(super) fn started(to: Sender<Result<T, Fault>>, server: &'static str) -> Self {
        let _ = (&to, server);
        todo!("interface only")
    }

    /// Answer.
    pub(super) fn send(self, answer: Result<T, Fault>) {
        let _ = (&self.to, self.server, answer);
        todo!("interface only")
    }
}
