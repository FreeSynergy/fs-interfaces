//! Where a program listens.
//!
//! Pattern: Factory — a listener from a path: refuses a second live server, removes a stale file,
//! creates the directory 0700 and the socket 0600.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tokio::net::{UnixListener, UnixStream};

/// `$XDG_RUNTIME_DIR/freesynergy/<program>.sock`, the same place for every program.
#[must_use]
pub fn default_socket(program: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(
        PathBuf::from(dir)
            .join("freesynergy")
            .join(format!("{program}.sock")),
    )
}

/// Bind the socket.
///
/// # Errors
/// A live server already owns it (`AddrInUse`), or the file system refuses.
pub async fn bind_socket(path: &Path) -> io::Result<UnixListener> {
    if let Some(dir) = path.parent() {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
            std::fs::set_permissions(dir, PermissionsExt::from_mode(0o700))?;
        }
    }
    if path.exists() {
        if UnixStream::connect(path).await.is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("already running on {}", path.display()),
            ));
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, PermissionsExt::from_mode(0o600))?;
    Ok(listener)
}
