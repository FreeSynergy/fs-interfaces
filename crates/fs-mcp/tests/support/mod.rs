//! Test doubles and a synthetic contract shaped like `messenger.capnp`, so the rules are tested without
//! touching the real schemas. Test helpers; no pattern of their own.
#![allow(dead_code)]

pub mod fake;
pub mod real;
#[allow(unused_imports)]
pub use fake::FakeMarks;

use fs_mcp::{Contract, Field, Interface, Method, StructDef, TypeName, TypeRef};

pub const FILE: &str = "messenger";

pub fn name(n: &str) -> TypeName {
    TypeName {
        file: FILE.into(),
        name: n.into(),
    }
}
pub fn field(n: &str, ty: TypeRef) -> Field {
    Field {
        name: n.into(),
        doc: String::new(),
        ty,
        in_union: false,
    }
}
pub fn member(n: &str, ty: TypeRef) -> Field {
    Field {
        in_union: true,
        ..field(n, ty)
    }
}
pub fn iface(n: &str) -> TypeRef {
    TypeRef::Interface(name(n))
}
pub fn st(n: &str) -> TypeRef {
    TypeRef::Struct(name(n))
}
pub fn method(n: &str, params: Vec<Field>, results: Vec<Field>) -> Method {
    Method {
        name: n.into(),
        doc: format!("doc of {n}"),
        params,
        results,
        streaming: false,
        unclear: false,
    }
}
pub fn interface(n: &str, methods: Vec<Method>) -> Interface {
    Interface {
        name: n.into(),
        doc: String::new(),
        methods,
    }
}
pub fn strukt(n: &str, fields: Vec<Field>) -> StructDef {
    StructDef {
        name: n.into(),
        doc: String::new(),
        fields,
    }
}

/// A small contract with every shape the generator must handle.
pub fn messenger_like() -> Vec<Contract> {
    let mut listener = method("event", vec![field("event", TypeRef::Text)], vec![]);
    listener.streaming = true;
    vec![Contract {
        file: FILE.into(),
        interfaces: vec![
            interface(
                "Messenger",
                vec![
                    method(
                        "capabilities",
                        vec![],
                        vec![field("ids", TypeRef::List(Box::new(TypeRef::Text)))],
                    ),
                    method(
                        "conversations",
                        vec![],
                        vec![field("conversations", iface("Conversations"))],
                    ),
                    method(
                        "subscribe",
                        vec![
                            field("filter", TypeRef::Text),
                            field("listener", iface("Listener")),
                        ],
                        vec![field("subscription", iface("Subscription"))],
                    ),
                    method("draft", vec![], vec![field("draft", iface("Draft"))]),
                    method("guard", vec![], vec![field("guard", iface("Guard"))]),
                ],
            ),
            interface(
                "Conversations",
                vec![method(
                    "open",
                    vec![field("id", TypeRef::Text)],
                    vec![field("conversation", iface("Conversation"))],
                )],
            ),
            interface(
                "Conversation",
                vec![method(
                    "act",
                    vec![
                        field("id", TypeRef::Text),
                        field("action", st("MessageAction")),
                    ],
                    vec![field("problem", st("Problem"))],
                )],
            ),
            interface(
                "Guard",
                vec![method("allow", vec![field("what", TypeRef::Text)], vec![])],
            ),
            interface("Listener", vec![listener]),
            interface("Subscription", vec![]),
            interface(
                "Accounts",
                vec![method(
                    "add",
                    vec![field("setup", st("AccountSetup"))],
                    vec![field("account", TypeRef::Text)],
                )],
            ),
            interface(
                "Draft",
                vec![method(
                    "send",
                    vec![field(
                        "at",
                        TypeRef::Int {
                            unsigned: false,
                            bits: 64,
                        },
                    )],
                    vec![field("id", TypeRef::Text)],
                )],
            ),
        ],
        structs: vec![
            strukt("Problem", vec![field("title", TypeRef::Text)]),
            strukt(
                "MessageAction",
                vec![
                    member("none", TypeRef::Void),
                    member("delete", TypeRef::Void),
                    member("flag", TypeRef::Bool),
                    member("decide", st("ToolCallDecision")),
                ],
            ),
            strukt(
                "ToolCallDecision",
                vec![
                    field("proof", TypeRef::Data),
                    field("signer", TypeRef::Text),
                ],
            ),
            strukt(
                "AccountSetup",
                vec![
                    field("address", TypeRef::Text),
                    field("secret", TypeRef::Text),
                ],
            ),
        ],
        enums: vec![],
    }]
}

pub mod helpers;
#[allow(unused_imports)]
pub use helpers::{contract, has_tool, read_all, run};
