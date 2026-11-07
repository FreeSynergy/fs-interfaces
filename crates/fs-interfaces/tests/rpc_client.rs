//! The client half: the worker thread, its deadlines and its guard, against a real server.
#![cfg(feature = "rpc")]

mod support;

use std::io::ErrorKind;
use std::os::unix::net::UnixListener;
use std::time::{Duration, Instant};

use fs_interfaces::rpc::client::problems::Failure;
use fs_interfaces::rpc::client::{Fault, Worker};
use support::{HANG, OK, OLD, PROBLEM, SLOW, Tasks, TestServer, block_on, delete, quick};

#[test]
fn start_reports_what_the_server_supports() {
    let server = TestServer::start();
    let (_worker, caps) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    assert_eq!(caps.provider(), "test-tasks");
    assert!(caps.is_supported("task.create"));
    assert!(!caps.is_supported("board.open"));
}

#[test]
fn a_call_gets_the_servers_answer_and_its_problem_stays_typed() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    assert!(block_on(delete(&worker, OK)).unwrap().is_ok());
    let error = block_on(delete(&worker, PROBLEM)).unwrap().unwrap_err();
    let Failure::Problem(p) = Failure::of(&error) else {
        panic!("not a problem: {error}");
    };
    assert_eq!(p.status, 404);
    let error = block_on(delete(&worker, OLD)).unwrap().unwrap_err();
    assert_eq!(Failure::of(&error), Failure::Unimplemented);
}

#[test]
fn calls_overlap_on_the_one_thread() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    let began = Instant::now();
    block_on(async {
        let all = futures::future::join_all((0..3).map(|_| delete(&worker, SLOW))).await;
        assert!(all.into_iter().all(|r| r.unwrap().is_ok()));
    });
    assert!(
        began.elapsed() < Duration::from_millis(800),
        "{:?}",
        began.elapsed()
    );
}

#[test]
fn calls_from_other_threads_reach_the_same_connection() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    std::thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| assert!(block_on(delete(&worker, OK)).unwrap().is_ok()));
        }
    });
}

#[test]
fn a_silent_server_ends_a_call_with_the_deadline_named() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    let fault = block_on(delete(&worker, HANG)).unwrap_err();
    assert_eq!(
        fault,
        Fault::TimedOut {
            server: "tasks",
            limit: Duration::from_millis(400)
        }
    );
    let text = fault.to_string();
    assert!(
        text.contains("tasks server did not answer within"),
        "{text}"
    );
    assert!(text.contains("400ms"), "{text}");
    // The connection is still good.
    assert!(block_on(delete(&worker, OK)).unwrap().is_ok());
}

#[test]
fn a_server_that_never_answers_the_handshake_ends_start_with_the_deadline_named() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mute.sock");
    let _listener = UnixListener::bind(&path).unwrap();
    let began = Instant::now();
    let error = Worker::<Tasks>::start("tasks", &path, quick())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert!(error.to_string().contains("300ms"), "{error}");
    assert!(began.elapsed() < Duration::from_secs(3));
}

#[test]
fn nothing_listening_is_an_error_not_a_hang() {
    let dir = tempfile::tempdir().unwrap();
    let error = Worker::<Tasks>::start("tasks", &dir.path().join("none.sock"), quick())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[test]
fn a_command_that_panics_reports_that_it_ended_without_an_answer() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    let fault = block_on(worker.call(|_: Tasks| async { panic!("boom") as u8 })).unwrap_err();
    assert_eq!(fault, Fault::Interrupted { server: "tasks" });
    assert!(fault.to_string().contains("without an answer"), "{fault}");
    // One bad command does not take the connection with it.
    assert!(block_on(delete(&worker, OK)).unwrap().is_ok());
}

#[test]
fn a_server_that_goes_away_is_an_answer_not_a_hang() {
    let server = TestServer::start();
    let (worker, _) = Worker::<Tasks>::start("tasks", &server.path, quick()).unwrap();
    drop(server);
    match block_on(delete(&worker, OK)) {
        Ok(Err(e)) => assert!(!matches!(Failure::of(&e), Failure::Unimplemented), "{e}"),
        Ok(Ok(())) => panic!("answered by a server that is gone"),
        Err(fault) => panic!("a gone server must answer with an RPC error, not {fault}"),
    }
}
