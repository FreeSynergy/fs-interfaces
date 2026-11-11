//! The program name in `Expected`: a server of another program is never stopped or replaced (a wrong `--socket`).
#![cfg(all(feature = "rpc", unix))]

use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{Expected, HandshakeError, connect_matching, stop_server};

mod handshake_support;
use handshake_support::*;

#[test]
fn a_server_of_another_program_is_left_alone() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("p.sock");
    start_first(&socket, "0.0.3"); // identity: program "test"
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let never = || -> std::io::Result<()> { panic!("must not start anything") };
        let expected = Expected::new("0.0.9").with_program("fs-people"); // other version AND other program
        let e = connect_matching::<blob::Client>(&socket, &expected, &never)
            .await
            .err()
            .unwrap();
        match &e {
            HandshakeError::CannotRestart {
                running, reason, ..
            } => {
                assert_eq!(running.as_ref().unwrap().program, "test");
                assert!(
                    reason.contains("test") && reason.contains("fs-people"),
                    "{reason}"
                );
            }
            other => panic!("{other}"),
        }
        // Still running, untouched.
        let c = connect_matching::<blob::Client>(
            &socket,
            &Expected::new("0.0.3").with_program("test"),
            &never,
        )
        .await
        .unwrap();
        assert_eq!(seven(&c.client).await, 7);
        // No name expected: as before.
        let c2 = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &never)
            .await
            .unwrap();
        assert_eq!(seven(&c2.client).await, 7);
        drop((c, c2));
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}

#[test]
fn accepts_refuses_another_program() {
    use fs_interfaces::rpc::{PROTOCOL, ServerInfo};
    let info = |program: &str| ServerInfo {
        program: program.to_owned(),
        version: "0.0.3".to_owned(),
        protocol: PROTOCOL,
    };
    let expected = Expected::new("0.0.3").with_program("fs-people");
    assert!(expected.accepts(&info("fs-people")));
    assert!(expected.accepts(&info(""))); // a server that does not say its name
    assert!(!expected.accepts(&info("fs-files")));
    assert!(Expected::new("0.0.3").accepts(&info("fs-files"))); // no name expected
    assert_eq!(expected.program(), "fs-people");
}
