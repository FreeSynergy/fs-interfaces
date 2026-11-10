//! Where a program listens, on Windows: a named pipe `\\.\pipe\freesynergy-<user>-<program>`. The Windows side of [`Endpoint`].
//!
//! Pattern: Factory + RAII — [`bind_socket`] builds a [`BoundSocket`]. The first-instance rule of named pipes decides who owns
//! the name (a second start fails, like the lock file on Unix), and the pipe goes with the last handle, so there is no file to
//! leave behind. Remote machines are refused (`reject_remote_clients`).
//!
//! Limit, said openly: the check "is the other end this user" that Unix does with `SO_PEERCRED` is not made here. The name
//! carries the user's name and the first instance is ours, which keeps others out in practice; a program of another user that
//! already holds the name before we start is not recognised by the client.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use tokio::sync::Mutex;

use super::endpoint::{Accepted, Endpoint};

/// `ERROR_PIPE_BUSY`: every instance is in use at this moment; try again shortly.
const PIPE_BUSY: i32 = 231;
/// `ERROR_ACCESS_DENIED`: what creating a second "first instance" answers.
const ACCESS_DENIED: i32 = 5;
/// The longest a pipe name may be is 256 characters.
const MAX_NAME: usize = 200;
const PREFIX: &str = r"\\.\pipe\";

fn clean(text: &str) -> String {
    let all: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    all[all.len().saturating_sub(MAX_NAME)..].to_owned()
}

/// `\\.\pipe\freesynergy-<user>-<program>`, the same place for every program.
#[must_use]
pub(super) fn default_socket(program: &str) -> Option<PathBuf> {
    let user = std::env::var("USERNAME").ok().filter(|u| !u.is_empty())?;
    Some(PathBuf::from(format!(
        "{PREFIX}freesynergy-{}-{}",
        clean(&user),
        clean(program)
    )))
}

/// A path that already is a pipe name stays; any other path (a `--socket` given as a file) becomes a pipe name made from it.
fn pipe_name(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.starts_with(PREFIX) {
        text.into_owned()
    } else {
        format!("{PREFIX}freesynergy-{}", clean(&text))
    }
}

/// A bound pipe, owned by this process until dropped.
#[derive(Debug)]
pub struct BoundSocket {
    name: String,
    path: PathBuf,
    next: Mutex<NamedPipeServer>,
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
    let name = pipe_name(path);
    let first = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)
        .map_err(|e| match e.raw_os_error() {
            Some(ACCESS_DENIED | PIPE_BUSY) => already_running(path),
            _ => e,
        })?;
    Ok(BoundSocket {
        name,
        path: path.to_owned(),
        next: Mutex::new(first),
    })
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
        let base = std::env::var_os("LOCALAPPDATA")
            .filter(|v| !v.is_empty())
            .map_or_else(std::env::temp_dir, PathBuf::from);
        let dir = base.join("FreeSynergy").join("run");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("{}.{extension}", clean(&pipe_name(socket))))
    }

    fn default_data_dir(program: &str) -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
        Some(PathBuf::from(base).join("FreeSynergy").join(program))
    }

    fn prepare_data_dir(dir: &Path) -> io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let user = std::env::var("USERNAME").map_err(io::Error::other)?;
        let domain = std::env::var("USERDOMAIN").unwrap_or_default();
        let who = if domain.is_empty() {
            user
        } else {
            format!("{domain}\\{user}")
        };
        // No inherited rights, full control for this person alone.
        let status = std::process::Command::new("icacls")
            .arg(dir)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("{who}:(OI)(CI)F"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "{} could not be made private (icacls: {status})",
                    dir.display()
                ),
            ));
        }
        dir.canonicalize()
    }

    async fn bind(path: &Path) -> io::Result<BoundSocket> {
        bind_socket(path).await
    }

    async fn accept(bound: &BoundSocket) -> io::Result<Accepted<NamedPipeServer>> {
        let mut slot = bound.next.lock().await;
        slot.connect().await?;
        let next = ServerOptions::new()
            .reject_remote_clients(true)
            .create(&bound.name)?;
        Ok(Accepted::Peer(std::mem::replace(&mut *slot, next)))
    }

    async fn connect(path: &Path) -> io::Result<NamedPipeClient> {
        let name = pipe_name(path);
        for _ in 0..100 {
            match ClientOptions::new().open(&name) {
                Err(e) if e.raw_os_error() == Some(PIPE_BUSY) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                other => return other,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("{} is busy", path.display()),
        ))
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
