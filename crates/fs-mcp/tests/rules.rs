//! The generator's rules, tested against the traits with a synthetic contract and in-memory marks.
//! Pattern: none (tests).

mod support;

use fs_mcp::{Blocklist, Generator, RiskClass, Target, ToolGenerator};
use support::{FILE, FakeMarks, messenger_like};

fn names(c: &fs_mcp::Catalog) -> Vec<String> {
    c.tools.iter().map(|t| t.name.clone()).collect()
}
#[test]
fn no_mark_means_unknown_and_is_listed() {
    let marks = FakeMarks::default();
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    let t = c
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "Draft", "send"))
        .unwrap();
    assert_eq!(t.risk, RiskClass::Unknown);
    assert!(t.unmarked && t.risk.asks_always());
    assert!(c.unmarked.contains(&t.source));
}

#[test]
fn read_only_hint_is_never_true() {
    for class in [
        RiskClass::Read,
        RiskClass::Add,
        RiskClass::Change,
        RiskClass::Destructive,
        RiskClass::ActsInName,
    ] {
        let mut marks = FakeMarks::default();
        for (o, i) in [
            ("Draft", "send"),
            ("Conversations", "open"),
            ("Messenger", "capabilities"),
        ] {
            marks.put(o, i, class);
        }
        let c = Generator::new(&marks, Blocklist::standard())
            .generate(&messenger_like())
            .unwrap();
        for t in c.to_json()["tools"].as_array().unwrap() {
            assert_eq!(t["annotations"]["readOnlyHint"], false, "{}", t["name"]);
            assert_eq!(t["annotations"]["destructiveHint"], true);
        }
    }
}

#[test]
fn union_members_become_one_tool_each() {
    let marks = FakeMarks::everything_read();
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    let n = names(&c);
    assert!(n.contains(&"messenger.conversation.act.delete".to_owned()));
    assert!(n.contains(&"messenger.conversation.act.flag".to_owned()));
    assert!(
        !n.iter().any(|x| x == "messenger.conversation.act"),
        "the union method itself is not a tool"
    );
}

#[test]
fn listeners_and_streams_are_no_tools_but_events_are_a_resource() {
    let marks = FakeMarks::everything_read();
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    assert!(
        !names(&c)
            .iter()
            .any(|n| n.contains("subscribe") || n.contains("listener"))
    );
    assert_eq!(c.resources.len(), 1);
    assert_eq!(
        c.resources[0].source,
        Target::item(FILE, "Messenger", "subscribe")
    );
    assert_eq!(c.resources[0].risk, RiskClass::Read);
}

#[test]
fn capability_id_names_the_tool_and_handles_are_flat_arguments() {
    let mut marks = FakeMarks::everything_read();
    marks.capability("Draft", "send", RiskClass::ActsInName, "message.send");
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    let send = c.tools.iter().find(|t| t.name == "message.send").unwrap();
    assert_eq!(send.risk, RiskClass::ActsInName);
    assert_eq!(send.handle_of.as_deref(), Some("Draft"));
    assert_eq!(send.input_schema["properties"]["handle"]["type"], "string");
    let root = c
        .tools
        .iter()
        .find(|t| t.name == "messenger.messenger.capabilities")
        .unwrap();
    assert!(root.handle_of.is_none() && root.input_schema["properties"].get("handle").is_none());
    let open = c
        .tools
        .iter()
        .find(|t| t.name == "messenger.conversations.open")
        .unwrap();
    assert_eq!(
        open.output_schema["properties"]["conversation"]["x-fs-handle"],
        "Conversation"
    );
}

#[test]
fn stale_marks_are_reported() {
    let mut marks = FakeMarks::everything_read();
    marks.put("Draft", "gone", RiskClass::Read);
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&messenger_like())
        .unwrap();
    assert_eq!(c.stale, vec![Target::item(FILE, "Draft", "gone")]);
}

#[test]
fn two_tools_with_one_name_are_an_error() {
    let mut marks = FakeMarks::everything_read();
    marks.capability("Draft", "send", RiskClass::Add, "same.name");
    marks.capability("Conversations", "open", RiskClass::Read, "same.name");
    assert!(
        Generator::new(&marks, Blocklist::standard())
            .generate(&messenger_like())
            .is_err()
    );
}
