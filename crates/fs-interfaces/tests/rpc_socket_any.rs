//! Sockets and connections of the shared RPC side (feature `rpc`), on every system: Unix socket or named pipe.
#![cfg(feature = "rpc")]

use std::io::ErrorKind;
use std::rc::Rc;

use capnp_rpc::new_client;
use fs_interfaces::UNSUPPORTED_PROBLEM_TYPE;
use fs_interfaces::Unsupported;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{Problem, bind_socket, connect, serve, unsupported};

/// A `Blob` that always answers "not possible", to see the problem cross a real connection.
struct NoBlob;

impl blob::Server for NoBlob {
    #[allow(clippy::unused_async_trait_impl)] // the generated server trait is async; nothing to await here
    async fn size(
        self: Rc<Self>,
        _: blob::SizeParams,
        _: blob::SizeResults,
    ) -> Result<(), capnp::Error> {
        Err(unsupported(&Unsupported::new("blob.size", "no bytes here")))
    }
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
