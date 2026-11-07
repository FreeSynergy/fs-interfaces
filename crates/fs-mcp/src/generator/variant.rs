//! Splits a method into the candidates that become tools: one per union member of a parameter (so that
//! `Conversation.act` is not one tool of mixed risk), else the method itself.
//!
//! Pattern: Iterator-style expansion (a method yields several variants). Trait: none (internal).

use super::run::Run;
use crate::{Field, Method, Target, TypeRef};

/// One candidate tool.
pub(super) struct Variant {
    /// Where the mark sits: the union member, or the method.
    pub target: Target,
    /// The member's name, for the tool name.
    pub member: Option<String>,
    /// The member's comment.
    pub member_doc: String,
    /// The arguments of this tool.
    pub inputs: Vec<Field>,
}

impl Run<'_> {
    pub(super) fn variants(&self, file: &str, iface: &str, m: &Method) -> Vec<Variant> {
        let whole = Target::item(file, iface, &m.name);
        let expandable = m.params.iter().enumerate().find_map(|(ix, p)| match &p.ty {
            TypeRef::Struct(n) => self
                .index
                .struct_def(n)
                .filter(|s| s.is_pure_union())
                .map(|s| (ix, n, s)),
            _ => None,
        });
        let Some((ix, name, def)) = expandable else {
            return vec![Variant {
                target: whole,
                member: None,
                member_doc: String::new(),
                inputs: m.params.clone(),
            }];
        };
        def.fields
            .iter()
            .map(|member| {
                let mut inputs = m.params.clone();
                if member.ty == TypeRef::Void {
                    inputs.remove(ix);
                } else {
                    inputs[ix] = Field {
                        name: member.name.clone(),
                        ty: member.ty.clone(),
                        doc: member.doc.clone(),
                        in_union: false,
                    };
                }
                Variant {
                    target: Target::item(&name.file, &def.name, &member.name),
                    member: Some(member.name.clone()),
                    member_doc: member.doc.clone(),
                    inputs,
                }
            })
            .collect()
    }
}
