//! Where a program listens, on Windows: a named pipe `\\.\pipe\freesynergy-<sid>-<program>`, `<sid>` being the security
//! identifier of the user. The Windows side of [`Endpoint`].
//!
//! Pattern: Factory + RAII — [`bind_socket`] builds a [`BoundSocket`]. The first-instance rule of named pipes decides who owns
//! the name (a second start fails, like the lock file on Unix), and the pipe goes with the last handle, so there is no file to
//! leave behind. Every instance is open to this user alone (a descriptor with one entry), remote machines are refused, and
//! both ends look at the process on the other end (what `SO_PEERCRED` does on Unix): the server drops a client that is not
//! this user, the client refuses a server that is not.

mod acl;
mod name;
mod sid;
#[cfg(test)]
mod tests;

use std::future::Future;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer};
use tokio::sync::Mutex;

use super::endpoint::{Accepted, Endpoint};
use name::{companion_stem, default_name, pipe_name};

/// `ERROR_PIPE_BUSY`: every instance is in use at this moment; try again shortly.
const PIPE_BUSY: i32 = 231;
/// `ERROR_ACCESS_DENIED`: what creating a second "first instance" answers.
const ACCESS_DENIED: i32 = 5;
/// The caller hung up before the server took it: `ERROR_NO_DATA`, `ERROR_PIPE_NOT_CONNECTED`, `ERROR_BROKEN_PIPE`,
/// `ERROR_BAD_PIPE`.
const CALLER_GONE: [i32; 4] = [232, 233, 109, 230];
/// A client that finds no instance for a moment (the server is making the next one) asks again, this often, this far apart.
const NOT_FOUND_TRIES: u32 = 8;
const NOT_FOUND_WAIT: Duration = Duration::from_millis(10);

/// `\\.\pipe\freesynergy-<sid>-<program>`, the same place for every program.
#[must_use]
pub(super) fn default_socket(program: &str) -> Option<PathBuf> {
    let sid = sid::my_sid_text().ok()?;
    Some(PathBuf::from(default_name(&sid, program)))
}

fn my_pipe_name(path: &Path) -> io::Result<String> {
    Ok(pipe_name(&sid::my_sid_text()?, path))
}

/// A bound pipe, owned by this process until dropped. `next` is the instance that waits for the next caller; it is missing
/// for a moment when the system could not make a new one.
#[derive(Debug)]
pub struct BoundSocket {
    name: String,
    path: PathBuf,
    next: Mutex<Option<NamedPipeServer>>,
}

impl BoundSocket {
    /// The socket's path, as it was given.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn already_running(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AddrInUse,
        format!("already running on {}", path.display()),
    )
}

/// Bind the pipe.
///
/// # Errors
/// A live server owns the name (`AddrInUse`), or the system refuses.
#[allow(clippy::unused_async)] // the same shape as the Unix side, where binding waits
pub(super) async fn bind_socket(path: &Path) -> io::Result<BoundSocket> {
    let name = my_pipe_name(path)?;
    let first = acl::create_pipe(&name, true).map_err(|e| match e.raw_os_error() {
        Some(ACCESS_DENIED | PIPE_BUSY) => already_running(path),
        _ => e,
    })?;
    Ok(BoundSocket {
        name,
        path: path.to_owned(),
        next: Mutex::new(Some(first)),
    })
}

/// Is `error` that the caller hung up?
fn caller_gone(error: &io::Error) -> bool {
    error
        .raw_os_error()
        .is_some_and(|c| CALLER_GONE.contains(&c))
}

/// A folder of this user's own for the files that belong to a pipe: below `%LOCALAPPDATA%`, else in a private folder of the
/// temporary one.
fn run_dir() -> PathBuf {
    let local = std::env::var_os("LOCALAPPDATA")
        .filter(|v| !v.is_empty())
        .map(|b| PathBuf::from(b).join("FreeSynergy").join("run"));
    if let Some(dir) = local.filter(|d| acl::make_private(d).is_ok()) {
        return dir;
    }
    let who = sid::my_sid_text().unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("FreeSynergy-run-{who}"));
    // If this fails too, the caller's attempt to make its file says why.
    let _ = acl::make_private(&dir);
    dir
}

