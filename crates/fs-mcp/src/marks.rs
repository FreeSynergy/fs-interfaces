//! Marks kept in files next to the generator: one `marks/<schema>.toml` per schema file.
//!
//! Pattern: Strategy — the stage 1 implementation of [`RiskSource`]; real annotations in the contracts
//! replace it later without touching the generator. Trait: [`RiskSource`].
//!
//! ```toml
//! [Draft]                     # interface or struct of that schema
//! send = { risk = "acts-in-name", capability = "message.send" }
//! setText = "change"          # short form: just the class
//! [Roles]
//! "@interface" = { risk = "excluded", reason = "admin" }   # all methods, present and future, and every handle to it
//! [format]                    # a free-format field: a format, not a secret field
//! "Persons.jscontact" = "jscontact"
//! [release]                   # a name that looks like a secret but is none, with the reason
//! "Reaction.key" = "a reaction key, not key material"
//! ```

use std::collections::HashMap;
use std::path::Path;

use toml::{Table, Value};

use crate::{Error, Mark, RiskClass, RiskSource, Target};

/// Marks and releases loaded from files.
#[derive(Debug, Default)]
pub struct MarkFiles {
    marks: HashMap<Target, Mark>,
    releases: HashMap<Target, String>,
    formats: HashMap<Target, String>,
}

impl MarkFiles {
    /// Load every `*.toml` of a directory.
    ///
    /// # Errors
    /// [`Error::Marks`] if a file cannot be read or holds anything that is not understood. Nothing is skipped.
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, Error> {
        let dir = dir.as_ref();
        let bad = |file: &str, p: String| Error::Marks {
            file: file.to_owned(),
            problem: p,
        };
        let mut sources = Vec::new();
        for e in
            std::fs::read_dir(dir).map_err(|e| bad(&dir.display().to_string(), e.to_string()))?
        {
            let path = e.map_err(|e| bad("?", e.to_string()))?.path();
            if path.extension().is_some_and(|x| x == "toml") {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let text = std::fs::read_to_string(&path).map_err(|e| bad(&stem, e.to_string()))?;
                sources.push((stem, text));
            }
        }
        Self::from_sources(
            &sources
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<_>>(),
        )
    }

    /// Parse `(schema stem, toml text)` pairs.
    ///
    /// # Errors
    /// [`Error::Marks`] on a syntax error, an unknown class, or a wrongly shaped entry.
    pub fn from_sources(sources: &[(&str, &str)]) -> Result<Self, Error> {
        let mut me = Self::default();
        for (file, text) in sources {
            let bad = |p: String| Error::Marks {
                file: (*file).to_owned(),
                problem: p,
            };
            let table: Table = text
                .parse()
                .map_err(|e: toml::de::Error| bad(e.to_string()))?;
            for (owner, body) in table {
                let Value::Table(body) = body else {
                    return Err(bad(format!("{owner}: a table is expected")));
                };
                for (name, value) in body {
                    if owner == "format" {
                        let Value::String(f) = value else {
                            return Err(bad(format!("format {name}: a format name is expected")));
                        };
                        if f.trim().is_empty() {
                            return Err(bad(format!(
                                "format {name}: the format must not be empty"
                            )));
                        }
                        let target = match name.split_once('.') {
                            Some((o, i)) => Target::item(file, o, i),
                            None => return Err(bad(format!("format {name}: Owner.name expected"))),
                        };
                        me.formats.insert(target, f);
                    } else if owner == "release" {
                        let Value::String(reason) = value else {
                            return Err(bad(format!("release {name}: a reason text is expected")));
                        };
                        if reason.trim().is_empty() {
                            return Err(bad(format!(
                                "release {name}: the reason must not be empty"
                            )));
                        }
                        let target = match name.split_once('.') {
                            Some((o, i)) => Target::item(file, o, i),
                            None => Target::owner(file, &name),
                        };
                        me.releases.insert(target, reason);
                    } else {
                        let mark =
                            parse_mark(&value).map_err(|p| bad(format!("{owner}.{name}: {p}")))?;
                        if name == "@interface" {
                            if mark.class != RiskClass::Excluded {
                                return Err(bad(format!(
                                    "{owner}: an interface mark can only exclude"
                                )));
                            }
                            me.marks.insert(Target::owner(file, &owner), mark);
                        } else {
                            me.marks.insert(Target::item(file, &owner, &name), mark);
                        }
                    }
                }
            }
        }
        Ok(me)
    }
}

fn parse_mark(v: &Value) -> Result<Mark, String> {
    let class_of = |s: &str| RiskClass::parse(s).ok_or_else(|| format!("unknown risk class {s:?}"));
    let plain = |class| Mark {
        class,
        capability: None,
        resource: false,
        reason: None,
    };
    match v {
        Value::String(s) => Ok(plain(class_of(s)?)),
        Value::Table(t) => {
            if let Some(k) = t
                .keys()
                .find(|k| !["risk", "capability", "resource", "reason"].contains(&k.as_str()))
            {
                return Err(format!("unknown key {k:?}"));
            }
            let text = |k: &str| match t.get(k) {
                None => Ok(None),
                Some(Value::String(s)) if !s.trim().is_empty() => Ok(Some(s.clone())),
                Some(_) => Err(format!("{k} must be a non-empty text")),
            };
            let risk = text("risk")?.ok_or("risk is missing")?;
            let class = class_of(&risk)?;
            let capability = text("capability")?;
            if let Some(c) = &capability {
                let valid = c.len() <= 128
                    && c.chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || "_.-".contains(ch));
                if !valid {
                    return Err(format!("capability {c:?} is not a valid tool name"));
                }
            }
            let resource = match t.get("resource") {
                None => false,
                Some(Value::Boolean(b)) => *b,
                Some(_) => return Err("resource must be true or false".to_owned()),
            };
            if resource && class != RiskClass::Read {
                return Err("only a method of class read may be a resource".to_owned());
            }
            Ok(Mark {
                class,
                capability,
                resource,
                reason: text("reason")?,
            })
        }
        _ => Err("a class name or a table is expected".to_owned()),
    }
}

impl RiskSource for MarkFiles {
    fn mark(&self, target: &Target) -> Option<Mark> {
        self.marks.get(target).cloned()
    }
    fn release(&self, target: &Target) -> Option<String> {
        self.releases.get(target).cloned()
    }
    fn format(&self, target: &Target) -> Option<String> {
        self.formats.get(target).cloned()
    }
    fn targets(&self) -> Vec<Target> {
        self.marks
            .keys()
            .chain(self.releases.keys())
            .chain(self.formats.keys())
            .cloned()
            .collect()
    }
}
