//! Structured basic types: name, address, image, choice.

use fs_types::{Address, AddressKind, BasicValue, Choice, Image, Name, NameKind, TypeProblem};

#[test]
fn name_from_components_displays_in_order() {
    let n = Name::new(
        "",
        &[
            (NameKind::Given, " Ada "),
            (NameKind::Surname, "Love  lace"),
            (NameKind::Credential, ""),
        ],
    )
    .unwrap();
    assert_eq!(n.components().len(), 2);
    assert_eq!(n.display(), "Ada Love lace");
    assert_eq!(n.clone().with_full().canonical(), "Ada Love lace");
}

#[test]
fn name_separator_attaches() {
    let n = Name::new(
        "",
        &[
            (NameKind::Surname, "Smith"),
            (NameKind::Separator, ", "),
            (NameKind::Given, "Jo"),
        ],
    )
    .unwrap();
    assert_eq!(n.display(), "Smith, Jo");
}

#[test]
fn name_typed_on_one_line_is_not_split() {
    let n = Name::parse("  Ada   Lovelace ").unwrap();
    assert_eq!(n.canonical(), "Ada Lovelace");
    assert!(n.components().is_empty());
    assert_eq!(Name::new("", &[]).err(), Some(TypeProblem::NoParts));
    assert_eq!(Name::parse("").err(), Some(TypeProblem::Empty));
    assert_eq!(Name::new("a\u{7}", &[]).err(), Some(TypeProblem::Character));
}

#[test]
fn address_keeps_the_given_full_text_and_checks_the_country() {
    let a = Address::new(
        &[
            (AddressKind::Name, "Main Street"),
            (AddressKind::Number, "1"),
            (AddressKind::Locality, "London"),
        ],
        "gb",
        "",
    )
    .unwrap();
    assert_eq!(a.country_code(), "GB");
    assert_eq!(a.display(), "Main Street, 1, London");
    let b = Address::new(
        &[(AddressKind::Locality, "London")],
        "",
        "1 Main Street\u{a0}London",
    )
    .unwrap();
    assert_eq!(b.display(), "1 Main Street London");
    assert_eq!(
        Address::new(&[(AddressKind::Locality, "X")], "GBR", "").err(),
        Some(TypeProblem::BadCountryCode)
    );
    assert_eq!(
        Address::new(&[(AddressKind::Locality, "X")], "G1", "").err(),
        Some(TypeProblem::BadCountryCode)
    );
    assert_eq!(
        Address::new(&[], "GB", " ").err(),
        Some(TypeProblem::NoParts)
    );
    assert_eq!(Address::parse("").err(), Some(TypeProblem::Empty));
}

#[test]
fn image_needs_a_link_and_an_image_media_type() {
    let i = Image::new("HTTPS://Ex.org/a.png", "Image/PNG", 64, 32).unwrap();
    assert_eq!(
        (i.uri(), i.media_type(), i.size()),
        ("https://ex.org/a.png", "image/png", (64, 32))
    );
    assert_eq!(
        Image::new("https://ex.org/a", "text/html", 0, 0).err(),
        Some(TypeProblem::NotImage)
    );
    assert_eq!(
        Image::new("https://ex.org/a", "image/", 0, 0).err(),
        Some(TypeProblem::NotImage)
    );
    assert_eq!(
        Image::new("a.png", "", 0, 0).err(),
        Some(TypeProblem::NotAbsolute)
    );
    assert_eq!(Image::parse("https://ex.org/a").unwrap().media_type(), "");
}

#[test]
fn choice_is_one_of_the_list() {
    let list: Vec<String> = ["needs-action", "in-process", "completed"]
        .map(String::from)
        .to_vec();
    assert_eq!(
        Choice::pick(" completed", &list).unwrap().as_str(),
        "completed"
    );
    assert_eq!(
        Choice::pick("done", &list).err(),
        Some(TypeProblem::NotInList)
    );
    assert_eq!(Choice::pick("", &list).err(), Some(TypeProblem::Empty));
    assert_eq!(Choice::suggest("IN", &list), vec!["in-process"]);
}

#[test]
fn name_separators_are_literal_and_a_trailing_line_end_is_no_error() {
    let n = Name::new(
        "",
        &[
            (NameKind::Surname, "Smith"),
            (NameKind::Separator, "-"),
            (NameKind::Surname, "Jones"),
            (NameKind::Given, "Ann"),
        ],
    )
    .unwrap();
    assert_eq!(n.display(), "Smith-Jones Ann");
    let n = Name::new(
        "",
        &[
            (NameKind::Given, "Ann"),
            (NameKind::Separator, " van "),
            (NameKind::Surname, "Dyck"),
        ],
    )
    .unwrap();
    assert_eq!(n.display(), "Ann van Dyck");
    assert_eq!(
        Name::parse("Ada Lovelace\n").unwrap().canonical(),
        "Ada Lovelace"
    );
}

#[test]
fn a_full_address_may_have_several_lines() {
    let a = Address::new(&[], "AT", "  Hauptstr. 1 \r\n\n1010   Wien\n").unwrap();
    assert_eq!(a.display(), "Hauptstr. 1\n1010 Wien");
    assert_eq!(
        Address::new(&[], "AT", "x\u{7}y").err(),
        Some(TypeProblem::Character)
    );
}
