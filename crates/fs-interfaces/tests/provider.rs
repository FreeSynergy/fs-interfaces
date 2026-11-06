//! Wire round trips of the provider-rule types.

use fs_interfaces::{Capabilities, Unsupported, common_capnp};

#[test]
fn unsupported_round_trip() {
    let u = Unsupported::new("message.edit", "no edit in this network");
    let mut msg = capnp::message::Builder::new_default();
    u.write(msg.init_root::<common_capnp::unsupported::Builder>());
    let r = msg
        .get_root_as_reader::<common_capnp::unsupported::Reader>()
        .unwrap();
    assert_eq!(r.get_problem().unwrap().get_status(), 501);
    assert_eq!(Unsupported::read(r).unwrap(), u);
    assert_eq!(
        u.to_string(),
        "not possible: message.edit (no edit in this network)"
    );
}

#[test]
fn capabilities_round_trip() {
    let mut caps = Capabilities::new("matrix");
    caps.supported("message.send")
        .unsupported("call.start", "not offered");
    let mut msg = capnp::message::Builder::new_default();
    caps.write(
        "0.1",
        msg.init_root::<common_capnp::support_report::Builder>(),
    );
    let r = msg
        .get_root_as_reader::<common_capnp::support_report::Reader>()
        .unwrap();
    let back = Capabilities::read(r).unwrap();
    assert_eq!(back, caps);
    assert!(back.is_supported("message.send"));
    assert_eq!(back.why_not("call.start").unwrap().reason(), "not offered");
    assert!(back.why_not("message.send").is_none());
}

#[test]
fn problem_type_constant_matches_schema() {
    assert_eq!(
        common_capnp::UNSUPPORTED_PROBLEM_TYPE,
        fs_interfaces::UNSUPPORTED_PROBLEM_TYPE
    );
}
