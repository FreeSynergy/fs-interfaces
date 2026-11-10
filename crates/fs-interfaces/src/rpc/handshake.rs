//! Connecting to the server that fits: the version handshake of the client, once for every program. Pattern: Template method
//! (ask, compare, replace, ask again) + Strategy (`ServerStarter`: the program says how its server is started).
//!
//! After an update an old server can still be running (`fs-files serve` 0.0.2) and answer the new window with "method not
//! implemented", and the window stays black. [`connect_matching`] asks `Control.version`, compares it with what the program
//! expects, and, if the server differs or is older than the method itself, ends it (`Control.stop`; an older server that
//! does not know `Control` gets SIGTERM on Unix, found through the socket's peer credentials), waits until the socket is
//! free, starts the right server through the program's [`ServerStarter`], and connects again. If any step fails it answers a
//! typed [`HandshakeError`], which the window shows as a visible message (ADR 0065), never a black screen.

use std::fmt;
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use capnp::capability::FromClientHook;

use super::control::{PROTOCOL, io_error, stop_server};
use super::serve::{Connection, connect};
use super::transport::{Endpoint, System};
use crate::common_capnp::control;

/// How a program starts its own server (the one `fs-<program> serve` runs), detached: it must go on when the caller ends.
/// The program provides it; the closure `|| -> io::Result<()>` and [`CommandStarter`] do.
pub trait ServerStarter {
    /// Starts the server and returns without waiting for it.
    ///
    /// # Errors
    /// The server could not be started.
    fn start(&self) -> io::Result<()>;
}

impl<F: Fn() -> io::Result<()>> ServerStarter for F {
    fn start(&self) -> io::Result<()> {
        self()
    }
}

/// Starts a server by running a command (usually the program's own executable with `serve`), detached from the caller:
/// no input or output, its own process group on Unix.
pub struct CommandStarter {
    program: std::path::PathBuf,
    args: Vec<std::ffi::OsString>,
}

impl CommandStarter {
    /// The command `program args...`.
    pub fn new<I, S>(program: impl Into<std::path::PathBuf>, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

impl ServerStarter for CommandStarter {
    fn start(&self) -> io::Result<()> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0000_0008); // DETACHED_PROCESS
        // Not waited for: the server outlives this call. (A child that ends early is reaped by the system when we end.)
        command.spawn().map(drop)
    }
}

/// What the program (the client) expects of its server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    /// The program's version; empty = any (only the protocol is compared).
    pub version: String,
    /// The protocol level, [`PROTOCOL`] of this crate.
    pub protocol: u32,
}

impl Expected {
    /// The program's own version (`env!("CARGO_PKG_VERSION")`) and the protocol level of this crate.
    #[must_use]
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            protocol: PROTOCOL,
        }
    }

    /// Whether a server that says `info` is the one expected. A server that does not say its version is compared by the
    /// protocol alone.
    #[must_use]
    pub fn accepts(&self, info: &ServerInfo) -> bool {
        info.protocol == self.protocol
            && (self.version.is_empty() || info.version.is_empty() || info.version == self.version)
    }
}

/// What the running server said about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerInfo {
    /// The program's name; empty = not said.
    pub program: String,
    /// The program's version; empty = not said.
    pub version: String,
    /// The protocol level; `0` for a server older than `Control.version`.
    pub protocol: u32,
}

impl fmt::Display for ServerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.protocol == 0 {
            return f.write_str("an older server (it does not report its version)");
        }
        let version = if self.version.is_empty() {
            "unknown version"
        } else {
            &self.version
        };
        write!(f, "{version} (protocol {})", self.protocol)
    }
}

/// Why no matching server could be reached. A window shows this as a visible message (ADR 0065), with the text of
/// [`Display`](fmt::Display), and offers to try again.
#[derive(Debug)]
pub enum HandshakeError {
    /// A server runs, was replaced, and the new one still is not what is expected (the program started an old executable).
    ServerMismatch {
        /// What runs now.
        running: ServerInfo,
        /// What the client wanted.
        expected: Expected,
    },
    /// The running server (or none) could not be replaced: it did not stop, or the program could not start a new one.
    CannotRestart {
        /// The server that was running; `None` = none was.
        running: Option<ServerInfo>,
        /// What the client wanted.
        expected: Expected,
        /// What failed, in words for the person.
        reason: String,
    },
    /// The connection failed for another reason (for example another user serves the socket).
    Io(io::Error),
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerMismatch { running, expected } => write!(
                f,
                "The running server is {running}, but {} (protocol {}) is needed. Close the program completely and start it again.",
                if expected.version.is_empty() {
                    "this version"
                } else {
                    &expected.version
                },
                expected.protocol
            ),
            Self::CannotRestart {
                running,
                expected,
                reason,
            } => {
                let was = running
                    .as_ref()
                    .map_or_else(|| "no server".to_owned(), ToString::to_string);
                write!(
                    f,
                    "The server could not be replaced (running: {was}; needed: {} protocol {}): {reason}",
                    expected.version, expected.protocol
                )
            }
            Self::Io(e) => write!(f, "The server cannot be reached: {e}"),
        }
    }
}

