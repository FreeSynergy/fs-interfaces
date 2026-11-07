//! The schema reader: what comes out of the compiler's `CodeGeneratorRequest`. Pattern: none (tests).

mod support;

use fs_mcp::TypeRef;
use support::real::{contracts, real_catalog};

#[test]
fn all_schema_files_are_read_with_their_interfaces() {
    let c = contracts();
    assert!(c.len() >= 15);
    let messenger = c.iter().find(|c| c.file == "messenger").unwrap();
    for name in ["Messenger", "Conversation", "Draft", "Listener"] {
        assert!(
            messenger.interfaces.iter().any(|i| i.name == name),
            "{name}"
        );
    }
}

#[test]
fn streaming_methods_unions_and_capabilities_are_recognised() {
    let c = contracts();
    let m = c.iter().find(|c| c.file == "messenger").unwrap();
    let listener = m.interfaces.iter().find(|i| i.name == "Listener").unwrap();
    assert!(listener.methods[0].streaming && listener.methods[0].results.is_empty());
    let action = m
        .structs
        .iter()
        .find(|s| s.name == "MessageAction")
        .unwrap();
    assert!(action.is_pure_union());
    assert!(
        ["none", "delete", "flag", "decide"]
            .iter()
            .all(|n| action.fields.iter().any(|f| f.name == *n && f.in_union))
    );
    let messenger = m.interfaces.iter().find(|i| i.name == "Messenger").unwrap();
    let subscribe = messenger
        .methods
        .iter()
        .find(|x| x.name == "subscribe")
        .unwrap();
    assert!(
        subscribe
            .params
            .iter()
            .any(|p| matches!(p.ty, TypeRef::Interface(_)))
    );
}

#[test]
fn comments_trailing_a_member_are_read() {
    let c = contracts();
    let m = c.iter().find(|c| c.file == "messenger").unwrap();
    let action = m
        .structs
        .iter()
        .find(|s| s.name == "MessageAction")
        .unwrap();
    let react = action.fields.iter().find(|f| f.name == "react").unwrap();
    assert!(react.doc.contains("reaction key"), "{:?}", react.doc);
}

#[test]
fn a_missing_directory_is_an_error_not_an_empty_result() {
    use fs_mcp::ContractSource;
    assert!(
        fs_mcp::CapnpcSource::new("/nonexistent/fs-mcp")
            .contracts()
            .is_err()
    );
}

#[test]
fn comments_of_the_schema_reach_the_descriptions() {
    let c = real_catalog();
    let send = c.tools.iter().find(|t| t.name == "message.send").unwrap();
    assert!(
        send.description.contains("schedule send"),
        "{}",
        send.description
    );
}
