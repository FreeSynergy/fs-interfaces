//! What a client expects of its server, what the server said, and why no fitting server could be reached. Pattern: Value
//! objects; `HandshakeError` is the typed error a window shows (ADR 0065).

use std::fmt;
use std::io;
use std::time::Duration;

use super::control::PROTOCOL;

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
