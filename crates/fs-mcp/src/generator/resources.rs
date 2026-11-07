//! Resource templates: change feeds (`subscribe`) and bytes (`Blob`) are read as resources, never called.
//!
//! Pattern: Value object — a [`ResourceTemplate`] is assembled once from the method's parts.
//! Trait: none (internal).

use super::run::Run;
use super::tools::lower_first;
use super::variant::Variant;
use super::vet::Verdict;
use crate::{Field, Method, ResourceKind, ResourceTemplate, Target, Why};

impl Run<'_> {
    /// The listener hands the events to the caller, so everything the listener's methods carry is read by
    /// the caller: it goes through the same screen as a result. A listener nobody defined is a problem.
    fn listener_problem(&self, m: &Method) -> Option<Why> {
        let screen = self.screen();
        for p in self.facts.callback_params(m) {
            for n in p.ty.interfaces() {
                let Some(def) = self.index.interface_def(n) else {
                    return Some(Why::Unclear {
                        path: format!("{}/{}", n.file, n.name),
                    });
                };
                if let Some(why) = screen.interface(&n.file, def) {
                    return Some(why);
                }
                for lm in &def.methods {
                    if lm.unclear {
                        return Some(Why::Unclear {
                            path: format!("{}/{}.{}", n.file, n.name, lm.name),
                        });
                    }
                    let fields: Vec<&Field> = lm.params.iter().collect();
                    if let Some(why) = screen.fields(&n.file, &n.name, &lm.name, &fields) {
                        return Some(why);
                    }
                }
            }
        }
        None
    }

    /// The free-format fields that the listener's events carry.
    fn listener_free(&self, m: &Method) -> Vec<(String, String)> {
        let screen = self.screen();
        let mut out = Vec::new();
        for n in self
            .facts
            .callback_params(m)
            .iter()
            .flat_map(|p| p.ty.interfaces())
        {
            for lm in self.index.interface_def(n).iter().flat_map(|d| &d.methods) {
                let fields: Vec<&Field> = lm.params.iter().collect();
                out.extend(screen.free_fields(&n.file, &n.name, &lm.name, &fields));
            }
        }
        out
    }

    /// A `subscribe`-like method (it takes a listener): its events are a resource.
    pub(super) fn events_resource(&mut self, file: &str, iface: &str, m: &Method, root: bool) {
        let target = Target::item(file, iface, &m.name);
        let cb = self.facts.callback_params(m);
        let inputs: Vec<_> = m
            .params
            .iter()
            .filter(|p| !cb.iter().any(|c| c.name == p.name))
            .cloned()
            .collect();
        let v = Variant {
            target: target.clone(),
            member: None,
            member_doc: String::new(),
            inputs,
            whole: None,
        };
        if let Some(why) = self.listener_problem(m) {
            return self.leave(target, why);
        }
        let Some(mut verdict) = self.vet(file, iface, m, &v) else {
            return;
        };
        verdict.free.extend(self.listener_free(m));
        verdict.free.sort();
        verdict.free.dedup();
        if verdict.class != crate::RiskClass::Read {
            return self.leave(
                target,
                Why::Marked("an events resource needs a mark of class read".to_owned()),
            );
        }
        let handle = if root {
            String::new()
        } else {
            "{handle}/".to_owned()
        };
        let query = query(&v.inputs);
        self.push(
            ResourceKind::Events,
            format!("fs://{file}/{iface}/{handle}events{query}"),
            file,
            iface,
            m,
            &target,
            &verdict,
        );
    }

    /// A method marked `resource = true` (bytes of a `Blob`).
    pub(super) fn blob_resource(
        &mut self,
        file: &str,
        iface: &str,
        m: &Method,
        v: &Variant,
        root: bool,
        verdict: &Verdict,
    ) {
        let handle = if root {
            String::new()
        } else {
            "{handle}/".to_owned()
        };
        let uri = format!("fs://{file}/{iface}/{handle}{}{}", m.name, query(&v.inputs));
        self.push(ResourceKind::Blob, uri, file, iface, m, &v.target, verdict);
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        kind: ResourceKind,
        uri: String,
        file: &str,
        iface: &str,
        m: &Method,
        target: &Target,
        v: &Verdict,
    ) {
        let doc = if m.doc.is_empty() {
            "(the contract has no comment on this)".to_owned()
        } else {
            m.doc.clone()
        };
        self.out.resources.push(ResourceTemplate {
            uri_template: uri,
            name: format!("{file}.{}.{}", lower_first(iface), m.name),
            description: doc,
            kind,
            risk: v.class,
            source: target.clone(),
            unmarked: v.mark.is_none(),
            free_content: v.free.clone(),
        });
    }
}

fn query(inputs: &[crate::Field]) -> String {
    if inputs.is_empty() {
        return String::new();
    }
    format!(
        "{{?{}}}",
        inputs
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    )
}
