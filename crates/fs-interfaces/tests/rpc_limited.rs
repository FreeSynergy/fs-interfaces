//! One deadline for every call on a bare connection (what `fs-files` has after `connect_matching`), and per call on the worker.
#![cfg(feature = "rpc")]

mod support;

use std::time::{Duration, Instant};

use fs_interfaces::rpc::client::{Fault, Worker, call_limited};
use fs_interfaces::rpc::connect;
use fs_interfaces::tasks_capnp::tasks;
use fs_interfaces::{rpc::write_uuid, tasks_capnp::items};
use support::{HANG, OK, SLOW, Tasks, TestServer, block_on, delete, quick};

async fn items_of(socket: &std::path::Path) -> (items::Client, impl Sized) {
    let connection = connect::<tasks::Client>(socket).await.unwrap();
    let items = connection
        .client
        .items_request()
        .send()
        .promise
        .await
        .unwrap();
    (items.get().unwrap().get_items().unwrap(), connection)
}

fn request(
    items: &items::Client,
    id: u128,
) -> capnp::capability::Request<items::delete_params::Owned, items::delete_results::Owned> {
    let mut r = items.delete_request();
    write_uuid(r.get().init_id(), Some(uuid::Uuid::from_u128(id)));
    r
}

fn local<F: std::future::Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&rt, f)
}

#[test]
fn a_bare_call_to_a_hanging_server_waits_for_ever_without_the_limit() {
    let server = TestServer::start();
    local(async {
        let (items, _keep) = items_of(&server.path).await;
        let bare = tokio::time::timeout(
            Duration::from_millis(700),
            request(&items, HANG).send().promise,
        )
        .await;
        assert!(bare.is_err(), "no answer, and no end either");
    });
}

#[test]
fn a_bare_call_ends_with_timed_out_within_the_limit_and_the_connection_stays_usable() {
    let server = TestServer::start();
    local(async {
        let (items, _keep) = items_of(&server.path).await;
        let began = Instant::now();
        let limit = Duration::from_millis(300);
        let fault = call_limited("tasks", limit, request(&items, HANG).send().promise)
            .await
            .err()
            .unwrap();
        assert_eq!(
            fault,
            Fault::TimedOut {
                server: "tasks",
                limit
            }
        );
        assert!(
            began.elapsed() < Duration::from_millis(900),
            "{:?}",
            began.elapsed()
        );
        let next = call_limited(
            "tasks",
            Duration::from_secs(5),
            request(&items, OK).send().promise,
        )
        .await;
        assert!(
            next.unwrap().is_ok(),
            "the connection is not broken by the timeout"
        );
    });
}

#[test]
fn a_slow_but_answering_server_is_not_cut() {
    let server = TestServer::start();
    local(async {
        let (items, _keep) = items_of(&server.path).await;
        let slow = call_limited(
            "tasks",
            Duration::from_secs(5),
            request(&items, SLOW).send().promise,
        )
        .await;
        assert!(slow.unwrap().is_ok());
    });
}

#[test]
fn the_worker_takes_a_deadline_per_call() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    // The default (400 ms) cuts a call of 300 ms + overhead only if it is slower; a longer limit lets the same call finish.
    let long = block_on(
        worker.call_within(Duration::from_secs(5), |r: Tasks| async move {
            let mut q = r.items.delete_request();
            write_uuid(q.get().init_id(), Some(uuid::Uuid::from_u128(SLOW)));
            q.send().promise.await.map(|_| ())
        }),
    );
    assert!(long.unwrap().is_ok());
    let began = Instant::now();
    let short = block_on(
        worker.call_within(Duration::from_millis(100), |r: Tasks| async move {
            let mut q = r.items.delete_request();
            write_uuid(q.get().init_id(), Some(uuid::Uuid::from_u128(HANG)));
            q.send().promise.await.map(|_| ())
        }),
    );
    assert_eq!(
        short.unwrap_err(),
        Fault::TimedOut {
            server: "tasks",
            limit: Duration::from_millis(100)
        }
    );
    assert!(began.elapsed() < Duration::from_millis(700));
    assert!(block_on(delete(&worker, OK)).unwrap().is_ok());
}
