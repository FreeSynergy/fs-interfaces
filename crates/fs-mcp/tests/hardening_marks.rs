//! More fail-closed checks: union members, releases per field, word splitting, strict resource marks.
//! Tests the generator against its traits with synthetic contracts.

mod support;

use fs_mcp::{
    Blocklist, Generator, MarkFiles, RiskClass, Target, ToolGenerator, TypeRef, name_looks_secret,
};
use support::{
    FILE, FakeMarks, contract, field, has_tool, interface, method, read_all, run, st, strukt,
};

#[test]
fn a_union_member_without_a_mark_is_unknown_and_an_excluded_method_holds_for_all() {
    let c = support::messenger_like();
    let mut marks = FakeMarks::everything_read();
    marks
        .marks
        .remove(&Target::item(FILE, "MessageAction", "flag"));
    let out = run(&c, &marks);
    let flag = out
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "MessageAction", "flag"))
        .unwrap();
    assert_eq!(flag.risk, RiskClass::Unknown);
    marks.put("Conversation", "act", RiskClass::Read);
    let out = run(&c, &marks);
    assert_eq!(
        out.tools
            .iter()
            .find(|t| t.source == Target::item(FILE, "MessageAction", "flag"))
            .unwrap()
            .risk,
        RiskClass::Unknown,
        "a mark on the method gives its members nothing"
    );
    marks.put("Conversation", "act", RiskClass::Excluded);
    let out = run(&c, &marks);
    assert!(
        out.tools
            .iter()
            .all(|t| !t.name.contains("conversation.act"))
    );
}

#[test]
fn a_release_covers_one_field_not_the_name_everywhere() {
    let c = contract(
        vec![interface(
            "Root",
            vec![
                method("a", vec![], vec![field("r", st("Reaction"))]),
                method("b", vec![], vec![field("o", st("Other"))]),
            ],
        )],
        vec![
            strukt("Reaction", vec![field("key", TypeRef::Text)]),
            strukt("Other", vec![field("key", TypeRef::Text)]),
        ],
    );
    let mut marks = read_all(&[("Root", "a"), ("Root", "b")]);
    marks
        .releases
        .insert(Target::item(FILE, "Reaction", "key"), "an emoji".into());
    let c = run(&c, &marks);
    assert!(has_tool(&c, "Root", "a") && !has_tool(&c, "Root", "b"));
}

#[test]
fn names_are_split_into_words_including_acronyms() {
    for n in [
        "IMAPPassword",
        "SMTPPassword",
        "APIToken",
        "JWTSecret",
        "totpCode",
        "otp",
        "oauthGrant",
        "passcode",
        "pin",
        "mnemonic",
        "bearer",
        "authorization",
        "cookies",
        "sessionId",
        "privateKey",
        "signature",
        "proof",
        "publisherKey",
        "LoginCredentials",
        "seedPhrase",
        "accessToken",
    ] {
        assert!(name_looks_secret(n), "{n} should look secret");
    }
    for n in [
        "pinned",
        "mapping",
        "tokenizer",
        "keyboard",
        "message",
        "unread",
        "assignee",
    ] {
        assert!(!name_looks_secret(n), "{n} should not");
    }
}

#[test]
fn resource_marks_are_strict() {
    let parse = |t: &str| MarkFiles::from_sources(&[(FILE, t)]);
    assert!(
        parse("[Draft]\nsend = { risk = \"acts-in-name\", resource = true }\n").is_err(),
        "only read may be a resource"
    );
    assert!(parse("[Draft]\nsend = { risk = \"read\", resource = \"yes\" }\n").is_err());
    assert!(parse("[Draft]\nsend = { risk = \"read\", capability = 7 }\n").is_err());
    assert!(parse("[Draft]\nsend = { risk = \"read\", reason = 7 }\n").is_err());
    assert!(parse("[Draft]\nsend = { risk = \"read\", capability = \"has space\" }\n").is_err());
    assert!(parse("[Draft]\nsend = { risk = \"read\", resource = true }\n").is_ok());
}

#[test]
fn a_resource_mark_on_a_method_without_bytes_is_an_error() {
    let c = contract(
        vec![interface(
            "Root",
            vec![method("size", vec![], vec![field("n", TypeRef::Text)])],
        )],
        vec![],
    );
    let marks = MarkFiles::from_sources(&[(
        FILE,
        "[Root]\nsize = { risk = \"read\", resource = true }\n",
    )])
    .unwrap();
    assert!(
        Generator::new(&marks, Blocklist::standard())
            .generate(&c)
            .is_err()
    );
}
