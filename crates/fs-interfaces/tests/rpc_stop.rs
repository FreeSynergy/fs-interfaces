//! A program ends itself in order with `stop_self`, on every system (feature `rpc`). Alone in its file: the request is
//! remembered by the whole process.
#![cfg(feature = "rpc")]

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{companion_path, run_until_signal, stop_self};

struct NoBlob;
impl blob::Server for NoBlob {}

#[test]
fn a_request_to_stop_ends_the_serving_cleanly_even_when_it_came_before() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.sock");
    // Before the server runs: remembered.
    stop_self();
    run_until_signal(&path, || async move {
        let root: blob::Client = new_client(NoBlob);
        Ok(root.client)
    })
    .unwrap();
    assert!(!path.exists());
}

#[test]
fn a_file_of_the_socket_has_the_extension_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let lease = companion_path(&dir.path().join("x.sock"), "lease");
    assert_eq!(lease.extension().and_then(|e| e.to_str()), Some("lease"));
    assert!(lease.parent().is_some_and(std::path::Path::exists));
}
