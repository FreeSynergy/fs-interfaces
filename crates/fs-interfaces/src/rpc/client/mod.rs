//! The client half every program's client shares (feature `rpc`).
//!
//! A `capnp-rpc` client is `!Send`, so a program's client (a frontend, a CLI, another program) keeps
//! it on one thread and talks to that thread through a queue. That thread, its deadlines and the way
//! back of an answer are the same for every contract; they live here once. What stays with each
//! client is its contract: the capabilities it takes from the root ([`Remote`]), its calls, and the
//! mapping of a [`Problem`](super::Problem) to the error type of its own program ([`problems`]).
//!
//! Pattern: Command. A call becomes a command (a closure that gets the capabilities and answers
//! through its own channel). Any thread builds it, only the thread of the connection executes it,
//! each as a local task, so calls overlap. Why: the capability cannot cross threads, a command can,
//! and one queue serves any number of callers without a lock. The strategy for the contract is the
//! trait [`Remote`]; the answer of a command is an RAII guard that answers by itself if the command
//! ends without one, so a lost job reads as [`Fault::Interrupted`] and not as a lost connection.

mod fault;
pub mod problems;
mod remote;
mod reply;
mod timeouts;
mod worker;

pub use fault::Fault;
pub use remote::Remote;
pub use timeouts::{CALL_TIMEOUT, CONNECT_TIMEOUT, Timeouts};
pub use worker::Worker;
