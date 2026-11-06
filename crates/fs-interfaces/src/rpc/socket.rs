//! Where a program listens.
//!
//! Pattern: Factory + RAII — [`bind_socket`] builds a [`BoundSocket`]: a lock file held for the life
//! of the process decides who owns the path, so two starts cannot both win. Only a socket nobody
//! answers on is ever removed; dropping the [`BoundSocket`] removes the socket while still holding
//! the lock, so it can never remove a successor's.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use tokio::net::{UnixListener, UnixStream};

/// `$XDG_RUNTIME_DIR/freesynergy/<program>.sock`, the same place for every program.
#[must_use]
pub fn default_socket(program: &str) -> Option<PathBuf> {
    // An empty variable counts as unset (XDG Base Directory).
    let dir = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty())?;
    Some(
        PathBuf::from(dir)
            .join("freesynergy")
            .join(format!("{program}.sock")),
    )
}

/// A bound socket, owned by this process until dropped.
#[derive(Debug)]
pub struct BoundSocket {
    pub(super) listener: UnixListener,
    path: PathBuf,
    _lock: File,
}

impl BoundSocket {
    /// The socket's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BoundSocket {
    fn drop(&mut self) {
        // Nothing to do if it is gone already; the fields, and with them the lock, drop after this.
        let _ = std::fs::remove_file(&self.path);
    }
}

fn already_running(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AddrInUse,
        format!("already running on {}", path.display()),
    )
}

/// Nobody else may be able to swap the socket or the lock: the directory is this user's, only this
/// user writes there, and the same holds up the path (root's directories count, and a sticky one
/// like `/tmp`, where others cannot rename what is ours). Reading is harmless, the socket is 0600.
fn prepare_directory(dir: &Path) -> io::Result<()> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
        std::fs::set_permissions(dir, PermissionsExt::from_mode(0o700))?;
    }
    let me = rustix::process::geteuid().as_raw();
    let refuse = |d: &Path| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} must be writable by this user alone", d.display()),
        )
    };
    let dir = dir.canonicalize()?;
    let meta = std::fs::metadata(&dir)?;
    if meta.uid() != me || meta.mode() & 0o022 != 0 {
        return Err(refuse(&dir));
    }
    for ancestor in dir.ancestors().skip(1) {
        let meta = std::fs::metadata(ancestor)?;
        let foreign = meta.uid() != me && meta.uid() != 0;
        if foreign || (meta.mode() & 0o022 != 0 && meta.mode() & 0o1000 == 0) {
            return Err(refuse(ancestor));
        }
    }
    Ok(())
}

fn take_lock(path: &Path) -> io::Result<File> {
    let mut name = path.as_os_str().to_owned();
    name.push(".lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(PathBuf::from(name))?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(already_running(path)),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// Remove what is at `path` only if it is a socket that really has nobody behind it.
async fn clear_stale(path: &Path) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if !meta.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists and is not a socket", path.display()),
        ));
    }
    match UnixStream::connect(path).await {
        Ok(_) => Err(already_running(path)),
        Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => std::fs::remove_file(path),
        Err(e) => Err(e),
    }
}

/// Bind the socket.
///
/// # Errors
/// A live server owns it (`AddrInUse`), something else is at the path, others can write in the
/// directory, or the file system refuses.
pub async fn bind_socket(path: &Path) -> io::Result<BoundSocket> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    prepare_directory(dir)?;
    let lock = take_lock(path)?;
    clear_stale(path).await?;
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, PermissionsExt::from_mode(0o600))?;
    Ok(BoundSocket {
        listener,
        path: path.to_path_buf(),
        _lock: lock,
    })
}
