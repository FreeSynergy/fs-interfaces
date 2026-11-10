//! The version handshake of the client (`connect_matching`): an old server is replaced, a matching one is left alone, a
//! failed replacement is a typed error. Unix: the old server is a child process (this test binary), ended by SIGTERM.
#![cfg(all(feature = "rpc", unix))]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{
    Expected, HandshakeError, RestartTimes, ServerIdentity, bind_socket, connect, connect_matching,
    connect_matching_with, run_until_signal_as, serve, stop_server,
};

/// One server run at a time in this process (`stop_self` is a flag of the process).
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

struct Seven;
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

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// The old server, in a child process: serves `Seven` without `Control` until SIGTERM.
#[test]
fn helper_old_server() {
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

fn spawn_old_server(socket: &Path) -> Child {
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
fn connect_once(socket: &Path) -> bool {
    std::os::unix::net::UnixStream::connect(socket).is_err()
}

/// Starts the new server (with `Control`) in this process on its own thread.
fn start_new(socket: &Path, version: &'static str) -> std::io::Result<()> {
    let socket: PathBuf = socket.to_owned();
    std::thread::spawn(move || {
        let _ = run_until_signal_as(&socket, ServerIdentity::new("test", version), || async {
            let root: blob::Client = new_client(Seven);
            Ok(root.client)
        });
    });
    Ok(())
}

/// Starts the first server and waits until it listens (else "no server" would be what the client finds).
fn start_first(socket: &Path, version: &'static str) {
    start_new(socket, version).unwrap();
    let began = Instant::now();
    while connect_once(socket) {
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

async fn seven(c: &blob::Client) -> u64 {
    c.size_request()
        .send()
        .promise
        .await
        .unwrap()
        .get()
        .unwrap()
        .get_bytes()
}

fn fast() -> RestartTimes {
    RestartTimes {
        call: Duration::from_secs(5),
        stop: Duration::from_secs(10),
        start: Duration::from_secs(10),
    }
}

async fn wait_gone(socket: &Path) {
    let began = Instant::now();
    while !connect_once(socket) {
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "server did not end"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[test]
fn stop_server_reports_an_old_server_as_unsupported() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("old.sock");
    let mut child = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let e = stop_server(&socket).await.unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::Unsupported, "{e}");
    });
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn an_old_server_without_control_is_replaced() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("a.sock");
    let mut old = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let started = AtomicBool::new(false);
        let starter = || {
            started.store(true, Ordering::SeqCst);
            start_new(&socket, "0.0.3")
        };
        let c = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .unwrap();
        assert!(started.load(Ordering::SeqCst));
        assert_eq!(seven(&c.client).await, 7);
        drop(c);
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
    assert!(old.wait().unwrap().success() || old.try_wait().unwrap().is_some());
}

#[test]
fn a_matching_server_is_not_interrupted() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("m.sock");
    start_first(&socket, "0.0.3");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        // A first client, held open across the second one.
        let began = Instant::now();
        let first = loop {
            match connect::<blob::Client>(&socket).await {
                Ok(c) => break c,
                Err(e) => {
                    assert!(began.elapsed() < Duration::from_secs(10), "no server: {e}");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            }
        };
        let never = || -> std::io::Result<()> { panic!("a matching server must not be restarted") };
        let c = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &never)
            .await
            .unwrap();
        assert_eq!(seven(&c.client).await, 7);
        assert_eq!(seven(&first.client).await, 7);
        drop((c, first));
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}

#[test]
fn a_server_of_another_version_is_stopped_by_request_and_replaced() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("v.sock");
    start_first(&socket, "0.0.2");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || start_new(&socket, "0.0.3");
        let c = connect_matching_with::<blob::Client>(
            &socket,
            &Expected::new("0.0.3"),
            &starter,
            fast(),
        )
        .await
        .unwrap();
        assert_eq!(seven(&c.client).await, 7);
        drop(c);
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}

#[test]
fn a_failed_start_is_a_typed_error() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("f.sock");
    let mut old = spawn_old_server(&socket);
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || Err(std::io::Error::other("no executable"));
        let e = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .err()
            .unwrap();
        match &e {
            HandshakeError::CannotRestart {
                running, reason, ..
            } => {
                assert_eq!(running.as_ref().unwrap().protocol, 0);
                assert!(reason.contains("no executable"), "{reason}");
            }
            other => panic!("{other}"),
        }
        assert!(!e.to_string().is_empty());
    });
    // The old server was ended by SIGTERM before the start failed.
    assert!(old.wait().is_ok());
}

#[test]
fn a_replacement_that_is_still_wrong_is_a_mismatch() {
    let _one = ONE_AT_A_TIME.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("w.sock");
    start_first(&socket, "0.0.2");
    let rt = runtime();
    tokio::task::LocalSet::new().block_on(&rt, async {
        let starter = || start_new(&socket, "0.0.2");
        let e = connect_matching::<blob::Client>(&socket, &Expected::new("0.0.3"), &starter)
            .await
            .err()
            .unwrap();
        match e {
            HandshakeError::ServerMismatch { running, expected } => {
                assert_eq!(running.version, "0.0.2");
                assert_eq!(expected.version, "0.0.3");
            }
            other => panic!("{other}"),
        }
        stop_server(&socket).await.unwrap();
        wait_gone(&socket).await;
    });
}
