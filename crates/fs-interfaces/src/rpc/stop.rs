//! Ending the program from inside, the way a signal does. Pattern: Observer (a flag and a wake-up that
//! [`run_until_signal`](super::run_until_signal) waits on, next to the signals of the system).
//!
//! A program that decides to stop (a lease that ran out) must not send itself a signal: Windows has none to send.

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static WAKE: Notify = Notify::const_new();

/// Asks [`run_until_signal`](super::run_until_signal) to end in order, as a signal would. Callable from any thread, at any
/// time (also before the server runs: the request is remembered).
pub fn stop_self() {
    REQUESTED.store(true, Ordering::SeqCst);
    WAKE.notify_one();
}

/// Resolves once [`stop_self`] was called.
pub(super) async fn requested() {
    while !REQUESTED.load(Ordering::SeqCst) {
        WAKE.notified().await;
    }
}
