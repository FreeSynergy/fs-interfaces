//! The fixed list of targets that never become tools, whatever any mark says.
//!
//! Pattern: Registry — a table of target to reason. It is code, not data in a mark file, so that a
//! forgotten or edited mark cannot lift it. It goes away only when the contracts carry machine-readable
//! exclusion marks (Thread 2). Trait: none (a table; the generator takes it by value so tests can swap it).

use crate::Target;

const SCHEMA: &str = "messenger";

/// Targets that are never generated.
#[derive(Clone, Debug)]
pub struct Blocklist {
    entries: Vec<(Target, &'static str)>,
}

impl Blocklist {
    /// The list in force.
    #[must_use]
    pub fn standard() -> Self {
        let e = |owner: &str, name: &str, why| (Target::item(SCHEMA, owner, name), why);
        Self {
            entries: vec![
                e(
                    "MessageAction",
                    "decide",
                    "toolcall.decide: only the person's device decides; an accept through the agent's client proves no person (ADR 0054)",
                ),
                e("MessageAction", "none", "does nothing and is refused; never a tool"),
                (
                    Target::owner(SCHEMA, "Guard"),
                    "the Guard is the brake itself; an agent never reaches it (ADR 0054)",
                ),
                e("Messenger", "guard", "gives access to the Guard"),
                e("AccountSetup", "secret", "a secret would pass through the model (ADR 0021)"),
            ],
        }
    }

    /// An empty list, for tests that must prove the other safeguards hold without it.
    #[must_use]
    pub fn empty() -> Self {
        Self { entries: vec![] }
    }

    /// Why `target` is blocked, if it is.
    #[must_use]
    pub fn reason(&self, target: &Target) -> Option<&'static str> {
        self.entries.iter().find(|(t, _)| t == target).map(|(_, r)| *r)
    }

    /// All blocked targets, for tests that check each one stays out.
    #[must_use]
    pub fn targets(&self) -> Vec<Target> {
        self.entries.iter().map(|(t, _)| t.clone()).collect()
    }
}
