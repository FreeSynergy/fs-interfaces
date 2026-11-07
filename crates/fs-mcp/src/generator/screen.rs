//! The screen: every type a tool or resource can carry, in or out, must pass it, or the candidate stays out.
//!
//! Pattern: Specification — a pure predicate over a field list (names, comments, types); it only reads the
//! block list, the releases and the type index, and answers with the first reason to keep the candidate
//! out. Inside, a Visitor walk: one arm per type variant, kept outside the data. Trait: none (internal).
//! Fail-closed: what is unknown, unclear, free-form or secret-looking is out until somebody releases it in
//! writing; a structure the index does not hold is a problem, never "no problem".

use std::collections::HashSet;

use crate::secrets::{doc_looks_secret, looks_free_form, name_looks_secret};
use crate::{Blocklist, Field, Index, RiskSource, Target, TypeName, TypeRef, Why};

/// The lookups the predicate needs.
pub(super) struct Screen<'a> {
    pub risk: &'a dyn RiskSource,
    pub blocklist: &'a Blocklist,
    pub index: &'a Index<'a>,
}

impl Screen<'_> {
    /// First problem in `fields` (parameters or results of `iface.method`) or in anything they reach.
    pub(super) fn fields(
        &self,
        file: &str,
        iface: &str,
        method: &str,
        fields: &[&Field],
    ) -> Option<Why> {
        let mut seen = HashSet::new();
        fields.iter().find_map(|f| {
            let target = Target::item(file, iface, &format!("{method}.{}", f.name));
            self.name(&target, &f.name, &f.doc, Some(&f.ty))
                .or_else(|| self.walk(&f.ty, &mut seen))
        })
    }

    /// One name (with its comment and, if known, its type): blocked, suspect, free-form, or fine.
    pub(super) fn name(
        &self,
        target: &Target,
        name: &str,
        doc: &str,
        ty: Option<&TypeRef>,
    ) -> Option<Why> {
        if let Some(why) = self.blocklist.reason(target) {
            return Some(Why::Blocked(why));
        }
        let path = || target.to_string();
        let released = || self.risk.release(target).is_some();
        if (name_looks_secret(name) || doc_looks_secret(doc)) && !released() {
            return Some(Why::SecretSuspect { path: path() });
        }
        (looks_free_form(name, ty) && !released()).then(|| Why::FreeForm { path: path() })
    }

    fn walk(&self, ty: &TypeRef, seen: &mut HashSet<TypeName>) -> Option<Why> {
        match ty {
            TypeRef::Any => Some(Why::Unclear {
                path: "AnyPointer".to_owned(),
            }),
            TypeRef::List(inner) => self.walk(inner, seen),
            TypeRef::Interface(n) => self.blocklist.type_reason(&n.name).map(Why::Blocked),
            TypeRef::Struct(n) if seen.insert(n.clone()) => {
                if let Some(why) = self.blocklist.type_reason(&n.name) {
                    return Some(Why::Blocked(why));
                }
                let Some(def) = self.index.struct_def(n) else {
                    return Some(Why::Unclear {
                        path: format!("{}/{}", n.file, n.name),
                    });
                };
                let own = Target::owner(&n.file, &def.name);
                if name_looks_secret(&def.name) && self.risk.release(&own).is_none() {
                    return Some(Why::SecretSuspect {
                        path: own.to_string(),
                    });
                }
                def.fields.iter().find_map(|f| {
                    let t = Target::item(&n.file, &def.name, &f.name);
                    self.name(&t, &f.name, &f.doc, Some(&f.ty))
                        .or_else(|| self.walk(&f.ty, seen))
                })
            }
            _ => None,
        }
    }
}
