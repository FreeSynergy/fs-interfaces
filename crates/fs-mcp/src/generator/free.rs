//! The free-format fields that a candidate carries, with the format written for each (for `freeContent`).
//!
//! Pattern: Visitor — a walk with one arm per type variant that only collects. Trait: none (internal).

use std::collections::HashSet;

use super::screen::Screen;
use crate::free_format::looks_free_format;
use crate::{Field, Target, TypeName, TypeRef};

impl Screen<'_> {
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
}
