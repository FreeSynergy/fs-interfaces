//! Describe a data type, register it, and check a record against it (what a generated frontend does).
//!
//! ```text
//! cargo run -p fs-types --example describe
//! ```

use std::collections::BTreeMap;

use fs_types::{
    BasicRules, BasicType, CommandDescription, CommandKind, DescribesType, FieldDescription,
    TypeDescription, TypeRegistry,
};

struct Contact;

impl DescribesType for Contact {
    fn describe() -> TypeDescription {
        TypeDescription::new("Contact", "fs-example", "RFC 9553 Card", "contact")
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
            .command(CommandDescription::new(
                "delete",
                CommandKind::Delete,
                "contact.delete",
            ))
    }
}

fn main() {
    let mut registry = TypeRegistry::new();
    registry.register::<Contact>().expect("sound description");
    let contact = registry.get("Contact").expect("registered");

    let record: BTreeMap<String, Vec<String>> = [(
        "emails".to_owned(),
        vec!["Ada@Example.org".to_owned(), "nope".to_owned()],
    )]
    .into();
    for issue in contact.check_record(&BasicRules::standard(), &record) {
        println!("{} #{}: {:?}", issue.field, issue.index, issue.problem);
    }
    let rules = BasicRules::standard();
    println!(
        "{:?}",
        contact.check_value(&rules, "emails", " Ada@Example.org ")
    );
}
