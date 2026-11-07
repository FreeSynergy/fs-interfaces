//! The test double of the risk source. Pattern: Strategy (a second `RiskSource`).
#![allow(dead_code)]

use std::collections::HashMap;

use fs_mcp::{Mark, RiskClass, RiskSource, Target};

use super::FILE;

/// Marks held in memory: the test double of [`RiskSource`].
#[derive(Default)]
pub struct FakeMarks {
    pub marks: HashMap<Target, Mark>,
    pub releases: HashMap<Target, String>,
}

impl FakeMarks {
    pub fn put(&mut self, owner: &str, item: &str, class: RiskClass) -> &mut Self {
        self.marks.insert(
            Target::item(FILE, owner, item),
            Mark {
                class,
                capability: None,
                resource: false,
                reason: None,
            },
        );
        self
    }
    pub fn capability(&mut self, owner: &str, item: &str, class: RiskClass, id: &str) -> &mut Self {
        let m = Mark {
            class,
            capability: Some(id.into()),
            resource: false,
            reason: None,
        };
        self.marks.insert(Target::item(FILE, owner, item), m);
        self
    }
    /// Every method and member of `messenger_like` marked `read`, even the forbidden ones.
    pub fn everything_read() -> Self {
        let mut m = Self::default();
        for (o, i) in [
            ("Messenger", "capabilities"),
            ("Messenger", "conversations"),
            ("Messenger", "subscribe"),
            ("Messenger", "guard"),
            ("Messenger", "draft"),
            ("Conversations", "open"),
            ("Guard", "allow"),
            ("Accounts", "add"),
            ("MessageAction", "none"),
            ("MessageAction", "delete"),
            ("MessageAction", "flag"),
            ("MessageAction", "decide"),
            ("Draft", "send"),
        ] {
            m.put(o, i, RiskClass::Read);
        }
        m
    }
}

impl RiskSource for FakeMarks {
    fn mark(&self, t: &Target) -> Option<Mark> {
        self.marks.get(t).cloned()
    }
    fn release(&self, t: &Target) -> Option<String> {
        self.releases.get(t).cloned()
    }
    fn targets(&self) -> Vec<Target> {
        self.marks
            .keys()
            .chain(self.releases.keys())
            .cloned()
            .collect()
    }
}
