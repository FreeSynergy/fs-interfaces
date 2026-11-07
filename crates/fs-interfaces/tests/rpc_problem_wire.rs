//! The text of a `Problem` in an error: the format of 0.36.0 is read as before, and a reader of
//! 0.36.0 still reads what is written now. Broken extensions cost only themselves.
#![cfg(feature = "rpc")]

use capnp::Error;
use fs_interfaces::rpc::Problem;

/// Text as 0.36.0 wrote it, with a cursor (`01 02 ff`). Frozen: do not edit.
const V036_WITH_NEXT: &str = "fs-problem\u{1f}413\u{1f}https://freesynergy.net/problems/too-large\u{1f}Too large\u{1f}one item\u{1f}0102ff";
/// Text as 0.36.0 wrote it, without a cursor. Frozen: do not edit.
const V036_PLAIN: &str = "fs-problem\u{1f}409\u{1f}https://freesynergy.net/problems/thing/conflict\u{1f}Conflict\u{1f}name taken";

/// The reader of 0.36.0, copied: `next` is the fifth part, everything after the detail.
/// What the old reader finds: status, type, title, detail, cursor.
type Old = (u16, String, String, String, Option<Vec<u8>>);

fn old_reader(text: &str) -> Option<Old> {
    let rest = text.strip_prefix("fs-problem\u{1f}")?;
    let mut parts = rest.splitn(5, '\u{1f}');
    let status = parts.next()?.parse().ok()?;
    let kind = parts.next()?.to_string();
    let title = parts.next()?.to_string();
    let detail = parts.next()?.to_string();
    let next = parts.next().and_then(|t| {
        if !t.is_ascii() || t.len() % 2 != 0 {
            return None;
        }
        (0..t.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
            .collect()
    });
    Some((status, kind, title, detail, next))
}

fn err(text: &str) -> Error {
    Error::failed(text.to_string())
}

#[test]
fn text_of_0_36_0_reads_as_before() {
    let p = Problem::from_error(&err(V036_WITH_NEXT)).expect("problem");
    assert_eq!((p.status, p.next.clone()), (413, Some(vec![1, 2, 255])));
    assert_eq!(
        (p.title.as_str(), p.detail.as_str()),
        ("Too large", "one item")
    );
    assert!(p.extensions.is_empty());
    let q = Problem::from_error(&err(V036_PLAIN)).expect("problem");
    assert_eq!((q.status, q.next, q.extensions.len()), (409, None, 0));
}

#[test]
fn a_problem_without_extensions_writes_the_same_bytes_as_0_36_0() {
    let p = Problem::new(
        "https://freesynergy.net/problems/thing/conflict",
        "Conflict",
        "name taken",
        409,
    );
    assert_eq!(p.into_error().extra, V036_PLAIN);
    let q = Problem::new(
        "https://freesynergy.net/problems/too-large",
        "Too large",
        "one item",
        413,
    )
    .with_next(Some(vec![1, 2, 255]));
    assert_eq!(q.into_error().extra, V036_WITH_NEXT);
}

#[test]
fn an_old_reader_still_reads_a_text_with_extensions() {
    let with_ext = Problem::new("t", "T", "d", 422)
        .with_ext("issue", "a")
        .into_error();
    let (status, kind, title, detail, next) = old_reader(&with_ext.extra).expect("old reader");
    assert_eq!(
        (status, kind.as_str(), title.as_str(), detail.as_str()),
        (422, "t", "T", "d")
    );
    assert_eq!(next, None);
}

#[test]
fn an_old_reader_loses_only_the_cursor_when_both_are_present() {
    let both = Problem::new("t", "T", "d", 413)
        .with_next(Some(vec![7]))
        .with_ext("size", "1")
        .into_error();
    let (status, _, _, detail, next) = old_reader(&both.extra).expect("old reader");
    assert_eq!((status, detail.as_str(), next), (413, "d", None));
    // The new reader gets both.
    let p = Problem::from_error(&both).expect("problem");
    assert_eq!(
        (p.next.clone(), p.ext_u64("size")),
        (Some(vec![7]), Some(1))
    );
}

#[test]
fn a_broken_extension_does_not_hide_the_rest() {
    let text = format!(
        "{V036_PLAIN}\u{1f}ext\u{1e}no-value\u{1f}ext\u{1e}\u{1e}empty-key\u{1f}junk\u{1f}ext\u{1e}ok\u{1e}yes\u{1f}ext\u{1e}"
    );
    let p = Problem::from_error(&err(&text)).expect("problem");
    assert_eq!(p.status, 409);
    assert_eq!(p.extensions, [("ok".to_string(), "yes".to_string())]);
    assert_eq!(p.next, None);
}

#[test]
fn a_broken_cursor_does_not_hide_the_extensions() {
    let text = format!("{V036_PLAIN}\u{1f}zz\u{1f}ext\u{1e}k\u{1e}v");
    let p = Problem::from_error(&err(&text)).expect("problem");
    assert_eq!((p.next.clone(), p.ext("k")), (None, Some("v")));
}

#[test]
fn extensions_after_a_cursor_and_the_rpc_prefix_are_read() {
    let text = format!("remote exception: {V036_WITH_NEXT}\u{1f}ext\u{1e}issue\u{1e}x");
    let p = Problem::from_error(&err(&text)).expect("problem");
    assert_eq!(
        (p.next.clone(), p.ext("issue")),
        (Some(vec![1, 2, 255]), Some("x"))
    );
}

#[test]
fn a_reader_does_not_trust_the_count() {
    let mut text = V036_PLAIN.to_string();
    for i in 0..500 {
        text.push_str(&format!("\u{1f}ext\u{1e}k{i}\u{1e}v"));
    }
    let p = Problem::from_error(&err(&text)).expect("problem");
    assert_eq!(p.extensions.len(), fs_interfaces::rpc::MAX_EXTENSIONS);
}

#[test]
fn status_501_keeps_the_kind_unimplemented_with_extensions() {
    let e = Problem::new("t", "T", "d", 501)
        .with_ext("kind", "x")
        .into_error();
    assert!(fs_interfaces::rpc::is_unimplemented(&e));
    assert_eq!(
        Problem::from_error(&e).and_then(|p| p.ext("kind").map(str::to_string)),
        Some("x".into())
    );
}

#[test]
fn a_record_separator_in_the_head_fields_is_kept_like_in_0_36_0() {
    let text = "fs-problem\u{1f}422\u{1f}t\u{1e}x\u{1f}Ti\u{1e}tle\u{1f}de\u{1e}tail";
    let p = Problem::from_error(&err(text)).expect("problem");
    assert_eq!(p.detail, "de\u{1e}tail");
    assert_eq!(
        p.into_error().extra,
        text,
        "byte-equal to what 0.36.0 wrote"
    );
}
