//! A real `tasks` server on a real socket, on a thread of its own, for the tests of the client half.
//!
//! `Items.delete` is the probe: the number in its id picks what the server does with the call.
#![cfg(feature = "rpc")]
#![allow(dead_code)]

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use capnp_rpc::new_client;
use fs_interfaces::Capabilities;
use fs_interfaces::rpc::{Problem, bind_socket, read_uuid, serve};
use fs_interfaces::tasks_capnp::{items, tasks};
use tempfile::TempDir;

/// Answers at once.
pub const OK: u128 = 1;
/// Never answers.
pub const HANG: u128 = 2;
/// Answers a problem `https://freesynergy.net/problems/test/not-found`, status 404.
pub const PROBLEM: u128 = 3;
/// Answers after 300 ms.
pub const SLOW: u128 = 4;
/// Answers `unimplemented` without a problem inside (a method an old server does not have).
pub const OLD: u128 = 5;

struct Items {
    gone: Arc<AtomicBool>,
}

impl Drop for Items {
    fn drop(&mut self) {
        self.gone.store(true, Ordering::SeqCst);
    }
}

impl items::Server for Items {
    async fn delete(
        self: Rc<Self>,
        params: items::DeleteParams,
        _: items::DeleteResults,
    ) -> Result<(), capnp::Error> {
        let id = read_uuid(params.get()?.get_id()?).map_or(0, |u| u.as_u128());
        match id {
            OK => Ok(()),
            SLOW => {
                tokio::time::sleep(Duration::from_millis(300)).await;
                Ok(())
            }
            PROBLEM => Err(Problem::new(
                "https://freesynergy.net/problems/test/not-found",
                "Not found",
                "task 42 not found",
                404,
            )
            .into_error()),
            OLD => Err(capnp::Error::unimplemented("no such method".into())),
            _ => std::future::pending().await,
        }
    }
}

struct Root {
    gone: Arc<AtomicBool>,
}

impl tasks::Server for Root {
    async fn support_report(
        self: Rc<Self>,
        _: tasks::SupportReportParams,
        mut results: tasks::SupportReportResults,
    ) -> Result<(), capnp::Error> {
        let mut caps = Capabilities::new("test-tasks");
        caps.supported("task.create")
            .unsupported("board.open", "not yet");
        caps.write("0", results.get().init_report());
        Ok(())
    }

    async fn items(
        self: Rc<Self>,
        _: tasks::ItemsParams,
        mut results: tasks::ItemsResults,
    ) -> Result<(), capnp::Error> {
        let items: items::Client = new_client(Items {
            gone: self.gone.clone(),
        });
        results.get().set_items(items);
        Ok(())
    }
}

/// The server, stopped and joined when dropped.
pub struct TestServer {
    pub path: PathBuf,
    /// Set when the server dropped its `Items`, which it does when the connection ends.
    pub items_gone: Arc<AtomicBool>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    _dir: TempDir,
}

impl TestServer {
    pub fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t").join("tasks.sock");
        let items_gone = Arc::new(AtomicBool::new(false));
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let (ready, listening) = std::sync::mpsc::channel();
        let (p, gone) = (path.clone(), items_gone.clone());
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async move {
                let bound = bind_socket(&p).await.unwrap();
                ready.send(()).unwrap();
                let root: tasks::Client = new_client(Root { gone });
                let _ = serve(bound, root.client, async {
                    let _ = stopped.await;
                })
                .await;
            });
        });
        listening.recv().unwrap();
        Self {
            path,
            items_gone,
            stop: Some(stop),
            thread: Some(thread),
            _dir: dir,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
