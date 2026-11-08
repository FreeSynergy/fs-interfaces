//! The shared RPC side, over a real Unix socket (feature `rpc`).
#![cfg(feature = "rpc")]

use fs_interfaces::common_capnp::{change, envelope};
use fs_interfaces::rpc::{
    DEFAULT_PAGE_SIZE, MAX_MESSAGE_BYTES, MAX_PAGE_BYTES, Notice, Page, Problem, TOO_LARGE,
    clamp_read, read_envelope, read_uuid, write_change, write_envelope,
};
use fs_objects::Envelope;

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
    assert!(Page::of(all.clone(), b"bad", DEFAULT_PAGE_SIZE).is_err());
    assert!(
        Page::of(all, &[], 0).is_err(),
        "size zero must not read as the end"
    );
}

#[test]
fn an_exactly_full_last_page_has_no_next() {
    let page = Page::of(vec![1, 2, 3], &[], 3).unwrap();
    assert_eq!(page.items, [1, 2, 3]);
    assert_eq!(page.next, [] as [u8; 0]);
}

#[test]
fn a_problem_can_be_read_back_from_the_wire() {
    let p = Problem::new("https://x/y", "t", "d", 409);
    let mut msg = capnp::message::Builder::new_default();
    p.write(msg.init_root::<fs_interfaces::common_capnp::problem::Builder>());
    let r = msg
        .get_root_as_reader::<fs_interfaces::common_capnp::problem::Reader>()
        .unwrap();
    assert_eq!(Problem::read(r).unwrap(), p);
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

#[test]
fn foreign_text_with_the_marker_is_no_problem_and_a_type_cannot_shift_fields() {
    let foreign =
        capnp::Error::failed("disk said: fs-problem\u{1f}501\u{1f}x\u{1f}y\u{1f}z".into());
    assert_eq!(Problem::from_error(&foreign), None);
    let p = Problem::new("https://x\u{1f}y", "t", "d", 400);
    let back = Problem::from_error(&p.into_error()).unwrap();
    assert_eq!((back.title.as_str(), back.detail.as_str()), ("t", "d"));
}

#[test]
fn an_envelope_without_its_times_is_refused_not_dated_1970() {
    let mut msg = capnp::message::Builder::new_default();
    let e = Envelope::new("Task", "Name");
    write_envelope(msg.init_root::<envelope::Builder>(), &e);
    let mut bare = capnp::message::Builder::new_default();
    {
        let source = msg.get_root_as_reader::<envelope::Reader>().unwrap();
        let mut out = bare.init_root::<envelope::Builder>();
        out.set_type(source.get_type().unwrap());
        out.set_id(source.get_id().unwrap()).unwrap();
    }
    assert!(read_envelope(bare.get_root_as_reader::<envelope::Reader>().unwrap()).is_err());
}

#[test]
fn heavy_pages_end_early_and_an_item_too_large_is_refused() {
    let weigh = |n: &usize| *n;
    let all = vec![MAX_PAGE_BYTES - 1, 2, MAX_MESSAGE_BYTES, 1];
    let first = Page::of_weighted(all.clone(), &[], 10, weigh).unwrap();
    assert_eq!(first.items, [MAX_PAGE_BYTES - 1]);
    let second = Page::of_weighted(all.clone(), &first.next, 10, weigh).unwrap();
    assert_eq!(second.items, [2]);
    let error = Page::of_weighted(all.clone(), &second.next, 10, weigh).unwrap_err();
    let problem = Problem::from_error(&error).unwrap();
    assert_eq!(
        problem.problem_type, TOO_LARGE,
        "better refused than the connection ended"
    );
    let behind = Page::of_weighted(all, &problem.next.unwrap(), 10, weigh).unwrap();
    assert_eq!(
        behind.items,
        [1],
        "the cursor in the problem skips exactly that item"
    );
    assert_eq!(clamp_read(u32::MAX) as usize, MAX_PAGE_BYTES);
    assert_eq!(clamp_read(7), 7);
}
