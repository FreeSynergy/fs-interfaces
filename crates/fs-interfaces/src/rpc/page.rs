//! One page of an ordered result.
//!
//! The contracts page with an opaque `Cursor`; the cores answer whole ordered lists. Here the cursor
//! is the position in that list. An object inserted or removed between two pages can therefore move
//! the boundary by one; a core that pages itself replaces this.
//!
//! Pattern: Iterator (external, resumable through the cursor).

/// Page size of the servers.
pub const DEFAULT_PAGE_SIZE: usize = 100;

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
    /// The cursor is not one this code handed out.
    pub fn of(all: Vec<T>, cursor: &[u8], size: usize) -> capnp::Result<Self> {
        let start = position(cursor)?;
        let total = all.len();
        let items: Vec<T> = all.into_iter().skip(start).take(size).collect();
        let reached = start.saturating_add(items.len());
        let next = if reached < total && !items.is_empty() {
            u64::try_from(reached)
                .unwrap_or(u64::MAX)
                .to_le_bytes()
                .to_vec()
        } else {
            Vec::new()
        };
        Ok(Self { items, next })
    }
}

fn position(cursor: &[u8]) -> capnp::Result<usize> {
    if cursor.is_empty() {
        return Ok(0);
    }
    let bytes: [u8; 8] = cursor
        .try_into()
        .map_err(|_| capnp::Error::failed("invalid cursor".into()))?;
    usize::try_from(u64::from_le_bytes(bytes))
        .map_err(|_| capnp::Error::failed("invalid cursor".into()))
}
