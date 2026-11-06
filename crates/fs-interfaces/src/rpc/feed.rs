//! Live changes to a subscribed consumer.
//!
//! Pattern: Observer — a program's feed puts its events into a channel; a local task turns each one
//! into a `Change` and calls the consumer's `ChangeListener`. Dropping the `Subscription` capability
//! (RAII) stops the task and drops the program's own subscription with it.

use std::any::Any;
use std::cell::RefCell;
use std::future::Future;

use capnp_rpc::new_client;
use fs_store::{Envelope, ObjectId};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use super::envelope::write_envelope;
use super::ids::write_object_id;
use crate::common_capnp::{change, change_listener, subscription};

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

struct Forwarding {
    task: RefCell<Option<JoinHandle<()>>>,
    _program_side: Box<dyn Any>,
}

impl subscription::Server for Forwarding {}

impl Drop for Forwarding {
    fn drop(&mut self) {
        if let Some(task) = self.task.borrow_mut().take() {
            task.abort();
        }
    }
}

/// Forward `events` to `listener` until the returned capability is dropped or the listener is gone.
/// `resolve` turns an event into a [`Notice`]; `None` skips it, for example an object already gone.
/// `program_side` is kept alive as long as the capability, usually the core's own subscription.
pub fn forward<E, F, Fut>(
    mut events: UnboundedReceiver<E>,
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
        while let Some(event) = events.recv().await {
            let Some(notice) = resolve(event).await else {
                continue;
            };
            let mut request = listener.change_request();
            write_change(request.get().init_change(), &notice);
            if request.send().await.is_err() {
                break;
            }
        }
    });
    new_client(Forwarding {
        task: RefCell::new(Some(task)),
        _program_side: Box::new(program_side),
    })
}
