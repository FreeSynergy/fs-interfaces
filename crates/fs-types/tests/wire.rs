//! A registry travels through `types.capnp` and comes back the same.

use capnp::message::{Builder, ReaderOptions};
use capnp::serialize;
use fs_interfaces::types_capnp::type_catalog;
use fs_types::{
    BasicType, CommandDescription, CommandKind, FieldDescription, TypeDescription, TypeRegistry,
};

fn registry() -> TypeRegistry {
    let mut reg = TypeRegistry::new();
    reg.add(
        TypeDescription::new("Person", "fs-people", "RFC 9553 Card", "person")
            .field(
                FieldDescription::new("name", BasicType::Name)
                    .required()
                    .label("person.name"),
            )
            .field(
                FieldDescription::new("emails", BasicType::Email)
                    .multiple()
                    .label("person.emails"),
            )
            .field(
                FieldDescription::new("kind", BasicType::Choice)
                    .label("person.kind")
                    .choices(&["individual", "org"]),
            )
            .field(
                FieldDescription::new("manager", BasicType::Reference)
                    .label("person.manager")
                    .target("Person"),
            )
            .command(
                CommandDescription::new("find", CommandKind::Find, "person.find")
                    .input(FieldDescription::new("term", BasicType::Text).label("person.term"))
                    .result(
                        FieldDescription::new("persons", BasicType::Reference)
                            .multiple()
                            .label("person.list")
                            .target("Person"),
                    ),
            ),
    )
    .unwrap();
    reg.add(TypeDescription::new(
        "Place", "fs-maps", "RFC 5870", "place",
    ))
    .unwrap();
    reg
}

fn answer(reg: &TypeRegistry) -> Vec<u8> {
    let mut msg = Builder::new_default();
    let results = msg.init_root::<type_catalog::describe_results::Builder<'_>>();
    let n = u32::try_from(reg.len()).unwrap();
    reg.write(results.init_types(n));
    serialize::write_message_to_words(&msg)
}

#[test]
fn registry_round_trips_through_the_wire_form() {
    let reg = registry();
    let bytes = answer(&reg);
    let msg = serialize::read_message(&mut &bytes[..], ReaderOptions::new()).unwrap();
    let back = TypeRegistry::read(
        msg.get_root::<type_catalog::describe_results::Reader<'_>>()
            .unwrap()
            .get_types()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(back.len(), 2);
    for t in reg.iter() {
        assert_eq!(back.get(&t.name), Some(t));
    }
}

#[test]
fn a_description_that_does_not_validate_is_refused_on_reading() {
    let mut bad = registry().get("Person").unwrap().clone();
    bad.fields[0].label = "Not A Key".into();
    let mut msg = Builder::new_default();
    let list = msg
        .init_root::<type_catalog::describe_results::Builder<'_>>()
        .init_types(1);
    let mut list = list;
    bad.write(list.reborrow().get(0));
    let err = TypeRegistry::read(
        msg.get_root_as_reader::<type_catalog::describe_results::Reader<'_>>()
            .unwrap()
            .get_types()
            .unwrap(),
    )
    .err()
    .unwrap();
    assert!(err.to_string().contains("bad-label"), "{err}");
}

/// A kind from a newer schema (ordinal 999) reads as `unknown`: patch every aligned u16 of the message and
/// demand that the patch that hits `basic` leaves everything else as it was.
#[test]
fn unknown_enum_values_read_as_unknown() {
    let reg = {
        let mut r = TypeRegistry::new();
        r.add(
            TypeDescription::new("Thing", "p", "s", "thing").field(
                FieldDescription::new("x", BasicType::Reference)
                    .required()
                    .label("thing.x"),
            ),
        )
        .unwrap();
        r
    };
    let bytes = answer(&reg);
    let mut hits = 0;
    for i in (0..bytes.len() - 1).step_by(2) {
        if bytes[i] != u8::try_from(BasicType::Reference as u16).unwrap() || bytes[i + 1] != 0 {
            continue;
        }
        let mut patched = bytes.clone();
        patched[i..i + 2].copy_from_slice(&999u16.to_le_bytes());
        let Ok(msg) = serialize::read_message(&mut &patched[..], ReaderOptions::new()) else {
            continue;
        };
        let Ok(root) = msg.get_root::<type_catalog::describe_results::Reader<'_>>() else {
            continue;
        };
        let Ok(list) = root.get_types() else { continue };
        let Ok(t) = TypeDescription::read(list.get(0)) else {
            continue;
        };
        if t.fields[0].basic == BasicType::Unknown
            && t.fields[0].required
            && t.fields[0].name == "x"
        {
            hits += 1;
        }
    }
    assert_eq!(hits, 1, "exactly the `basic` field turns into unknown");
}

/// What a newer program may send: a kind and a command kind this version reads as `unknown`, the field carrying a list or
/// a target of its own. The whole catalog must still be readable.
#[test]
fn a_description_from_a_newer_schema_is_still_readable() {
    let newer = TypeDescription::new("Thing", "p", "s", "thing")
        .field(
            FieldDescription::new("tint", BasicType::Unknown)
                .label("thing.tint")
                .target("Colour")
                .choices(&["red"]),
        )
        .command(CommandDescription::new(
            "odd",
            CommandKind::Unknown,
            "thing.odd",
        ));
    assert!(
        newer.validate().is_err(),
        "a program must not register this itself"
    );
    let mut msg = Builder::new_default();
    let mut list = msg
        .init_root::<type_catalog::describe_results::Builder<'_>>()
        .init_types(1);
    newer.write(list.reborrow().get(0));
    let reg = TypeRegistry::read(
        msg.get_root_as_reader::<type_catalog::describe_results::Reader<'_>>()
            .unwrap()
            .get_types()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(reg.get("Thing"), Some(&newer));
}
