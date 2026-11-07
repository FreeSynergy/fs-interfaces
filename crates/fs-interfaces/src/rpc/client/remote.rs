//! What a client says about its contract.
//!
//! Pattern: Strategy — the one part of the connection that differs per contract: which root
//! capability to reach, and which capabilities a call may use. The worker is generic over it.

use capnp::capability::FromClientHook;

use crate::Capabilities;

/// The capabilities of one server that a call may use, taken from its root when connecting.
///
/// It is cloned for every call and lives only on the thread of the connection, so it may hold
/// `!Send` capability clients.
pub trait Remote: Clone + 'static {
    /// The root capability of the contract (`tasks::Client`, `people::Client` ...).
    type Root: FromClientHook + Clone;

    /// The handshake: ask the root what the server supports (`supportReport`) and take the
    /// capabilities the client needs from it. Runs on the thread of the connection, under the
    /// connect deadline.
    ///
    /// # Errors
    /// The server does not answer the contract.
    #[allow(async_fn_in_trait)] // the future stays on its thread, so it needs no `Send`
    async fn open(root: Self::Root) -> Result<(Self, Capabilities), capnp::Error>;
}
