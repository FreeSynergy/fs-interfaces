//! Extension members of a problem: the limits and the typed readers.
//!
//! Pattern: none (limits and readers of [`Problem`]). An extension is a (key, value) pair of plain
//! text that is admitted whole or not at all, so what [`admitted`] lets through can always be sent, on both ways out. A hand-filled
//! `extensions` field is limited when it leaves.

use super::Problem;
use super::wire::{MAX_DETAIL_BYTES, clean};

/// Most extension entries of one problem.
pub const MAX_EXTENSIONS: usize = 64;
/// Longest key in bytes; a longer key is left out, not cut.
pub const MAX_KEY_BYTES: usize = 64;

/// The entry that `key` and `value` become when added to `existing`: separators replaced, nothing
/// cut. `None` if the key is empty or longer than [`MAX_KEY_BYTES`], there are [`MAX_EXTENSIONS`]
/// already, or the keys and values together would pass [`MAX_DETAIL_BYTES`]. A value is left out
/// whole rather than cut: a cut number is another number.
pub(super) fn admit(
    existing: &[(String, String)],
    key: &str,
    value: &str,
) -> Option<(String, String)> {
    if existing.len() >= MAX_EXTENSIONS || key.is_empty() || key.len() > MAX_KEY_BYTES {
        return None;
    }
    let used: usize = existing.iter().map(|(k, v)| k.len() + v.len()).sum();
    let total = used.checked_add(key.len())?.checked_add(value.len())?;
    (total <= MAX_DETAIL_BYTES).then(|| (clean(key), clean(value)))
}

/// What of `entries` can be sent: each one in turn through [`admit`], so the order decides and
/// what does not fit is left out. Both ways out (error text and wire `Problem`) use this.
pub(super) fn admitted(entries: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (key, value) in entries {
        if let Some(entry) = admit(&out, key, value) {
            out.push(entry);
        }
    }
    out
}

impl Problem {
    /// This problem with one more extension. A key may repeat (a list). Separators in key and
    /// value become spaces; an entry beyond the limits ([`MAX_EXTENSIONS`], [`MAX_KEY_BYTES`], all
    /// texts together [`MAX_DETAIL_BYTES`]) is left out whole, never cut.
    #[must_use]
    pub fn with_ext(mut self, key: &str, value: &str) -> Self {
        if let Some(entry) = admit(&self.extensions, key, value) {
            self.extensions.push(entry);
        }
        self
    }

    /// Like [`Problem::with_ext`] with a number as decimal text.
    #[must_use]
    pub fn with_ext_u64(self, key: &str, value: u64) -> Self {
        self.with_ext(key, &value.to_string())
    }

    /// The first value of `key`.
    #[must_use]
    pub fn ext(&self, key: &str) -> Option<&str> {
        self.extensions
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.as_str()))
    }

    /// The first value of `key` as a number; `None` if there is none or it is not a `u64`.
    #[must_use]
    pub fn ext_u64(&self, key: &str) -> Option<u64> {
        let text = self.ext(key)?;
        // `parse` takes a leading `+`; a number here is digits only.
        text.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| text.parse().ok())?
    }

    /// Every value of `key`, in order.
    pub fn ext_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> {
        self.extensions
            .iter()
            .filter(move |(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}
