//! The registry of rules reaches every kind by `BasicType` alone.

use fs_types::{BasicRules, BasicType, TypeProblem};

#[test]
fn every_known_kind_has_its_own_rule() {
    let rules = BasicRules::standard();
    for k in [
        BasicType::Text,
        BasicType::MultilineText,
        BasicType::Email,
        BasicType::Phone,
        BasicType::Name,
        BasicType::Address,
        BasicType::Place,
        BasicType::Date,
        BasicType::DateTime,
        BasicType::Language,
        BasicType::Link,
        BasicType::Image,
        BasicType::Key,
        BasicType::Choice,
        BasicType::Keywords,
        BasicType::Reference,
    ] {
        assert_eq!(rules.rule(k).kind(), k);
    }
}

#[test]
fn unknown_kind_is_plain_text() {
    let rules = BasicRules::standard();
    assert_eq!(rules.rule(BasicType::Unknown).kind(), BasicType::Text);
    assert_eq!(
        rules
            .check(BasicType::Unknown, "  anything  ", &[])
            .unwrap(),
        "anything"
    );
}

#[test]
fn check_dispatches_and_passes_the_choices() {
    let rules = BasicRules::standard();
    assert_eq!(
        rules.check(BasicType::Email, "a@b.org", &[]).unwrap(),
        "a@b.org"
    );
    assert_eq!(
        rules.check(BasicType::Date, "2024-13-01", &[]).err(),
        Some(TypeProblem::BadDate)
    );
    let choices = vec!["low".to_owned(), "high".to_owned()];
    assert_eq!(
        rules.check(BasicType::Choice, "high", &choices).unwrap(),
        "high"
    );
    assert_eq!(
        rules.check(BasicType::Choice, "mid", &choices).err(),
        Some(TypeProblem::NotInList)
    );
    assert_eq!(
        rules.suggest(BasicType::Choice, "h", &[], &choices),
        vec!["high"]
    );
    assert_eq!(
        rules.suggest(BasicType::Email, "ada", &["x@y.org"], &[]),
        vec!["ada@y.org"]
    );
}

#[test]
fn problem_codes_are_unique_translation_keys() {
    use std::collections::HashSet;
    let all = [
        TypeProblem::Empty,
        TypeProblem::TooLong,
        TypeProblem::Character,
        TypeProblem::BadLanguage,
        TypeProblem::BadUri,
        TypeProblem::NotAbsolute,
        TypeProblem::BadDate,
        TypeProblem::BadDateTime,
        TypeProblem::BadPlace,
        TypeProblem::PlaceRange,
        TypeProblem::BadFingerprint,
        TypeProblem::NotInList,
        TypeProblem::NotImage,
        TypeProblem::BadKeyword,
        TypeProblem::NoParts,
        TypeProblem::BadCountryCode,
    ];
    let codes: HashSet<&str> = all.iter().map(|p| p.code()).collect();
    assert_eq!(codes.len(), all.len());
    assert_eq!(TypeProblem::BadDate.to_string(), "problem.date.bad");
}
