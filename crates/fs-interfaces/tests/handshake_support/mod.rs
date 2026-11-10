//! Shared by the handshake tests: an old server in a child process (this test binary, started again), the new server in a thread.
#![allow(dead_code)] // each test file uses a part

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{RestartTimes, ServerIdentity, bind_socket, run_until_signal_as, serve};

/// One server run at a time in this process (`stop_self` is a flag of the process).
pub static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

pub struct Seven;
impl blob::Server for Seven {
    #[allow(clippy::unused_async_trait_impl)] // the generated server trait is async; nothing to await here
    async fn size(
        self: capnp::capability::Rc<Self>,
        _: blob::SizeParams,
        mut results: blob::SizeResults,
    ) -> Result<(), capnp::Error> {
        results.get().set_bytes(7);
        Ok(())
    }
}

pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// The old server, in a child process: serves `Seven` without `Control` until SIGTERM.
pub fn old_server_main() {
    let Some(socket) = std::env::var_os("FS_OLD_SERVER") else {
        return;
    };
    let runtime = runtime();
    tokio::task::LocalSet::new().block_on(&runtime, async {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        let bound = bind_socket(Path::new(&socket)).await.unwrap();
        let root: blob::Client = new_client(Seven);
        serve(bound, root.client, async move {
            term.recv().await;
        })
        .await
        .unwrap();
    });
}

pub fn spawn_old_server(socket: &Path) -> Child {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper_old_server", "--nocapture"])
        .env("FS_OLD_SERVER", socket)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let began = Instant::now();
    while connect_once(socket) {
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "old server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    child
}

/// True while nothing answers on `socket`.
pub fn connect_once(socket: &Path) -> bool {
    std::os::unix::net::UnixStream::connect(socket).is_err()
}

/// Starts the new server (with `Control`) in this process on its own thread.
pub fn start_new(socket: &Path, version: &'static str) {
    let socket: PathBuf = socket.to_owned();
    std::thread::spawn(move || {
        let _ = run_until_signal_as(&socket, ServerIdentity::new("test", version), || async {
            let root: blob::Client = new_client(Seven);
            Ok(root.client)
        });
    });
}

/// Starts the first server and waits until it listens (else "no server" would be what the client finds).
pub fn start_first(socket: &Path, version: &'static str) {
    start_new(socket, version);
    let began = Instant::now();
    while connect_once(socket) {
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub async fn seven(c: &blob::Client) -> u64 {
    c.size_request()
        .send()
        .promise
        .await
        .unwrap()
        .get()
        .unwrap()
        .get_bytes()
}

pub fn fast() -> RestartTimes {
    RestartTimes {
        call: Duration::from_secs(5),
        stop: Duration::from_secs(10),
        start: Duration::from_secs(10),
    }
}

pub async fn wait_gone(socket: &Path) {
    let began = Instant::now();
    while !connect_once(socket) {
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "server did not end"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
