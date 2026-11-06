//! Where a program listens.
//!
//! Pattern: Factory + RAII — [`bind_socket`] builds a [`BoundSocket`]: a lock file held for the life
//! of the process decides who owns the path, so two starts cannot both win. Only a socket nobody
//! answers on is ever removed; dropping the [`BoundSocket`] removes the socket while still holding
//! the lock, so it can never remove a successor's.

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
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

/// Create a directory and the missing ones above it for this user alone, whatever the umask.
pub(super) fn create_private(dir: &Path) -> io::Result<()> {
    if dir.exists() {
        return Ok(());
    }
    DirBuilder::new().recursive(true).mode(0o700).create(dir)
}

fn refuse(d: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{} must be writable by this user alone", d.display()),
    )
}

/// Nobody else may be able to swap anything up the path: each directory above is this user's or
/// root's, and only its owner writes there, or it is sticky like `/tmp`, where others cannot rename
/// what is ours. `dir` must be canonical, so a symlink in the path cannot hide one.
pub(super) fn check_ancestors(dir: &Path) -> io::Result<()> {
    let me = rustix::process::geteuid().as_raw();
    for ancestor in dir.ancestors().skip(1) {
        let meta = std::fs::metadata(ancestor)?;
        let foreign = meta.uid() != me && meta.uid() != 0;
        if foreign || (meta.mode() & 0o022 != 0 && meta.mode() & 0o1000 == 0) {
            return Err(refuse(ancestor));
        }
    }
    Ok(())
}

/// Reading is harmless, the socket is 0600; writing in the directory is not. Answers the canonical
/// path, the one that was checked and is the one to use.
fn prepare_directory(dir: &Path) -> io::Result<PathBuf> {
    create_private(dir)?;
    let dir = dir.canonicalize()?;
    let meta = std::fs::metadata(&dir)?;
    if meta.uid() != rustix::process::geteuid().as_raw() || meta.mode() & 0o022 != 0 {
        return Err(refuse(&dir));
    }
    check_ancestors(&dir)?;
    Ok(dir)
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
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no name", path.display()),
        )
    })?;
    let path = prepare_directory(dir)?.join(name);
    let lock = take_lock(&path)?;
    clear_stale(&path).await?;
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, PermissionsExt::from_mode(0o600))?;
    Ok(BoundSocket {
        listener,
        path,
        _lock: lock,
    })
}
