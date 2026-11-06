//! Serving a capability on a Unix socket, and reaching it.
//!
//! Pattern: Adapter (transport) — streams become two-party RPC systems; Template method in
//! [`run_until_signal`]: bind, serve until a signal, remove the socket. Must run on a `LocalSet`.

use std::future::Future;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use capnp::capability::{Client, FromClientHook};
use capnp::message::ReaderOptions;
use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{RpcSystem, twoparty};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::socket::bind_socket;

fn system(stream: UnixStream, side: Side, bootstrap: Option<Client>) -> RpcSystem<Side> {
    let (r, w) = stream.into_split();
    let network =
        twoparty::VatNetwork::new(r.compat(), w.compat_write(), side, ReaderOptions::default());
    RpcSystem::new(Box::new(network), bootstrap)
}

/// Hand `bootstrap` to every connection until `shutdown` resolves. Only the socket's owner may
/// connect (`SO_PEERCRED`); others are dropped.
///
/// # Errors
/// The socket's metadata cannot be read.
pub async fn serve(
    listener: UnixListener,
    socket: &Path,
    bootstrap: Client,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    let owner = std::fs::metadata(socket)?.uid();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                if !stream.peer_cred().is_ok_and(|c| c.uid() == owner) {
                    continue;
                }
                let rpc = system(stream, Side::Server, Some(bootstrap.clone()));
                tokio::task::spawn_local(async move {
                    drop(rpc.await);
                });
            }
        }
    }
}

/// Reach the capability served on `socket`. The connection lives on a local task.
///
/// # Errors
/// Nothing listens there.
pub async fn connect<C: FromClientHook>(socket: &Path) -> io::Result<C> {
    let stream = UnixStream::connect(socket).await?;
    let mut rpc = system(stream, Side::Client, None);
    let client: C = rpc.bootstrap(Side::Server);
    tokio::task::spawn_local(async move {
        drop(rpc.await);
    });
    Ok(client)
}

/// A program's main loop: bind `socket`, serve what `open` builds until Ctrl-C or SIGTERM, then
/// remove the socket. Runs its own single-threaded runtime and `LocalSet`.
///
/// # Errors
/// The runtime, the socket or `open` failed.
pub fn run_until_signal<F, Fut>(socket: &Path, open: F) -> io::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = io::Result<Client>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let bootstrap = open().await?;
        let listener = bind_socket(socket).await?;
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let stop = async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        };
        let served = serve(listener, socket, bootstrap, stop).await;
        drop(std::fs::remove_file(socket));
        served
    })
}
