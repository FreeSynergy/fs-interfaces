//! The two entry points every program uses to say where it listens. Pattern: Facade over [`Endpoint`] of this system.

use std::io;
use std::path::{Path, PathBuf};

use super::{Endpoint, System};

/// A bound socket, owned by this process until dropped.
pub type BoundSocket = <System as Endpoint>::Bound;

/// Where a program listens when nobody says otherwise: `$XDG_RUNTIME_DIR/freesynergy/<program>.sock` on Unix,
/// `\\.\pipe\freesynergy-<user>-<program>` on Windows. The same place for every program.
#[must_use]
pub fn default_socket(program: &str) -> Option<PathBuf> {
    System::default_socket(program)
}

/// A file that belongs to the socket (`x.sock` and its `lease` or `log`): next to the socket on Unix; in the person's own
/// folder `%LOCALAPPDATA%\\FreeSynergy\\run` on Windows, where a pipe has no folder to put it in.
#[must_use]
pub fn companion_path(socket: &Path, extension: &str) -> PathBuf {
    System::companion(socket, extension)
}

/// Bind the socket. A lock decides who owns it, so two starts cannot both win.
///
/// # Errors
/// A live server owns it (`AddrInUse`), something else is at the path, others can write in the directory (Unix), or the
/// system refuses.
pub async fn bind_socket(path: &Path) -> io::Result<BoundSocket> {
    System::bind(path).await
}
