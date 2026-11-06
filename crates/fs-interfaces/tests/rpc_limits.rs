//! What does not fit one message, over a real connection: the connection stays and what is behind stays readable.
#![cfg(feature = "rpc")]

use std::path::Path;
use std::rc::Rc;

use capnp_rpc::new_client;
use fs_interfaces::places_capnp::places;
use fs_interfaces::rpc::{
    MAX_DETAIL_BYTES, MAX_ITEM_BYTES, MAX_MESSAGE_BYTES, MAX_PAGE_BYTES, Page, Problem, ReadLimits,
    TOO_LARGE, bind_socket, connect, connect_with, failed, invalid_request, serve,
};

const MIB: usize = 1024 * 1024;
const STRUCTURE: usize = 1024;
const _: () = assert!(MAX_PAGE_BYTES < MAX_ITEM_BYTES && MAX_ITEM_BYTES < MAX_MESSAGE_BYTES);

/// A `Places` provider whose places are only their text, enough to see an answer cross the wire.
struct Texts(Vec<String>);

fn write(out: &mut places::ListResults, page: &Page<String>) -> capnp::Result<()> {
    let mut answer = out.get().init_page();
    let mut items = answer
        .reborrow()
        .init_items(u32::try_from(page.items.len()).unwrap());
    for (i, text) in (0u32..).zip(&page.items) {
        items.reborrow().get(i).set_geojson(text.as_str());
    }
    answer.init_next().set_opaque(&page.next);
    Ok(())
}

impl places::Server for Texts {
    async fn list(
        self: Rc<Self>,
        params: places::ListParams,
        mut results: places::ListResults,
    ) -> Result<(), capnp::Error> {
        let cursor = params.get()?.get_cursor()?.get_opaque()?;
        let page = Page::of_weighted(self.0.clone(), cursor, 10, |t: &String| t.len() + STRUCTURE)?;
        write(&mut results, &page)
    }

    /// Answers a text only this method's caller can read: above what a connection reads by default.
    async fn within(
        self: Rc<Self>,
        _: places::WithinParams,
        mut results: places::WithinResults,
    ) -> Result<(), capnp::Error> {
        let mut items = results.get().init_page().init_items(1);
        items
            .reborrow()
            .get(0)
            .set_geojson("x".repeat(MAX_MESSAGE_BYTES + MIB).as_str());
        Ok(())
    }

    /// Fails the way a program does that quotes what it was sent.
    async fn find_by_name(
        self: Rc<Self>,
        params: places::FindByNameParams,
        _: places::FindByNameResults,
    ) -> Result<(), capnp::Error> {
        let huge = "€".repeat(30 * MIB);
        if params.get()?.get_term()?.to_str()? == "problem" {
            Err(invalid_request(huge))
        } else {
            Err(failed(huge))
        }
    }
}

async fn on_socket<F, Fut>(texts: Vec<String>, test: F)
where
    F: FnOnce(std::path::PathBuf) -> Fut,
    Fut: Future<Output = ()>,
{
    tokio::task::LocalSet::new()
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("x.sock");
            let bound = bind_socket(&path).await.unwrap();
            let root: places::Client = new_client(Texts(texts));
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let serving = tokio::task::spawn_local(serve(bound, root.client, async {
                let _ = stopped.await;
            }));
            test(path).await;
            let _ = stop.send(());
            serving.await.unwrap().unwrap();
        })
        .await;
}

/// The text lengths of one page and its cursor, or the error.
async fn page(client: &places::Client, cursor: &[u8]) -> capnp::Result<(Vec<usize>, Vec<u8>)> {
    let mut call = client.list_request();
    call.get().init_cursor().set_opaque(cursor);
    let answer = call.send().promise.await?;
    let page = answer.get()?.get_page()?;
    let sizes = page
        .get_items()?
        .iter()
        .map(|p| Ok(p.get_geojson()?.to_str()?.len()))
        .collect::<capnp::Result<_>>()?;
    Ok((sizes, page.get_next()?.get_opaque()?.to_vec()))
}

