//! Exclusions of the generator against the traits: block list, secrets, releases, excluded marks.
//! Pattern: none (tests).

mod support;

use fs_mcp::{Blocklist, Generator, RiskClass, Target, ToolGenerator, Why};
use support::{FILE, FakeMarks, messenger_like};

fn names(c: &fs_mcp::Catalog) -> Vec<String> {
    c.tools.iter().map(|t| t.name.clone()).collect()
}
fn left(c: &fs_mcp::Catalog, owner: &str, item: Option<&str>) -> Option<Why> {
    let t = item.map_or_else(
        || Target::owner(FILE, owner),
        |i| Target::item(FILE, owner, i),
    );
    c.left_out
        .iter()
        .find(|l| l.target == t)
        .map(|l| l.why.clone())
}

#[test]
fn blocked_targets_stay_out_even_when_marked_readable() {
    let marks = FakeMarks::everything_read();
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    for t in &c.tools {
        assert!(
            !t.name.contains("decide") && !t.name.contains("guard") && !t.name.ends_with(".none"),
            "{}",
            t.name
        );
        assert!(!t.name.contains("conversation.act.none"));
    }
    assert!(matches!(
        left(&c, "MessageAction", Some("decide")),
        Some(Why::Blocked(_))
    ));
    assert!(matches!(
        left(&c, "MessageAction", Some("none")),
        Some(Why::Blocked(_))
    ));
    assert!(matches!(left(&c, "Guard", None), Some(Why::Blocked(_))));
    assert!(matches!(
        left(&c, "Messenger", Some("guard")),
        Some(Why::Blocked(_))
    ));
}

#[test]
fn a_forgotten_block_entry_does_not_release_a_secret() {
    // The block list is empty and nobody marked anything: the secret field still keeps the tool out.
    let marks = FakeMarks::everything_read();
    let c = Generator::new(&marks, Blocklist::empty())
        .generate(&messenger_like())
        .unwrap();
    assert!(!names(&c).iter().any(|n| n.contains("accounts.add")));
    assert!(matches!(
        left(&c, "Accounts", Some("add")),
        Some(Why::SecretSuspect { .. })
    ));
    // `proof`/`signer` of the decision do not look secret, but with an empty list `decide` is then a tool: only the
    // block list holds it back. That is why the list exists, and why the real-schema test pins its entries.
    assert!(names(&c).iter().any(|n| n.contains("act.decide")));
}

#[test]
fn a_release_lets_a_harmless_name_through_and_only_that_one() {
    let mut marks = FakeMarks::everything_read();
    marks
        .releases
        .insert(Target::item(FILE, "AccountSetup", "secret"), "test".into());
    let c = Generator::new(&marks, Blocklist::empty())
        .generate(&messenger_like())
        .unwrap();
    assert!(names(&c).iter().any(|n| n.contains("accounts.add")));
}

#[test]
fn an_excluded_mark_keeps_the_tool_out() {
    let mut marks = FakeMarks::everything_read();
    marks.put("Draft", "send", RiskClass::Excluded);
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    assert!(!names(&c).iter().any(|n| n.contains("draft.send")));
    assert!(matches!(
        left(&c, "Draft", Some("send")),
        Some(Why::Marked(_))
    ));
}
