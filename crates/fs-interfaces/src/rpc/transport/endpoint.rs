//! Pattern: Strategy (the transport of the system). Unix: a Unix socket, `SO_PEERCRED`, SIGTERM. Windows: a named pipe, the
//! first-instance rule, Ctrl-C. A program's server and client never name either: they go through [`System`](super::System).

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncRead, AsyncWrite};

/// What the listener of a server found when it looked at the next caller.
pub enum Accepted<S> {
    /// A caller that is this user: serve it.
    Peer(S),
    /// A caller that is somebody else, or that gave up: dropped.
    Skip,
    /// The system is out of descriptors or memory for now: wait a little, then ask again.
    Pause,
}

/// The transport of one system.
pub trait Endpoint {
    /// The server end of a connection.
    type ServerStream: AsyncRead + AsyncWrite + Unpin + 'static;
    /// The client end of a connection.
    type ClientStream: AsyncRead + AsyncWrite + Unpin + 'static;
    /// The place that is bound.
    type Bound;

    /// Where the server of `program` listens when nobody says otherwise.
    fn default_socket(program: &str) -> Option<PathBuf>;

    /// Where the data of `program` lives when nobody says otherwise.
    fn default_data_dir(program: &str) -> Option<PathBuf>;

    /// Makes the data directory for this user alone and answers the path to use (see `prepare_data_dir`).
    ///
    /// # Errors
    /// Others have access (`PermissionDenied`), or the file system refuses.
    fn prepare_data_dir(dir: &Path) -> io::Result<PathBuf>;

    /// Binds `path`, so that a second start never touches the data of the first.
    fn bind(path: &Path) -> impl Future<Output = io::Result<Self::Bound>>;

    /// Waits for the next caller.
    ///
    /// # Errors
    /// The listener failed in a way that will not pass.
    fn accept(
        bound: &Self::Bound,
    ) -> impl Future<Output = io::Result<Accepted<Self::ServerStream>>>;

    /// Reaches the server on `path`; refuses one that another user runs (`PermissionDenied`).
    fn connect(path: &Path) -> impl Future<Output = io::Result<Self::ClientStream>>;

    /// Resolves when the person or the system asks the program to end (Ctrl-C, SIGTERM, a closed console).
    ///
    /// # Errors
    /// The handlers could not be registered.
    fn stop_signal() -> io::Result<impl Future<Output = ()>>;
}
