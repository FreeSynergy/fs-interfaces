//! Type descriptions: validation, registry, checking a record.

use std::collections::BTreeMap;

mod support;

use fs_types::{
    BasicRules, BasicType, CommandKind, DescribesType, DescriptionProblem as P, FieldDescription,
    RecordIssue, RecordProblem, TypeDescription, TypeProblem, TypeRegistry,
};
use support::Contact;

fn issue(field: &str, index: usize, problem: RecordProblem) -> RecordIssue {
    RecordIssue {
        field: field.to_owned(),
        index,
        problem,
    }
}

fn record(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
        .collect()
}

#[test]
fn a_sound_description_registers_and_is_found() {
    let mut reg = TypeRegistry::new();
    reg.register::<Contact>().unwrap();
    assert_eq!(reg.len(), 1);
    let t = reg.get("Contact").unwrap();
    assert_eq!(t.find_field("emails").unwrap().basic, BasicType::Email);
    assert_eq!(t.find_command("create").unwrap().kind, CommandKind::Create);
    assert!(reg.unresolved_targets().is_empty());
    assert_eq!(
        reg.register::<Contact>().err(),
        Some(P::TypeExists("Contact".into()))
    );
}

#[test]
fn unresolved_reference_targets_are_listed() {
    let mut reg = TypeRegistry::new();
    let t = TypeDescription::new("Task", "fs-test", "RFC 9253", "task").field(
        FieldDescription::new("assignee", BasicType::Reference)
            .label("task.assignee")
            .target("Person"),
    );
    reg.add(t).unwrap();
    assert_eq!(
        reg.unresolved_targets(),
        vec![("Task".into(), "assignee".into(), "Person".into())]
    );
}

#[test]
fn a_record_is_checked_by_the_rules_of_its_fields() {
    let t = Contact::describe();
    let rules = BasicRules::standard();
    let ok = record(&[
        ("name", &["Ada Lovelace"]),
        ("emails", &["ada@example.org", ""]),
        ("kind", &["org"]),
    ]);
    assert!(t.check_record(&rules, &ok).is_empty());

    let bad = record(&[
        ("emails", &["ada@example.org", "nope", "b@c.org"]),
        ("kind", &["alien", "org"]),
        ("shoe", &["42"]),
        ("manager", &["people/1"]),
    ]);
    assert_eq!(
        t.check_record(&rules, &bad),
        vec![
            issue("shoe", 0, RecordProblem::UnknownField),
            issue("name", 0, RecordProblem::Missing),
            issue(
                "emails",
                1,
                RecordProblem::Value(TypeProblem::Contact(
                    fs_interfaces::values::ValueProblem::MissingAt
                ))
            ),
            issue("kind", 0, RecordProblem::NotMultiple),
            issue("kind", 0, RecordProblem::Value(TypeProblem::NotInList)),
            issue("manager", 0, RecordProblem::Value(TypeProblem::NotAbsolute)),
        ]
    );
}

#[test]
fn one_value_can_be_checked_and_normalised_on_its_own() {
    let t = Contact::describe();
    let rules = BasicRules::standard();
    assert_eq!(
        t.check_value(&rules, "emails", " Ada@EXAMPLE.org "),
        Ok("Ada@example.org".to_owned())
    );
    assert_eq!(
        t.check_value(&rules, "colour", "red"),
        Err(RecordProblem::UnknownField)
    );
}

#[test]
fn issues_name_the_index_the_values_were_sent_at() {
    let t = Contact::describe();
    let rec = record(&[("name", &["Ada"]), ("emails", &["", "  ", "bad"])]);
    let issues = t.check_record(&BasicRules::standard(), &rec);
    assert_eq!(issues.len(), 1);
    assert_eq!((issues[0].field.as_str(), issues[0].index), ("emails", 2));
}
