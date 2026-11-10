//! A SIGTERM during the start ends the program cleanly (feature `rpc`). Alone in its file: the signal goes to this whole
//! process. Before the fix the handler was registered after `open`, and the signal killed the process hard.
#![cfg(feature = "rpc")]

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::run_until_signal;
use rustix::process::{Signal, getpid, kill_process};

struct NoBlob;
impl blob::Server for NoBlob {}

/// TERM and INT in turn inside one test: the signal goes to the whole process, so two tests would hear each other's.
#[test]
fn a_stop_signal_while_the_program_opens_its_store_stops_it_cleanly() {
    for (name, signal) in [("x.sock", Signal::TERM), ("y.sock", Signal::INT)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        run_until_signal(&path, || async move {
            // The store is being opened (slow under load) when the stop comes.
            kill_process(getpid(), signal).unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let root: blob::Client = new_client(NoBlob);
            Ok(root.client)
        })
        .unwrap();
        assert!(!path.exists(), "the socket stayed behind after {signal:?}");
    }
}
