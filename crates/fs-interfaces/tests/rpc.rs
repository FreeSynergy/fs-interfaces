//! The shared RPC side, over a real Unix socket (feature `rpc`).
#![cfg(feature = "rpc")]

use std::rc::Rc;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::{blob, change, envelope};
use fs_interfaces::rpc::{
    DEFAULT_PAGE_SIZE, Notice, Page, Problem, bind_socket, connect, read_envelope, read_uuid,
    serve, unsupported, write_change, write_envelope,
};
use fs_interfaces::{UNSUPPORTED_PROBLEM_TYPE, Unsupported};
use fs_store::Envelope;

fn roundtrip_envelope(e: &Envelope) -> Envelope {
    let mut msg = capnp::message::Builder::new_default();
    write_envelope(msg.init_root::<envelope::Builder>(), e);
    read_envelope(msg.get_root_as_reader::<envelope::Reader>().unwrap()).unwrap()
}

#[test]
fn envelope_keeps_every_field() {
    let mut e = Envelope::new("Task", "Name");
    e.iri = Some("https://x/tasks/1".into());
    e.to = vec!["a".into(), "b".into()];
    e.context = Some("tasks".into());
    e.sensitive = vec!["dpv:Health".into()];
    let back = roundtrip_envelope(&e);
    assert_eq!(back.id, e.id);
    assert_eq!(back.iri, e.iri);
    assert_eq!(back.to, e.to);
    assert_eq!(back.context, e.context);
    assert_eq!(back.sensitive, e.sensitive);
    assert_eq!(back.start_time, None);
    assert_eq!(
        back.published.timestamp_millis(),
        e.published.timestamp_millis()
    );
}

#[test]
fn deleted_change_carries_the_id() {
    let e = Envelope::new("Task", "Name");
    let mut msg = capnp::message::Builder::new_default();
    write_change(msg.init_root::<change::Builder>(), &Notice::Deleted(e.id));
    let r = msg.get_root_as_reader::<change::Reader>().unwrap();
    let change::Which::Deleted(id) = r.which().unwrap() else {
        panic!("not deleted");
    };
    assert_eq!(read_uuid(id.unwrap()), Some(e.id.as_uuid()));
}

#[test]
fn pages_cover_everything_exactly_once() {
    let all: Vec<u32> = (0..7).collect();
    let mut seen = Vec::new();
    let mut cursor = Vec::new();
    loop {
        let page = Page::of(all.clone(), &cursor, 3).unwrap();
        seen.extend(page.items);
        if page.next.is_empty() {
            break;
        }
        cursor = page.next;
    }
    assert_eq!(seen, all);
    assert!(Page::of(all, b"bad", DEFAULT_PAGE_SIZE).is_err());
}

#[test]
fn an_exactly_full_last_page_has_no_next() {
    let page = Page::of(vec![1, 2, 3], &[], 3).unwrap();
    assert_eq!(page.items, [1, 2, 3]);
    assert!(page.next.is_empty());
}

#[test]
fn a_problem_survives_the_error_text() {
    let p = Problem::new("https://x/name-conflict", "Name taken", "a.txt exists", 409);
    let e = p.clone().into_error();
    assert!(matches!(e.kind, capnp::ErrorKind::Failed));
    assert_eq!(Problem::from_error(&e), Some(p));
    assert_eq!(
        Problem::from_error(&capnp::Error::failed("plain".into())),
        None
    );
}

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

#[tokio::test(flavor = "current_thread")]
async fn a_problem_crosses_a_real_socket_and_a_second_server_is_refused() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("sub").join("x.sock");
            let listener = bind_socket(&path).await.unwrap();
            let refused = bind_socket(&path).await.unwrap_err();
            assert_eq!(refused.kind(), std::io::ErrorKind::AddrInUse);

            let server: blob::Client = new_client(NoBlob);
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let at = path.clone();
            let serving = tokio::task::spawn_local(async move {
                serve(listener, &at, server.client, async {
                    drop(stopped.await);
                })
                .await
            });

            let client: blob::Client = connect(&path).await.unwrap();
            let error = client.size_request().send().promise.await.err().unwrap();
            assert!(matches!(error.kind, capnp::ErrorKind::Unimplemented));
            let problem = Problem::from_error(&error).expect("typed problem");
            assert_eq!(problem.problem_type, UNSUPPORTED_PROBLEM_TYPE);
            assert_eq!(problem.status, 501);
            assert!(problem.is_unsupported());
            assert_eq!(problem.detail, "blob.size: no bytes here");

            let _ = stop.send(());
            serving.await.unwrap().unwrap();
        })
        .await;
}
