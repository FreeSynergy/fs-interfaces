//! v0.0.20 appends `PackageKind::Server @4`: the schema of v0.0.19 (kept as generated code in `compat/`) reads the new value
//! as "not in my schema" (no crash, the rest of the package is intact), and the new schema reads every old value unchanged.

#[allow(unused_imports)] // the frozen generated code below refers to `crate::common_capnp`
use fs_interfaces::common_capnp;
use fs_interfaces::store_capnp as new;

#[path = "compat/store_v0_0_19_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod store_capnp;

fn bytes_with_new_kind(kind: new::PackageKind) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut pkg = msg.init_root::<new::package::Builder<'_>>();
        pkg.set_name("org.freesynergy.Kanidm");
        pkg.set_version("1.12.1");
        pkg.set_kind(kind);
    }
    capnp::serialize::write_message_to_words(&msg)
}

fn reader_of(bytes: &[u8]) -> capnp::message::Reader<impl capnp::message::ReaderSegments + '_> {
    capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap()
}

#[test]
fn the_new_schema_roundtrips_server() {
    let bytes = bytes_with_new_kind(new::PackageKind::Server);
    let reader = reader_of(&bytes);
    let pkg = reader.get_root::<new::package::Reader<'_>>().unwrap();
    assert_eq!(pkg.get_kind().unwrap(), new::PackageKind::Server);
    assert_eq!(u16::from(new::PackageKind::Server), 4);
}

#[test]
fn the_old_schema_reads_server_as_unknown_and_the_rest_intact() {
    let bytes = bytes_with_new_kind(new::PackageKind::Server);
    let reader = reader_of(&bytes);
    let old = reader
        .get_root::<store_capnp::package::Reader<'_>>()
        .unwrap();
    assert!(matches!(old.get_kind(), Err(capnp::NotInSchema(4))));
    assert_eq!(
        old.get_name().unwrap().to_str().unwrap(),
        "org.freesynergy.Kanidm"
    );
    assert_eq!(old.get_version().unwrap().to_str().unwrap(), "1.12.1");
}

#[test]
fn the_new_schema_reads_every_old_kind_unchanged() {
    for (old_kind, expected) in [
        (store_capnp::PackageKind::Program, new::PackageKind::Program),
        (
            store_capnp::PackageKind::Frontend,
            new::PackageKind::Frontend,
        ),
        (
            store_capnp::PackageKind::Component,
            new::PackageKind::Component,
        ),
        (store_capnp::PackageKind::Theme, new::PackageKind::Theme),
    ] {
        let mut msg = capnp::message::Builder::new_default();
        msg.init_root::<store_capnp::package::Builder<'_>>()
            .set_kind(old_kind);
        let bytes = capnp::serialize::write_message_to_words(&msg);
        let reader = reader_of(&bytes);
        let pkg = reader.get_root::<new::package::Reader<'_>>().unwrap();
        assert_eq!(pkg.get_kind().unwrap(), expected);
    }
}
