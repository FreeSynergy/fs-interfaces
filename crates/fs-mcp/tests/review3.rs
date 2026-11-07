//! Third review round: a union parameter is screened as a whole before it is split, a listener is judged like
//! any interface, roots are only what a root can reach. Synthetic contracts against the traits.

mod support;

use fs_mcp::{Mark, RiskClass, Target, TypeName, TypeRef};
use support::{
    FILE, FakeMarks, contract, field, has_tool, iface, interface, member, method, read_all, run,
    st, strukt,
};

fn text() -> TypeRef {
    TypeRef::Text
}
fn tn(n: &str) -> TypeRef {
    TypeRef::Struct(TypeName {
        file: FILE.into(),
        name: n.into(),
    })
}

#[test]
fn a_union_parameter_is_screened_as_a_whole_before_it_is_split() {
    let mut secret_choice = strukt(
        "SecretChoice",
        vec![member("a", text()), member("b", text())],
    );
    secret_choice.doc = "holds the password".into();
    let c = contract(
        vec![interface(
            "Root",
            vec![
                method(
                    "configure",
                    vec![field("credentials", st("Choice"))],
                    vec![],
                ),
                method("go", vec![field("d", tn("Outer.ToolCallDecision"))], vec![]),
                method("pick", vec![field("c", st("SecretChoice"))], vec![]),
                method("fine", vec![field("c", st("Plain"))], vec![]),
            ],
        )],
        vec![
            strukt(
                "Choice",
                vec![member("plain", text()), member("hashed", text())],
            ),
            strukt(
                "Outer.ToolCallDecision",
                vec![member("accept", text()), member("reject", text())],
            ),
            secret_choice,
            strukt("Plain", vec![member("x", text()), member("y", text())]),
        ],
    );
    let mut marks = FakeMarks::default();
    for (o, i, k) in [
        ("Choice", "plain", 1),
        ("Choice", "hashed", 1),
        ("Outer.ToolCallDecision", "accept", 1),
        ("Outer.ToolCallDecision", "reject", 1),
        ("SecretChoice", "a", 1),
        ("SecretChoice", "b", 1),
        ("Plain", "x", 1),
        ("Plain", "y", 1),
    ] {
        let _ = k;
        marks.put(o, i, RiskClass::Read);
    }
    let out = run(&c, &marks);
    for (o, m) in [
        ("Choice", "plain"),
        ("Choice", "hashed"),
        ("Outer.ToolCallDecision", "accept"),
        ("SecretChoice", "a"),
    ] {
        assert!(
            !has_tool(&out, o, m),
            "{o}.{m} slipped through the union split"
        );
    }
    assert!(
        has_tool(&out, "Plain", "x"),
        "a harmless union still splits"
    );
}

#[test]
fn the_real_survey_credential_does_not_become_a_tool() {
    let c = support::real::real_catalog();
    assert!(
        c.tools
            .iter()
            .all(|t| !t.name.starts_with("surveys.respond.submit")),
        "submit slipped through"
    );
}

fn listener_contract(name: &str) -> Vec<fs_mcp::Contract> {
    contract(
        vec![
            interface(
                "Root",
                vec![method("subscribe", vec![field("l", iface(name))], vec![])],
            ),
            interface(
                name,
                vec![method("event", vec![field("e", text())], vec![])],
            ),
        ],
        vec![],
    )
}

#[test]
fn a_listener_is_judged_like_any_interface() {
    let mut marks = read_all(&[("Root", "subscribe")]);
    assert_eq!(
        run(&listener_contract("AuditListener"), &marks)
            .resources
            .len(),
        1
    );
    marks.marks.insert(
        Target::owner(FILE, "AuditListener"),
        Mark {
            class: RiskClass::Excluded,
            capability: None,
            resource: false,
            reason: Some("test".into()),
        },
    );
    assert!(
        run(&listener_contract("AuditListener"), &marks)
            .resources
            .is_empty(),
        "an excluded listener gave a resource"
    );
    let marks = read_all(&[("Root", "subscribe")]);
    assert!(
        run(&listener_contract("SessionTokenListener"), &marks)
            .resources
            .is_empty(),
        "a secret-named listener gave a resource"
    );
}

#[test]
fn a_type_only_seen_by_a_listener_is_no_root() {
    let mut l = method("event", vec![field("item", iface("Vault"))], vec![]);
    l.streaming = true;
    let c = contract(
        vec![
            interface(
                "Root",
                vec![method(
                    "subscribe",
                    vec![field("l", iface("FooListener"))],
                    vec![],
                )],
            ),
            interface("FooListener", vec![l]),
            interface(
                "Vault",
                vec![method("show", vec![], vec![field("x", text())])],
            ),
        ],
        vec![],
    );
    let out = run(&c, &read_all(&[("Root", "subscribe"), ("Vault", "show")]));
    let t = out
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "Vault", "show"))
        .expect("a tool");
    assert_eq!(t.handle_of.as_deref(), Some("Vault"), "it needs a handle");
}

#[test]
fn newly_listed_secret_words_are_recognised() {
    for n in [
        "recoveryCode",
        "backupCodes",
        "resetLink",
        "magicLink",
        "webhookUrl",
        "connectionString",
        "nonce",
        "salt",
        "hmac",
        "creds",
    ] {
        assert!(fs_mcp::name_looks_secret(n), "{n}");
    }
}
