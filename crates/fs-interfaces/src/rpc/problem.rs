//! RFC 9457 problems as RPC errors.
//!
//! The contracts answer failures as exceptions, and an exception only carries a kind and a text. So
//! a [`Problem`] travels inside the text behind a marker and comes back typed on the other side; a
//! consumer never has to read an error message to tell "name taken" from "not possible".
//!
//! Pattern: Adapter — [`Problem`] <-> `capnp::Error`.

use capnp::{Error, ErrorKind};

use crate::provider::{UNSUPPORTED_PROBLEM_TYPE, Unsupported};

const MARKER: &str = "fs-problem\u{1f}";
const SEPARATOR: char = '\u{1f}';

/// One RFC 9457 problem (`Problem` in `common.capnp`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The problem type URI.
    pub problem_type: String,
    /// Short summary of the type.
    pub title: String,
    /// What happened this time. For "not possible": `capability: reason`.
    pub detail: String,
    /// HTTP-like status.
    pub status: u16,
}

impl Problem {
    /// A problem of `problem_type`.
    pub fn new(
        problem_type: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
        status: u16,
    ) -> Self {
        Self {
            problem_type: problem_type.into(),
            title: title.into(),
            detail: detail.into(),
            status,
        }
    }

    /// This problem as an RPC error. Status 501 becomes the exception kind `unimplemented`.
    #[must_use]
    pub fn into_error(self) -> Error {
        let text = format!(
            "{MARKER}{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.status,
            self.problem_type.replace(SEPARATOR, " "),
            self.title.replace(SEPARATOR, " "),
            self.detail.replace(SEPARATOR, " "),
        );
        if self.status == 501 {
            Error::unimplemented(text)
        } else {
            Error::failed(text)
        }
    }

    /// The problem inside an RPC error, if it carries one.
    #[must_use]
    pub fn from_error(error: &Error) -> Option<Self> {
        // The RPC layer may put this prefix in front; foreign text that only contains the marker is no problem.
        let mut text = error.extra.as_str();
        while let Some(rest) = text.strip_prefix("remote exception: ") {
            text = rest;
        }
        let rest = text.strip_prefix(MARKER)?;
        let mut parts = rest.splitn(4, SEPARATOR);
        let status = parts.next()?.parse().ok()?;
        Some(Self {
            problem_type: parts.next()?.to_string(),
            title: parts.next()?.to_string(),
            detail: parts.next()?.to_string(),
            status,
        })
    }

    /// Whether this is the explicit "not possible" of the provider rule (ADR 0045).
    #[must_use]
    pub fn is_unsupported(&self) -> bool {
        self.problem_type == UNSUPPORTED_PROBLEM_TYPE
    }
}

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

/// Any other failure of a provider, without a problem type of its own.
pub fn failed(error: impl std::fmt::Display) -> Error {
    Error::failed(error.to_string())
}

/// `true` if the error is of the kind `unimplemented`, also without a [`Problem`] inside: a method
/// the server does not know at all, for example an older server.
#[must_use]
pub fn is_unimplemented(error: &Error) -> bool {
    matches!(error.kind, ErrorKind::Unimplemented)
}
