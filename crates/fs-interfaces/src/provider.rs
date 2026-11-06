//! The provider rule (ADR 0045): like a parent class, every provider offers **all** methods and
//! attributes of its interface. If something is missing we first look for a way to make it work.
//! If there truly is none, the provider answers explicitly "not possible": [`Unsupported`].
//!
//! In Rust the interface is a trait whose methods all have a default body returning
//! `Err(Unsupported)`; a provider overrides what it can do and reports it as [`Capabilities`].
//!
//! Pattern: Null Object - a default method answering [`Unsupported`] is the explicit "not possible".
//!
//! ```
//! use fs_interfaces::{Capabilities, Unsupported};
//!
//! /// Topic interface "messages". Every provider offers every method.
//! trait Messages {
//!     fn send(&self, text: &str) -> Result<(), Unsupported>;
//!
//!     fn edit(&self, _id: &str, _text: &str) -> Result<(), Unsupported> {
//!         Err(Unsupported::new("message.edit", "this network has no edit"))
//!     }
//!
//!     fn capabilities(&self) -> Capabilities;
//! }
//!
//! struct Telegram;
//! impl Messages for Telegram {
//!     fn send(&self, _text: &str) -> Result<(), Unsupported> { Ok(()) }
//!     fn capabilities(&self) -> Capabilities {
//!         let mut caps = Capabilities::new("telegram");
//!         caps.supported("message.send");
//!         caps.unsupported("message.edit", "this network has no edit");
//!         caps
//!     }
//! }
//!
//! let t = Telegram;
//! assert!(t.send("hi").is_ok());
//! let err = t.edit("1", "x").unwrap_err();
//! assert_eq!(err.capability(), "message.edit");
//! assert_eq!(err.status(), 501);
//! assert!(!t.capabilities().is_supported("message.edit"));
//! ```

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

/// One line of a provider's report: can it do this capability, and if not, why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Support {
    /// Capability id, e.g. `message.send`.
    pub capability: String,
    /// `false` is the explicit "not possible".
    pub supported: bool,
    /// Reason when `supported` is `false`; empty otherwise.
    pub reason: String,
}

/// What a provider can do: one [`Support`] per capability of the interface it implements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    provider: String,
    entries: Vec<Support>,
}

impl Capabilities {
    /// Empty report of `provider`.
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            entries: Vec::new(),
        }
    }

    /// Provider name.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Record that `capability` works.
    pub fn supported(&mut self, capability: impl Into<String>) -> &mut Self {
        self.entries.push(Support {
            capability: capability.into(),
            supported: true,
            reason: String::new(),
        });
        self
    }

    /// Record that `capability` is not possible, with the reason.
    pub fn unsupported(
        &mut self,
        capability: impl Into<String>,
        reason: impl Into<String>,
    ) -> &mut Self {
        self.entries.push(Support {
            capability: capability.into(),
            supported: false,
            reason: reason.into(),
        });
        self
    }

    /// All entries in the order recorded.
    #[must_use]
    pub fn entries(&self) -> &[Support] {
        &self.entries
    }

    /// Whether `capability` is recorded as supported.
    #[must_use]
    pub fn is_supported(&self, capability: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.capability == capability && e.supported)
    }

    /// The "not possible" answer for `capability`, if the report says so.
    #[must_use]
    pub fn why_not(&self, capability: &str) -> Option<Unsupported> {
        self.entries
            .iter()
            .find(|e| e.capability == capability && !e.supported)
            .map(|e| Unsupported::new(e.capability.as_str(), e.reason.as_str()))
    }

    /// Fill a wire `SupportReport` (`common.capnp`).
    pub fn write(
        &self,
        contract_version: &str,
        mut out: crate::common_capnp::support_report::Builder<'_>,
    ) {
        out.set_contract_version(contract_version);
        out.set_provider(self.provider.as_str());
        let mut list = out.init_entries(u32::try_from(self.entries.len()).unwrap_or(u32::MAX));
        for (i, e) in (0u32..).zip(&self.entries) {
            let mut s = list.reborrow().get(i);
            s.set_capability(e.capability.as_str());
            s.set_supported(e.supported);
            s.set_reason(e.reason.as_str());
        }
    }

    /// Read a wire `SupportReport` back.
    ///
    /// # Errors
    /// The message is malformed.
    pub fn read(r: crate::common_capnp::support_report::Reader<'_>) -> capnp::Result<Self> {
        let mut caps = Self::new(r.get_provider()?.to_string()?);
        for s in r.get_entries()? {
            caps.entries.push(Support {
                capability: s.get_capability()?.to_string()?,
                supported: s.get_supported(),
                reason: s.get_reason()?.to_string()?,
            });
        }
        Ok(caps)
    }
}
