//! The fixed order of checks for one candidate (a method, or one member of a union parameter).
//!
//! Pattern: Template method — every candidate, tool or resource, passes the same steps in the same
//! order: block list, mark, exclusion, secret scan. Trait: none (internal).

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
        let (target, inputs) = (&v.target, &v.inputs);
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
        let method_target = Target::item(file, iface, &m.name);
        let ins: Vec<&Field> = inputs.iter().collect();
        let outs: Vec<&Field> = m.results.iter().collect();
        let member = v
            .member
            .as_deref()
            .and_then(|name| self.check(target, name, &v.member_doc));
        let problem = member
            .or_else(|| self.check(&method_target, &m.name, &m.doc))
            .or_else(|| self.scan(file, iface, &m.name, &ins))
            .or_else(|| self.scan(file, iface, &m.name, &outs));
        if let Some(why) = problem {
            self.leave(target.clone(), why);
            return None;
        }
        Some(Verdict { class, mark })
    }
}
