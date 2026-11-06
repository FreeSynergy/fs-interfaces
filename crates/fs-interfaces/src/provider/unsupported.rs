//! The explicit "not possible" answer of a provider (ADR 0045).
//!
//! Pattern: Null Object - returned by default trait methods instead of a silent no-op.

use std::fmt;

/// Problem type URI of "not possible" (RFC 9457 `type`); equals `unsupportedProblemType` in `common.capnp`.
pub const UNSUPPORTED_PROBLEM_TYPE: &str = "https://freesynergy.net/problems/unsupported";

/// The explicit answer "not possible": the provider looked for a way and there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    capability: String,
    reason: String,
}

impl Unsupported {
    /// "Not possible" for `capability` (an id like `message.edit`), with the reason.
    pub fn new(capability: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            capability: capability.into(),
            reason: reason.into(),
        }
    }

    /// Capability id that is not possible.
    #[must_use]
    pub fn capability(&self) -> &str {
        &self.capability
    }

    /// Why it is not possible (and what was tried).
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// RFC 9457 `type`: [`UNSUPPORTED_PROBLEM_TYPE`].
    #[must_use]
    pub fn problem_type(&self) -> &'static str {
        UNSUPPORTED_PROBLEM_TYPE
    }

    /// HTTP-like status of the problem: 501 (not implemented).
    #[must_use]
    pub fn status(&self) -> u16 {
        501
    }

    /// Fill a wire `Unsupported` (`common.capnp`).
    pub fn write(&self, mut out: crate::common_capnp::unsupported::Builder<'_>) {
        out.set_capability(self.capability.as_str());
        let mut p = out.init_problem();
        p.set_type(UNSUPPORTED_PROBLEM_TYPE);
        p.set_title("Not possible");
        p.set_detail(self.reason.as_str());
        p.set_status(self.status());
    }

    /// Read a wire `Unsupported` back.
    ///
    /// # Errors
    /// The message is malformed.
    pub fn read(r: crate::common_capnp::unsupported::Reader<'_>) -> capnp::Result<Self> {
        let detail = r.get_problem()?.get_detail()?.to_string()?;
        Ok(Self {
            capability: r.get_capability()?.to_string()?,
            reason: detail,
        })
    }
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not possible: {} ({})", self.capability, self.reason)
    }
}

impl std::error::Error for Unsupported {}
