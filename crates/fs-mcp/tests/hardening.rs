//! Fail-closed checks found by review: what is unclear stays out. Each test names the way it used to slip through.
//! Tests the generator against its traits with synthetic contracts.

mod support;

use fs_mcp::{Contract, Target, TypeRef, Why};
use support::{
    FILE, FakeMarks, contract, field, has_tool, iface, interface, member, method, read_all, run,
    st, strukt,
};

fn events_contract(extra: Vec<fs_mcp::Field>) -> Vec<Contract> {
    let mut state = vec![field("id", TypeRef::Text)];
    state.extend(extra);
    let mut ev = method("event", vec![field("event", st("Event"))], vec![]);
    ev.streaming = true;
    contract(
        vec![
            interface(
                "Root",
                vec![method(
                    "subscribe",
                    vec![field("listener", iface("Listener"))],
                    vec![],
                )],
            ),
            interface("Listener", vec![ev]),
        ],
        vec![
            strukt("Event", vec![member("state", st("StateChange"))]),
            strukt("StateChange", state),
        ],
    )
}

#[test]
fn an_event_type_with_a_secret_keeps_the_events_resource_out() {
    let marks = read_all(&[("Root", "subscribe")]);
    let ok = run(&events_contract(vec![]), &marks);
    assert_eq!(ok.resources.len(), 1, "the clean feed is offered");
    let mut bad = field("sessionToken", TypeRef::Text);
    bad.doc = "the account password".into();
    let c = run(&events_contract(vec![bad]), &marks);
    assert!(
        c.resources.is_empty(),
        "a secret inside Event reached a resource"
    );
    assert!(c.left_out.iter().any(|l| matches!(&l.why, Why::SecretSuspect { path } if path.ends_with("StateChange.sessionToken"))));
}

#[test]
fn an_event_type_nobody_defined_keeps_the_resource_out() {
    let mut c = events_contract(vec![]);
    c[0].structs.retain(|s| s.name != "StateChange");
    assert!(
        run(&c, &read_all(&[("Root", "subscribe")]))
            .resources
            .is_empty()
    );
}

#[test]
fn unknown_structures_and_any_pointers_are_never_a_tool() {
    let nowhere = TypeRef::Struct(fs_mcp::TypeName {
        file: "nowhere".into(),
        name: "Secret".into(),
    });
    let c = contract(
        vec![interface(
            "Root",
            vec![
                method("lost", vec![], vec![field("x", nowhere)]),
                method("any", vec![], vec![field("x", TypeRef::Any)]),
                method("fine", vec![], vec![field("x", TypeRef::Text)]),
            ],
        )],
        vec![],
    );
    let c = run(
        &c,
        &read_all(&[("Root", "lost"), ("Root", "any"), ("Root", "fine")]),
    );
    assert!(!has_tool(&c, "Root", "lost") && !has_tool(&c, "Root", "any"));
    assert!(has_tool(&c, "Root", "fine"));
}

#[test]
fn the_decision_types_are_blocked_wherever_they_appear() {
    let c = contract(
        vec![
            interface(
                "Root",
                vec![
                    method("answer", vec![field("d", st("ToolCallDecision"))], vec![]),
                    method("grant", vec![], vec![field("g", st("AiGrant"))]),
                    method("open", vec![], vec![field("g", iface("Guard"))]),
                    method("hidden", vec![field("w", st("Wrapper"))], vec![]),
                    method("fine", vec![], vec![field("x", TypeRef::Text)]),
                ],
            ),
            interface("Guard", vec![method("allow", vec![], vec![])]),
        ],
        vec![
            strukt("ToolCallDecision", vec![field("signer", TypeRef::Text)]),
            strukt("AiGrant", vec![field("who", TypeRef::Text)]),
            strukt("GuardProof", vec![field("who", TypeRef::Text)]),
            strukt("Wrapper", vec![field("inner", st("GuardProof"))]),
        ],
    );
    let all = read_all(&[
        ("Root", "answer"),
        ("Root", "grant"),
        ("Root", "open"),
        ("Root", "hidden"),
        ("Root", "fine"),
        ("Guard", "allow"),
    ]);
    for marks in [&all, &FakeMarks::default()] {
        let c = run(&c, marks);
        for m in ["answer", "grant", "open", "hidden"] {
            assert!(!has_tool(&c, "Root", m), "{m} became a tool");
        }
        assert!(!has_tool(&c, "Guard", "allow"));
        assert!(has_tool(&c, "Root", "fine"));
    }
}

#[test]
fn free_form_inputs_stay_out_unless_released_in_writing() {
    let build = |name: &str, ty: TypeRef| {
        contract(
            vec![interface(
                "Root",
                vec![method("put", vec![field(name, ty)], vec![])],
            )],
            vec![],
        )
    };
    let marks = read_all(&[("Root", "put")]);
    for (name, ty) in [
        ("bytes", TypeRef::Data),
        ("payload", TypeRef::Text),
        ("config", TypeRef::Text),
        ("json", TypeRef::Text),
        ("proof", TypeRef::Data),
    ] {
        let c = run(&build(name, ty), &marks);
        assert!(!has_tool(&c, "Root", "put"), "{name} slipped through");
    }
    let mut released = read_all(&[("Root", "put")]);
    released.releases.insert(
        Target::item(FILE, "Root", "put.bytes"),
        "free of secrets, test".into(),
    );
    assert!(has_tool(
        &run(&build("bytes", TypeRef::Data), &released),
        "Root",
        "put"
    ));
    // a struct field of type Data is caught too, and released per field
    let c = contract(
        vec![interface(
            "Root",
            vec![method("get", vec![], vec![field("p", st("Page"))])],
        )],
        vec![strukt("Page", vec![field("opaque", TypeRef::Data)])],
    );
    assert!(!has_tool(
        &run(&c, &read_all(&[("Root", "get")])),
        "Root",
        "get"
    ));
}

#[test]
fn a_secret_in_a_type_name_or_in_a_parameter_comment_keeps_the_tool_out() {
    let mut documented = field("x", TypeRef::Text);
    documented.doc = "the account password".into();
    let c = contract(
        vec![interface(
            "Root",
            vec![
                method("login", vec![field("l", st("LoginCredentials"))], vec![]),
                method("doc", vec![documented], vec![]),
            ],
        )],
        vec![strukt(
            "LoginCredentials",
            vec![field("who", TypeRef::Text)],
        )],
    );
    let c = run(&c, &read_all(&[("Root", "login"), ("Root", "doc")]));
    assert!(!has_tool(&c, "Root", "login") && !has_tool(&c, "Root", "doc"));
}