impl std::error::Error for HandshakeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// The waits of [`connect_matching_with`].
#[derive(Clone, Copy, Debug)]
pub struct RestartTimes {
    /// For one call of the handshake and for the stop request.
    pub call: Duration,
    /// How long an old server gets to end (and free the socket).
    pub stop: Duration,
    /// How long a new server gets to listen.
    pub start: Duration,
}

impl Default for RestartTimes {
    fn default() -> Self {
        Self {
            call: Duration::from_secs(5),
            stop: Duration::from_secs(10),
            start: Duration::from_secs(10),
        }
    }
}

const POLL: Duration = Duration::from_millis(50);

/// [`connect_matching_with`] with the default waits.
///
/// # Errors
/// See [`HandshakeError`].
pub async fn connect_matching<C: FromClientHook>(
    socket: &Path,
    expected: &Expected,
    starter: &dyn ServerStarter,
) -> Result<Connection<C>, HandshakeError> {
    connect_matching_with(socket, expected, starter, RestartTimes::default()).await
}

/// Reaches the server on `socket` that is the one `expected`. A matching server is used as it is, without any interruption.
/// No server: `starter` starts one. A server that differs, or is older than `Control.version`: it is stopped, `starter`
/// starts the right one, and the call connects again. Must run on a `LocalSet`, like [`connect`].
///
/// # Errors
/// [`HandshakeError`]: the replacement failed (`CannotRestart`), did not help (`ServerMismatch`), or the connection failed.
pub async fn connect_matching_with<C: FromClientHook>(
    socket: &Path,
    expected: &Expected,
    starter: &dyn ServerStarter,
    times: RestartTimes,
) -> Result<Connection<C>, HandshakeError> {
    let running = match ask::<C>(socket, times.call).await {
        Ok((connection, info)) if expected.accepts(&info) => return Ok(connection),
        Ok((connection, info)) => {
            drop(connection);
            Some(info)
        }
        Err(e) if is_absent(&e) => None,
        Err(e) => return Err(HandshakeError::Io(e)),
    };
    let cannot = |reason: String| HandshakeError::CannotRestart {
        running: running.clone(),
        expected: expected.clone(),
        reason,
    };
    if running.is_some() {
        end_server(socket, times).await.map_err(cannot)?;
    }
    starter
        .start()
        .map_err(|e| cannot(format!("the server did not start: {e}")))?;
    let began = Instant::now();
    let (connection, info) = loop {
        match ask::<C>(socket, times.call).await {
            Ok(found) => break found,
            Err(e) if is_absent(&e) && began.elapsed() < times.start => {
                tokio::time::sleep(POLL).await;
            }
            Err(e) => return Err(cannot(format!("the new server cannot be reached: {e}"))),
        }
    };
    if expected.accepts(&info) {
        Ok(connection)
    } else {
        Err(HandshakeError::ServerMismatch {
            running: info,
            expected: expected.clone(),
        })
    }
}

fn is_absent(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}

/// Connects and asks `Control.version`; a server that does not know it is [`ServerInfo::default`].
async fn ask<C: FromClientHook>(
    socket: &Path,
    call: Duration,
) -> io::Result<(Connection<C>, ServerInfo)> {
    let timed_out = || io::Error::new(io::ErrorKind::TimedOut, "the server did not answer");
    let connection = tokio::time::timeout(call, connect::<C>(socket))
        .await
        .map_err(|_| timed_out())??;
    let control = control::Client::new(connection.client.as_client_hook().add_ref());
    let answer = tokio::time::timeout(call, control.version_request().send().promise)
        .await
        .map_err(|_| timed_out())?;
    let info = match answer {
        Ok(response) => {
            let r = response.get().map_err(io::Error::other)?;
            let text = |t: capnp::Result<capnp::text::Reader<'_>>| -> io::Result<String> {
                Ok(t.map_err(io::Error::other)?
                    .to_str()
                    .map_err(io::Error::other)?
                    .to_owned())
            };
            ServerInfo {
                program: text(r.get_program())?,
                version: text(r.get_version())?,
                protocol: r.get_protocol(),
            }
        }
        Err(e) => match io_error(e) {
            u if u.kind() == io::ErrorKind::Unsupported => ServerInfo::default(),
            other => return Err(other),
        },
    };
    Ok((connection, info))
}

/// Ends the running server and waits until its socket is free.
async fn end_server(socket: &Path, times: RestartTimes) -> Result<(), String> {
    let stopped = tokio::time::timeout(times.call, stop_server(socket))
        .await
        .unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::TimedOut, "no answer")));
    match stopped {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::Unsupported => {
            // Older than `Control`: only the system can end it.
            let pid = System::server_pid(socket).await.ok().flatten();
            let Some(pid) = pid else {
                return Err(
                    "the running server is too old to be stopped by request, and its process is unknown: end it by hand"
                        .to_owned(),
                );
            };
            System::terminate(pid)
                .map_err(|e| format!("the old server (process {pid}) could not be ended: {e}"))?;
        }
        Err(e) => return Err(format!("the running server did not accept the stop: {e}")),
    }
    let began = Instant::now();
    while System::connect(socket).await.is_ok() {
        if began.elapsed() >= times.stop {
            return Err("the old server did not end in time".to_owned());
        }
        tokio::time::sleep(POLL).await;
    }
    Ok(())
}
