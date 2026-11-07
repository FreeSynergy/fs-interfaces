//! Fourth review round: every interface a method takes is no root; free-format fields need a written format mark.

mod support;

use fs_mcp::{Blocklist, Generator, MarkFiles, Target, ToolGenerator, TypeRef};
use support::{FILE, contract, field, has_tool, iface, interface, method, read_all, run};

fn text() -> TypeRef {
    TypeRef::Text
}

#[test]
fn an_interface_taken_as_a_parameter_is_no_root() {
    let c = contract(
        vec![
            interface(
                "Svc",
                vec![method("a7", vec![field("w", iface("Watcher"))], vec![])],
            ),
            interface(
                "Watcher",
                vec![method("changed", vec![field("note", text())], vec![])],
            ),
        ],
        vec![],
    );
    let out = run(&c, &read_all(&[("Svc", "a7"), ("Watcher", "changed")]));
    let t = out
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "Watcher", "changed"))
        .unwrap();
    assert_eq!(t.handle_of.as_deref(), Some("Watcher"));
    let a7 = out
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "Svc", "a7"))
        .unwrap();
    assert!(a7.handle_of.is_none(), "Svc is still a root");
}

fn notes() -> Vec<fs_mcp::Contract> {
    contract(
        vec![interface(
            "Root",
            vec![method("save", vec![field("notes", text())], vec![])],
        )],
        vec![],
    )
}

#[test]
fn a_free_format_field_needs_a_written_format_mark() {
    let plain = MarkFiles::from_sources(&[(FILE, "[Root]\nsave = \"change\"\n")]).unwrap();
    let out = Generator::new(&plain, Blocklist::standard())
        .generate(&notes())
        .unwrap();
    assert!(
        !has_tool(&out, "Root", "save"),
        "a new free field slipped through without a mark"
    );
    let marked = MarkFiles::from_sources(&[(
        FILE,
        "[Root]\nsave = \"change\"\n[format]\n\"Root.save.notes\" = \"plain text\"\n",
    )])
    .unwrap();
    let out = Generator::new(&marked, Blocklist::standard())
        .generate(&notes())
        .unwrap();
    let t = out
        .tools
        .iter()
        .find(|t| t.source == Target::item(FILE, "Root", "save"))
        .expect("offered with the mark");
    assert_eq!(
        t.free_content,
        vec![("Root.save.notes".to_owned(), "plain text".to_owned())]
    );
    let json = t.to_json();
    assert_eq!(
        json["_meta"]["net.freesynergy/mcp"]["freeContent"][0]["format"],
        "plain text"
    );
}

#[test]
fn a_format_mark_does_not_lift_a_secret_name() {
    let c = contract(
        vec![interface(
            "Root",
            vec![method("save", vec![field("passwordNotes", text())], vec![])],
        )],
        vec![],
    );
    let marks = MarkFiles::from_sources(&[(
        FILE,
        "[Root]\nsave = \"change\"\n[format]\n\"Root.save.passwordNotes\" = \"plain text\"\n",
    )])
    .unwrap();
    let out = Generator::new(&marks, Blocklist::standard())
        .generate(&c)
        .unwrap();
    assert!(!has_tool(&out, "Root", "save"));
}
