//! A data type used by several test files.

use fs_types::{
    BasicType, CommandDescription, CommandKind, DescribesType, FieldDescription, TypeDescription,
};

pub struct Contact;

impl DescribesType for Contact {
    fn describe() -> TypeDescription {
        TypeDescription::new("Contact", "fs-test", "RFC 9553 Card", "contact")
            .field(
                FieldDescription::new("name", BasicType::Name)
                    .required()
                    .label("contact.name"),
            )
            .field(
                FieldDescription::new("emails", BasicType::Email)
                    .multiple()
                    .label("contact.emails"),
            )
            .field(
                FieldDescription::new("kind", BasicType::Choice)
                    .label("contact.kind")
                    .choices(&["individual", "org"]),
            )
            .field(
                FieldDescription::new("manager", BasicType::Reference)
                    .label("contact.manager")
                    .target("Contact"),
            )
            .command(
                CommandDescription::new("create", CommandKind::Create, "contact.create")
                    .input(
                        FieldDescription::new("name", BasicType::Name)
                            .required()
                            .label("contact.name"),
                    )
                    .result(
                        FieldDescription::new("id", BasicType::Reference)
                            .required()
                            .label("contact.id"),
                    ),
            )
            .command(CommandDescription::new(
                "delete",
                CommandKind::Delete,
                "contact.delete",
            ))
    }
}
