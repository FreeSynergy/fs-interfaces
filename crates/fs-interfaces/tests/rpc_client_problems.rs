//! The readers of a `Problem` that every client needs.
#![cfg(feature = "rpc")]

use fs_interfaces::Unsupported;
use fs_interfaces::rpc::Problem;
use fs_interfaces::rpc::client::problems::{
    Failure, PROBLEM_BASE, first_number, local_type, object_id_in, unsupported_of,
    without_store_prefix,
};
use fs_store::ObjectId;

fn problem(kind: &str, detail: &str, status: u16) -> Problem {
    Problem::new(kind, "T", detail, status)
}

#[test]
fn a_problem_in_an_error_comes_back_typed() {
    let p = problem(&format!("{PROBLEM_BASE}people/conflict"), "name taken", 409);
    assert_eq!(Failure::of(&p.clone().into_error()), Failure::Problem(p));
}

#[test]
fn a_not_possible_is_a_problem_and_a_bare_unimplemented_is_not() {
    let u = Unsupported::new("board.open", "not yet");
    let error = fs_interfaces::rpc::unsupported(&u);
    let Failure::Problem(p) = Failure::of(&error) else {
        panic!("status 501 must come back as a problem");
    };
    assert!(p.is_unsupported());
    assert_eq!(
        Failure::of(&capnp::Error::unimplemented("x".into())),
        Failure::Unimplemented
    );
}

#[test]
fn any_other_error_keeps_its_text() {
    let Failure::Other(text) = Failure::of(&capnp::Error::failed("socket closed".into())) else {
        panic!("not other");
    };
    assert!(text.contains("socket closed"), "{text}");
}

#[test]
fn foreign_text_that_only_contains_the_marker_is_no_problem() {
    let error = capnp::Error::failed("see fs-problem\u{1f}404 for details".into());
    assert!(matches!(Failure::of(&error), Failure::Other(_)));
}

#[test]
fn the_local_type_loses_the_base_and_a_foreign_type_stays_whole() {
    let ours = problem(&format!("{PROBLEM_BASE}people/not-found"), "", 404);
    assert_eq!(local_type(&ours), "people/not-found");
    let theirs = problem("https://example.org/x", "", 400);
    assert_eq!(local_type(&theirs), "https://example.org/x");
}

#[test]
fn the_first_number_is_read_or_is_zero() {
    assert_eq!(
        first_number("message of 70000000 bytes, limit 65536"),
        70_000_000
    );
    assert_eq!(first_number("no digits"), 0);
    assert_eq!(first_number(""), 0);
    assert_eq!(first_number("99999999999999999999999999"), 0);
}

#[test]
fn an_object_id_is_found_among_words() {
    let id = ObjectId::new();
    let text = format!("object store: object {id} is marked sensitive");
    assert_eq!(object_id_in(&text), Some(id));
    assert_eq!(object_id_in("object store: nothing here"), None);
}

#[test]
fn the_store_prefix_is_removed_once() {
    let p = "object store: invalid input: ";
    assert_eq!(without_store_prefix(&format!("{p}bad name")), "bad name");
    assert_eq!(without_store_prefix("bad name"), "bad name");
    assert_eq!(
        without_store_prefix(&format!("{p}{p}bad")),
        format!("{p}bad")
    );
}

#[test]
fn a_not_possible_detail_splits_into_capability_and_reason() {
    let u = unsupported_of("board.open: not yet", "tasks");
    assert_eq!((u.capability(), u.reason()), ("board.open", "not yet"));
    let u = unsupported_of("no colon here", "tasks");
    assert_eq!((u.capability(), u.reason()), ("tasks", "no colon here"));
    let u = unsupported_of("a: b: c", "tasks");
    assert_eq!((u.capability(), u.reason()), ("a", "b: c"));
}
