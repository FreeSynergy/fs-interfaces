//! Builds the [`Tool`] of a vetted candidate: name, description, input and output schema.
//!
//! Pattern: Value object — a [`Tool`] is assembled once from the parts and never edited afterwards.
//! Trait: none (internal).

use serde_json::{Value, json};

use super::run::Run;
use super::variant::Variant;
use crate::{Error, Field, Interface, Method, SchemaBuilder, Target, Tool, TypeRef};

const NO_DOC: &str = "(the contract has no comment on this)";

pub(super) fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or_else(String::new, |f| f.to_lowercase().chain(c).collect())
}

/// Does the method hand out bytes (a `Data` result)? Only such a method may be a resource.
fn returns_bytes(m: &Method) -> bool {
    m.results.iter().any(|f| match &f.ty {
        TypeRef::Data => true,
        TypeRef::List(inner) => **inner == TypeRef::Data,
        _ => false,
    })
}

impl Run<'_> {
    pub(super) fn method(&mut self, file: &str, iface: &Interface, m: &Method, root: bool) {
        let target = Target::item(file, &iface.name, &m.name);
        if self.g.risk.mark(&target).is_some_and(|k| k.resource) && !returns_bytes(m) {
            self.errors.push(Error::Marks {
                file: file.to_owned(),
                problem: format!("{target}: resource = true, but the method returns no bytes"),
            });
            return;
        }
        if let Some(why) = self.g.blocklist.reason(&target) {
            return self.leave(target, crate::Why::Blocked(why));
        }
        if m.streaming {
            return self.leave(
                target,
                crate::Why::NotACall("a stream: events become a resource"),
            );
        }
        // An exclusion on the method holds for every union member it would be split into.
        if let Some(k) = self
            .g
            .risk
            .mark(&target)
            .filter(|k| k.class == crate::RiskClass::Excluded)
        {
            let why = k.reason.unwrap_or_else(|| "marked excluded".to_owned());
            return self.leave(target, crate::Why::Marked(why));
        }
        if !self.facts.callback_params(m).is_empty() {
            return self.events_resource(file, &iface.name, m, root);
        }
        for v in self.variants(file, &iface.name, m) {
            self.tool(file, iface, m, &v, root);
        }
    }

    fn tool(&mut self, file: &str, iface: &Interface, m: &Method, v: &Variant, root: bool) {
        let Some(verdict) = self.vet(file, &iface.name, m, v) else {
            return;
        };
        if verdict.mark.as_ref().is_some_and(|k| k.resource) {
            return self.blob_resource(file, &iface.name, m, v, root, &verdict);
        }
        let base = format!("{file}.{}.{}", lower_first(&iface.name), m.name);
        let name = verdict
            .mark
            .as_ref()
            .and_then(|k| k.capability.clone())
            .unwrap_or_else(|| match &v.member {
                Some(member) => format!("{base}.{member}"),
                None => base,
            });
        let handle_of = (!root).then(|| iface.name.clone());
        let docs: Vec<&str> = [m.doc.as_str(), v.member_doc.as_str()]
            .into_iter()
            .filter(|d| !d.is_empty())
            .collect();
        self.out.tools.push(Tool {
            name,
            description: if docs.is_empty() {
                NO_DOC.to_owned()
            } else {
                docs.join("\n")
            },
            input_schema: self.input_schema(&v.inputs, handle_of.as_deref()),
            output_schema: self.output_schema(&m.results),
            risk: verdict.class,
            source: v.target.clone(),
            handle_of,
            unmarked: verdict.mark.is_none(),
        });
    }

    pub(super) fn input_schema(&self, inputs: &[Field], handle_of: Option<&str>) -> Value {
        let mut b = SchemaBuilder::new(&self.index);
        let mut obj = b.object(&inputs.iter().collect::<Vec<_>>(), "");
        obj["additionalProperties"] = json!(false);
        if let Some(h) = handle_of {
            obj["properties"]["handle"] = json!({
                "type": "string",
                "description": format!("Handle of a {h}; checked on every call, and it expires."),
                "x-fs-handle": h,
            });
            obj["required"] = json!(["handle"]);
        }
        b.finish(obj)
    }

    fn output_schema(&self, results: &[Field]) -> Value {
        let mut b = SchemaBuilder::new(&self.index);
        let obj = b.object(&results.iter().collect::<Vec<_>>(), "");
        b.finish(obj)
    }
}
