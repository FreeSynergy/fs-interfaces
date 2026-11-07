//! The secret check: walks every type a tool can carry, in or out, and stops at the first name or comment
//! that looks like a secret and was not released in writing.
//!
//! Pattern: none (a plain recursive walk). Trait: none (internal). Fail-closed: a forgotten mark, a new
//! field or a renamed one cannot let a secret through; it can only cost a tool until someone releases it.

use std::collections::HashSet;

use super::run::Run;
use crate::secrets::{doc_looks_secret, name_looks_secret};
use crate::{Field, Target, TypeName, TypeRef, Why};

impl Run<'_> {
    /// First problem found in `fields` (a tool's parameters or results) or in anything they reach.
    pub(super) fn scan(&self, file: &str, iface: &str, method: &str, fields: &[&Field]) -> Option<Why> {
        let mut seen = HashSet::new();
        for f in fields {
            let target = Target::item(file, iface, &format!("{method}.{}", f.name));
            if let Some(why) = self.check(&target, &f.name, &f.doc).or_else(|| self.walk(&f.ty, &mut seen)) {
                return Some(why);
            }
        }
        None
    }

    /// One name: blocked, suspect, or fine.
    pub(super) fn check(&self, target: &Target, name: &str, doc: &str) -> Option<Why> {
        if let Some(why) = self.g.blocklist.reason(target) {
            return Some(Why::Blocked(why));
        }
        let looks = name_looks_secret(name) || doc_looks_secret(doc);
        (looks && self.g.risk.release(target).is_none()).then(|| Why::SecretSuspect { path: target.to_string() })
    }

    fn walk(&self, ty: &TypeRef, seen: &mut HashSet<TypeName>) -> Option<Why> {
        match ty {
            TypeRef::List(inner) => self.walk(inner, seen),
            TypeRef::Struct(n) if seen.insert(n.clone()) => {
                let def = self.index.struct_def(n)?;
                def.fields.iter().find_map(|f| {
                    self.check(&Target::item(&n.file, &def.name, &f.name), &f.name, &f.doc)
                        .or_else(|| self.walk(&f.ty, seen))
                })
            }
            _ => None,
        }
    }
}