/// `\\?\C:\x` as `C:\x`: what a program shows and compares is the plain form.
fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path,
    }
}

/// The Windows transport.
pub struct Windows;

impl Endpoint for Windows {
    type ServerStream = NamedPipeServer;
    type ClientStream = NamedPipeClient;
    type Bound = BoundSocket;

    fn default_socket(program: &str) -> Option<PathBuf> {
        default_socket(program)
    }

    fn companion(socket: &Path, extension: &str) -> PathBuf {
        let stem = my_pipe_name(socket).map_or_else(
            |_| companion_stem(&socket.to_string_lossy()),
            |n| companion_stem(&n),
        );
        run_dir().join(format!("{stem}.{extension}"))
    }

    fn default_data_dir(program: &str) -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
        Some(PathBuf::from(base).join("FreeSynergy").join(program))
    }

    fn prepare_data_dir(dir: &Path) -> io::Result<PathBuf> {
        acl::make_private(dir)?;
        dir.canonicalize().map(plain)
    }

    async fn bind(path: &Path) -> io::Result<BoundSocket> {
        bind_socket(path).await
    }

    async fn accept(bound: &BoundSocket) -> io::Result<Accepted<NamedPipeServer>> {
        let mut slot = bound.next.lock().await;
        if slot.is_none() {
            match acl::create_pipe(&bound.name, false) {
                Ok(instance) => *slot = Some(instance),
                Err(_) => return Ok(Accepted::Pause),
            }
        }
        let Some(instance) = slot.as_ref() else {
            return Ok(Accepted::Pause);
        };
        match instance.connect().await {
            Ok(()) => {}
            Err(e) if caller_gone(&e) => {
                // This instance is spent: a new one takes its place.
                *slot = None;
                return Ok(Accepted::Skip);
            }
            Err(e) => return Err(e),
        }
        // The next instance exists before this one is handed over, so the name is never without one for longer than this;
        // if the system cannot make it now, the next `accept` pauses and tries again.
        let next = acl::create_pipe(&bound.name, false).ok();
        let Some(served) = std::mem::replace(&mut *slot, next) else {
            return Ok(Accepted::Pause);
        };
        if sid::client_is_me(served.as_raw_handle()) {
            Ok(Accepted::Peer(served))
        } else {
            Ok(Accepted::Skip)
        }
    }

    #[allow(clippy::unused_async_trait_impl)] // the trait says async; a pipe has nothing to ask
    async fn server_pid(_path: &Path) -> io::Result<Option<u32>> {
        Ok(None)
    }

    fn terminate(_pid: u32) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows has no signal to end another program",
        ))
    }

    async fn connect(path: &Path) -> io::Result<NamedPipeClient> {
        let name = my_pipe_name(path)?;
        let (mut busy, mut missing) = (0, 0);
        let client = loop {
            match ClientOptions::new().open(&name) {
                Err(e) if e.raw_os_error() == Some(PIPE_BUSY) && busy < 100 => {
                    busy += 1;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound && missing < NOT_FOUND_TRIES => {
                    missing += 1;
                    tokio::time::sleep(NOT_FOUND_WAIT).await;
                }
                Err(e) if e.raw_os_error() == Some(PIPE_BUSY) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("{} is busy", path.display()),
                    ));
                }
                other => break other?,
            }
        };
        if !sid::server_is_me(client.as_raw_handle()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} is served by another user", path.display()),
            ));
        }
        Ok(client)
    }

    fn stop_signal() -> io::Result<impl Future<Output = ()>> {
        use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close, ctrl_shutdown};
        // Registered now, so a stop that comes while the program opens its store is remembered.
        let (mut c, mut b) = (ctrl_c()?, ctrl_break()?);
        let (mut close, mut shutdown) = (ctrl_close()?, ctrl_shutdown()?);
        Ok(async move {
            tokio::select! {
                _ = c.recv() => {}
                _ = b.recv() => {}
                _ = close.recv() => {}
                _ = shutdown.recv() => {}
            }
        })
    }
}
