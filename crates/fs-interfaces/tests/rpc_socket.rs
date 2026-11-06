//! Sockets and connections of the shared RPC side (feature `rpc`).
#![cfg(feature = "rpc")]

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::rc::Rc;

use capnp_rpc::new_client;
use fs_interfaces::UNSUPPORTED_PROBLEM_TYPE;
use fs_interfaces::Unsupported;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{Problem, bind_socket, connect, serve, unsupported};

/// A `Blob` that always answers "not possible", to see the problem cross a real connection.
struct NoBlob;

impl blob::Server for NoBlob {
    async fn size(
        self: Rc<Self>,
        _: blob::SizeParams,
        _: blob::SizeResults,
    ) -> Result<(), capnp::Error> {
        Err(unsupported(&Unsupported::new("blob.size", "no bytes here")))
    }
}

fn open_fds() -> usize {
    std::fs::read_dir("/proc/self/fd").unwrap().count()
}

async fn local<F: Future<Output = ()>>(test: F) {
    tokio::task::LocalSet::new().run_until(test).await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_problem_crosses_a_real_socket_and_the_socket_goes_with_the_server() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("x.sock");
        let bound = bind_socket(&path).await.unwrap();
        assert_eq!(
            bind_socket(&path).await.unwrap_err().kind(),
            ErrorKind::AddrInUse
        );
        let server: blob::Client = new_client(NoBlob);
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let serving = tokio::task::spawn_local(serve(bound, server.client, async {
            let _ = stopped.await;
        }));

        let client = connect::<blob::Client>(&path).await.unwrap();
        let error = client
            .client
            .size_request()
            .send()
            .promise
            .await
            .err()
            .unwrap();
        assert!(matches!(error.kind, capnp::ErrorKind::Unimplemented));
        let problem = Problem::from_error(&error).expect("typed problem");
        assert_eq!(problem.problem_type, UNSUPPORTED_PROBLEM_TYPE);
        assert_eq!(problem.status, 501);
        assert!(problem.is_unsupported());
        assert_eq!(problem.detail, "blob.size: no bytes here");

        let _ = stop.send(());
        serving.await.unwrap().unwrap();
        assert!(!path.exists());
        bind_socket(&path).await.expect("free again");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_connections_close_on_both_sides() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.sock");
        let bound = bind_socket(&path).await.unwrap();
        let server: blob::Client = new_client(NoBlob);
        tokio::task::spawn_local(serve(bound, server.client, std::future::pending()));
        let before = open_fds();
        for _ in 0..20 {
            let client = connect::<blob::Client>(&path).await.unwrap();
            let _ = client.client.size_request().send().promise.await;
            drop(client);
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(open_fds() <= before + 2, "{before} -> {}", open_fds());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn only_a_dead_socket_is_ever_removed() {
    local(async {
        let dir = tempfile::tempdir().unwrap();

        let file = dir.path().join("file.sock");
        std::fs::write(&file, "data").unwrap();
        assert_eq!(
            bind_socket(&file).await.unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "data");

        let link = dir.path().join("link.sock");
        std::os::unix::fs::symlink(dir.path().join("nowhere"), &link).unwrap();
        assert_eq!(
            bind_socket(&link).await.unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );

        let stale = dir.path().join("stale.sock");
        drop(std::os::unix::net::UnixListener::bind(&stale).unwrap());
        bind_socket(&stale).await.expect("stale socket replaced");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_starts_have_exactly_one_winner() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        for round in 0..50 {
            let path = dir.path().join(format!("{round}.sock"));
            drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
            let (a, b) = tokio::join!(bind_socket(&path), bind_socket(&path));
            assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_directory_others_can_reach_is_refused() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let open = dir.path().join("open");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, PermissionsExt::from_mode(0o777)).unwrap();
        let refused = bind_socket(&open.join("x.sock")).await.unwrap_err();
        assert_eq!(refused.kind(), ErrorKind::PermissionDenied);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_writable_ancestor_is_refused_too() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let open = dir.path().join("open");
        let inner = open.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::set_permissions(&inner, PermissionsExt::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&open, PermissionsExt::from_mode(0o777)).unwrap();
        let refused = bind_socket(&inner.join("x.sock")).await.unwrap_err();
        assert_eq!(refused.kind(), ErrorKind::PermissionDenied);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn calls_after_the_connection_was_dropped_fail_instead_of_hanging() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.sock");
        let bound = bind_socket(&path).await.unwrap();
        let server: blob::Client = new_client(NoBlob);
        tokio::task::spawn_local(serve(bound, server.client, std::future::pending()));
        let connection = connect::<blob::Client>(&path).await.unwrap();
        let client = connection.client.clone();
        drop(connection);
        let answer = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            client.size_request().send().promise,
        )
        .await
        .expect("the call hung");
        let error = answer.err().unwrap();
        assert!(
            matches!(error.kind, capnp::ErrorKind::Disconnected),
            "{error:?}"
        );
    })
    .await;
}

#[test]
fn options_refuse_what_they_cannot_use() {
    use fs_interfaces::rpc::ServerOptions;
    let parse =
        |args: &[&str]| ServerOptions::parse("x", args.iter().map(std::ffi::OsString::from));
    let full = ["--socket", "/s", "--data", "/d", "--actor", "me"];
    let options = parse(&full).unwrap().unwrap();
    assert_eq!(options.actor, "me");
    assert_eq!(parse(&["--help"]), Ok(None));
    assert!(parse(&["--nonsense"]).is_err());
    assert!(parse(&["--socket"]).is_err());
    assert!(parse(&["--socket", "--data"]).is_err());
    assert!(parse(&["--socket", "/s", "--data", "/d", "--actor", ""]).is_err());
}

#[test]
fn an_instance_is_a_domain() {
    use fs_interfaces::rpc::ServerOptions;
    let parse = |name: &str| {
        let args = ["--socket", "/s", "--data", "/d", "--instance", name];
        ServerOptions::parse("x", args.map(Into::into))
    };
    for good in [
        "example.org",
        "a.b-c.example",
        "xn--bcher-kva.example",
        "localhost",
    ] {
        let options = parse(good).unwrap().unwrap();
        assert_eq!(options.instance.as_deref(), Some(good));
    }
    let long_label = "a".repeat(64);
    let longest = ["a"; 127].join(".");
    assert_eq!(longest.len(), 253);
    assert!(parse(&longest).is_ok());
    let long_name = ["a"; 128].join(".");
    for bad in [
        "a/b",
        "x y",
        "a@b",
        "a:80",
        "a..b",
        ".a",
        "a.",
        "-a",
        "a-",
        "bücher.example",
        "a?b",
        "a#b",
    ] {
        assert!(parse(bad).is_err(), "{bad}");
    }
    assert!(parse(&long_label).is_err());
    assert!(parse(&long_name).is_err());
}

#[test]
fn a_data_directory_is_for_this_user_alone() {
    use fs_interfaces::rpc::prepare_data_dir;
    let dir = tempfile::tempdir().unwrap();
    let fresh = dir.path().join("a").join("b");
    assert_eq!(
        prepare_data_dir(&fresh).unwrap(),
        fresh.canonicalize().unwrap()
    );
    let through = dir.path().join("link");
    std::os::unix::fs::symlink(dir.path().join("a"), &through).unwrap();
    assert_eq!(
        prepare_data_dir(&through.join("b")).unwrap(),
        fresh.canonicalize().unwrap(),
        "the path that was checked, not the one through the link"
    );
    let mode = std::fs::metadata(&fresh).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    std::fs::set_permissions(&fresh, PermissionsExt::from_mode(0o755)).unwrap();
    let refused = prepare_data_dir(&fresh).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::PermissionDenied);
}

#[test]
fn a_data_directory_below_a_writable_parent_is_refused() {
    use fs_interfaces::rpc::prepare_data_dir;
    let dir = tempfile::tempdir().unwrap();
    let open = dir.path().join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, PermissionsExt::from_mode(0o777)).unwrap();
    let refused = prepare_data_dir(&open.join("data")).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::PermissionDenied);
}

#[tokio::test(flavor = "current_thread")]
async fn a_fresh_path_is_private_at_every_level() {
    local(async {
        let dir = tempfile::tempdir().unwrap();
        let top = dir.path().join("a");
        let path = top.join("b").join("x.sock");
        let _bound = bind_socket(&path).await.unwrap();
        for d in [top.clone(), top.join("b")] {
            let mode = std::fs::metadata(&d).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "{}", d.display());
        }
    })
    .await;
}
