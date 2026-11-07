//! Strict reading of one mark entry: a class name, or a table with `risk`, `capability`, `resource`, `reason`.
//!
//! Pattern: Value object construction — the text becomes a [`Mark`] or an error, never a default. Trait: none.

use toml::Value;

use crate::{Mark, RiskClass};

pub(crate) fn parse_mark(v: &Value) -> Result<Mark, String> {
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
