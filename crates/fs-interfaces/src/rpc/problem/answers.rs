//! The ready-made problems every server answers with.
//!
//! Pattern: none, free functions that each build one fixed [`Problem`] and turn it into an error.

use capnp::{Error, ErrorKind};

use super::{Problem, capped};
use crate::provider::Unsupported;

/// Problem type of [`invalid_request`].
pub const INVALID_REQUEST: &str = "https://freesynergy.net/problems/invalid-request";
/// Problem type of [`too_large`].
pub const TOO_LARGE: &str = "https://freesynergy.net/problems/too-large";

/// The explicit "not possible" as an RPC error, with exception kind `unimplemented`.
#[must_use]
pub fn unsupported(not_possible: &Unsupported) -> Error {
    Problem::new(
        not_possible.problem_type(),
        "Not possible",
        format!("{}: {}", not_possible.capability(), not_possible.reason()),
        not_possible.status(),
    )
    .into_error()
}

/// The request itself cannot be accepted: a missing or malformed id, cursor, time or value (422).
pub fn invalid_request(detail: impl Into<String>) -> Error {
    Problem::new(INVALID_REQUEST, "Invalid request", detail, 422).into_error()
}

/// The request or its answer would not fit into one message (413), when the size is not known.
/// With the numbers at hand use [`too_large_by`].
pub fn too_large(detail: impl Into<String>) -> Error {
    Problem::new(TOO_LARGE, "Too large", detail, 413).into_error()
}

/// Like [`too_large`], and the problem carries `size` and `limit` (bytes) as the extensions
/// [`super::keys::SIZE`] and [`super::keys::LIMIT`], so a client reads them instead of the text.
pub fn too_large_by(detail: impl Into<String>, size: u64, limit: u64) -> Error {
    Problem::too_large(detail, size, limit).into_error()
}

/// Any other failure of a provider, without a problem type of its own. Not for an RPC error from
/// elsewhere: a program that passes one on returns it unchanged, or its problem is lost. The text is
/// cut to [`super::MAX_DETAIL_BYTES`].
pub fn failed(error: impl std::fmt::Display) -> Error {
    Error::failed(capped(&error.to_string()).to_string())
}

/// `true` if the error is of the kind `unimplemented`, also without a [`Problem`] inside: a method
/// the server does not know at all, for example an older server.
#[must_use]
pub fn is_unimplemented(error: &Error) -> bool {
    matches!(error.kind, ErrorKind::Unimplemented)
}
