//! A [`Problem`] as the text of an RPC error, and back.
//!
//! Pattern: Adapter — [`Problem`] <-> `capnp::Error`. The contracts answer failures as exceptions,
//! and an exception only carries a kind and a text, so the problem travels inside the text behind a
//! marker. An error too large for one message would end the connection, so every text that leaves
//! here is cut to [`MAX_DETAIL_BYTES`] and the extensions are limited (see `ext`).
//!
//! Text, fields separated by U+001F: `fs-problem<US>status<US>type<US>title<US>detail`, then the
//! optional `next` as hex, then any number of `ext<RS>key<RS>value` fields (RS = U+001E). Without
//! `next` the first extension takes its place; the hex can never start with `ext`. Separators inside
//! texts are replaced by a space. A reader ignores fields it does not understand, so a broken
//! extension costs only itself. A reader before 0.37 takes everything after `detail` as `next`: for
//! a text with extensions it finds no cursor, the rest it reads as before.

use std::fmt::Write as _;

use capnp::Error;

use super::Problem;
use super::ext::admit;

/// Most bytes of a problem's texts and of `failed`, cut on a character boundary.
pub const MAX_DETAIL_BYTES: usize = 64 * 1024;

const MARKER: &str = "fs-problem\u{1f}";
const SEPARATOR: char = '\u{1f}';
const INNER: char = '\u{1e}';
const EXT_PREFIX: &str = "ext\u{1e}";

/// The longest start of `text` of at most [`MAX_DETAIL_BYTES`], for a text that goes into an answer.
#[must_use]
pub fn capped(text: &str) -> &str {
    let mut end = text.len().min(MAX_DETAIL_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// `text` with both separators replaced by a space (same length in bytes), for extensions.
pub(super) fn clean(text: &str) -> String {
    text.replace([SEPARATOR, INNER], " ")
}

/// `text` cut and with the field separator replaced, for the head fields: as 0.36.0 did, so the
/// text of a problem without extensions stays byte-equal.
fn clean_head(text: &str) -> String {
    capped(text).replace(SEPARATOR, " ")
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
    /// This problem as an RPC error, its texts cut to [`MAX_DETAIL_BYTES`] and its extensions
    /// admitted again under the limits. Status 501 becomes the exception kind `unimplemented`.
    #[must_use]
    pub fn into_error(self) -> Error {
        let mut text = format!(
            "{MARKER}{}{SEPARATOR}{}{SEPARATOR}{}{SEPARATOR}{}",
            self.status,
            clean_head(&self.problem_type),
            clean_head(&self.title),
            clean_head(&self.detail),
        );
        // Only if there is one: a text without it reads as before.
        if let Some(next) = &self.next {
            text.push(SEPARATOR);
            text.push_str(&to_hex(next));
        }
        let mut admitted: Vec<(String, String)> = Vec::new();
        for (key, value) in &self.extensions {
            if let Some(entry) = admit(&admitted, key, value) {
                admitted.push(entry);
            }
        }
        for (key, value) in &admitted {
            let _ = write!(text, "{SEPARATOR}{EXT_PREFIX}{key}{INNER}{value}");
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
        let mut parts = text.strip_prefix(MARKER)?.split(SEPARATOR);
        let status = parts.next()?.parse().ok()?;
        let mut problem = Self::new(parts.next()?, parts.next()?, parts.next()?, status);
        for (index, field) in parts.enumerate() {
            if let Some(entry) = field.strip_prefix(EXT_PREFIX) {
                let admitted = entry
                    .split_once(INNER)
                    .and_then(|(key, value)| admit(&problem.extensions, key, value));
                problem.extensions.extend(admitted);
            } else if index == 0 {
                problem.next = from_hex(field);
            }
        }
        Some(problem)
    }
}
