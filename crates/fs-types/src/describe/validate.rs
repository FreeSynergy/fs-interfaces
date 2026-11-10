//! `TypeDescription::validate`: the rules a description has to meet before it is registered.
//!
//! Pattern: Specification (`coding.md` §2 has no closer name) — a fixed set of rules over a description, applied in one
//! pass over type, fields and commands; the rules are those of the comments in `types.capnp`. [`Mode`] chooses how strict.

use std::collections::BTreeSet;

use fs_interfaces::types_capnp::{BasicType, CommandKind};

use super::{DescriptionProblem as P, FieldDescription, TypeDescription};

fn upper_camel(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_ascii_uppercase()) && c.all(|c| c.is_ascii_alphanumeric())
}

fn lower_camel(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase()) && c.all(|c| c.is_ascii_alphanumeric())
}

/// A translation key: lower-case words of letters and digits, joined by `.` or `-`, no empty word.
pub(crate) fn label_key(k: &str) -> bool {
    !k.is_empty()
        && k.split(['.', '-']).all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn name_ok(n: &str, ok: fn(&str) -> bool) -> Result<(), P> {
    if n.is_empty() {
        Err(P::EmptyName)
    } else if ok(n) {
        Ok(())
    } else {
        Err(P::BadName(n.to_owned()))
    }
}

fn label_ok(k: &str) -> Result<(), P> {
    if label_key(k) {
        Ok(())
    } else {
        Err(P::BadLabel(k.to_owned()))
    }
}

/// How strict: `Strict` for a description a program registers itself; `Lenient` for one read from the wire, which may
/// come from a newer schema (a kind, or a command kind, this version reads as `unknown`) and must still be usable.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Strict,
    Lenient,
}

fn fields_ok(fields: &[FieldDescription], scope: &str, mode: Mode) -> Result<(), P> {
    let mut seen = BTreeSet::new();
    for f in fields {
        name_ok(&f.name, lower_camel)?;
        if !seen.insert(f.name.as_str()) {
            return Err(P::Duplicate(format!("{scope}.{}", f.name)));
        }
        label_ok(&f.label)?;
        let at = format!("{scope}.{}", f.name);
        // A kind we do not know may be a choice or a reference in the sender's schema: its lists are not ours to judge.
        if f.basic == BasicType::Unknown && mode == Mode::Lenient {
            continue;
        }
        match (f.basic == BasicType::Choice, f.choices.is_empty()) {
            (true, true) => return Err(P::NoChoices(at)),
            (false, false) => return Err(P::ChoicesOnNonChoice(at)),
            _ => {}
        }
        let mut keys = BTreeSet::new();
        for k in &f.choices {
            if !label_key(k) {
                return Err(P::BadChoice(format!("{at}.{k}")));
            }
            if !keys.insert(k.as_str()) {
                return Err(P::DuplicateChoice(format!("{at}.{k}")));
            }
        }
        if !f.target.is_empty() {
            if f.basic != BasicType::Reference {
                return Err(P::TargetOnNonReference(at));
            }
            name_ok(&f.target, upper_camel)?;
        }
    }
    Ok(())
}

impl TypeDescription {
    /// Check the description.
    ///
    /// # Errors
    /// The first defect found.
    pub fn validate(&self) -> Result<(), P> {
        self.validate_in(Mode::Strict)
    }

    pub(crate) fn validate_in(&self, mode: Mode) -> Result<(), P> {
        name_ok(&self.name, upper_camel)?;
        if self.program.is_empty() {
            return Err(P::EmptyProgram);
        }
        label_ok(&self.label)?;
        fields_ok(&self.fields, &self.name, mode)?;
        let mut seen = BTreeSet::new();
        for c in &self.commands {
            name_ok(&c.name, lower_camel)?;
            if !seen.insert(c.name.as_str()) {
                return Err(P::Duplicate(format!("{}.{}", self.name, c.name)));
            }
            label_ok(&c.label)?;
            if c.kind == CommandKind::Unknown && mode == Mode::Strict {
                return Err(P::UnknownCommandKind(format!("{}.{}", self.name, c.name)));
            }
            fields_ok(&c.input, &format!("{}.{}.input", self.name, c.name), mode)?;
            fields_ok(&c.result, &format!("{}.{}.result", self.name, c.name), mode)?;
        }
        Ok(())
    }
}
