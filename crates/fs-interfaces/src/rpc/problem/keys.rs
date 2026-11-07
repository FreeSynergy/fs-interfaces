//! The standard extension keys of a [`Problem`](super::Problem).
//!
//! Pattern: none, constants. The names are fixed so every server fills them the same way and a
//! client reads a fact by key instead of taking a sentence apart (`coding.md` §2a). A key outside
//! this list is allowed; it belongs to the type URI of the problem that uses it. The standard keys have no priority, the entries that come first are
//! admitted and later ones may be left out at the limits. `size` and `limit` are left out of a
//! `too-large` that carries a cursor on purpose (the numbers are in the detail), so a client does
//! not rely on them in general. Extensions are statements of the server: a client checks [`ID`] against what it asked for before it acts on it.

/// The object the problem is about, as the text of its id (an `ObjectId`, the canonical form).
/// At most once. Example: the task that was not found, the person whose name is taken.
pub const ID: &str = "id";

/// The limit that was crossed, as a decimal `u64` (read with `ext_u64`). The unit is that of the
/// problem type: bytes for `too-large`, a count for "too many items". Pairs with [`SIZE`].
pub const LIMIT: &str = "limit";

/// The measured value that crossed [`LIMIT`], as a decimal `u64`, same unit as the limit.
/// `too_large_by` fills it.
pub const SIZE: &str = "size";

/// One reason why a value was refused, as a short machine-readable token or field name
/// (`name`, `start-after-end`). Repeats: one entry per reason, in the order found, so a client
/// shows them all at once.
pub const ISSUE: &str = "issue";

/// The kind of the object named by [`ID`] or the thing that was refused (`person`, `task`,
/// `folder`, `page`), the contract's own noun in lower case. At most once.
pub const KIND: &str = "kind";
