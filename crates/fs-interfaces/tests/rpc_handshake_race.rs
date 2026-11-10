//! Two clients at once after an update (two windows): one new server, both connect.
#![cfg(all(feature = "rpc", unix))]

use std::sync::atomic::Ordering;

use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{Expected, connect_matching, stop_server};

mod handshake_support;
use handshake_support::*;

#[test]
fn helper_old_server() {
    old_server_main();
}

#[test]
fn two_clients_at_once_after_an_update_end_with_one_new_server() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("two.sock");
    let mut old = spawn_old_server(&socket);
    let starts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let clients: Vec<_> = (0..2)
        .map(|_| {
            let (socket, starts) = (socket.clone(), starts.clone());
            std::thread::spawn(move || {
                let rt = runtime();
                tokio::task::LocalSet::new().block_on(&rt, async {
                    let starter = || {
                        // A second start finds the socket taken and ends; the client then uses the running one.
                        starts.fetch_add(1, Ordering::SeqCst);
                        start_new(&socket, "0.0.3");
                        Ok(())
                    };
                    let c = connect_matching::<blob::Client>(
                        &socket,
                        &Expected::new("0.0.3"),
                        &starter,
                    )
                    .await
                    .unwrap();
                    assert_eq!(seven(&c.client).await, 7);
                });
            })
        })
        .collect();
    for c in clients {
        c.join().unwrap();
    }
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        // The new server still runs, and is the matching one.
        let never = || -> std::io::Result<()> { panic!("must not restart") };
        let c = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &never)
            .await
            .unwrap();
        drop(c);
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
    old.wait().unwrap();
}
