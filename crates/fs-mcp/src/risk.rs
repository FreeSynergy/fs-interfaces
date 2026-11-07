//! Risk classes and where marks come from.
//!
//! Pattern: Strategy — [`RiskSource`] hides where marks are kept. Stage 1 reads them from mark files
//! ([`crate::MarkFiles`]); later they become annotations in the contracts, and only a new implementation
//! of this trait is needed, nothing in the generator changes. Trait: [`RiskSource`].

use crate::Target;

/// How much a call can hurt. There is deliberately no class "harmless": reading needs a release too
/// (ADR 0054, "Lesen ist nicht harmlos").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RiskClass {
    /// Reads. Not harmless: what it reads may be a secret or sensitive (ADR 0029), which the engine
    /// decides at the object, never the generator.
    Read,
    /// Adds something, changes nothing that exists.
    Add,
    /// Changes something that exists.
    Change,
    /// Deletes or cannot be undone.
    Destructive,
    /// Acts in the person's name (send, report outward). Asks every time.
    ActsInName,
    /// Never becomes a tool or resource (secret, admin, engine only).
    Excluded,
    /// No mark. Counts as destructive: asks every time.
    Unknown,
}

impl RiskClass {
    /// Name in mark files and in `_meta`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Add => "add",
            Self::Change => "change",
            Self::Destructive => "destructive",
            Self::ActsInName => "acts-in-name",
            Self::Excluded => "excluded",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a name from a mark file; `None` for anything else (never a silent default).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Read,
            Self::Add,
            Self::Change,
            Self::Destructive,
            Self::ActsInName,
            Self::Excluded,
            Self::Unknown,
        ]
        .into_iter()
        .find(|c| c.as_str() == s)
    }

    /// Classes with no "always allow" (ADR 0054): destructive, in the person's name, and unknown.
    #[must_use]
    pub fn asks_always(self) -> bool {
        matches!(self, Self::Destructive | Self::ActsInName | Self::Unknown)
    }
}

/// What a source says about one target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    /// Risk class.
    pub class: RiskClass,
    /// Capability id the contract knows, such as `message.send`; used as the tool name.
    pub capability: Option<String>,
    /// Offered as a resource (a file or a feed), not as a tool.
    pub resource: bool,
    /// Why, for exclusions.
    pub reason: Option<String>,
}

/// Where marks and releases come from.
pub trait RiskSource {
    /// The mark of a method or union member; `None` = no mark, which the generator treats as `unknown`
    /// and lists. Never a default here.
    fn mark(&self, target: &Target) -> Option<Mark>;

    /// A written release of a field, parameter or method whose name looks like a secret but is not
    /// (for example a reaction key). Returns the reason. Without it the generator excludes the tool.
    fn release(&self, target: &Target) -> Option<String>;

    /// Every target that carries a mark or release, to find marks that point at nothing.
    fn targets(&self) -> Vec<Target>;
}
