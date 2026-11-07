//! Walks the nodes of a request: interfaces, structs (and groups), enums, per requested file.
//!
//! Pattern: Adapter — the foreign form is read here and nowhere else. Trait: none (internal).

use capnp::schema_capnp::{code_generator_request, field, node};

use super::names::{Tables, stem, text};
use super::types::map;
use crate::{Contract, EnumDef, Error, Field, Interface, Method, StructDef, TypeName, TypeRef};

const NO_DISCRIMINANT: u16 = 0xffff;

pub(super) fn read_request(
    req: code_generator_request::Reader<'_>,
) -> Result<Vec<Contract>, Error> {
    let tables = Tables::build(req)?;
    let mut out = Vec::new();
    for file in req.get_requested_files()? {
        let mut contract = Contract {
            file: stem(&text(file.get_filename()?)?),
            ..Contract::default()
        };
        collect(&tables, file.get_id(), &mut contract)?;
        out.push(contract);
    }
    Ok(out)
}

fn collect(t: &Tables<'_>, id: u64, c: &mut Contract) -> Result<(), Error> {
    let Some(node) = t.nodes.get(&id).copied() else {
        return Ok(());
    };
    for nested in node.get_nested_nodes()? {
        let nid = nested.get_id();
        let Some(n) = t.nodes.get(&nid).copied() else {
            continue;
        };
        let name = t.type_name(nid).name;
        match n.which()? {
            node::Which::Struct(_) => read_struct(t, nid, &name, c)?,
            node::Which::Enum(e) => {
                let values = e
                    .get_enumerants()?
                    .iter()
                    .map(|v| text(v.get_name()?))
                    .collect::<Result<_, _>>()?;
                c.enums.push(EnumDef {
                    name: name.clone(),
                    values,
                });
            }
            node::Which::Interface(i) => {
                let doc = t.doc(nid);
                let methods = i
                    .get_methods()?
                    .iter()
                    .enumerate()
                    .map(|(ix, m)| read_method(t, nid, ix, m))
                    .collect::<Result<_, _>>()?;
                c.interfaces.push(Interface {
                    name: name.clone(),
                    doc,
                    methods,
                });
            }
            _ => {}
        }
        collect(t, nid, c)?;
    }
    Ok(())
}

fn read_struct(t: &Tables<'_>, id: u64, name: &str, c: &mut Contract) -> Result<(), Error> {
    let Some(node) = t.nodes.get(&id).copied() else {
        return Ok(());
    };
    let node::Which::Struct(s) = node.which()? else {
        return Ok(());
    };
    let file = t.names.get(&id).map(|n| n.file.clone()).unwrap_or_default();
    let mut fields = Vec::new();
    for (ix, f) in s.get_fields()?.iter().enumerate() {
        let fname = text(f.get_name()?)?;
        let ty = match f.which()? {
            field::Which::Slot(slot) => map(t, slot.get_type()?)?,
            field::Which::Group(g) => {
                let gname = format!("{name}.{fname}");
                let gid = g.get_type_id();
                let mut inner = Contract::default();
                read_struct(t, gid, &gname, &mut inner)?;
                c.structs.append(&mut inner.structs);
                TypeRef::Struct(TypeName {
                    file: file.clone(),
                    name: gname,
                })
            }
        };
        fields.push(Field {
            name: fname,
            doc: t.member_doc(id, ix),
            ty,
            in_union: f.get_discriminant_value() != NO_DISCRIMINANT,
        });
    }
    c.structs.push(StructDef {
        name: name.to_owned(),
        doc: t.doc(id),
        fields,
    });
    Ok(())
}

fn params(t: &Tables<'_>, id: u64) -> Result<(Vec<Field>, bool), Error> {
    let Some(node) = t.nodes.get(&id).copied() else {
        return Ok((vec![], false));
    };
    let streaming = text(node.get_display_name()?)?.ends_with("StreamResult");
    let node::Which::Struct(s) = node.which()? else {
        return Ok((vec![], false));
    };
    let mut out = Vec::new();
    for (ix, f) in s.get_fields()?.iter().enumerate() {
        if let field::Which::Slot(slot) = f.which()? {
            out.push(Field {
                name: text(f.get_name()?)?,
                doc: t.member_doc(id, ix),
                ty: map(t, slot.get_type()?)?,
                in_union: false,
            });
        }
    }
    Ok((out, streaming))
}

fn read_method(
    t: &Tables<'_>,
    iface: u64,
    ix: usize,
    m: capnp::schema_capnp::method::Reader<'_>,
) -> Result<Method, Error> {
    let (params, _) = params(t, m.get_param_struct_type())?;
    let (results, streaming) = params_of(t, m.get_result_struct_type())?;
    Ok(Method {
        name: text(m.get_name()?)?,
        doc: t.member_doc(iface, ix),
        params,
        results,
        streaming,
    })
}

fn params_of(t: &Tables<'_>, id: u64) -> Result<(Vec<Field>, bool), Error> {
    let (fields, streaming) = params(t, id)?;
    Ok(if streaming {
        (vec![], true)
    } else {
        (fields, false)
    })
}
