//! Dropping the worker of the client half: what it cuts, what it waits for.
#![cfg(feature = "rpc")]

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fs_interfaces::rpc::client::{Timeouts, Worker};
use fs_interfaces::rpc::write_uuid;
use support::{HANG, OK, Tasks, TestServer, block_on, delete, quick};

#[test]
fn dropping_the_worker_closes_the_connection_and_joins() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    assert!(block_on(delete(&worker, OK)).unwrap().is_ok());
    assert!(!server.items_gone.load(Ordering::SeqCst));
    drop(worker);
    let until = Instant::now() + Duration::from_secs(5);
    while !server.items_gone.load(Ordering::SeqCst) {
        assert!(Instant::now() < until, "the server never saw the end");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn slow_timeouts() -> Timeouts {
    Timeouts {
        connect: Duration::from_secs(3),
        call: Duration::from_secs(5),
    }
}

/// Start a command and leave it running: poll once so it is queued, then let the future go.
fn start_and_leave<T: Send + 'static, Fut: Future<Output = T> + 'static>(
    worker: &Worker<Tasks>,
    work: impl FnOnce(Tasks) -> Fut + Send + 'static,
) {
    use futures::FutureExt;
    assert!(worker.call(work).now_or_never().is_none());
}

#[test]
fn dropping_the_worker_cuts_the_connection_under_a_running_command() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, slow_timeouts()).unwrap();
    let seen = Arc::new(Mutex::new(None));
    let slot = seen.clone();
    start_and_leave(&worker, move |r: Tasks| async move {
        let mut request = r.items.delete_request();
        write_uuid(request.get().init_id(), Some(uuid::Uuid::from_u128(HANG)));
        let answer = request.send().promise.await.map(|_| ());
        *slot.lock().unwrap() = Some(answer);
    });
    std::thread::sleep(Duration::from_millis(200));
    let began = Instant::now();
    drop(worker);
    // Joined: the command has ended, and not by its deadline of 5 s.
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "{:?}",
        began.elapsed()
    );
    let answer = seen.lock().unwrap().take().expect("the command ended");
    let error = answer.expect_err("a write under way is cut, not finished");
    assert_eq!(error.kind, capnp::ErrorKind::Disconnected, "{error}");
}

#[test]
fn dropping_waits_for_a_command_that_does_not_wait_on_the_connection() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    start_and_leave(&worker, |_: Tasks| async {
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    let began = Instant::now();
    drop(worker);
    // Nothing cuts a timer: the drop waits for the call deadline (400 ms here), no longer.
    let took = began.elapsed();
    assert!(took >= Duration::from_millis(300), "{took:?}");
    assert!(took < Duration::from_secs(3), "{took:?}");
}

#[test]
fn the_last_handle_dropped_by_a_command_does_not_join_its_own_thread() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, slow_timeouts()).unwrap();
    let worker = Arc::new(worker);
    let mine = worker.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let flag = finished.clone();
    start_and_leave(&worker, move |_: Tasks| async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(mine); // by now `worker` below is gone: this is the last handle
        flag.store(true, Ordering::SeqCst);
    });
    drop(worker);
    let until = Instant::now() + Duration::from_secs(3);
    while !finished.load(Ordering::SeqCst) {
        assert!(Instant::now() < until, "the command died in the drop");
        std::thread::sleep(Duration::from_millis(20));
    }
}