async fn client_on(path: &Path) -> fs_interfaces::rpc::Connection<places::Client> {
    connect::<places::Client>(path).await.unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn what_is_behind_an_item_too_large_stays_readable() {
    let sizes = [
        1,
        MAX_ITEM_BYTES + 1,
        MAX_ITEM_BYTES - STRUCTURE,
        40 * MIB,
        3,
    ];
    let texts = sizes.iter().map(|n| "x".repeat(*n)).collect();
    on_socket(texts, |path| async move {
        let connection = client_on(&path).await;
        let client = &connection.client;

        let (first, cursor) = page(client, &[]).await.unwrap();
        assert_eq!(first, [1], "the page ends before the item too large");

        let refused = page(client, &cursor).await.err().unwrap();
        let problem = Problem::from_error(&refused).expect("a typed problem");
        assert_eq!(
            (problem.problem_type.as_str(), problem.status),
            (TOO_LARGE, 413)
        );
        let past = problem.next.expect("a cursor past the item");

        let (largest, cursor) = page(client, &past).await.unwrap();
        assert_eq!(
            largest,
            [MAX_ITEM_BYTES - STRUCTURE],
            "the largest item comes alone"
        );
        let (alone, cursor) = page(client, &cursor).await.unwrap();
        assert_eq!(alone, [40 * MIB], "above a page size it still comes, alone");
        let (last, cursor) = page(client, &cursor).await.unwrap();
        assert_eq!(last, [3]);
        assert!(cursor.is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_item_too_large_at_the_very_end_has_no_cursor() {
    let texts = vec!["x".to_string(), "x".repeat(MAX_ITEM_BYTES + 1)];
    on_socket(texts, |path| async move {
        let connection = client_on(&path).await;
        let (_, cursor) = page(&connection.client, &[]).await.unwrap();
        let refused = page(&connection.client, &cursor).await.err().unwrap();
        assert_eq!(Problem::from_error(&refused).unwrap().next, None);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_error_with_a_huge_text_does_not_end_the_connection() {
    on_socket(vec!["x".into()], |path| async move {
        let connection = client_on(&path).await;
        let client = &connection.client;
        for term in ["problem", "plain"] {
            let mut call = client.find_by_name_request();
            call.get().set_term(term);
            let error = call.send().promise.await.err().unwrap();
            assert!(
                error.extra.len() < 2 * MAX_DETAIL_BYTES,
                "{}",
                error.extra.len()
            );
            assert!(!matches!(error.kind, capnp::ErrorKind::Disconnected));
        }
        assert_eq!(page(client, &[]).await.unwrap().0, [1], "still connected");
    })
    .await;
}

#[test]
fn a_detail_is_cut_on_a_character_boundary() {
    // 2 + 3 k bytes: the limit of 65536 falls inside a three-byte character.
    assert_eq!(MAX_DETAIL_BYTES, 65_536);
    let long = format!("xx{}", "€".repeat(MAX_DETAIL_BYTES));
    let error = Problem::new("t", "T", long.as_str(), 422).into_error();
    let detail = Problem::from_error(&error).unwrap().detail;
    assert_eq!(detail.len(), 65_534);
    assert_eq!(failed(&long).extra.len(), 65_534);
    let short = Problem::new("t", "T", "short", 422).into_error();
    assert_eq!(Problem::from_error(&short).unwrap().detail, "short");
}

#[test]
fn a_cursor_survives_the_error_text_and_nonsense_is_no_cursor() {
    let p = Problem::new("t", "T", "d", 413).with_next(Some(vec![0, 1, 0xab, 0xff]));
    assert_eq!(Problem::from_error(&p.clone().into_error()), Some(p));
    let none = Problem::new("t", "T", "d", 413);
    assert_eq!(Problem::from_error(&none.clone().into_error()), Some(none));
    let nonsense = capnp::Error::failed("fs-problem\u{1f}413\u{1f}t\u{1f}T\u{1f}d\u{1f}zz€".into());
    let read = Problem::from_error(&nonsense).unwrap();
    assert_eq!((read.detail.as_str(), read.next), ("d", None));
}

#[tokio::test(flavor = "current_thread")]
async fn read_limits_are_those_of_the_end_that_reads() {
    on_socket(vec![], |path| async move {
        let larger = ReadLimits {
            traversal_limit_in_words: Some(2 * MAX_MESSAGE_BYTES / 8),
            ..ReadLimits::default()
        };
        let raised = connect_with::<places::Client>(&path, larger).await.unwrap();
        let answer = raised.client.within_request().send().promise.await.unwrap();
        let items = answer
            .get()
            .unwrap()
            .get_page()
            .unwrap()
            .get_items()
            .unwrap();
        let text = items.get(0).get_geojson().unwrap();
        assert_eq!(text.to_str().unwrap().len(), MAX_MESSAGE_BYTES + MIB);

        let default = client_on(&path).await;
        let lost = default.client.within_request().send().promise.await;
        assert!(lost.is_err(), "the default reads no more than a message");
    })
    .await;
}
