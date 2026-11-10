//! Serving a capability on the program's socket (a Unix socket, or a named pipe on Windows), and reaching it.
//!
//! Pattern: Adapter (transport) — streams become two-party RPC systems; RAII in [`Connection`];
//! Template method in [`run_until_signal`]: register the signals, bind, open, serve until a signal. Must run on a `LocalSet`.

use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

use capnp::capability::{Client, FromClientHook};
use capnp::message::ReaderOptions;
use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{RpcSystem, twoparty};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::control::ServerIdentity;
use super::finish::{FINISH_TIMEOUT, finish_underway};
use super::transport::{Accepted, BoundSocket, Endpoint, System, bind_socket};

/// What this end of a connection reads at most in one message; the default is
/// [`MAX_MESSAGE_BYTES`](super::MAX_MESSAGE_BYTES). It says nothing about what the other end reads, so
/// a message above the default needs a higher limit on the receiving end, and for an answer that is
/// the client. [`Page`](super::Page), [`check_answer`](super::check_answer) and the other limits of this
/// module do not follow it: they stay capped at the default, and a program that answers more must cut
/// its answers itself.
pub type ReadLimits = ReaderOptions;

/// Pause after the system ran out of descriptors or memory, instead of spinning on `accept`.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

fn system<S: AsyncRead + AsyncWrite + Unpin + 'static>(
    stream: S,
    side: Side,
    bootstrap: Option<Client>,
    limits: ReadLimits,
) -> RpcSystem<Side> {
    let (r, w) = tokio::io::split(stream);
    let network = twoparty::VatNetwork::new(r.compat(), w.compat_write(), side, limits);
    RpcSystem::new(Box::new(network), bootstrap)
}

/// Hand `bootstrap` to every connection until `shutdown` resolves. Only this user may connect
/// (checked by the transport: `SO_PEERCRED` on Unix); others are dropped. The socket is removed on return.
///
/// # Errors
/// The listener failed in a way that will not pass; a shortage of descriptors or memory only pauses.
pub async fn serve(
    socket: BoundSocket,
    bootstrap: Client,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    serve_with(socket, bootstrap, shutdown, ReadLimits::default()).await
}

/// [`serve`] with other read limits.
///
/// # Errors
/// As for [`serve`].
pub async fn serve_with(
    socket: BoundSocket,
    bootstrap: Client,
    shutdown: impl Future<Output = ()>,
    limits: ReadLimits,
) -> io::Result<()> {
    accept_until(&socket, bootstrap, shutdown, limits).await
}

async fn accept_until(
    socket: &BoundSocket,
    bootstrap: Client,
    shutdown: impl Future<Output = ()>,
    limits: ReadLimits,
) -> io::Result<()> {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = System::accept(socket) => match accepted? {
                Accepted::Peer(stream) => {
                    let rpc = system(stream, Side::Server, Some(bootstrap.clone()), limits);
                    tokio::task::spawn_local(async move {
                        drop(rpc.await);
                    });
                }
                Accepted::Skip => {}
                Accepted::Pause => {
                    tokio::select! {
                        () = &mut shutdown => return Ok(()),
                        () = tokio::time::sleep(ACCEPT_PAUSE) => {}
                    }
                }
            }
        }
    }
}

/// A capability on another program, and the connection it lives on. Dropping this closes the
/// connection: every call on a capability taken from it, also one under way, fails as disconnected.
pub struct Connection<C> {
    /// The root capability.
    pub client: C,
    _close: oneshot::Sender<()>,
}

/// Reach the capability served on `socket`.
///
/// # Errors
/// Nothing listens there, or the server runs as another user (`PermissionDenied`).
pub async fn connect<C: FromClientHook>(socket: &Path) -> io::Result<Connection<C>> {
    connect_with(socket, ReadLimits::default()).await
}

/// [`connect`] with other read limits.
///
/// # Errors
/// As for [`connect`].
pub async fn connect_with<C: FromClientHook>(
    socket: &Path,
    limits: ReadLimits,
) -> io::Result<Connection<C>> {
    let stream = System::connect(socket).await?;
    let mut rpc = system(stream, Side::Client, None, limits);
    let client: C = rpc.bootstrap(Side::Server);
    let disconnector = rpc.get_disconnector();
    let (close, closed) = oneshot::channel::<()>();
    tokio::task::spawn_local(async move {
        tokio::pin!(rpc);
        tokio::select! {
            _ = &mut rpc => return,
            _ = closed => {}
        }
        let _ = futures::future::join(disconnector, rpc).await;
    });
    Ok(Connection {
        client,
        _close: close,
    })
}

/// A program's main loop: bind `socket` first, so a second start never touches the data of the first;
/// then serve what `open` builds (and `Control`, see [`stop_server`](super::stop_server)) until Ctrl-C, SIGTERM (a stop request of the system), [`stop_server`](super::stop_server) or [`stop_self`](super::stop_self). Connections wait in the backlog meanwhile.
/// After the signal it waits up to [`FINISH_TIMEOUT`] for [`run_to_end`](super::run_to_end) work, and
/// holds the socket and its lock until then, so a successor cannot open the same data during the wait.
/// Runs its own single-threaded runtime and `LocalSet`.
///
/// # Errors
/// The runtime, the socket or `open` failed, or work under way did not finish in time.
pub fn run_until_signal<F, Fut>(socket: &Path, open: F) -> io::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = io::Result<Client>>,
{
    run_until_signal_as(socket, ServerIdentity::default(), open)
}

/// [`run_until_signal`] that also tells who it is (`Control.version`), so a client of another version can replace this server
/// ([`connect_matching`](super::connect_matching)). Every program should use this one, with its own name and version.
///
/// # Errors
/// As for [`run_until_signal`].
pub fn run_until_signal_as<F, Fut>(
    socket: &Path,
    identity: ServerIdentity,
    open: F,
) -> io::Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = io::Result<Client>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        // A request to stop belongs to this run: it is forgotten when the run ends, so a next run in the same process starts.
        let _run = super::stop::Run;
        // Handlers first: a stop during the start (a lease that ran out, a stop under load) must reach the clean shutdown
        // below, not kill the process. A signal that comes now is remembered and ends the serving as soon as it starts.
        let signal = System::stop_signal()?;
        let stop = async move {
            tokio::select! {
                () = signal => {}
                () = super::stop::requested() => {}
            }
        };
        let bound = bind_socket(socket).await?;
        let bootstrap = super::control::with_control(open().await?, identity);
        let served = accept_until(&bound, bootstrap, stop, ReadLimits::default()).await;
        let finished = finish_underway(FINISH_TIMEOUT).await;
        drop(bound);
        served.and(finished)
    })
}
