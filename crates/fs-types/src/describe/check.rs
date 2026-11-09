//! Check a record against a type description.
//!
//! Pattern: Template method — one fixed flow (unknown field, missing, too many, each value through the rule of its basic
//! type) that every program shares, so the admin frontend and a program's own `check` command say the same thing.

use std::collections::BTreeMap;

use crate::{BasicRules, TypeProblem};

use super::TypeDescription;

/// What is wrong with one field of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordProblem {
    /// The type has no such field.
    UnknownField,
    /// A required field has no value.
    Missing,
    /// More than one value for a field that takes one.
    NotMultiple,
    /// A value breaks the rule of its basic type.
    Value(TypeProblem),
}

/// One finding: which field, which value (0-based; 0 for field-wide findings), what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordIssue {
    /// The field name.
    pub field: String,
    /// Index of the value; 0 for findings about the field as a whole.
    pub index: usize,
    /// What is wrong.
    pub problem: RecordProblem,
}

impl TypeDescription {
    /// Check one typed value of `field`; the canonical text.
    ///
    /// # Errors
    /// [`RecordProblem::UnknownField`] or the value's problem.
    pub fn check_value(
        &self,
        rules: &BasicRules,
        field: &str,
        typed: &str,
    ) -> Result<String, RecordProblem> {
        let f = self.find_field(field).ok_or(RecordProblem::UnknownField)?;
        rules
            .check(f.basic, typed, &f.choices)
            .map_err(RecordProblem::Value)
    }

    /// Check a whole record (`field name` → typed values). Empty values (after trimming) count as not given.
    #[must_use]
    pub fn check_record(
        &self,
        rules: &BasicRules,
        record: &BTreeMap<String, Vec<String>>,
    ) -> Vec<RecordIssue> {
        let mut issues = Vec::new();
        let mut push = |field: &str, index, problem| {
            issues.push(RecordIssue {
                field: field.to_owned(),
                index,
                problem,
            });
        };
        for name in record.keys().filter(|n| self.find_field(n).is_none()) {
            push(name, 0, RecordProblem::UnknownField);
        }
        for f in &self.fields {
            let given: Vec<&String> = record.get(&f.name).map_or_else(Vec::new, |v| {
                v.iter().filter(|t| !t.trim().is_empty()).collect()
            });
            if given.is_empty() {
                if f.required {
                    push(&f.name, 0, RecordProblem::Missing);
                }
                continue;
            }
            if given.len() > 1 && !f.multiple {
                push(&f.name, 0, RecordProblem::NotMultiple);
            }
            for (i, typed) in given.iter().enumerate() {
                if let Err(p) = rules.check(f.basic, typed, &f.choices) {
                    push(&f.name, i, RecordProblem::Value(p));
                }
            }
        }
        issues
    }
}
