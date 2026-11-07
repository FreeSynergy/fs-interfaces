//! Why a call got no answer from the server.
//!
//! Pattern: Value object — three reasons, each named with the server it was about, so a client maps
//! it to its own error with one line and the text stays the same in every program.

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
    /// The call was taken up but ended without an answer (it panicked, or the thread was shut down
    /// under it).
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
        let _ = f;
        todo!("interface only")
    }
}

impl std::error::Error for Fault {}
