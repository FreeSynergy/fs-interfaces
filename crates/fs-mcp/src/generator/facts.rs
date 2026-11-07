//! Facts about the contracts that the checks need: which interfaces are callbacks, which are roots,
//! and which marks point at nothing.
//!
//! Pattern: none (pure functions over the model). Trait: none (internal).

use std::collections::HashSet;

use crate::{Contract, RiskSource, Target, TypeName};

/// Interface usage across all contracts.
pub(crate) struct Facts {
    callbacks: HashSet<TypeName>,
    returned: HashSet<TypeName>,
}

fn key(file: &str, name: &str) -> TypeName {
    TypeName {
        file: file.to_owned(),
        name: name.to_owned(),
    }
}

impl Facts {
    pub(crate) fn collect(contracts: &[Contract]) -> Self {
        let mut callbacks = HashSet::new();
        let mut returned = HashSet::new();
        for c in contracts {
            for m in c.interfaces.iter().flat_map(|i| &i.methods) {
                callbacks.extend(m.params.iter().flat_map(|f| f.ty.interfaces()).cloned());
                returned.extend(m.results.iter().flat_map(|f| f.ty.interfaces()).cloned());
            }
            returned.extend(
                c.structs
                    .iter()
                    .flat_map(|s| &s.fields)
                    .flat_map(|f| f.ty.interfaces())
                    .cloned(),
            );
        }
        Self {
            callbacks,
            returned,
        }
    }

    /// Handed in by the caller as a parameter: a listener.
    pub(crate) fn is_callback(&self, file: &str, name: &str) -> bool {
        self.callbacks.contains(&key(file, name))
    }

    /// Reachable through some result or struct field; else it is a root and needs no handle argument.
    pub(crate) fn is_returned(&self, file: &str, name: &str) -> bool {
        self.returned.contains(&key(file, name))
    }

    pub(crate) fn callback_params<'m>(&self, m: &'m crate::Method) -> Vec<&'m crate::Field> {
        m.params
            .iter()
            .filter(|f| {
                f.ty.interfaces()
                    .iter()
                    .any(|n| self.callbacks.contains(*n))
            })
            .collect()
    }
}

/// Marks and releases whose target is not in the contracts.
pub(crate) fn stale(contracts: &[Contract], risk: &dyn RiskSource) -> Vec<Target> {
    let mut known = HashSet::new();
    for c in contracts {
        for i in &c.interfaces {
            known.insert(Target::owner(&c.file, &i.name));
            for m in &i.methods {
                known.insert(Target::item(&c.file, &i.name, &m.name));
                for p in m.params.iter().chain(&m.results) {
                    known.insert(Target::item(
                        &c.file,
                        &i.name,
                        &format!("{}.{}", m.name, p.name),
                    ));
                }
            }
        }
        for s in &c.structs {
            known.insert(Target::owner(&c.file, &s.name));
            for f in &s.fields {
                known.insert(Target::item(&c.file, &s.name, &f.name));
            }
        }
    }
    let mut out: Vec<Target> = risk
        .targets()
        .into_iter()
        .filter(|t| !known.contains(t))
        .collect();
    out.sort();
    out
}
