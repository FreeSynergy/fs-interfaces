//! The checks for one candidate (a method, or one member of a union parameter), in a fixed order.
//!
//! Pattern: Specification — a candidate is admitted only if every predicate holds: not blocked, not
//! marked excluded, and passing the screen (names, comments, types) on its method, member, inputs and
//! outputs. Trait: none (internal).

use super::run::Run;
use super::variant::Variant;
use crate::{Field, Mark, Method, RiskClass, Target, Why};

/// A candidate that passed every check.
pub(super) struct Verdict {
    pub class: RiskClass,
    pub mark: Option<Mark>,
}

impl Run<'_> {
    /// `None` means the candidate was left out and the reason is recorded.
    pub(super) fn vet(
        &mut self,
        file: &str,
        iface: &str,
        m: &Method,
        v: &Variant,
    ) -> Option<Verdict> {
        let target = &v.target;
        if let Some(why) = self.g.blocklist.reason(target) {
            self.leave(target.clone(), Why::Blocked(why));
            return None;
        }
        let mark = self.g.risk.mark(target);
        if mark.is_none() {
            self.out.unmarked.push(target.clone());
        }
        let class = mark.as_ref().map_or(RiskClass::Unknown, |m| m.class);
        if class == RiskClass::Excluded {
            let reason = mark
                .and_then(|m| m.reason)
                .unwrap_or_else(|| "marked excluded".to_owned());
            self.leave(target.clone(), Why::Marked(reason));
            return None;
        }
        let ins: Vec<&Field> = v.inputs.iter().collect();
        let outs: Vec<&Field> = m.results.iter().collect();
        let screen = self.screen();
        let problem = v
            .member
            .as_deref()
            .and_then(|name| screen.name(target, name, &v.member_doc, None))
            .or_else(|| screen.name(&Target::item(file, iface, &m.name), &m.name, &m.doc, None))
            .or_else(|| screen.fields(file, iface, &m.name, &ins))
            .or_else(|| screen.fields(file, iface, &m.name, &outs));
        if let Some(why) = problem {
            self.leave(target.clone(), why);
            return None;
        }
        Some(Verdict { class, mark })
    }
}
