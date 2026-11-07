//! The generator's own view of the contracts: plain data, independent of how they were read.
//!
//! Pattern: Adapter target — [`crate::ContractSource`] implementations translate a foreign form (the
//! `CodeGeneratorRequest` of the Cap'n Proto compiler) into these types, so the generator and its tests
//! never touch compiler internals. Trait: [`crate::ContractSource`].

use std::collections::HashMap;

/// A type by schema file stem and name; groups are `Struct.group`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeName {
    /// Schema file stem, such as `messenger`.
    pub file: String,
    /// Declared name, such as `MessageAction`.
    pub name: String,
}

/// The type of a field or parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeRef {
    /// `Void`.
    Void,
    /// `Bool`.
    Bool,
    /// Integers; `unsigned` and the bit width.
    Int {
        /// Unsigned?
        unsigned: bool,
        /// 8, 16, 32 or 64.
        bits: u8,
    },
    /// `Float32` or `Float64`.
    Float,
    /// `Text`.
    Text,
    /// `Data`.
    Data,
    /// `List(T)`.
    List(Box<TypeRef>),
    /// An enum.
    Enum(TypeName),
    /// A struct or group.
    Struct(TypeName),
    /// A capability.
    Interface(TypeName),
    /// `AnyPointer`: nothing is known.
    Any,
}

/// A struct field, union member, method parameter or method result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Name as declared.
    pub name: String,
    /// Comment from the schema; empty if none.
    pub doc: String,
    /// Type.
    pub ty: TypeRef,
    /// Member of the struct's union?
    pub in_union: bool,
}

/// A struct or group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructDef {
    /// Name, `Outer.group` for a group.
    pub name: String,
    /// Comment from the schema.
    pub doc: String,
    /// Fields in declaration order.
    pub fields: Vec<Field>,
}

impl StructDef {
    /// True if the struct is nothing but a union (two or more members): the form of `MessageAction`.
    #[must_use]
    pub fn is_pure_union(&self) -> bool {
        self.fields.len() >= 2 && self.fields.iter().all(|f| f.in_union)
    }
}

/// An enum with its names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDef {
    /// Name.
    pub name: String,
    /// Enumerant names in ordinal order.
    pub values: Vec<String>,
}

/// One method of an interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    /// Name as declared.
    pub name: String,
    /// Comment from the schema.
    pub doc: String,
    /// Parameters.
    pub params: Vec<Field>,
    /// Results.
    pub results: Vec<Field>,
    /// `-> stream`: a flow, not a call with an answer.
    pub streaming: bool,
}

/// An interface (a capability).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    /// Name.
    pub name: String,
    /// Comment from the schema.
    pub doc: String,
    /// Methods in declaration order.
    pub methods: Vec<Method>,
}

/// One schema file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contract {
    /// File stem, such as `messenger`.
    pub file: String,
    /// Interfaces.
    pub interfaces: Vec<Interface>,
    /// Structs and groups.
    pub structs: Vec<StructDef>,
    /// Enums.
    pub enums: Vec<EnumDef>,
}

/// Lookup over all contracts, for following type references across files.
pub struct Index<'a> {
    structs: HashMap<TypeName, &'a StructDef>,
    enums: HashMap<TypeName, &'a EnumDef>,
}

impl<'a> Index<'a> {
    /// Index every struct and enum of `contracts`.
    #[must_use]
    pub fn new(contracts: &'a [Contract]) -> Self {
        let name = |c: &Contract, n: &str| TypeName {
            file: c.file.clone(),
            name: n.to_owned(),
        };
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        for c in contracts {
            for s in &c.structs {
                structs.insert(name(c, &s.name), s);
            }
            for e in &c.enums {
                enums.insert(name(c, &e.name), e);
            }
        }
        Self { structs, enums }
    }

    /// The struct behind a reference, if the contracts hold it.
    #[must_use]
    pub fn struct_def(&self, t: &TypeName) -> Option<&'a StructDef> {
        self.structs.get(t).copied()
    }

    /// The enum behind a reference.
    #[must_use]
    pub fn enum_def(&self, t: &TypeName) -> Option<&'a EnumDef> {
        self.enums.get(t).copied()
    }
}
