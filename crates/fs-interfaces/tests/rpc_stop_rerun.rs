//! A request to stop belongs to one run: the next run in the same process is not ended by it (feature `rpc`). Alone in its file:
//! the request is remembered by the whole process.
#![cfg(feature = "rpc")]

use std::time::{Duration, Instant};

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{run_until_signal, stop_self};

struct NoBlob;
impl blob::Server for NoBlob {}

fn run(path: &std::path::Path) {
    run_until_signal(path, || async move {
        let root: blob::Client = new_client(NoBlob);
        Ok(root.client)
    })
    .unwrap();
}

#[test]
fn a_stop_request_ends_one_run_and_not_the_next() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.sock");
    stop_self();
    run(&path); // ended by the request made before it
    let later = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        stop_self();
    });
    let began = Instant::now();
    run(&path);
    assert!(
        began.elapsed() >= Duration::from_millis(300),
        "the second run ended at once, by the request of the first"
    );
    later.join().unwrap();
}
