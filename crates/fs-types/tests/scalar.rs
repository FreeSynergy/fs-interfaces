//! Single-valued basic types: check, normalise, suggest.

use fs_types::{
    BasicValue, Date, DateTime, EmailAddress, Key, Keywords, Language, Link, MultilineText,
    PhoneNumber, Place, Reference, Text, TypeProblem,
};

fn canon<T: BasicValue>(typed: &str) -> String {
    T::parse(typed).unwrap().canonical()
}

fn bad<T: BasicValue>(typed: &str) -> TypeProblem {
    T::parse(typed).err().expect("should be refused")
}

#[test]
fn text_is_trimmed_composed_and_one_line() {
    assert_eq!(canon::<Text>("  Ame\u{301}lie  "), "Am\u{e9}lie");
    assert_eq!(bad::<Text>("   "), TypeProblem::Empty);
    assert_eq!(bad::<Text>("a\nb"), TypeProblem::Character);
    assert_eq!(bad::<Text>(&"x".repeat(4097)), TypeProblem::TooLong);
}

#[test]
fn multiline_text_unifies_line_ends() {
    assert_eq!(
        canon::<MultilineText>("\r\nline one  \r\nline two\r\n\r\n"),
        "line one\nline two"
    );
    assert_eq!(bad::<MultilineText>("\n \n"), TypeProblem::Empty);
    assert_eq!(bad::<MultilineText>("a\u{0}b"), TypeProblem::Character);
}

#[test]
fn email_goes_through_the_shared_value_and_keeps_its_code() {
    assert_eq!(
        canon::<EmailAddress>(" Ada@Example.org "),
        "Ada@example.org"
    );
    assert_eq!(
        bad::<EmailAddress>("ada.example.org"),
        TypeProblem::Contact(fs_interfaces::values::ValueProblem::MissingAt)
    );
    assert_eq!(bad::<EmailAddress>(""), TypeProblem::Empty);
    assert_eq!(
        bad::<EmailAddress>("ada@example").code(),
        "email.domain-without-dot"
    );
}

#[test]
fn email_domains_are_suggested_by_frequency() {
    let known = ["a@work.org", "b@work.org", "c@home.net", "d@work.org"];
    assert_eq!(
        EmailAddress::suggest("ada", &known),
        vec!["ada@work.org", "ada@home.net"]
    );
    assert_eq!(EmailAddress::suggest("ada@h", &known), vec!["ada@home.net"]);
    assert!(EmailAddress::suggest("", &known).is_empty());
    assert!(EmailAddress::suggest("a@b@c", &known).is_empty());
}

#[test]
fn phone_is_e164_and_has_no_suggestion() {
    assert_eq!(canon::<PhoneNumber>("+49 (30) 123-456"), "+4930123456");
    assert_eq!(
        bad::<PhoneNumber>("030 123456").code(),
        "phone.no-country-code"
    );
    assert!(PhoneNumber::suggest("+49", &["+4930123456"]).is_empty());
}

#[test]
fn language_tags_get_their_case_and_are_suggested() {
    assert_eq!(canon::<Language>("PT-br"), "pt-BR");
    assert_eq!(canon::<Language>("zh-hant-tw"), "zh-Hant-TW");
    assert_eq!(bad::<Language>("de_DE"), TypeProblem::BadLanguage);
    assert_eq!(bad::<Language>("toolongsubtag99"), TypeProblem::BadLanguage);
    assert_eq!(
        Language::suggest("p", &["de", "pt-BR", "pt", "en"]),
        vec!["pt", "pt-BR"]
    );
}

#[test]
fn link_is_absolute_and_normalised() {
    assert_eq!(
        canon::<Link>("HTTP://Example.COM/a/./b/../c"),
        "http://example.com/a/c"
    );
    assert_eq!(
        canon::<Link>("mailto:ada@example.org"),
        "mailto:ada@example.org"
    );
    assert_eq!(
        canon::<Link>("https://b\u{fc}cher.example/\u{e4}"),
        "https://b\u{fc}cher.example/\u{e4}"
    );
    assert_eq!(bad::<Link>("example.org/x"), TypeProblem::NotAbsolute);
    assert_eq!(bad::<Link>("http://exa mple.org"), TypeProblem::BadUri);
    assert_eq!(
        Link::suggest("example.org/x", &[]),
        vec!["https://example.org/x"]
    );
    assert!(Link::suggest("hello", &[]).is_empty());
}

