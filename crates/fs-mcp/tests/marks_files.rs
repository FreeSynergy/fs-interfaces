//! The mark file format: strict, so a typo can never read as a harmless class. Pattern: none (tests).

use fs_mcp::{MarkFiles, RiskClass, RiskSource, Target};

fn parse(text: &str) -> Result<MarkFiles, fs_mcp::Error> {
    MarkFiles::from_sources(&[("messenger", text)])
}

#[test]
fn short_and_long_forms_are_read() {
    let m = parse("[Draft]\nsend = { risk = \"acts-in-name\", capability = \"message.send\" }\nsetText = \"change\"\n").unwrap();
    let send = m.mark(&Target::item("messenger", "Draft", "send")).unwrap();
    assert_eq!(
        (send.class, send.capability.as_deref()),
        (RiskClass::ActsInName, Some("message.send"))
    );
    assert_eq!(
        m.mark(&Target::item("messenger", "Draft", "setText"))
            .unwrap()
            .class,
        RiskClass::Change
    );
    assert!(
        m.mark(&Target::item("messenger", "Draft", "save"))
            .is_none()
    );
}

#[test]
fn anything_not_understood_is_an_error_and_never_a_default() {
    assert!(
        parse("[Draft]\nsend = \"harmless\"\n").is_err(),
        "unknown class"
    );
    assert!(
        parse("[Draft]\nsend = { capability = \"x.y\" }\n").is_err(),
        "no class"
    );
    assert!(
        parse("[Draft]\nsend = { risk = \"read\", colour = \"red\" }\n").is_err(),
        "unknown key"
    );
    assert!(parse("[Draft]\nsend = 3\n").is_err(), "wrong type");
    assert!(
        parse("[release]\n\"Reaction.key\" = \"\"\n").is_err(),
        "a release needs its reason"
    );
    assert!(
        parse("[release]\nkey = \"x\"\n").is_err(),
        "a release names Owner.name"
    );
    assert!(parse("send = \"read\"\n").is_err(), "a table is expected");
    assert!(parse("not toml at all [").is_err());
}

#[test]
fn there_is_no_harmless_class() {
    for name in ["harmless", "safe", "readonly", "read-only", "none"] {
        assert!(RiskClass::parse(name).is_none(), "{name}");
    }
    assert!(
        RiskClass::Unknown.asks_always(),
        "no mark asks every time, like destructive"
    );
}

#[test]
fn releases_carry_their_reason() {
    let m = parse("[release]\n\"Reaction.key\" = \"an emoji\"\n\"Blobs.put.key\" = \"a path\"\n")
        .unwrap();
    assert_eq!(
        m.release(&Target::item("messenger", "Reaction", "key"))
            .as_deref(),
        Some("an emoji")
    );
    assert!(
        m.release(&Target::item("messenger", "Blobs", "put.key"))
            .is_some()
    );
    assert_eq!(m.targets().len(), 2);
}
