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

impl<T> Reply<T> {
    /// The command has started.
    pub(super) fn started(to: Sender<Result<T, Fault>>, server: &'static str) -> Self {
        Self {
            to: Some(to),
            server,
        }
    }

    /// Answer.
    pub(super) fn send(mut self, answer: Result<T, Fault>) {
        if let Some(to) = self.to.take() {
            let _ = to.send(answer);
        }
    }
}

impl<T> Drop for Reply<T> {
    fn drop(&mut self) {
        if let Some(to) = self.to.take() {
            let _ = to.send(Err(Fault::Interrupted {
                server: self.server,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot::channel;

    use super::*;

    #[test]
    fn dropped_after_it_started_it_answers_interrupted() {
        let (to, mut from) = channel::<Result<u8, Fault>>();
        drop(Reply::started(to, "x"));
        assert_eq!(
            from.try_recv().unwrap().unwrap_err(),
            Fault::Interrupted { server: "x" }
        );
    }

    #[test]
    fn once_answered_it_does_not_answer_again() {
        let (to, mut from) = channel::<Result<u8, Fault>>();
        Reply::started(to, "x").send(Ok(7));
        assert_eq!(from.try_recv().unwrap().unwrap(), 7);
    }
}
