//! texts 0.2: the old methods keep their ordinals, the new ones only append, and the typed
//! conflict problem round-trips. Golden: a change here is a contract change.
#![cfg(feature = "rpc")]

use fs_interfaces::rpc::{Problem, problem_keys as keys};
use fs_interfaces::texts_capnp as t;

const SCHEMA: &str = include_str!("../../../schema/texts.capnp");

/// (name, ordinal) of the methods of `interface Texts`, read from the schema text.
fn methods() -> Vec<(String, u32)> {
    let body = &SCHEMA[SCHEMA.find("interface Texts").expect("interface")..];
    body.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let (head, _) = l.split_once('(')?;
            let (name, ord) = head.trim().split_once('@')?;
            Some((name.trim().to_string(), ord.trim().parse().ok()?))
        })
        .collect()
}

#[test]
fn the_sixteen_old_methods_keep_their_ordinals() {
    let old = [
        "supportReport",
        "create",
        "edit",
        "move",
        "delete",
        "get",
        "getByPath",
        "list",
        "find",
        "links",
        "backlinks",
        "importText",
        "exportText",
        "subscribe",
        "moveFolder",
        "editIf",
    ];
    let m = methods();
    for (i, name) in old.iter().enumerate() {
        assert_eq!(m[i], ((*name).to_string(), u32::try_from(i).unwrap()));
    }
}

#[test]
fn new_methods_are_appended_without_gaps() {
    let m = methods();
    for (i, (_, ord)) in m.iter().enumerate() {
        assert_eq!(*ord, u32::try_from(i).unwrap(), "{m:?}");
    }
    let new: Vec<&str> = m[16..].iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        new,
        [
            "editWith",
            "history",
            "getRevision",
            "restoreRevision",
            "listTrash",
            "restoreFromTrash",
            "purge",
            "createFolder",
            "renameFolder",
            "deleteFolder",
            "getFolder",
            "children",
            "reorder",
            "setTemplate",
            "createFromTemplate",
            "setRedirect",
            "resolve",
            "search",
            "purgeRevision",
            "trash"
        ]
    );
}

#[test]
fn problem_types_and_limits_are_golden() {
    assert_eq!(
        t::CONFLICT_PROBLEM_TYPE,
        "https://freesynergy.net/problems/texts/conflict"
    );
    assert_eq!(
        t::NOT_FOUND_PROBLEM_TYPE,
        "https://freesynergy.net/problems/texts/not-found"
    );
    assert_eq!(
        t::NOT_IN_TRASH_PROBLEM_TYPE,
        "https://freesynergy.net/problems/texts/not-in-trash"
    );
    assert_eq!(
        t::NOT_EMPTY_PROBLEM_TYPE,
        "https://freesynergy.net/problems/texts/not-empty"
    );
    assert_eq!(
        (
            t::CONFLICT_REVISION_KEY,
            t::CONFLICT_EXPECTED_KEY,
            t::STALE_REVISION_ISSUE
        ),
        ("revision", "expected", "stale-revision")
    );
    assert_eq!(
        (
            t::MAX_REDIRECT_HOPS,
            t::MAX_SUMMARY_BYTES,
            t::MAX_SNIPPET_BYTES,
            t::MAX_SNIPPETS_PER_HIT
        ),
        (8, 200, 240, 3)
    );
    assert_eq!(
        (
            t::MAX_HISTORY_PAGE,
            t::MAX_CHILDREN_PAGE,
            t::MAX_SEARCH_PAGE
        ),
        (100, 200, 50)
    );
}

#[test]
fn an_old_entry_reads_with_the_new_fields_at_their_defaults() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut e = msg.init_root::<t::text_entry::Builder>();
        e.set_path("manual/start");
        e.set_revision(7);
    }
    let r = msg.get_root_as_reader::<t::text_entry::Reader>().unwrap();
    assert!(!r.get_template());
    assert!(r.get_redirect_to().unwrap().is_empty());
    assert!(r.get_updated_by().unwrap().is_empty());
    let mut m2 = capnp::message::Builder::new_default();
    let ex = m2.init_root::<t::expect::Builder>().into_reader();
    assert!(matches!(ex.which().unwrap(), t::expect::Which::Any(())));
    let mut m3 = capnp::message::Builder::new_default();
    let q = m3.init_root::<t::search_filter::Builder>().into_reader();
    assert!(matches!(q.get_template().unwrap(), t::Tri::Any));
    assert!(matches!(q.get_redirect().unwrap(), t::Tri::Any));
}

#[test]
fn the_typed_conflict_round_trips_over_the_error_text() {
    let p = Problem::new(
        t::CONFLICT_PROBLEM_TYPE,
        "Conflict",
        "conflict: text changed",
        409,
    )
    .with_ext(keys::ID, "0190-abc")
    .with_ext(keys::KIND, "text")
    .with_ext(keys::ISSUE, t::STALE_REVISION_ISSUE)
    .with_ext_u64(t::CONFLICT_REVISION_KEY, 42)
    .with_ext_u64(t::CONFLICT_EXPECTED_KEY, 41);
    let back = Problem::from_error(&p.clone().into_error()).expect("problem");
    assert_eq!(back, p);
    assert_eq!(back.problem_type, t::CONFLICT_PROBLEM_TYPE);
    assert_eq!(back.ext_u64(t::CONFLICT_REVISION_KEY), Some(42));
    assert_eq!(back.ext(keys::ISSUE), Some("stale-revision"));
    assert!(back.detail.starts_with("conflict:"));
}

#[test]
fn search_hit_with_highlights_round_trips() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let hit = msg.init_root::<t::search_hit::Builder>();
        let mut sn = hit.init_snippets(1).get(0);
        sn.set_field("body");
        sn.set_text("a wiki page");
        let mut h = sn.reborrow().init_highlights(1);
        h.reborrow().get(0).set_start(2);
        h.reborrow().get(0).set_end(6);
    }
    let r = msg.get_root_as_reader::<t::search_hit::Reader>().unwrap();
    let sn = r.get_snippets().unwrap().get(0);
    let h = sn.get_highlights().unwrap().get(0);
    let text = sn.get_text().unwrap().to_str().unwrap();
    assert_eq!(&text[h.get_start() as usize..h.get_end() as usize], "wiki");
}

#[test]
fn a_missing_enum_value_is_never_success() {
    // ordinal 0 is the neutral value, so an unset or unknown field cannot read as "resolved"/"first"/"page"
    assert_eq!(t::Outcome::Unknown as u16, 0);
    assert_eq!(t::Outcome::Resolved as u16, 1);
    assert_eq!(t::Outcome::TooManyHops as u16, 4);
    assert_eq!(t::Placement::Unspecified as u16, 0);
    assert_eq!(t::Placement::After as u16, 4);
    assert_eq!(t::NodeKind::Unknown as u16, 0);
    assert_eq!(t::NodeKind::Space as u16, 3);
    assert_eq!(t::Tri::Any as u16, 0);
    let mut m = capnp::message::Builder::new_default();
    let r = m.init_root::<t::resolution::Builder>().into_reader();
    assert!(matches!(r.get_outcome().unwrap(), t::Outcome::Unknown));
}

#[test]
fn the_added_consts_and_problem_types_are_golden() {
    assert_eq!(
        t::BAD_PATH_PROBLEM_TYPE,
        "https://freesynergy.net/problems/texts/bad-path"
    );
    assert_eq!((t::MAX_SEARCH_WORDS, t::MIN_PREFIX_BYTES), (8, 2));
}
