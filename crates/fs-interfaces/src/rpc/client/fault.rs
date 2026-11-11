//! Why a call got no answer from the server.
//!
//! Pattern: none, a plain data type: three reasons, each named with the server it was
//! about, so a client maps it to its own error with one line and the text stays the same in every program.

use std::fmt;
use std::time::Duration;

/// A call that got no answer from the server (as opposed to an answer that is an error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The thread of the connection has ended; nothing can be called any more.
    Gone {
        /// The server, as the client names it (`"people"`).
        server: &'static str,
    },
    /// The call was taken up but ended without an answer: it panicked, or the connection was closed
    /// under it (the worker was dropped; a write may then be left half done).
    Interrupted {
        /// The server, as the client names it.
        server: &'static str,
    },
    /// The server did not answer within the deadline. A write may still have happened.
    TimedOut {
        /// The server, as the client names it.
        server: &'static str,
        /// The deadline that ran out.
        limit: Duration,
    },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone { server } => write!(f, "{server} server: the connection is gone"),
            Self::Interrupted { server } => {
                write!(f, "{server} client: a call ended without an answer")
            }
            Self::TimedOut { server, limit } => {
                write!(f, "{server} server did not answer within {limit:?}")
            }
        }
    }
}

impl std::error::Error for Fault {}

impl Fault {
    /// Whether the server is out of reach for good (the connection thread ended): a window offers to connect again
    /// ([`connect_matching`](crate::rpc::connect_matching)) instead of showing a plain failure.
    #[must_use]
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, Self::Gone { .. })
    }
}

/// Whether a call failed because the connection ended under it (the server went away mid-session, for example after an
/// update replaced it): `Disconnected` in the protocol. A window reads this as "the server went away" and connects again; any
/// other error is the server's answer.
#[must_use]
pub fn connection_lost(e: &capnp::Error) -> bool {
    e.kind == capnp::ErrorKind::Disconnected
}
