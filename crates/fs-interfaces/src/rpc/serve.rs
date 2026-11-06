//! Serving a capability on a Unix socket, and reaching it.
//!
//! Pattern: Adapter (transport) — streams become two-party RPC systems; RAII in [`Connection`];
//! Template method in [`run_until_signal`]: bind, open, serve until a signal. Must run on a `LocalSet`.

use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

use capnp::capability::{Client, FromClientHook};
use capnp::message::ReaderOptions;
use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{RpcSystem, twoparty};
use rustix::io::Errno;
use tokio::net::UnixStream;
use tokio::sync::oneshot;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::finish::{FINISH_TIMEOUT, finish_underway};
use super::socket::{BoundSocket, bind_socket};

/// What this end of a connection reads at most in one message; the default is
/// [`MAX_MESSAGE_BYTES`](super::MAX_MESSAGE_BYTES). It says nothing about what the other end reads, so
/// a message above the default needs a higher limit on the receiving end, and for an answer that is
/// the client. [`Page`](super::Page), [`check_answer`](super::check_answer) and the other limits of this
/// module do not follow it: they stay capped at the default, and a program that answers more must cut
/// its answers itself.
pub type ReadLimits = ReaderOptions;

/// Pause after the system ran out of descriptors or memory, instead of spinning on `accept`.
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);

fn system(
    stream: UnixStream,
    side: Side,
    bootstrap: Option<Client>,
    limits: ReadLimits,
) -> RpcSystem<Side> {
    let (r, w) = stream.into_split();
    let network = twoparty::VatNetwork::new(r.compat(), w.compat_write(), side, limits);
    RpcSystem::new(Box::new(network), bootstrap)
}

fn is_me(stream: &UnixStream) -> bool {
    let me = rustix::process::geteuid().as_raw();
    stream.peer_cred().is_ok_and(|c| c.uid() == me)
}

/// Hand `bootstrap` to every connection until `shutdown` resolves. Only this user may connect
/// (`SO_PEERCRED`); others are dropped. The socket is removed on return.
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
            accepted = socket.listener.accept() => match accepted {
                Ok((stream, _)) => {
                    if !is_me(&stream) {
                        continue;
                    }
                    let rpc = system(stream, Side::Server, Some(bootstrap.clone()), limits);
                    tokio::task::spawn_local(async move {
                        drop(rpc.await);
                    });
                }
                Err(e) => match Errno::from_io_error(&e) {
                    Some(Errno::CONNABORTED) => {}
                    Some(Errno::MFILE | Errno::NFILE | Errno::NOMEM | Errno::NOBUFS) => {
                        tokio::select! {
                            () = &mut shutdown => return Ok(()),
                            () = tokio::time::sleep(ACCEPT_PAUSE) => {}
                        }
                    }
                    _ => return Err(e),
                },
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
    let stream = UnixStream::connect(socket).await?;
    if !is_me(&stream) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is served by another user", socket.display()),
        ));
    }
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
/// then serve what `open` builds until Ctrl-C or SIGTERM. Connections wait in the backlog meanwhile.
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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let bound = bind_socket(socket).await?;
        let bootstrap = open().await?;
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let stop = async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        };
        let served = accept_until(&bound, bootstrap, stop, ReadLimits::default()).await;
        let finished = finish_underway(FINISH_TIMEOUT).await;
        drop(bound);
        served.and(finished)
    })
}
