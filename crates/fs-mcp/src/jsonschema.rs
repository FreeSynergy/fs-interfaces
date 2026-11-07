//! Contract types to JSON Schema (draft 2020-12, the dialect MCP uses).
//!
//! Pattern: Builder — [`SchemaBuilder`] collects the `$defs` of every struct it meets, once each, and
//! `finish` attaches them to the root. Trait: none (internal).

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{Field, Index, TypeName, TypeRef};

/// Builds one schema document.
pub struct SchemaBuilder<'a> {
    index: &'a Index<'a>,
    defs: BTreeMap<String, Value>,
}

fn def_key(t: &TypeName) -> String {
    format!("{}.{}", t.file, t.name)
}

impl<'a> SchemaBuilder<'a> {
    /// A builder that follows struct references through `index`.
    #[must_use]
    pub fn new(index: &'a Index<'a>) -> Self {
        Self { index, defs: BTreeMap::new() }
    }

    /// Schema of one type. A capability is a string handle (flat tools, ADR 0021 extension).
    pub fn ty(&mut self, ty: &TypeRef) -> Value {
        match ty {
            TypeRef::Void => json!({"type": "null"}),
            TypeRef::Bool => json!({"type": "boolean"}),
            TypeRef::Int { unsigned: true, .. } => json!({"type": "integer", "minimum": 0}),
            TypeRef::Int { .. } => json!({"type": "integer"}),
            TypeRef::Float => json!({"type": "number"}),
            TypeRef::Text => json!({"type": "string"}),
            TypeRef::Data => json!({"type": "string", "contentEncoding": "base64"}),
            TypeRef::List(inner) => json!({"type": "array", "items": self.ty(inner)}),
            TypeRef::Enum(n) => match self.index.enum_def(n) {
                Some(e) => json!({"type": "string", "enum": e.values}),
                None => json!({"type": "string"}),
            },
            TypeRef::Interface(n) => json!({"type": "string", "x-fs-handle": n.name}),
            TypeRef::Any => json!({}),
            TypeRef::Struct(n) => self.struct_ref(n),
        }
    }

    fn struct_ref(&mut self, n: &TypeName) -> Value {
        let key = def_key(n);
        if !self.defs.contains_key(&key) {
            let Some(def) = self.index.struct_def(n) else { return json!({}) };
            self.defs.insert(key.clone(), json!({}));
            let body = self.object(&def.fields.iter().collect::<Vec<_>>(), &def.doc);
            self.defs.insert(key.clone(), body);
        }
        json!({"$ref": format!("#/$defs/{key}")})
    }

    /// An object schema over `fields`; union members are alternatives (exactly one is set).
    pub fn object(&mut self, fields: &[&Field], doc: &str) -> Value {
        let mut props = Map::new();
        for f in fields {
            let mut s = self.ty(&f.ty);
            if !f.doc.is_empty() {
                s = match s {
                    Value::Object(mut o) if !o.contains_key("$ref") => {
                        o.insert("description".into(), f.doc.clone().into());
                        Value::Object(o)
                    }
                    other => json!({"description": f.doc, "allOf": [other]}),
                };
            }
            props.insert(f.name.clone(), s);
        }
        let mut obj = json!({"type": "object", "properties": props});
        if !doc.is_empty() {
            obj["description"] = doc.into();
        }
        let alternatives: Vec<Value> =
            fields.iter().filter(|f| f.in_union).map(|f| json!({"required": [f.name]})).collect();
        if !alternatives.is_empty() {
            obj["oneOf"] = alternatives.into();
        }
        obj
    }

    /// Attach the collected `$defs` to `root`.
    #[must_use]
    pub fn finish(self, mut root: Value) -> Value {
        if !self.defs.is_empty() {
            root["$defs"] = Value::Object(self.defs.into_iter().collect());
        }
        root
    }
}
