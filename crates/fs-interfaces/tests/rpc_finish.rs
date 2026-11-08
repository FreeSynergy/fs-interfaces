//! Writes that finish whatever the client does (feature `rpc`).
#![cfg(feature = "rpc")]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use fs_interfaces::rpc::{finish_underway, run_to_end};

#[tokio::test(flavor = "current_thread")]
async fn work_goes_on_when_the_caller_is_dropped() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let steps = Rc::new(Cell::new(0));
            let seen = steps.clone();
            let call = run_to_end(async move {
                for _ in 0..3 {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    seen.set(seen.get() + 1);
                }
            });
            let _ = tokio::time::timeout(Duration::from_millis(30), call).await;
            assert!(steps.get() < 3, "the test must drop the call midway");
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert_eq!(steps.get(), 3);
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_panic_answers_an_error() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let answer = run_to_end(async {
                panic!("boom");
            })
            .await;
            assert!(answer.is_err());
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_wait_for_work_under_way_has_an_end_and_says_so() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let stuck = run_to_end(std::future::pending::<()>());
            let _ = tokio::time::timeout(Duration::from_millis(10), stuck).await;
            let left = finish_underway(Duration::from_millis(20))
                .await
                .unwrap_err();
            assert_eq!(left.kind(), std::io::ErrorKind::TimedOut);
            assert!(left.to_string().contains("1 writes"), "{left}");
            assert!(
                run_to_end(async {}).await.is_ok(),
                "new work after the wait"
            );
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn nothing_under_way_means_no_wait_at_all() {
    tokio::task::LocalSet::new()
        .run_until(async {
            finish_underway(Duration::ZERO).await.unwrap();
        })
        .await;
}
