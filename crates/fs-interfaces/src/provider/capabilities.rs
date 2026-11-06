//! What a provider can do: one entry per capability id (ADR 0045).
//!
//! Pattern: Builder - a provider assembles its report step by step.

use super::Unsupported;

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
