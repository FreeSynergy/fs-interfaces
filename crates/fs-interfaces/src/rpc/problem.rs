//! RFC 9457 problems as RPC errors.
//!
//! The contracts answer failures as exceptions, and an exception only carries a kind and a text. So
//! a [`Problem`] travels inside the text behind a marker and comes back typed on the other side; a
//! consumer never has to read an error message to tell "name taken" from "not possible".
//!
//! An error too large for one message would end the connection instead of answering, so every text
//! that leaves here is cut to [`MAX_DETAIL_BYTES`].
//!
//! Pattern: Adapter — [`Problem`] <-> `capnp::Error`.

use std::fmt::Write as _;

use capnp::{Error, ErrorKind};

use crate::provider::{UNSUPPORTED_PROBLEM_TYPE, Unsupported};

/// Problem type of [`invalid_request`].
pub const INVALID_REQUEST: &str = "https://freesynergy.net/problems/invalid-request";
/// Problem type of [`too_large`].
pub const TOO_LARGE: &str = "https://freesynergy.net/problems/too-large";

/// Most bytes of a problem's texts and of [`failed`], cut on a character boundary.
pub const MAX_DETAIL_BYTES: usize = 64 * 1024;

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
    /// Cursor past what could not be sent, so a consumer can read on; `None` if nothing is behind it.
    /// Lives in the error text only: the wire `Problem` has no such member.
    pub next: Option<Vec<u8>>,
}

/// The longest start of `text` of at most [`MAX_DETAIL_BYTES`], for a text that goes into an answer.
#[must_use]
pub fn capped(text: &str) -> &str {
    let mut end = text.len().min(MAX_DETAIL_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn to_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(text, "{b:02x}");
    }
    text
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.is_ascii() || !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
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
            next: None,
        }
    }

    /// This problem with a cursor past what could not be sent.
    #[must_use]
    pub fn with_next(mut self, next: Option<Vec<u8>>) -> Self {
        self.next = next;
        self
    }

    /// This problem as an RPC error, its texts cut to [`MAX_DETAIL_BYTES`]. Status 501 becomes the
    /// exception kind `unimplemented`.
    #[must_use]
    pub fn into_error(self) -> Error {
        let clean = |text: &str| capped(text).replace(SEPARATOR, " ");
        let mut text = format!(
            "{MARKER}{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.status,
            clean(&self.problem_type),
            clean(&self.title),
            clean(&self.detail),
        );
        // Last, and only if there is one: a text without it reads as before.
        if let Some(next) = &self.next {
            text.push(SEPARATOR);
            text.push_str(&to_hex(next));
        }
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
        let mut parts = rest.splitn(5, SEPARATOR);
        let status = parts.next()?.parse().ok()?;
        Some(Self {
            problem_type: parts.next()?.to_string(),
            title: parts.next()?.to_string(),
            detail: parts.next()?.to_string(),
            status,
            next: parts.next().and_then(from_hex),
        })
    }

    /// Fill a wire `Problem`, its detail cut to [`MAX_DETAIL_BYTES`]. `next` stays behind.
    pub fn write(&self, mut out: crate::common_capnp::problem::Builder<'_>) {
        out.set_type(self.problem_type.as_str());
        out.set_title(self.title.as_str());
        out.set_detail(capped(&self.detail));
        out.set_status(self.status);
    }

    /// Read a wire `Problem`.
    ///
    /// # Errors
    /// The message is malformed.
    pub fn read(r: crate::common_capnp::problem::Reader<'_>) -> capnp::Result<Self> {
        Ok(Self {
            problem_type: r.get_type()?.to_string()?,
            title: r.get_title()?.to_string()?,
            detail: r.get_detail()?.to_string()?,
            status: r.get_status(),
            next: None,
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

/// The request itself cannot be accepted: a missing or malformed id, cursor, time or value (422).
pub fn invalid_request(detail: impl Into<String>) -> Error {
    Problem::new(INVALID_REQUEST, "Invalid request", detail, 422).into_error()
}

/// The request or its answer would not fit into one message (413).
pub fn too_large(detail: impl Into<String>) -> Error {
    Problem::new(TOO_LARGE, "Too large", detail, 413).into_error()
}

/// Any other failure of a provider, without a problem type of its own. Not for an RPC error from
/// elsewhere: a program that passes one on returns it unchanged, or its problem is lost. The text is
/// cut to [`MAX_DETAIL_BYTES`].
pub fn failed(error: impl std::fmt::Display) -> Error {
    Error::failed(capped(&error.to_string()).to_string())
}

/// `true` if the error is of the kind `unimplemented`, also without a [`Problem`] inside: a method
/// the server does not know at all, for example an older server.
#[must_use]
pub fn is_unimplemented(error: &Error) -> bool {
    matches!(error.kind, ErrorKind::Unimplemented)
}
