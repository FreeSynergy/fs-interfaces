//! Extension members of a `Problem`: round trips, separators, limits, and the old text.
#![cfg(feature = "rpc")]

use fs_interfaces::rpc::{
    MAX_DETAIL_BYTES, MAX_EXTENSIONS, MAX_KEY_BYTES, Problem, problem_keys as keys, too_large_by,
};

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
fn at_most_64_entries() {
    let mut p = base();
    for i in 0..MAX_EXTENSIONS + 10 {
        p = p.with_ext("issue", &i.to_string());
    }
    assert_eq!(p.extensions.len(), MAX_EXTENSIONS);
    assert_eq!(round(p).extensions.len(), MAX_EXTENSIONS);
}

#[test]
fn a_key_of_64_bytes_fits_and_a_longer_one_is_left_out() {
    let fits = "ä".repeat(MAX_KEY_BYTES / 2);
    let p = base()
        .with_ext(&fits, "v")
        .with_ext(&format!("{fits}x"), "w");
    assert_eq!(p.extensions.len(), 1);
    assert_eq!(round(p).ext(&fits), Some("v"));
}

#[test]
fn all_extension_texts_together_stay_within_the_detail_limit() {
    let big = "x".repeat(MAX_DETAIL_BYTES);
    let p = base()
        .with_ext("a", &big)
        .with_ext("b", &big)
        .with_ext("c", "tiny");
    let total: usize = p.extensions.iter().map(|(k, v)| k.len() + v.len()).sum();
    assert!(total <= MAX_DETAIL_BYTES, "{total}");
    // The first entry took all room, so nothing else fits.
    assert_eq!(p.extensions.len(), 1);
    let error = round(p).into_error();
    assert!(error.extra.len() < MAX_DETAIL_BYTES + 1024);
}

#[test]
fn a_hand_filled_field_is_limited_when_it_leaves() {
    let mut p = base();
    p.extensions = (0..200)
        .map(|i| (format!("k{i}"), "v".repeat(2000)))
        .collect();
    let back = round(p);
    let total: usize = back.extensions.iter().map(|(k, v)| k.len() + v.len()).sum();
    assert!(back.extensions.len() <= MAX_EXTENSIONS && total <= MAX_DETAIL_BYTES);
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
fn a_value_that_does_not_fit_is_left_out_not_cut() {
    let filler = "x".repeat(MAX_DETAIL_BYTES - 12);
    let p = base()
        .with_ext("filler", &filler)
        .with_ext_u64(keys::SIZE, 123_456);
    assert_eq!(p.ext_u64(keys::SIZE), None, "never a cut number");
    assert_eq!(p.extensions.len(), 1);
    let q = base().with_ext("v", &"y".repeat(MAX_DETAIL_BYTES + 1));
    assert!(
        q.extensions.is_empty(),
        "a value above the limit is no value"
    );
    let long_key = base().with_ext(&"k".repeat(MAX_KEY_BYTES + 1), "v");
    assert!(
        long_key.extensions.is_empty(),
        "a cut key would name another key"
    );
}

#[test]
fn the_reader_leaves_out_what_it_cannot_take_whole() {
    let mut p = base();
    p.extensions = vec![
        ("filler".into(), "x".repeat(MAX_DETAIL_BYTES - 12)),
        (keys::SIZE.into(), "123456".into()),
    ];
    let back = round(p);
    assert_eq!(back.ext_u64(keys::SIZE), None);
    assert_eq!(back.extensions.len(), 1);
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

#[test]
fn the_wire_problem_carries_the_extensions_too() {
    use fs_interfaces::common_capnp::problem;
    let p = base()
        .with_ext(keys::ISSUE, "a")
        .with_ext(keys::ISSUE, "b")
        .with_ext_u64(keys::SIZE, 9);
    let mut message = capnp::message::Builder::new_default();
    p.write(message.init_root::<problem::Builder>());
    let back = Problem::read(message.get_root_as_reader::<problem::Reader>().unwrap()).unwrap();
    assert_eq!(back.extensions, p.extensions);
}

#[test]
fn an_item_too_large_with_a_cursor_stays_readable_by_old_readers() {
    use fs_interfaces::rpc::{MAX_MESSAGE_BYTES, Page};
    let all = vec![MAX_MESSAGE_BYTES, 1];
    let error = Page::of_weighted(all.clone(), &[], 10, |n| *n).unwrap_err();
    let p = Problem::from_error(&error).unwrap();
    assert!(
        p.next.is_some() && p.extensions.is_empty(),
        "numbers are in the detail already"
    );
    let last = Page::of_weighted(vec![MAX_MESSAGE_BYTES], &[], 10, |n| *n).unwrap_err();
    let p = Problem::from_error(&last).unwrap();
    assert!(p.next.is_none() && p.ext_u64(keys::SIZE) == Some(MAX_MESSAGE_BYTES as u64));
}
