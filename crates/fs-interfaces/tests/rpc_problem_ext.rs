//! Extension members of a `Problem`: round trips, separators, typed readers.
#![cfg(feature = "rpc")]

use fs_interfaces::rpc::{Problem, problem_keys as keys, too_large_by};

fn base() -> Problem {
    Problem::new(
        "https://freesynergy.net/problems/x/bad",
        "Bad",
        "it broke",
        422,
    )
}

fn round(p: Problem) -> Problem {
    Problem::from_error(&p.into_error()).expect("a problem comes back")
}

#[test]
fn every_field_survives_with_a_cursor_and_extensions() {
    let p = base()
        .with_next(Some(vec![1, 2, 255]))
        .with_ext(keys::ID, "0190-abc")
        .with_ext_u64(keys::SIZE, 70_000)
        .with_ext(keys::KIND, "task");
    assert_eq!(round(p.clone()), p);
}

#[test]
fn extensions_alone_and_none_at_all() {
    let p = base().with_ext(keys::KIND, "person");
    let back = round(p.clone());
    assert_eq!(
        (back.next.clone(), back.ext(keys::KIND)),
        (None, Some("person"))
    );
    assert_eq!(back, p);
    let plain = round(base());
    assert!(plain.extensions.is_empty() && plain.next.is_none());
}

#[test]
fn a_key_may_repeat_and_keeps_its_order() {
    let p = base()
        .with_ext(keys::ISSUE, "name")
        .with_ext(keys::KIND, "x")
        .with_ext(keys::ISSUE, "start-after-end")
        .with_ext(keys::ISSUE, "");
    let back = round(p);
    assert_eq!(
        back.ext_all(keys::ISSUE).collect::<Vec<_>>(),
        ["name", "start-after-end", ""]
    );
    assert_eq!(back.ext(keys::ISSUE), Some("name"));
    assert_eq!(back.ext_all("nothing").count(), 0);
}

#[test]
fn numbers_are_typed_and_a_non_number_is_none() {
    let back = round(
        base()
            .with_ext_u64(keys::LIMIT, u64::MAX)
            .with_ext(keys::SIZE, "big"),
    );
    assert_eq!(back.ext_u64(keys::LIMIT), Some(u64::MAX));
    assert_eq!(back.ext_u64(keys::SIZE), None);
    assert_eq!(back.ext_u64("absent"), None);
}

#[test]
fn separators_in_keys_and_values_become_spaces_and_break_nothing() {
    let p = base()
        .with_next(Some(vec![9]))
        .with_ext("a\u{1f}b\u{1e}c", "v1\u{1f}ext\u{1e}k\u{1e}v2")
        .with_ext("after", "ok");
    let back = round(p);
    assert_eq!(back.next, Some(vec![9]));
    assert_eq!(back.ext("a b c"), Some("v1 ext k v2"));
    assert_eq!(back.ext("after"), Some("ok"));
    assert_eq!(back.extensions.len(), 2);
}

#[test]
fn unicode_and_newlines_are_kept() {
    let back = round(base().with_ext("größe", "日本語\nzwei Zeilen 🎉"));
    assert_eq!(back.ext("größe"), Some("日本語\nzwei Zeilen 🎉"));
}

#[test]
fn an_empty_key_is_left_out() {
    assert!(base().with_ext("", "v").extensions.is_empty());
    assert!(base().with_ext("\u{1f}", "v").extensions.len() == 1); // becomes " "
}

#[test]
fn too_large_by_names_size_and_limit() {
    let p = Problem::from_error(&too_large_by("five bytes", 5, 3)).expect("problem");
    assert_eq!(
        (p.status, p.ext_u64(keys::SIZE), p.ext_u64(keys::LIMIT)),
        (413, Some(5), Some(3))
    );
}

#[test]
fn a_leading_plus_is_no_number() {
    let p = base()
        .with_ext("n", "+5")
        .with_ext("m", "-5")
        .with_ext("o", " 5")
        .with_ext("q", "5");
    assert_eq!(
        ["n", "m", "o", "q"].map(|k| p.ext_u64(k)),
        [None, None, None, Some(5)]
    );
}
