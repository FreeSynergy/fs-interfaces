//! A server is ended from outside through the protocol (`stop_server`), on every system (feature `rpc`). Alone in its file: the
//! request is remembered by the whole process.
#![cfg(feature = "rpc")]

use std::time::{Duration, Instant};

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::blob;
use fs_interfaces::rpc::{connect, run_until_signal, stop_server};

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

#[test]
fn a_stop_request_ends_the_server_in_order_and_its_own_calls_still_pass() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.sock");
    let socket = path.clone();
    let server = std::thread::spawn(move || {
        run_until_signal(&socket, || async move {
            let root: blob::Client = new_client(Seven);
            Ok(root.client)
        })
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&runtime, async {
        // The server starts on its own thread: wait until it answers.
        let began = Instant::now();
        let connection = loop {
            match connect::<blob::Client>(&path).await {
                Ok(c) => break c,
                Err(e) => {
                    assert!(began.elapsed() < Duration::from_secs(10), "no server: {e}");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            }
        };
        // A call of the program's own interface goes through the wrapper that answers `Control`.
        let answer = connection
            .client
            .size_request()
            .send()
            .promise
            .await
            .unwrap();
        assert_eq!(answer.get().unwrap().get_bytes(), 7);
        drop(connection);
        stop_server(&path).await.unwrap();
    });
    server.join().unwrap().unwrap();
    assert!(!path.exists());
}
