//! `--facet NAME`: one parser for the server and for the apps (feature `rpc`).
#![cfg(feature = "rpc")]

use std::ffi::OsString;

use fs_interfaces::rpc::{ServerOptions, facet_from_args, parse_facet};

fn args(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

#[test]
fn the_server_reads_the_facet() {
    let o = ServerOptions::parse(
        "people",
        args(&["--socket", "s", "--data", "d", "--facet", "identity"]),
    )
    .unwrap()
    .unwrap();
    assert_eq!(o.facet.as_deref(), Some("identity"));
    let o = ServerOptions::parse("people", args(&["--socket", "s", "--data", "d"]))
        .unwrap()
        .unwrap();
    assert_eq!(o.facet, None);
}

#[test]
fn the_apps_read_the_same_value_between_their_own_arguments() {
    let found = facet_from_args(args(&[
        "--window", "main", "--facet", "people", "--zoom", "2",
    ]))
    .unwrap();
    assert_eq!(found.as_deref(), Some("people"));
    assert_eq!(facet_from_args(args(&["--window"])).unwrap(), None);
}

#[test]
fn a_value_that_is_no_facet_name_is_refused_by_both() {
    for bad in ["", "People", "a_b", "-a", "a-", "a b", &"x".repeat(64)] {
        assert!(parse_facet(bad).is_err(), "{bad:?}");
    }
    assert!(facet_from_args(args(&["--facet", "Bad"])).is_err());
    assert!(facet_from_args(args(&["--facet"])).is_err());
    assert!(
        ServerOptions::parse(
            "p",
            args(&["--socket", "s", "--data", "d", "--facet", "Bad"])
        )
        .is_err()
    );
}
