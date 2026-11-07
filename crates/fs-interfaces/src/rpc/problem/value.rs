//! The problem itself and its wire `Problem`.
//!
//! Pattern: Value object — [`Problem`] is small and compared by value; extensions are admitted
//! through the limits of `ext`. The `with_*` methods are plain chained setters, not the Builder
//! pattern of `coding.md` §2 (there is no `build` step and nothing to validate at the end).

use super::{MAX_EXTENSIONS, TOO_LARGE, capped, keys};
use crate::provider::UNSUPPORTED_PROBLEM_TYPE;

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
    /// RFC 9457 extension members as (key, value), in order; a key may repeat (a list). Read them
    /// with [`Problem::ext`], [`Problem::ext_u64`], [`Problem::ext_all`]; the standard keys are in
    /// [`keys`]. Lives in the error text only, like `next`. Fill with [`Problem::with_ext`] so the
    /// limits hold. Extensions are statements of the server: a client checks `id` against what it
    /// asked for before it acts on it.
    pub extensions: Vec<(String, String)>,
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
            extensions: Vec::new(),
        }
    }

    /// The problem [`TOO_LARGE`] (413) with the extensions [`keys::SIZE`] and [`keys::LIMIT`].
    pub fn too_large(detail: impl Into<String>, size: u64, limit: u64) -> Self {
        Self::new(TOO_LARGE, "Too large", detail, 413)
            .with_ext_u64(keys::SIZE, size)
            .with_ext_u64(keys::LIMIT, limit)
    }

    /// This problem with a cursor past what could not be sent.
    #[must_use]
    pub fn with_next(mut self, next: Option<Vec<u8>>) -> Self {
        self.next = next;
        self
    }

    /// Fill a wire `Problem`, its detail cut to [`super::MAX_DETAIL_BYTES`], with the extensions
    /// (`Problem.extensions`). `next` stays behind: it travels in the error text only.
    pub fn write(&self, mut out: crate::common_capnp::problem::Builder<'_>) {
        out.set_type(self.problem_type.as_str());
        out.set_title(self.title.as_str());
        out.set_detail(capped(&self.detail));
        out.set_status(self.status);
        let shown = &self.extensions[..self.extensions.len().min(MAX_EXTENSIONS)];
        let mut list = out.init_extensions(u32::try_from(shown.len()).unwrap_or(0));
        for (i, (key, value)) in (0..).zip(shown) {
            let mut entry = list.reborrow().get(i);
            entry.set_key(key.as_str());
            entry.set_value(value.as_str());
        }
    }

    /// Read a wire `Problem`.
    ///
    /// # Errors
    /// The message is malformed.
    pub fn read(r: crate::common_capnp::problem::Reader<'_>) -> capnp::Result<Self> {
        let mut problem = Self::new(
            r.get_type()?.to_string()?,
            r.get_title()?.to_string()?,
            r.get_detail()?.to_string()?,
            r.get_status(),
        );
        for entry in r.get_extensions()? {
            problem = problem.with_ext(entry.get_key()?.to_str()?, entry.get_value()?.to_str()?);
        }
        Ok(problem)
    }

    /// Whether this is the explicit "not possible" of the provider rule (ADR 0045).
    #[must_use]
    pub fn is_unsupported(&self) -> bool {
        self.problem_type == UNSUPPORTED_PROBLEM_TYPE
    }
}
