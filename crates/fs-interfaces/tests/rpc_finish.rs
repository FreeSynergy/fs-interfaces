//! Writes that finish whatever the client does (feature `rpc`).
#![cfg(feature = "rpc")]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use fs_interfaces::rpc::run_to_end;

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
                if true {
                    panic!("boom");
                }
            })
            .await;
            assert!(answer.is_err());
        })
        .await;
}
