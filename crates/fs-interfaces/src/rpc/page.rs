//! One page of an ordered result, and what fits into one message.
//!
//! The contracts page with an opaque `Cursor`: a consumer only hands back what it got. Today it marks
//! a place in the core's ordered list, so an object inserted or removed between two pages can move
//! the boundary by one. Before 1.0 it becomes a stable key (`DESIGN.md`); the leading version byte
//! lets both forms be told apart without a contract change.
//!
//! Pattern: Iterator (external, resumable through the cursor).

use super::problem::{invalid_request, too_large};

/// Page size of the servers.
pub const DEFAULT_PAGE_SIZE: usize = 100;
/// Largest message the RPC layer reads (`ReaderOptions::default`); a larger one ends the connection.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024 * 1024;
/// Largest payload a server puts into one answer, leaving room for the framing.
pub const MAX_PAGE_BYTES: usize = 32 * 1024 * 1024;

/// Largest single value a server accepts: its answer may carry it about twice (content and
/// envelope), and that still has to fit.
pub const MAX_INTAKE_BYTES: usize = MAX_PAGE_BYTES / 2;

const CURSOR_V1: u8 = 1;

/// Refuse a value that could not come back in an answer, before it is stored.
///
/// # Errors
/// `len` is above [`MAX_INTAKE_BYTES`].
pub fn check_intake(len: usize) -> capnp::Result<()> {
    if len > MAX_INTAKE_BYTES {
        return Err(too_large(format!(
            "{len} bytes, at most {MAX_INTAKE_BYTES} fit"
        )));
    }
    Ok(())
}

/// Refuse an answer that would end the connection.
///
/// # Errors
/// `len` is above [`MAX_PAGE_BYTES`].
pub fn check_answer(len: usize) -> capnp::Result<()> {
    if len > MAX_PAGE_BYTES {
        return Err(too_large(format!(
            "the answer has {len} bytes, at most {MAX_PAGE_BYTES} fit"
        )));
    }
    Ok(())
}

/// One page and the cursor of the next one (empty = this was the last).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The items of this page.
    pub items: Vec<T>,
    /// Cursor of the next page; empty at the end.
    pub next: Vec<u8>,
}

impl<T> Page<T> {
    /// Cut the page at `cursor` (empty = start) out of the whole ordered list.
    ///
    /// # Errors
    /// The cursor is not one this code handed out, or `size` is zero.
    pub fn of(all: Vec<T>, cursor: &[u8], size: usize) -> capnp::Result<Self> {
        Self::of_weighted(all, cursor, size, |_| 0)
    }

    /// Like [`Page::of`], and the page also ends before `weight` of its items passes
    /// [`MAX_PAGE_BYTES`]. A page always holds at least one item, so paging always advances.
    ///
    /// # Errors
    /// The cursor is not one this code handed out, or `size` is zero.
    pub fn of_weighted(
        all: Vec<T>,
        cursor: &[u8],
        size: usize,
        weight: impl Fn(&T) -> usize,
    ) -> capnp::Result<Self> {
        if size == 0 {
            return Err(capnp::Error::failed("page size is zero".into()));
        }
        let start = position(cursor)?;
        let total = all.len();
        let mut items = Vec::new();
        let mut bytes = 0usize;
        for item in all.into_iter().skip(start).take(size) {
            bytes = bytes.saturating_add(weight(&item));
            if bytes > MAX_PAGE_BYTES && !items.is_empty() {
                break;
            }
            items.push(item);
        }
        let reached = start.saturating_add(items.len());
        let next = if reached < total && !items.is_empty() {
            encode(reached)
        } else {
            Vec::new()
        };
        Ok(Self { items, next })
    }
}

/// The length a `Blob.read` really answers: what a client asked for, up to what fits a message. The
/// contract allows fewer bytes than asked; only an empty answer means the end.
#[must_use]
pub fn clamp_read(length: u32) -> u32 {
    length.min(u32::try_from(MAX_PAGE_BYTES).unwrap_or(u32::MAX))
}

fn encode(place: usize) -> Vec<u8> {
    let mut out = vec![CURSOR_V1];
    out.extend_from_slice(&u64::try_from(place).unwrap_or(u64::MAX).to_le_bytes());
    out
}

fn position(cursor: &[u8]) -> capnp::Result<usize> {
    if cursor.is_empty() {
        return Ok(0);
    }
    let invalid = || invalid_request("invalid cursor");
    let (&version, rest) = cursor.split_first().ok_or_else(invalid)?;
    let bytes: [u8; 8] = rest.try_into().map_err(|_| invalid())?;
    if version != CURSOR_V1 {
        return Err(invalid());
    }
    usize::try_from(u64::from_le_bytes(bytes)).map_err(|_| invalid())
}
