//! Reading back a [`Problem`] and the helpers every client needs to do it.
//!
//! Pattern: Adapter — the inverse of what a server did in `Problem::into_error`: an RPC error becomes
//! a [`Failure`] with the problem typed, and the small readers below take the facts out of its
//! text. Which variant of a program's own error a problem type stands for stays with that program.

use capnp::Error;
use fs_store::ObjectId;

use crate::{Unsupported, rpc::Problem};

/// Start of every problem type URI of FreeSynergy.
pub const PROBLEM_BASE: &str = "https://freesynergy.net/problems/";

/// What the server (or the wire) said about a failed call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// An RFC 9457 problem.
    Problem(Problem),
    /// The server does not know this method of the contract at all (an older server).
    Unimplemented,
    /// Anything else, as text (a lost connection, a malformed message).
    Other(String),
}

impl Failure {
    /// Sort out an RPC error. A problem comes first: status 501 has the kind `unimplemented` too.
    #[must_use]
    pub fn of(error: &Error) -> Self {
        let _ = error;
        todo!("interface only")
    }
}

/// The type of `problem` without [`PROBLEM_BASE`] (`people/not-found`, `too-large`). A type of
/// another base stays whole.
#[must_use]
pub fn local_type(problem: &Problem) -> &str {
    let _ = problem;
    todo!("interface only")
}

/// The first run of digits in `text`, 0 if there is none (`too-large` texts name a size).
#[must_use]
pub fn first_number(text: &str) -> usize {
    let _ = text;
    todo!("interface only")
}

/// The first word of `text` that is an object id.
#[must_use]
pub fn object_id_in(text: &str) -> Option<ObjectId> {
    let _ = text;
    todo!("interface only")
}

/// `detail` without the store's `object store: invalid input: ` in front, which the server's error
/// text carries and a client's own text must not repeat.
#[must_use]
pub fn without_store_prefix(detail: &str) -> &str {
    let _ = detail;
    todo!("interface only")
}

/// The "not possible" of the provider rule from a problem's detail, which reads
/// `capability: reason`. Without the colon the whole text is the reason of `default_capability`.
#[must_use]
pub fn unsupported_of(detail: &str, default_capability: &str) -> Unsupported {
    let _ = (detail, default_capability);
    todo!("interface only")
}
