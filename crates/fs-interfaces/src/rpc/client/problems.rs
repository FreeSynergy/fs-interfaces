//! Reading back a [`Problem`] and the helpers every client needs to do it.
//!
//! Pattern: Adapter — the inverse of what a server did in `Problem::into_error`: an RPC error becomes
//! a [`Failure`] with the problem typed, and the small readers below take the facts out of its
//! text. Which variant of a program's own error a problem type stands for stays with that program.

use capnp::Error;
use fs_store::ObjectId;

use crate::Unsupported;
use crate::rpc::{Problem, is_unimplemented};

const STORE_PREFIX: &str = "object store: invalid input: ";

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
        match Problem::from_error(error) {
            Some(problem) => Self::Problem(problem),
            None if is_unimplemented(error) => Self::Unimplemented,
            None => Self::Other(error.to_string()),
        }
    }
}

/// The type of `problem` without [`PROBLEM_BASE`] (`people/not-found`, `too-large`). A type of
/// another base stays whole.
#[must_use]
pub fn local_type(problem: &Problem) -> &str {
    problem
        .problem_type
        .strip_prefix(PROBLEM_BASE)
        .unwrap_or(&problem.problem_type)
}

/// The first run of digits in `text`, 0 if there is none (`too-large` texts name a size).
#[must_use]
pub fn first_number(text: &str) -> usize {
    let digits: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap_or(0)
}

/// The first word of `text` that is an object id.
#[must_use]
pub fn object_id_in(text: &str) -> Option<ObjectId> {
    text.split_whitespace()
        .find_map(|word| word.parse::<ObjectId>().ok())
}

/// `detail` without the store's `object store: invalid input: ` in front, which the server's error
/// text carries and a client's own text must not repeat.
#[must_use]
pub fn without_store_prefix(detail: &str) -> &str {
    detail.strip_prefix(STORE_PREFIX).unwrap_or(detail)
}

/// The "not possible" of the provider rule from a problem's detail, which reads
/// `capability: reason`. Without the colon the whole text is the reason of `default_capability`.
#[must_use]
pub fn unsupported_of(detail: &str, default_capability: &str) -> Unsupported {
    let (capability, reason) = detail
        .split_once(": ")
        .unwrap_or((default_capability, detail));
    Unsupported::new(capability, reason)
}