#[test]
fn reference_is_kept_as_written() {
    let id = "https://ex.org/people/0190a1b2";
    assert_eq!(canon::<Reference>(&format!(" {id} ")), id);
    assert_eq!(
        canon::<Reference>("HTTPS://EX.org/Path"),
        "HTTPS://EX.org/Path"
    );
    assert_eq!(bad::<Reference>("people/1"), TypeProblem::NotAbsolute);
    assert_eq!(
        Reference::suggest("0190", &[id, "https://ex.org/other"]),
        vec![id]
    );
}

#[test]
fn date_must_exist_and_be_padded() {
    assert_eq!(canon::<Date>("2024-02-29"), "2024-02-29");
    assert_eq!(bad::<Date>("2023-02-29"), TypeProblem::BadDate);
    assert_eq!(bad::<Date>("2024-2-9"), TypeProblem::BadDate);
    assert_eq!(bad::<Date>("09.02.2024"), TypeProblem::BadDate);
    assert_eq!(
        Date::parse("2024-02-29").unwrap().as_naive().to_string(),
        "2024-02-29"
    );
}

#[test]
fn date_time_needs_an_offset_and_prints_z_for_utc() {
    assert_eq!(
        canon::<DateTime>("2026-11-10t09:30:00+00:00"),
        "2026-11-10T09:30:00Z"
    );
    assert_eq!(
        canon::<DateTime>("2026-11-10T09:30:00.250+01:00"),
        "2026-11-10T09:30:00.250+01:00"
    );
    assert_eq!(
        bad::<DateTime>("2026-11-10T09:30:00"),
        TypeProblem::BadDateTime
    );
    assert_eq!(
        bad::<DateTime>("2026-13-10T09:30:00Z"),
        TypeProblem::BadDateTime
    );
}

#[test]
fn place_is_a_wgs84_geo_uri_in_range() {
    let p = Place::parse(" geo:48.2010,16.3695 ").unwrap();
    assert!((p.latitude() - 48.2010).abs() < 1e-9 && (p.longitude() - 16.3695).abs() < 1e-9);
    assert!(canon::<Place>("geo:48.2010,16.3695,171;u=20").starts_with("geo:48.201,16.3695,171"));
    assert_eq!(bad::<Place>("geo:91,10"), TypeProblem::PlaceRange);
    assert_eq!(bad::<Place>("geo:10,181"), TypeProblem::PlaceRange);
    assert_eq!(bad::<Place>("48.2,16.3"), TypeProblem::BadPlace);
    assert_eq!(
        bad::<Place>("geo:48.2,16.3;crs=mars"),
        TypeProblem::BadPlace
    );
}

#[test]
fn key_is_a_bare_upper_case_fingerprint() {
    let v4 = "0123 4567 89ab cdef 0123 4567 89AB CDEF 0123 4567";
    assert_eq!(canon::<Key>(v4), "0123456789ABCDEF0123456789ABCDEF01234567");
    assert_eq!(
        canon::<Key>(&format!("openpgp4fpr:{}", "ab".repeat(20))),
        "AB".repeat(20)
    );
    assert_eq!(canon::<Key>(&"cd".repeat(32)).len(), 64);
    assert_eq!(
        Key::parse(&"ab".repeat(20)).unwrap().grouped(),
        "ABAB ABAB ABAB ABAB ABAB ABAB ABAB ABAB ABAB ABAB"
    );
    assert_eq!(bad::<Key>(&"ab".repeat(19)), TypeProblem::BadFingerprint);
    assert_eq!(bad::<Key>(&"zz".repeat(20)), TypeProblem::BadFingerprint);
}

#[test]
fn keywords_are_a_sorted_lower_case_set() {
    assert_eq!(
        canon::<Keywords>("Family, friend ,, FAMILY, Club"),
        "club, family, friend"
    );
    assert_eq!(Keywords::parse("a,b,a").unwrap().len(), 2);
    assert_eq!(bad::<Keywords>(" , "), TypeProblem::Empty);
    assert_eq!(bad::<Keywords>(&"k".repeat(65)), TypeProblem::BadKeyword);
    let known = ["family", "friend", "fire"];
    assert_eq!(Keywords::suggest("club, fr", &known), vec!["club, friend"]);
    assert_eq!(
        Keywords::suggest("family, f", &known),
        vec!["family, fire", "family, friend"]
    );
}
