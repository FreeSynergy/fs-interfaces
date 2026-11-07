//! Resource templates: change feeds (`subscribe`) and bytes (`Blob`) are read as resources, never called.
//!
//! Pattern: Builder (a template is assembled from the method's parts). Trait: none (internal).

use super::tools::lower_first;
use super::variant::Variant;
use super::vet::Verdict;
use super::run::Run;
use crate::{Method, ResourceKind, ResourceTemplate, Target};

impl Run<'_> {
    /// A `subscribe`-like method (it takes a listener): its events are a resource.
    pub(super) fn events_resource(&mut self, file: &str, iface: &str, m: &Method, root: bool) {
        let target = Target::item(file, iface, &m.name);
        let inputs: Vec<_> = m.params.iter().filter(|p| p.ty.interfaces().is_empty()).cloned().collect();
        let Some(v) = self.vet(file, iface, m, &target, &inputs) else { return };
        let handle = if root { String::new() } else { "{handle}/".to_owned() };
        let query = query(&inputs);
        self.push(ResourceKind::Events, format!("fs://{file}/{iface}/{handle}events{query}"), file, iface, m, &target, &v);
    }

    /// A method marked `resource = true` (bytes of a `Blob`).
    pub(super) fn blob_resource(&mut self, file: &str, iface: &str, m: &Method, v: &Variant, root: bool, verdict: &Verdict) {
        let handle = if root { String::new() } else { "{handle}/".to_owned() };
        let uri = format!("fs://{file}/{iface}/{handle}{}{}", m.name, query(&v.inputs));
        self.push(ResourceKind::Blob, uri, file, iface, m, &v.target, verdict);
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, kind: ResourceKind, uri: String, file: &str, iface: &str, m: &Method, target: &Target, v: &Verdict) {
        let doc = if m.doc.is_empty() { "(the contract has no comment on this)".to_owned() } else { m.doc.clone() };
        self.out.resources.push(ResourceTemplate {
            uri_template: uri,
            name: format!("{file}.{}.{}", lower_first(iface), m.name),
            description: doc,
            kind,
            risk: v.class,
            source: target.clone(),
            unmarked: v.mark.is_none(),
        });
    }
}

fn query(inputs: &[crate::Field]) -> String {
    if inputs.is_empty() {
        return String::new();
    }
    format!("{{?{}}}", inputs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(","))
}
