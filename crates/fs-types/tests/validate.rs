//! A description is refused for each defect, by name.

mod support;

use fs_types::{
    BasicType, CommandDescription, CommandKind, DescribesType, DescriptionProblem as P,
    FieldDescription, TypeDescription,
};
use support::Contact;

fn with(f: impl FnOnce(TypeDescription) -> TypeDescription) -> Result<(), P> {
    f(Contact::describe()).validate()
}

#[test]
fn defects_are_named() {
    assert_eq!(
        with(|mut t| {
            t.name = String::new();
            t
        }),
        Err(P::EmptyName)
    );
    assert_eq!(
        with(|mut t| {
            t.name = "contact".into();
            t
        }),
        Err(P::BadName("contact".into()))
    );
    assert_eq!(
        with(|mut t| {
            t.program = String::new();
            t
        }),
        Err(P::EmptyProgram)
    );
    assert_eq!(
        with(|mut t| {
            t.label = "Contact Label".into();
            t
        }),
        Err(P::BadLabel("Contact Label".into()))
    );
    assert_eq!(
        with(|t| t.field(FieldDescription::new("name", BasicType::Text).label("x"))),
        Err(P::Duplicate("Contact.name".into()))
    );
    assert_eq!(
        with(|t| t.field(FieldDescription::new("Bad_Name", BasicType::Text).label("x"))),
        Err(P::BadName("Bad_Name".into()))
    );
    assert_eq!(
        with(|t| t.field(FieldDescription::new("pick", BasicType::Choice).label("x"))),
        Err(P::NoChoices("Contact.pick".into()))
    );
    assert_eq!(
        with(|t| t.field(
            FieldDescription::new("pick", BasicType::Text)
                .label("x")
                .choices(&["a"])
        )),
        Err(P::ChoicesOnChoice("Contact.pick".into()))
    );
    assert_eq!(
        with(|t| t.field(
            FieldDescription::new("pick", BasicType::Choice)
                .label("x")
                .choices(&["a", "a"])
        )),
        Err(P::DuplicateChoice("Contact.pick.a".into()))
    );
    assert_eq!(
        with(|t| t.field(
            FieldDescription::new("site", BasicType::Link)
                .label("x")
                .target("Contact")
        )),
        Err(P::TargetOnReference("Contact.site".into()))
    );
    assert_eq!(
        with(|t| t.command(CommandDescription::new("create", CommandKind::Create, "x"))),
        Err(P::Duplicate("Contact.create".into()))
    );
    assert_eq!(
        with(|t| t.command(CommandDescription::new("odd", CommandKind::Unknown, "x"))),
        Err(P::UnknownCommandKind("Contact.odd".into()))
    );
    assert_eq!(
        with(|t| t.command(
            CommandDescription::new("odd", CommandKind::Other, "x")
                .input(FieldDescription::new("a", BasicType::Text).label("x"))
                .input(FieldDescription::new("a", BasicType::Text).label("x"))
        )),
        Err(P::Duplicate("Contact.odd.input.a".into()))
    );
    assert_eq!(
        with(|t| t.field(FieldDescription::new("nolabel", BasicType::Text))),
        Err(P::BadLabel(String::new()))
    );
}
