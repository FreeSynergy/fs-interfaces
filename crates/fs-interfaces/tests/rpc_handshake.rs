//! The version handshake of the client (`connect_matching`): an old server is replaced, a matching one is left alone, a
//! failed replacement is a typed error. Unix: the old server is a child process (this test binary), ended by SIGTERM.
#![cfg(all(feature = "rpc", unix))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{
    Expected, HandshakeError, connect, connect_matching, connect_matching_with, stop_server,
};

mod handshake_support;
use handshake_support::*;

#[test]
fn helper_old_server() {
    old_server_main();
}

#[test]
fn stop_server_reports_an_old_server_as_unsupported() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("old.sock");
    let mut child = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let e = stop_server(&socket).await.unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::Unsupported, "{e}");
    });
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn an_old_server_without_control_is_replaced() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("a.sock");
    let mut old = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let called = AtomicBool::new(false);
        let starter = || {
            called.store(true, Ordering::SeqCst);
            start_new(&socket, "0.0.3");
            Ok(())
        };
        let c = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .unwrap();
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(seven(&c.client).await, 7);
        drop(c);
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
    assert!(old.wait().unwrap().success() || old.try_wait().unwrap().is_some());
}

#[test]
fn a_matching_server_is_not_interrupted() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("m.sock");
    start_first(&socket, "0.0.3");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        // A first client, held open across the second one.
        let began = Instant::now();
        let first = loop {
            match connect::<blob::Client>(&socket).await {
                Ok(c) => break c,
                Err(e) => {
                    assert!(began.elapsed() < Duration::from_secs(10), "no server: {e}");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            }
        };
        let never = || -> std::io::Result<()> { panic!("a matching server must not be restarted") };
        let c = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &never)
            .await
            .unwrap();
        assert_eq!(seven(&c.client).await, 7);
        assert_eq!(seven(&first.client).await, 7);
        drop((c, first));
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}

#[test]
fn a_server_of_another_version_is_stopped_by_request_and_replaced() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("v.sock");
    start_first(&socket, "0.0.2");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || {
            start_new(&socket, "0.0.3");
            Ok(())
        };
        let c = connect_matching_with::<blob::Client>(
            &socket,
            &Expected::new("0.0.3"),
            &starter,
            fast(),
        )
        .await
        .unwrap();
        assert_eq!(seven(&c.client).await, 7);
        drop(c);
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}

#[test]
fn a_failed_start_is_a_typed_error() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("f.sock");
    let mut old = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || Err(std::io::Error::other("no executable"));
        let e = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .err()
            .unwrap();
        match &e {
            HandshakeError::CannotRestart {
                running, reason, ..
            } => {
                assert_eq!(running.as_ref().unwrap().protocol, 0);
                assert!(reason.contains("no executable"), "{reason}");
            }
            other => panic!("{other}"),
        }
        assert_ne!(e.to_string(), "");
    });
    // The old server was ended by SIGTERM before the start failed.
    assert!(old.wait().is_ok());
}

#[test]
fn a_replacement_that_is_still_wrong_is_a_mismatch() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("w.sock");
    start_first(&socket, "0.0.2");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || {
            start_new(&socket, "0.0.2");
            Ok(())
        };
        let e = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .err()
            .unwrap();
        match e {
            HandshakeError::ServerMismatch { running, expected } => {
                assert_eq!(running.version, "0.0.2");
                assert_eq!(expected.version, "0.0.3");
            }
            other => panic!("{other}"),
        }
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}
