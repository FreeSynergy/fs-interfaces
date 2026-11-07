//! The screen: every type a tool or resource can carry, in or out, must pass it, or the candidate stays out.
//!
//! Pattern: Specification — a pure predicate over a field list (names, comments, types); it only reads the
//! block list, the releases and the type index, and answers with the first reason to keep the candidate
//! out. Inside, a Visitor walk: one arm per type variant, kept outside the data. Trait: none (internal).
//! Fail-closed: what is unknown, unclear, free-form or secret-looking is out until somebody releases it in
//! writing; a structure the index does not hold is a problem, never "no problem".

use std::collections::HashSet;

use crate::secrets::{doc_looks_secret, looks_free_form, looks_free_format, name_looks_secret};
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

    /// The free-format fields in `fields` or in anything they reach, with the format written for each.
    pub(super) fn free_fields(
        &self,
        file: &str,
        iface: &str,
        method: &str,
        fields: &[&Field],
    ) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for f in fields {
            let t = Target::item(file, iface, &format!("{method}.{}", f.name));
            self.collect(&t, &f.name, &f.ty, &mut seen, &mut out);
        }
        out
    }

    fn collect(
        &self,
        t: &Target,
        name: &str,
        ty: &TypeRef,
        seen: &mut HashSet<TypeName>,
        out: &mut Vec<(String, String)>,
    ) {
        if let (true, Some(f)) = (looks_free_format(name, Some(ty)), self.risk.format(t)) {
            let path = t
                .as_str()
                .split_once('/')
                .map_or_else(|| t.to_string(), |(_, r)| r.to_owned());
            out.push((path, f));
        }
        match ty {
            TypeRef::List(inner) => self.collect(t, name, inner, seen, out),
            TypeRef::Struct(n) if seen.insert(n.clone()) => {
                if let Some(def) = self.index.struct_def(n) {
                    for f in &def.fields {
                        self.collect(
                            &Target::item(&n.file, &def.name, &f.name),
                            &f.name,
                            &f.ty,
                            seen,
                            out,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// A union parameter as declared, before it is split into one tool per member: its name and comment, the
    /// name and comment of the union, and the block list on its type. The members are judged on their own.
    pub(super) fn union_param(
        &self,
        file: &str,
        iface: &str,
        method: &str,
        f: &Field,
    ) -> Option<Why> {
        let target = Target::item(file, iface, &format!("{method}.{}", f.name));
        if let Some(why) = self.name(&target, &f.name, &f.doc, None) {
            return Some(why);
        }
        let TypeRef::Struct(n) = &f.ty else {
            return None;
        };
        if let Some(why) = self.blocklist.type_reason(&n.name) {
            return Some(Why::Blocked(why));
        }
        let def = self.index.struct_def(n)?;
        let own = Target::owner(&n.file, &def.name);
        let looks = name_looks_secret(&def.name) || doc_looks_secret(&def.doc);
        (looks && self.risk.release(&own).is_none()).then(|| Why::SecretSuspect {
            path: own.to_string(),
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
        if looks_free_form(name, ty) && !released() {
            return Some(Why::FreeForm { path: path() });
        }
        (looks_free_format(name, ty) && self.risk.format(target).is_none())
            .then(|| Why::FreeForm { path: path() })
    }

    /// An interface as a whole, or as a handle that a tool hands out: blocked by type, marked excluded, or a
    /// name or comment that looks like a secret and was not released. Its methods are judged when their own
    /// tools are made.
    pub(super) fn interface(&self, file: &str, def: &crate::Interface) -> Option<Why> {
        if let Some(why) = self.blocklist.type_reason(&def.name) {
            return Some(Why::Blocked(why));
        }
        let own = Target::owner(file, &def.name);
        if let Some(k) = self
            .risk
            .mark(&own)
            .filter(|k| k.class == crate::RiskClass::Excluded)
        {
            return Some(Why::Marked(
                k.reason
                    .unwrap_or_else(|| "interface marked excluded".to_owned()),
            ));
        }
        let looks = name_looks_secret(&def.name) || doc_looks_secret(&def.doc);
        (looks && self.risk.release(&own).is_none()).then(|| Why::SecretSuspect {
            path: own.to_string(),
        })
    }

    fn walk(&self, ty: &TypeRef, seen: &mut HashSet<TypeName>) -> Option<Why> {
        match ty {
            TypeRef::Any => Some(Why::Unclear {
                path: "AnyPointer".to_owned(),
            }),
            TypeRef::List(inner) => self.walk(inner, seen),
            TypeRef::Interface(n) => match self.index.interface_def(n) {
                Some(def) => self.interface(&n.file, def),
                None => Some(Why::Unclear {
                    path: format!("{}/{}", n.file, n.name),
                }),
            },
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
                if (name_looks_secret(&def.name) || doc_looks_secret(&def.doc))
                    && self.risk.release(&own).is_none()
                {
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
