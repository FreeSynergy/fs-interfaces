//! A write under way survives SIGTERM (feature `rpc`). Alone in its file: the signal goes to this whole process.
#![cfg(all(unix, feature = "rpc"))] // signals sent with `kill`

use std::cell::Cell;
use std::io::ErrorKind;
use std::rc::Rc;
use std::time::Duration;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{bind_socket, run_to_end, run_until_signal};
use rustix::process::{Signal, getpid, kill_process};

struct NoBlob;
impl blob::Server for NoBlob {}

#[test]
fn sigterm_waits_for_the_write_under_way_and_holds_the_socket_meanwhile() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.sock");
    let (steps, second_start, late) = (
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(None)),
        Rc::new(Cell::new(false)),
    );
    let (seen, second, refused, again) = (
        steps.clone(),
        second_start.clone(),
        late.clone(),
        path.clone(),
    );

    run_until_signal(&path, || async move {
        tokio::task::spawn_local(async move {
            let work = run_to_end(async move {
                for _ in 0..5 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    seen.set(seen.get() + 1);
                }
                second.set(bind_socket(&again).await.err().map(|e| e.kind()));
            });
            // The client goes away and the work goes on; then the program is told to stop.
            let _ = tokio::time::timeout(Duration::from_millis(50), work).await;
            kill_process(getpid(), Signal::TERM).unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            refused.set(run_to_end(async {}).await.is_err());
        });
        let root: blob::Client = new_client(NoBlob);
        Ok(root.client)
    })
    .unwrap();

    assert_eq!(steps.get(), 5, "the signal cut the write");
    assert_eq!(
        second_start.get(),
        Some(ErrorKind::AddrInUse),
        "a successor could start"
    );
    assert!(
        late.get(),
        "work that begins after the signal could be cut in half"
    );
    assert!(!path.exists(), "the socket stayed behind");
}
