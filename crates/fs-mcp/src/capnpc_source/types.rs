//! Cap'n Proto types to the generator's [`TypeRef`].
//!
//! Pattern: none (one pure mapping function). Trait: none.

use capnp::schema_capnp::type_;

use super::names::Tables;
use crate::{Error, TypeRef};

pub(super) fn map(t: &Tables<'_>, ty: type_::Reader<'_>) -> Result<TypeRef, Error> {
    use type_::Which as W;
    let int = |unsigned, bits| TypeRef::Int { unsigned, bits };
    Ok(match ty.which()? {
        W::Void(()) => TypeRef::Void,
        W::Bool(()) => TypeRef::Bool,
        W::Int8(()) => int(false, 8),
        W::Int16(()) => int(false, 16),
        W::Int32(()) => int(false, 32),
        W::Int64(()) => int(false, 64),
        W::Uint8(()) => int(true, 8),
        W::Uint16(()) => int(true, 16),
        W::Uint32(()) => int(true, 32),
        W::Uint64(()) => int(true, 64),
        W::Float32(()) | W::Float64(()) => TypeRef::Float,
        W::Text(()) => TypeRef::Text,
        W::Data(()) => TypeRef::Data,
        W::List(l) => TypeRef::List(Box::new(map(t, l.get_element_type()?)?)),
        W::Enum(e) => TypeRef::Enum(t.type_name(e.get_type_id())),
        W::Struct(s) => TypeRef::Struct(t.type_name(s.get_type_id())),
        W::Interface(i) => TypeRef::Interface(t.type_name(i.get_type_id())),
        W::AnyPointer(_) => TypeRef::Any,
    })
}
