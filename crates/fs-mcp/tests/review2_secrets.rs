//! Secret gaps found by the second review: struct comments, short forms, run-together words. Synthetic contracts.

mod support;

use fs_mcp::{Target, TypeRef};
use support::{FILE, contract, field, has_tool, interface, method, read_all, run, st, strukt};

fn text() -> TypeRef {
    TypeRef::Text
}

#[test]
fn secret_gaps_found_by_review_are_closed() {
    let mut s = strukt("Box", vec![field("x", text())]);
    s.doc = "the account password".into();
    let mut pem = field("pem", text());
    pem.doc = "the private key of the account".into();
    let c = contract(
        vec![interface(
            "Root",
            vec![
                method("a", vec![], vec![field("b", st("Box"))]),
                method("b", vec![pem], vec![]),
                method("c", vec![field("accesstoken", text())], vec![]),
                method("d", vec![field("pwd", text())], vec![]),
                method("e", vec![field("authToken", text())], vec![]),
                method("f", vec![field("sshKey", text())], vec![]),
                method("g", vec![field("fine", text())], vec![]),
            ],
        )],
        vec![s],
    );
    let out = run(
        &c,
        &read_all(&[
            ("Root", "a"),
            ("Root", "b"),
            ("Root", "c"),
            ("Root", "d"),
            ("Root", "e"),
            ("Root", "f"),
            ("Root", "g"),
        ]),
    );
    for m in ["a", "b", "c", "d", "e", "f"] {
        assert!(!has_tool(&out, "Root", m), "{m} slipped through");
    }
    assert!(has_tool(&out, "Root", "g"));
}

#[test]
fn a_released_run_together_name_comes_through_only_for_that_field() {
    let c = contract(
        vec![interface(
            "Root",
            vec![method("c", vec![field("accesstoken", text())], vec![])],
        )],
        vec![],
    );
    let mut marks = read_all(&[("Root", "c")]);
    marks
        .releases
        .insert(Target::item(FILE, "Root", "c.accesstoken"), "test".into());
    assert!(has_tool(&run(&c, &marks), "Root", "c"));
}
