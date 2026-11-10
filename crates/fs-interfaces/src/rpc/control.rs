//! Ending a server from outside, through the protocol: `Control.stop`. Pattern: Decorator (the program's capability, wrapped so
//! that it also answers the interface [`control`]) + Command (the stop).
//!
//! SIGTERM does this on Unix; Windows has no signal to send to another program, so the request travels as a call. The server
//! (see [`run_until_signal`](super::run_until_signal)) answers `Control` itself and hands every other call to the program's own
//! capability, unchanged. Who may ask is decided by the transport: only this user can connect (`SO_PEERCRED` on Unix, the
//! rights of the pipe and the check of the process on Windows), so every caller that reaches this is the owner.

use std::io;
use std::path::Path;
use std::time::Duration;

use capnp::any_pointer;
use capnp::capability::{
    Client, DispatchCallResult, FromServer, Params, Promise, Rc, Results, Server,
};
use capnp::traits::HasTypeId;

use super::serve::connect;
use crate::common_capnp::control;

/// The time the answer gets to leave before the server goes.
const ANSWER_GRACE: Duration = Duration::from_millis(100);

struct Controlled {
    inner: Client,
}

#[derive(Clone)]
struct Dispatch(Rc<Controlled>);

impl FromServer<Controlled> for Client {
    type Dispatch = Dispatch;
    fn from_server(s: Rc<Controlled>) -> Dispatch {
        Dispatch(s)
    }
}

impl std::ops::Deref for Dispatch {
    type Target = Controlled;
    fn deref(&self) -> &Controlled {
        &self.0
    }
}

fn unimplemented() -> DispatchCallResult {
    DispatchCallResult::new(
        Promise::err(capnp::Error::unimplemented(
            "Method not implemented.".into(),
        )),
        false,
    )
}

impl Server for Dispatch {
    fn dispatch_call(
        self,
        interface_id: u64,
        method_id: u16,
        params: Params<any_pointer::Owned>,
        mut results: Results<any_pointer::Owned>,
    ) -> DispatchCallResult {
        if interface_id == <control::Client as HasTypeId>::TYPE_ID {
            if method_id != 0 {
                return unimplemented();
            }
            // Answer first, then go: the program ends in order, as after a signal.
            tokio::task::spawn_local(async {
                tokio::time::sleep(ANSWER_GRACE).await;
                super::stop_self();
            });
            return DispatchCallResult::new(Promise::ok(()), false);
        }
        let mut request = self
            .0
            .inner
            .new_call::<any_pointer::Owned, any_pointer::Owned>(interface_id, method_id, None);
        DispatchCallResult::new(
            Promise::from_future(async move {
                request.get().set_as(params.get()?)?;
                let response = request.send().promise.await?;
                results.get().set_as(response.get()?)?;
                Ok(())
            }),
            false,
        )
    }

    fn as_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
}

/// `capability`, and `Control` next to it.
pub(super) fn with_control(capability: Client) -> Client {
    capnp_rpc::new_client(Controlled { inner: capability })
}

/// Asks the server on `socket` to end in order, and answers when it has taken the request (or is gone already). A program
/// that does not end within its own limits ([`FINISH_TIMEOUT`](super::FINISH_TIMEOUT)) is the program's to report. Must run on
/// a `LocalSet`, like [`connect`].
///
/// # Errors
/// Nothing listens on `socket`, it is served by another user, or the server refused (a server older than `Control`
/// answers `unimplemented`).
pub async fn stop_server(socket: &Path) -> io::Result<()> {
    let connection = connect::<control::Client>(socket).await?;
    match connection.client.stop_request().send().promise.await {
        Ok(_) => Ok(()),
        Err(e) if e.kind == capnp::ErrorKind::Disconnected => Ok(()),
        Err(e) => Err(io::Error::other(e)),
    }
}
