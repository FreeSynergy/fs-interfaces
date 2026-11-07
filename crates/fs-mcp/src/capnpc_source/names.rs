//! Which node id is which declared name, and the comments of each node.
//!
//! Pattern: Registry — id to (file, name) and id to comment tables, built once per request.
//! Trait: none (internal).

use std::collections::HashMap;

use capnp::schema_capnp::{code_generator_request, node};

use crate::{Error, TypeName};

/// Id tables of one request.
pub(super) struct Tables<'a> {
    pub nodes: HashMap<u64, node::Reader<'a>>,
    pub names: HashMap<u64, TypeName>,
    pub docs: HashMap<u64, (String, Vec<String>)>,
}

pub(super) fn text(r: capnp::text::Reader<'_>) -> Result<String, Error> {
    Ok(r.to_str()?.to_owned())
}

/// File stem of a requested file name (`messenger.capnp` -> `messenger`).
pub(super) fn stem(filename: &str) -> String {
    let base = filename.rsplit('/').next().unwrap_or(filename);
    base.strip_suffix(".capnp").unwrap_or(base).to_owned()
}

impl<'a> Tables<'a> {
    pub(super) fn build(req: code_generator_request::Reader<'a>) -> Result<Self, Error> {
        let mut t = Self {
            nodes: HashMap::new(),
            names: HashMap::new(),
            docs: HashMap::new(),
        };
        for n in req.get_nodes()? {
            t.nodes.insert(n.get_id(), n);
        }
        for info in req.get_source_info()? {
            let members = info
                .get_members()?
                .iter()
                .map(|m| text(m.get_doc_comment()?))
                .collect::<Result<Vec<_>, _>>()?;
            t.docs
                .insert(info.get_id(), (text(info.get_doc_comment()?)?, members));
        }
        for file in req.get_requested_files()? {
            let file_stem = stem(&text(file.get_filename()?)?);
            t.name_nested(file.get_id(), &file_stem, "")?;
        }
        Ok(t)
    }

    fn name_nested(&mut self, id: u64, file: &str, prefix: &str) -> Result<(), Error> {
        let Some(node) = self.nodes.get(&id).copied() else {
            return Ok(());
        };
        for nested in node.get_nested_nodes()? {
            let name = format!("{prefix}{}", text(nested.get_name()?)?);
            let nid = nested.get_id();
            self.names.insert(
                nid,
                TypeName {
                    file: file.to_owned(),
                    name: name.clone(),
                },
            );
            self.name_nested(nid, file, &format!("{name}."))?;
        }
        Ok(())
    }

    /// Name of a type id; types of files that were not requested keep their display name under file `?`.
    pub(super) fn type_name(&self, id: u64) -> TypeName {
        self.names.get(&id).cloned().unwrap_or_else(|| TypeName {
            file: "?".to_owned(),
            name: self
                .nodes
                .get(&id)
                .and_then(|n| n.get_display_name().ok())
                .and_then(|d| d.to_str().ok())
                .unwrap_or("?")
                .to_owned(),
        })
    }

    pub(super) fn doc(&self, id: u64) -> String {
        self.docs.get(&id).map(|d| d.0.clone()).unwrap_or_default()
    }

    pub(super) fn member_doc(&self, id: u64, index: usize) -> String {
        self.docs
            .get(&id)
            .and_then(|d| d.1.get(index).cloned())
            .unwrap_or_default()
    }
}
