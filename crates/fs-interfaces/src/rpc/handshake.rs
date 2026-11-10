//! Connecting to the server that fits: the version handshake of the client, once for every program. Pattern: Template method
//! (ask, compare, replace, ask again) + Strategy (`ServerStarter`: the program says how its server is started).
//!
//! After an update an old server can still be running (`fs-files serve` 0.0.2) and answer the new window with "method not
//! implemented", and the window stays black. [`connect_matching`] asks `Control.version`, compares it with what the program
//! expects, and, if the server differs or is older than the method itself, ends it (`Control.stop`; an older server that
//! does not know `Control` gets SIGTERM on Unix, found through the socket's peer credentials), waits until the socket is
//! free, starts the right server through the program's [`ServerStarter`], and connects again. If any step fails it answers a
//! typed [`HandshakeError`], which the window shows as a visible message (ADR 0065), never a black screen.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use capnp::capability::FromClientHook;

use super::control::{io_error, stop_server};
use super::expected::{Expected, HandshakeError, RestartTimes, ServerInfo};
use super::serve::{Connection, connect};
use super::starter::ServerStarter;
use super::transport::{Endpoint, System};
use crate::common_capnp::control;

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
        // Another client may have replaced the server meanwhile (two windows after an update): then this is the one.
        if let Some(connection) = end_server::<C>(socket, expected, times)
            .await
            .map_err(cannot)?
        {
            return Ok(connection);
        }
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

/// Ends the running server and waits until its socket is free. Asks the version once more right before the stop, and watches
/// while it waits: a server that fits (started by another client in the meantime) is left alone and returned. A server that
/// is gone already counts as ended.
async fn end_server<C: FromClientHook>(
    socket: &Path,
    expected: &Expected,
    times: RestartTimes,
) -> Result<Option<Connection<C>>, String> {
    match ask::<C>(socket, times.call).await {
        Ok((connection, info)) if expected.accepts(&info) => return Ok(Some(connection)),
        Ok(_) => {}
        Err(e) if is_absent(&e) => return Ok(None),
        Err(e) => return Err(format!("the running server cannot be asked: {e}")),
    }
    let stopped = tokio::time::timeout(times.call, stop_server(socket))
        .await
        .unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::TimedOut, "no answer")));
    match stopped {
        Ok(()) => {}
        Err(e) if is_absent(&e) => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::Unsupported => {
            // Older than `Control` (also a server of `serve` alone): only the system can end it.
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
    loop {
        match ask::<C>(socket, times.call).await {
            Err(e) if is_absent(&e) => return Ok(None),
            Ok((connection, info)) if expected.accepts(&info) => return Ok(Some(connection)),
            _ => {}
        }
        if began.elapsed() >= times.stop {
            return Err("the old server did not end in time".to_owned());
        }
        tokio::time::sleep(POLL).await;
    }
}
