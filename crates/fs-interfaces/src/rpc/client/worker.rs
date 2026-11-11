//! The thread that owns the Cap'n Proto connection.
//!
//! Pattern: Command (see the module above) — a current-thread runtime and a `LocalSet` on a thread of
//! its own, a queue of commands in, the guard [`Reply`](super::reply::Reply) out.
//!
//! Dropping the [`Worker`] closes the queue and the connection first: a command under way gets
//! `Disconnected` from the server's side and ends (a write may be cut half done; the server finishes
//! what it has begun). Then the drop joins the thread, which waits for every command to end. A command
//! that waits on nothing from the connection (a timer, a lock) is not cut: the drop then waits for
//! its call deadline. Dropped on the connection thread itself (a command held the last handle),
//! nothing is joined.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;

use crate::Capabilities;
use crate::rpc::connect;

use super::reply::Reply;
use super::{Fault, Remote, Timeouts, call_limited};

/// A command for the thread of the connection.
type Job<R> = Box<dyn FnOnce(R) -> Pin<Box<dyn Future<Output = ()>>> + Send>;

/// The thread of one connection, and the way to reach it. Any thread may call; calls overlap.
pub struct Worker<R: Remote> {
    server: &'static str,
    timeouts: Timeouts,
    jobs: UnboundedSender<Job<R>>,
    thread: Option<JoinHandle<()>>,
}

fn other(server: &str, e: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("{server} server: {e}"))
}

/// The connection, and what the thread learned on it.
async fn handshake<R: Remote>(
    server: &'static str,
    socket: &Path,
) -> io::Result<(R, Capabilities, impl Sized)> {
    let connection = connect::<R::Root>(socket).await?;
    let root = connection.client.clone();
    let (remote, capabilities) = R::open(root).await.map_err(|e| other(server, e))?;
    Ok((remote, capabilities, connection))
}

fn run<R: Remote>(
    server: &'static str,
    socket: &Path,
    limit: Duration,
    mut queue: tokio::sync::mpsc::UnboundedReceiver<Job<R>>,
    ready: &mpsc::Sender<io::Result<Capabilities>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return drop(ready.send(Err(e))),
    };
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let shaken = tokio::time::timeout(limit, handshake::<R>(server, socket)).await;
        let (remote, capabilities, connection) = match shaken {
            Ok(Ok(done)) => done,
            Ok(Err(e)) => return drop(ready.send(Err(e))),
            Err(_) => {
                let silent = Fault::TimedOut { server, limit };
                return drop(ready.send(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    silent.to_string(),
                ))));
            }
        };
        let _ = ready.send(Ok(capabilities));
        while let Some(job) = queue.recv().await {
            tokio::task::spawn_local(job(remote.clone()));
        }
        drop(remote);
        drop(connection);
    });
    // Let the disconnect and the commands still under way finish.
    runtime.block_on(local);
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
        let (jobs, queue) = unbounded_channel::<Job<R>>();
        let (ready, answer) = mpsc::channel::<io::Result<Capabilities>>();
        let socket: PathBuf = socket.to_path_buf();
        let thread = std::thread::Builder::new()
            .name(format!("fs-{server}-client"))
            .spawn(move || run::<R>(server, &socket, timeouts.connect, queue, &ready))?;
        let capabilities = answer
            .recv()
            .map_err(|_| other(server, "the connection thread ended"))??;
        Ok((
            Self {
                server,
                timeouts,
                jobs,
                thread: Some(thread),
            },
            capabilities,
        ))
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
        self.call_within(self.timeouts.call, work).await
    }

    /// [`call`](Self::call) with a deadline of its own, for a call that may take longer (a transfer) or must be quick.
    ///
    /// # Errors
    /// As for [`call`](Self::call), with `limit` as the deadline.
    pub async fn call_within<T, F, Fut>(&self, limit: Duration, work: F) -> Result<T, Fault>
    where
        T: Send + 'static,
        F: FnOnce(R) -> Fut + Send + 'static,
        Fut: Future<Output = T> + 'static,
    {
        let server = self.server;
        let gone = Fault::Gone { server };
        let (to, answer) = oneshot::channel();
        let job: Job<R> = Box::new(move |remote| {
            Box::pin(async move {
                let reply = Reply::started(to, server);
                reply.send(call_limited(server, limit, work(remote)).await);
            })
        });
        self.jobs.send(job).map_err(|_| gone.clone())?;
        answer.await.map_err(|_| gone)?
    }
}

impl<R: Remote> Drop for Worker<R> {
    /// Close the queue and the connection, then join the thread. Commands under way are cut: they
    /// get `Disconnected`, and a write among them may be left half done. A command that does not wait
    /// on the connection keeps the drop waiting up to the call deadline. On the connection thread
    /// itself nothing is joined.
    fn drop(&mut self) {
        // The sender goes with `self.jobs` after this; close the queue first by swapping it out.
        let (closed, _) = unbounded_channel();
        drop(std::mem::replace(&mut self.jobs, closed));
        if let Some(thread) = self.thread.take() {
            // A command can hold the last handle: then this runs on the thread, which cannot join itself.
            if thread.thread().id() != std::thread::current().id() {
                let _ = thread.join();
            }
        }
    }
}
