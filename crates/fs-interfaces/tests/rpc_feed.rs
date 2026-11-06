//! The shared change feed (feature `rpc`).
#![cfg(feature = "rpc")]

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::change_listener;
use fs_interfaces::rpc::{FEED_CAPACITY, Notice, change_channel, forward};
use fs_store::ObjectId;

/// Counts changes; can fail on one of them, or never answer at all.
#[derive(Clone, Default)]
struct Listener {
    seen: Rc<Cell<usize>>,
    fail_at: Option<usize>,
    stall: bool,
}

impl change_listener::Server for Listener {
    async fn change(self: Rc<Self>, _: change_listener::ChangeParams) -> Result<(), capnp::Error> {
        self.seen.set(self.seen.get() + 1);
        if self.stall {
            std::future::pending::<()>().await;
        }
        if self.fail_at == Some(self.seen.get()) {
            return Err(capnp::Error::failed("listener broke".into()));
        }
        Ok(())
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(50)).await;
}

async fn deleted(_: u32) -> Option<Notice> {
    Some(Notice::Deleted(ObjectId::new()))
}

async fn local<F: Future<Output = ()>>(test: F) {
    tokio::task::LocalSet::new().run_until(test).await;
}

#[tokio::test(flavor = "current_thread")]
async fn every_event_arrives_and_a_skipped_one_does_not() {
    local(async {
        let listener = Listener::default();
        let client: change_listener::Client = new_client(listener.clone());
        let (tx, rx) = change_channel();
        let _subscription = forward(rx, (), client, |n: u32| async move {
            (n != 1).then(|| Notice::Deleted(ObjectId::new()))
        });
        for n in 0..3 {
            tx.send(n);
        }
        settle().await;
        assert_eq!(listener.seen.get(), 2);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_the_capability_ends_it_and_frees_the_program_side() {
    local(async {
        let guard = Arc::new(());
        let client: change_listener::Client = new_client(Listener::default());
        let (tx, rx) = change_channel();
        let subscription = forward(rx, guard.clone(), client, deleted);
        tx.send(1);
        settle().await;
        assert_eq!(Arc::strong_count(&guard), 2);
        drop(subscription);
        settle().await;
        assert_eq!(Arc::strong_count(&guard), 1);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_failing_listener_frees_the_program_side_while_the_capability_lives() {
    local(async {
        let guard = Arc::new(());
        let listener = Listener {
            fail_at: Some(1),
            ..Listener::default()
        };
        let client: change_listener::Client = new_client(listener);
        let (tx, rx) = change_channel();
        let _subscription = forward(rx, guard.clone(), client, deleted);
        for n in 0..4 {
            tx.send(n);
            settle().await;
        }
        assert_eq!(Arc::strong_count(&guard), 1);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_consumer_that_stops_answering_cannot_grow_memory() {
    local(async {
        let guard = Arc::new(());
        let listener = Listener {
            stall: true,
            ..Listener::default()
        };
        let client: change_listener::Client = new_client(listener);
        let (tx, rx) = change_channel();
        let _subscription = forward(rx, guard.clone(), client, deleted);
        settle().await;
        for n in 0..=u32::try_from(FEED_CAPACITY * 100).unwrap() {
            tx.send(n);
            if n % 1024 == 0 {
                tokio::task::yield_now().await;
            }
        }
        settle().await;
        assert_eq!(Arc::strong_count(&guard), 1, "the feed must have ended");
    })
    .await;
}
