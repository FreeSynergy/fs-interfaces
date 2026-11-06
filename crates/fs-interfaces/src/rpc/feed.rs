//! Live changes to a subscribed consumer.
//!
//! Pattern: Observer — a program's feed puts its events into a bounded channel; a local task turns
//! each one into a `Change` and calls the consumer's `ChangeListener`. Dropping the `Subscription`
//! capability (RAII) stops the task; the task owns the program's own subscription, so that ends with
//! it. A consumer that stops answering is not allowed to grow memory: on overflow what is queued is
//! still delivered, then the feed ends. Every end the consumer did not ask for is told through
//! `ChangeListener.ended`; a feed never ends silently.

use std::any::Any;
use std::cell::RefCell;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use capnp_rpc::new_client;
use fs_store::{Envelope, ObjectId};
use tokio::sync::Notify;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use super::envelope::write_envelope;
use super::ids::write_object_id;
use super::problem::Problem;
use super::pump::pump;
use crate::common_capnp::{change, change_listener, subscription};

/// Events a consumer may fall behind before its feed ends. Large enough for a bulk change made
/// without a pause on the thread that also delivers.
pub const FEED_CAPACITY: usize = 16 * 1024;
/// After an overflow: how long a consumer may take to accept one change while the queue drains.
pub const FEED_STALL: Duration = Duration::from_secs(5);
/// Problem type of `ChangeListener.ended`.
pub const FEED_ENDED_PROBLEM_TYPE: &str = "https://freesynergy.net/problems/feed-ended";

/// One change, as it goes on the wire.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    /// An object was created.
    Created(Envelope),
    /// An object was changed.
    Updated(Envelope),
    /// An object was deleted.
    Deleted(ObjectId),
}

/// Write a [`Notice`] as `Change`.
pub fn write_change(out: change::Builder<'_>, notice: &Notice) {
    match notice {
        Notice::Created(e) => write_envelope(out.init_created(), e),
        Notice::Updated(e) => write_envelope(out.init_updated(), e),
        Notice::Deleted(id) => write_object_id(out.init_deleted(), *id),
    }
}

/// The program's end of a feed. Safe to call from the thread that made the change: never blocks.
pub struct ChangeSender<E> {
    events: Sender<E>,
    overflow: Arc<Notify>,
}

impl<E> ChangeSender<E> {
    /// Hand over an event. If the consumer is too far behind, its feed ends.
    pub fn send(&self, event: E) {
        if matches!(
            self.events.try_send(event),
            Err(mpsc::error::TrySendError::Full(_))
        ) {
            self.overflow.notify_one();
        }
    }
}

/// The forwarder's end of a feed.
pub struct ChangeReceiver<E> {
    pub(super) events: Receiver<E>,
    pub(super) overflow: Arc<Notify>,
}

/// A feed with room for [`FEED_CAPACITY`] events.
#[must_use]
pub fn change_channel<E>() -> (ChangeSender<E>, ChangeReceiver<E>) {
    let (events, receiver) = mpsc::channel(FEED_CAPACITY);
    let overflow = Arc::new(Notify::new());
    let sender = ChangeSender {
        events,
        overflow: overflow.clone(),
    };
    let receiver = ChangeReceiver {
        events: receiver,
        overflow,
    };
    (sender, receiver)
}

struct Forwarding {
    task: RefCell<Option<JoinHandle<()>>>,
}

impl subscription::Server for Forwarding {}

impl Drop for Forwarding {
    fn drop(&mut self) {
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
    }
}

/// Forward `feed` to `listener` until the returned capability is dropped, the listener fails, the
/// source closes or the consumer falls too far behind; all but the first end with `ended`. `resolve` turns an event into a [`Notice`]; `None` skips it, for
/// example an object already gone. `program_side` lives as long as the forwarding, usually the core's
/// own subscription.
pub fn forward<E, F, Fut>(
    feed: ChangeReceiver<E>,
    program_side: impl Any,
    listener: change_listener::Client,
    resolve: F,
) -> subscription::Client
where
    E: 'static,
    F: Fn(E) -> Fut + 'static,
    Fut: Future<Output = Option<Notice>> + 'static,
{
    let task = tokio::task::spawn_local(async move {
        let why = pump(feed, &listener, resolve).await;
        drop(program_side);
        let mut request = listener.ended_request();
        Problem::new(FEED_ENDED_PROBLEM_TYPE, "The feed ended", why, 410)
            .write(request.get().init_problem());
        // The consumer may be the reason this ended; nothing else is left to do for it.
        let _ = request.send().promise.await;
    });
    new_client(Forwarding {
        task: RefCell::new(Some(task)),
    })
}
