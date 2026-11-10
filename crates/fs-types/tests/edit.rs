//! Changes to a record, field by field (rules decided 2026-11-10).

mod support;

use std::collections::BTreeMap;

use fs_interfaces::types_capnp::type_catalog;
use fs_types::{
    BasicRules, BasicType, CommandDescription, CommandKind, DescribesType, Edit, FieldDescription,
    RecordProblem, TypeDescription, TypeRegistry,
};
use support::Contact;

fn record() -> BTreeMap<String, Vec<String>> {
    [
        ("name", &["Ada"][..]),
        ("emails", &["a@x.org", "b@x.org"]),
        ("kind", &["org"]),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
    .collect()
}

#[test]
fn missing_stays_empty_clears_and_values_replace() {
    let desc = Contact::describe();
    let edit = Edit::new()
        .set(&["emails"], BasicType::Email, &[" C@X.org "])
        .clear(&["kind"], BasicType::Choice);
    let edit = edit.check(&desc, &BasicRules::standard()).unwrap();
    assert_eq!(edit.fields[0].values, vec!["C@x.org"]);
    let mut r = record();
    edit.apply(&mut r);
    assert_eq!(r["name"], vec!["Ada"], "not in the edit: unchanged");
    assert_eq!(r["emails"], vec!["C@x.org"], "replaced");
    assert!(!r.contains_key("kind"), "no values: cleared");
}

#[test]
fn findings_are_by_path() {
    let desc = Contact::describe();
    let edit = Edit::new()
        .set(&["name"], BasicType::Email, &["a@b.org"])
        .clear(&["name"], BasicType::Name)
        .set(&["kind"], BasicType::Choice, &["org", "alien"])
        .set(&["emails", "k1", "address"], BasicType::Email, &["x@y.org"])
        .set(&["shoe"], BasicType::Text, &["42"])
        .set(&["emails"], BasicType::Email, &["nope"]);
    let issues = edit.check(&desc, &BasicRules::standard()).unwrap_err();
    let got: Vec<(&str, &RecordProblem)> = issues
        .iter()
        .map(|i| (i.field.as_str(), &i.problem))
        .collect();
    assert!(got.contains(&("name", &RecordProblem::WrongType)));
    assert!(got.contains(&("name", &RecordProblem::DuplicatePath)));
    assert!(got.contains(&("kind", &RecordProblem::NotMultiple)));
    assert!(got.contains(&("emails/k1/address", &RecordProblem::NestedPath)));
    assert!(got.contains(&("shoe", &RecordProblem::UnknownField)));
    assert!(
        issues
            .iter()
            .any(|i| i.field == "emails" && matches!(i.problem, RecordProblem::Value(_)))
    );
}

#[test]
fn clearing_a_required_field_is_refused() {
    let issues = Edit::new()
        .clear(&["name"], BasicType::Name)
        .check(&Contact::describe(), &BasicRules::standard())
        .unwrap_err();
    assert_eq!(issues[0].problem, RecordProblem::Missing);
}

#[test]
fn pointer_escapes_like_rfc_6901() {
    let e = Edit::new().set(&["emails", "a/b~c", "address"], BasicType::Email, &[]);
    assert_eq!(e.fields[0].pointer(), "emails/a~1b~0c/address");
}

#[test]
fn an_edit_travels_through_the_wire_form() {
    let e = Edit::new()
        .set(&["emails"], BasicType::Email, &["a@x.org", "b@x.org"])
        .clear(&["kind"], BasicType::Choice);
    let mut msg = capnp::message::Builder::new_default();
    e.write(msg.init_root::<fs_interfaces::types_capnp::edit::Builder<'_>>());
    let back = Edit::read(
        msg.get_root_as_reader::<fs_interfaces::types_capnp::edit::Reader<'_>>()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(back, e);
}

fn typed() -> TypeDescription {
    TypeDescription::new("Task", "fs-test", "RFC 9253", "task")
        .field(
            FieldDescription::new("title", BasicType::Text)
                .required()
                .label("task.title"),
        )
        .command(CommandDescription::new(
            "create",
            CommandKind::Create,
            "task.create",
        ))
        .command(CommandDescription::standard_find("task.find", "Task"))
        .command(CommandDescription::new(
            "change",
            CommandKind::Change,
            "task.change",
        ))
        .command(CommandDescription::new(
            "delete",
            CommandKind::Delete,
            "task.delete",
        ))
        .command(
            CommandDescription::new("complete", CommandKind::Other, "task.complete").input(
                FieldDescription::new("task", BasicType::Reference)
                    .required()
                    .label("task.ref")
                    .target("Task"),
            ),
        )
        .command(
            CommandDescription::new("assign", CommandKind::Other, "task.assign").input(
                FieldDescription::new("who", BasicType::Reference)
                    .label("task.who")
                    .target("Person"),
            ),
        )
        .command(CommandDescription::new("merge", CommandKind::Other, "task.merge").unsupported())
}

#[test]
fn record_commands_are_the_ones_that_take_the_type_itself_plus_change_and_delete() {
    let t = typed();
    t.validate().unwrap();
    let names = |v: Vec<&CommandDescription>| v.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(t.record_commands()), ["change", "delete", "complete"]);
    assert_eq!(
        names(t.type_commands()),
        ["create", "find", "assign", "merge"]
    );
}

#[test]
fn the_standard_find_is_the_list_and_change_takes_no_input_fields() {
    let find = CommandDescription::standard_find("x.find", "Task");
    assert_eq!(
        (
            find.kind,
            find.input[0].name.as_str(),
            find.result[0].multiple
        ),
        (CommandKind::Find, "term", true)
    );
    let bad = typed().command(
        CommandDescription::new("edit", CommandKind::Change, "task.edit")
            .input(FieldDescription::new("a", BasicType::Text).label("x")),
    );
    assert_eq!(bad.validate().unwrap_err().code(), "change-with-input");
}

#[test]
fn supported_travels_and_defaults_to_true_in_the_builder() {
    let mut reg = TypeRegistry::new();
    reg.add(typed()).unwrap();
    let mut msg = capnp::message::Builder::new_default();
    let list = msg
        .init_root::<type_catalog::describe_results::Builder<'_>>()
        .init_types(1);
    reg.write(list);
    let back = TypeRegistry::read(
        msg.get_root_as_reader::<type_catalog::describe_results::Reader<'_>>()
            .unwrap()
            .get_types()
            .unwrap(),
    )
    .unwrap();
    let t = back.get("Task").unwrap();
    assert!(!t.find_command("merge").unwrap().supported);
    assert!(t.find_command("create").unwrap().supported);
}

#[test]
fn empty_texts_in_an_edit_count_as_not_given() {
    let desc = Contact::describe();
    let rules = BasicRules::standard();
    let cleared = Edit::new()
        .set(&["kind"], BasicType::Choice, &[" "])
        .check(&desc, &rules)
        .unwrap();
    assert!(cleared.fields[0].values.is_empty());
    let kept = Edit::new()
        .set(&["emails"], BasicType::Email, &["a@x.org", ""])
        .check(&desc, &rules)
        .unwrap();
    assert_eq!(kept.fields[0].values, vec!["a@x.org"]);
}

#[test]
fn a_read_only_field_is_shown_but_not_edited_and_travels_on_the_wire() {
    let t = TypeDescription::new("Card", "fs-test", "RFC 9553", "card")
        .field(
            FieldDescription::new("uid", BasicType::Text)
                .label("card.uid")
                .read_only(),
        )
        .field(FieldDescription::new("note", BasicType::Text).label("card.note"));
    let issues = Edit::new()
        .set(&["uid"], BasicType::Text, &["x"])
        .check(&t, &BasicRules::standard())
        .unwrap_err();
    assert_eq!(issues[0].problem, RecordProblem::ReadOnly);
    assert!(
        Edit::new()
            .set(&["note"], BasicType::Text, &["x"])
            .check(&t, &BasicRules::standard())
            .is_ok()
    );
    let mut reg = TypeRegistry::new();
    reg.add(t).unwrap();
    let mut msg = capnp::message::Builder::new_default();
    reg.write(
        msg.init_root::<type_catalog::describe_results::Builder<'_>>()
            .init_types(1),
    );
    let back = TypeRegistry::read(
        msg.get_root_as_reader::<type_catalog::describe_results::Reader<'_>>()
            .unwrap()
            .get_types()
            .unwrap(),
    )
    .unwrap();
    let card = back.get("Card").unwrap();
    assert!(
        card.find_field("uid").unwrap().read_only && !card.find_field("note").unwrap().read_only
    );
}
