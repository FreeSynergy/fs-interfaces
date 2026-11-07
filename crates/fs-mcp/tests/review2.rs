//! Second review round: interface-level exclusion, nested type names, interfaces as types, what a listener is,
//! and secret gaps (struct comments, short forms, run-together words). Synthetic contracts against the traits.

mod support;

use fs_mcp::{Blocklist, Generator, MarkFiles, RiskClass, ToolGenerator, TypeName, TypeRef};
use support::{
    FILE, FakeMarks, contract, field, has_tool, iface, interface, method, read_all, run, strukt,
};

fn text() -> TypeRef {
    TypeRef::Text
}
fn nested(n: &str) -> TypeRef {
    TypeRef::Struct(TypeName {
        file: FILE.into(),
        name: n.into(),
    })
}

#[test]
fn an_interface_mark_excludes_all_methods_present_and_future_and_every_handle_to_it() {
    let marks = MarkFiles::from_sources(&[(FILE, "[Roles]\n\"@interface\" = { risk = \"excluded\", reason = \"admin\" }\n[Root]\nroles = \"read\"\nfine = \"read\"\n")]).unwrap();
    let c = contract(
        vec![
            interface(
                "Root",
                vec![
                    method("roles", vec![], vec![field("r", iface("Roles"))]),
                    method("fine", vec![], vec![field("x", text())]),
                ],
            ),
            interface(
                "Roles",
                vec![
                    method("grantAdmin", vec![], vec![]),
                    method("list", vec![], vec![field("x", text())]),
                ],
            ),
        ],
        vec![],
    );
    let out = Generator::new(&marks, Blocklist::standard())
        .generate(&c)
        .unwrap();
    assert!(!has_tool(&out, "Roles", "grantAdmin") && !has_tool(&out, "Roles", "list"));
    assert!(
        !has_tool(&out, "Root", "roles"),
        "a handle to an excluded interface is excluded"
    );
    assert!(has_tool(&out, "Root", "fine"));
    assert!(
        !out.unmarked.iter().any(|t| t.as_str().contains("Roles")),
        "no marks are demanded inside it"
    );
    let parse = |t: &str| MarkFiles::from_sources(&[(FILE, t)]);
    assert!(
        parse("[Roles]\n\"@interface\" = \"read\"\n").is_err(),
        "an interface mark can only exclude"
    );
}

#[test]
fn a_nested_type_cannot_hide_a_blocked_name() {
    let c = contract(
        vec![interface(
            "Root",
            vec![method("d", vec![field("w", nested("Wrap"))], vec![])],
        )],
        vec![
            strukt("Wrap", vec![field("d", nested("Wrap.ToolCallDecision"))]),
            strukt("Wrap.ToolCallDecision", vec![field("option", text())]),
        ],
    );
    assert!(!has_tool(
        &run(&c, &read_all(&[("Root", "d")])),
        "Root",
        "d"
    ));
}

#[test]
fn an_interface_as_a_type_is_judged_by_name_and_comment() {
    let mut vault = interface(
        "Vault",
        vec![method("reveal", vec![], vec![field("x", text())])],
    );
    vault.doc = "holds the private key of the account".into();
    let c = contract(
        vec![
            interface(
                "Root",
                vec![
                    method("store", vec![], vec![field("s", iface("SecretStore"))]),
                    method("vault", vec![], vec![field("s", iface("Vault"))]),
                ],
            ),
            interface(
                "SecretStore",
                vec![method("reveal", vec![], vec![field("x", text())])],
            ),
            vault,
        ],
        vec![],
    );
    let out = run(
        &c,
        &read_all(&[
            ("Root", "store"),
            ("Root", "vault"),
            ("SecretStore", "reveal"),
            ("Vault", "reveal"),
        ]),
    );
    for (o, i) in [
        ("Root", "store"),
        ("Root", "vault"),
        ("SecretStore", "reveal"),
        ("Vault", "reveal"),
    ] {
        assert!(!has_tool(&out, o, i), "{o}.{i}");
    }
}

#[test]
fn only_an_interface_named_listener_is_a_listener() {
    let c = contract(
        vec![
            interface(
                "Root",
                vec![method("take", vec![field("source", iface("Blob"))], vec![])],
            ),
            interface(
                "Blob",
                vec![method("read", vec![], vec![field("x", text())])],
            ),
        ],
        vec![],
    );
    let out = run(&c, &read_all(&[("Root", "take"), ("Blob", "read")]));
    assert!(
        out.resources.is_empty(),
        "a handle passed in is no listener"
    );
    assert!(has_tool(&out, "Root", "take") && has_tool(&out, "Blob", "read"));
}

fn feed(extra_method: bool) -> Vec<fs_mcp::Contract> {
    let mut root = vec![method(
        "subscribe",
        vec![field("l", iface("FooListener"))],
        vec![],
    )];
    if extra_method {
        root.push(method(
            "watch",
            vec![field("l", iface("FooListener"))],
            vec![],
        ));
    }
    contract(
        vec![
            interface("Root", root),
            interface(
                "FooListener",
                vec![method("event", vec![field("e", text())], vec![])],
            ),
        ],
        vec![],
    )
}

#[test]
fn an_events_resource_needs_class_read_and_its_uri_must_be_unique() {
    assert_eq!(
        run(&feed(false), &read_all(&[("Root", "subscribe")]))
            .resources
            .len(),
        1
    );
    let mut change = FakeMarks::default();
    change.put("Root", "subscribe", RiskClass::Change);
    assert!(
        run(&feed(false), &change).resources.is_empty(),
        "only class read may be a resource"
    );
    assert!(
        run(&feed(false), &FakeMarks::default())
            .resources
            .is_empty(),
        "unknown is not read"
    );
    let both = read_all(&[("Root", "subscribe"), ("Root", "watch")]);
    assert!(
        Generator::new(&both, Blocklist::standard())
            .generate(&feed(true))
            .is_err(),
        "equal URIs are an error"
    );
}
