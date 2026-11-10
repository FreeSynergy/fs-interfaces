//! `Edit`: a change to a record, field by field, typed.
//!
//! Pattern: Command (`coding.md` §2) — an `Edit` is the data of the `change` command: a list of field paths with typed
//! values, applied to a record (`apply`) or turned by the backend into the standard's patch. The rules (decided
//! 2026-11-10, written in `types.capnp`): a field not in the edit stays as it is; a field with no values is cleared;
//! otherwise the values replace the old ones. No JSON anywhere.

use std::collections::{BTreeMap, BTreeSet};

use fs_interfaces::types_capnp::BasicType;

use super::TypeDescription;
use super::check::{RecordIssue, RecordProblem};
use crate::BasicRules;

/// One field of an edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldEdit {
    /// The field; one step for a field of the description, more for a map key and a property below it.
    pub path: Vec<String>,
    /// The basic type of the values.
    pub basic: BasicType,
    /// The new values; empty = clear the field.
    pub values: Vec<String>,
}

impl FieldEdit {
    /// The path as a JSON Pointer (RFC 6901: `~` is `~0`, `/` is `~1`), which is the key of a PatchObject (RFC 8984).
    /// Only the path is text of that standard; the value stays typed.
    #[must_use]
    pub fn pointer(&self) -> String {
        self.path
            .iter()
            .map(|s| s.replace('~', "~0").replace('/', "~1"))
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// The changes to one record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Edit {
    /// At most one entry per path.
    pub fields: Vec<FieldEdit>,
}

impl Edit {
    /// An edit that changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the values of a field.
    #[must_use]
    pub fn set(mut self, path: &[&str], basic: BasicType, values: &[&str]) -> Self {
        self.fields.push(FieldEdit {
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            basic,
            values: values.iter().map(|s| (*s).to_owned()).collect(),
        });
        self
    }

    /// Clear a field (the null of a PatchObject).
    #[must_use]
    pub fn clear(self, path: &[&str], basic: BasicType) -> Self {
        self.set(path, basic, &[])
    }

    /// Check the edit against the description and return it with every value in canonical form.
    ///
    /// # Errors
    /// Every finding, by path (steps joined with `/`).
    pub fn check(
        &self,
        desc: &TypeDescription,
        rules: &BasicRules,
    ) -> Result<Self, Vec<RecordIssue>> {
        let mut issues = Vec::new();
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for f in &self.fields {
            let name = f.path.join("/");
            let mut push = |index, problem| {
                issues.push(RecordIssue {
                    field: name.clone(),
                    index,
                    problem,
                });
            };
            if !seen.insert(&f.path) {
                push(0, RecordProblem::DuplicatePath);
                continue;
            }
            let Some(first) = f.path.first() else {
                push(0, RecordProblem::UnknownField);
                continue;
            };
            let Some(field) = desc.find_field(first) else {
                push(0, RecordProblem::UnknownField);
                continue;
            };
            if f.path.len() > 1 {
                push(0, RecordProblem::NestedPath);
                continue;
            }
            if f.basic != field.basic {
                push(0, RecordProblem::WrongType);
                continue;
            }
            if f.values.is_empty() && field.required {
                push(0, RecordProblem::Missing);
            }
            if f.values.len() > 1 && !field.multiple {
                push(0, RecordProblem::NotMultiple);
            }
            let mut values = Vec::new();
            for (i, v) in f.values.iter().enumerate() {
                match rules.check(field.basic, v, &field.choices) {
                    Ok(c) => values.push(c),
                    Err(p) => push(i, RecordProblem::Value(p)),
                }
            }
            out.push(FieldEdit {
                path: f.path.clone(),
                basic: f.basic,
                values,
            });
        }
        if issues.is_empty() {
            Ok(Self { fields: out })
        } else {
            Err(issues)
        }
    }

    /// Apply to a record (field name to values): untouched fields stay, empty ones are removed, others replaced. Paths
    /// of more than one step are skipped (use `check` first).
    pub fn apply(&self, record: &mut BTreeMap<String, Vec<String>>) {
        for f in self.fields.iter().filter(|f| f.path.len() == 1) {
            if f.values.is_empty() {
                record.remove(&f.path[0]);
            } else {
                record.insert(f.path[0].clone(), f.values.clone());
            }
        }
    }
}
