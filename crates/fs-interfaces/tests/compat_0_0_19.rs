//! v0.0.19 only appends: the schema of v0.0.18 (kept as generated code in `compat/`) reads what the new one writes, and the
//! new one reads what the old wrote. Plus roundtrips of the new types.

use fs_interfaces::common_capnp;
use fs_interfaces::store_capnp as new;

#[path = "compat/store_v0_0_18_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod store_capnp;

fn new_package_bytes() -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut pkg = msg.init_root::<new::package::Builder<'_>>();
        pkg.set_name("org.freesynergy.People");
        pkg.set_kind(new::PackageKind::Program);
        pkg.set_version("0.2.0");
        pkg.set_format(new::PackageFormat::Rpm);
        pkg.set_source("ref");
        pkg.set_summary("people");
        pkg.reborrow().init_facets(1).set(0, "contacts");
        pkg.set_size(4096);
        pkg.reborrow()
            .init_released()
            .set_unix_millis(1_700_000_000_000);
        let mut rel = pkg.reborrow().init_releases(2);
        rel.reborrow().get(0).set_version("0.2.0");
        rel.reborrow().get(0).set_changes("new things");
        rel.reborrow().get(1).set_version("0.1.0");
        {
            let mut orig = pkg.reborrow().init_origin();
            orig.set_source("https://example.org/p");
            orig.set_maker("Maker");
            orig.set_maker_signature("ABCD");
            orig.set_source_code("https://example.org/p-src.tar.gz");
        }
        pkg.set_trust(new::Trust::MakerAndMirror);
        let mut prov = pkg.reborrow().init_provides(1);
        prov.reborrow().get(0).set_name("people.persons");
        prov.reborrow().get(0).set_version("1.2");
        pkg.reborrow()
            .init_requires(1)
            .get(0)
            .set_name("identity.oidc");
        pkg.reborrow()
            .init_recommends(1)
            .get(0)
            .set_name("comments");
        pkg.set_icon_sha256("00ff");
        let mut pic = pkg.reborrow().init_pictures(1);
        pic.reborrow().get(0).set_url("https://example.org/a.png");
        pic.reborrow().get(0).set_sha256("aa");
        pic.reborrow().get(0).set_license("CC0-1.0");
        pkg.reborrow().get_description().set_wiki_block("block-1");
    }
    capnp::serialize::write_message_to_words(&msg)
}

#[test]
fn old_schema_reads_a_new_package() {
    let bytes = new_package_bytes();
    // The old schema reads the same bytes: every old field intact, the new ones are simply not seen.
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let old = reader
        .get_root::<store_capnp::package::Reader<'_>>()
        .unwrap();
    assert_eq!(
        old.get_name().unwrap().to_str().unwrap(),
        "org.freesynergy.People"
    );
    assert_eq!(old.get_version().unwrap().to_str().unwrap(), "0.2.0");
    assert_eq!(old.get_summary().unwrap().to_str().unwrap(), "people");
    assert_eq!(
        old.get_facets().unwrap().get(0).unwrap().to_str().unwrap(),
        "contacts"
    );
    assert!(matches!(
        old.get_format(),
        Ok(store_capnp::PackageFormat::Rpm)
    ));
    assert!(matches!(
        old.get_kind(),
        Ok(store_capnp::PackageKind::Program)
    ));
}

#[test]
fn new_schema_reads_the_new_fields_back() {
    let bytes = new_package_bytes();
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let newer = reader.get_root::<new::package::Reader<'_>>().unwrap();
    assert_eq!(newer.get_size(), 4096);
    assert_eq!(
        newer.get_released().unwrap().get_unix_millis(),
        1_700_000_000_000
    );
    assert_eq!(newer.get_releases().unwrap().len(), 2);
    assert_eq!(
        newer
            .get_releases()
            .unwrap()
            .get(0)
            .get_changes()
            .unwrap()
            .to_str()
            .unwrap(),
        "new things"
    );
    let orig = newer.get_origin().unwrap();
    assert_eq!(orig.get_maker().unwrap().to_str().unwrap(), "Maker");
    assert_eq!(
        orig.get_source_code().unwrap().to_str().unwrap(),
        "https://example.org/p-src.tar.gz"
    );
    assert!(matches!(newer.get_trust(), Ok(new::Trust::MakerAndMirror)));
    assert_eq!(
        newer
            .get_provides()
            .unwrap()
            .get(0)
            .get_version()
            .unwrap()
            .to_str()
            .unwrap(),
        "1.2"
    );
    assert_eq!(
        newer
            .get_requires()
            .unwrap()
            .get(0)
            .get_name()
            .unwrap()
            .to_str()
            .unwrap(),
        "identity.oidc"
    );
    assert_eq!(newer.get_recommends().unwrap().len(), 1);
    assert_eq!(newer.get_icon_sha256().unwrap().to_str().unwrap(), "00ff");
    let pic = newer.get_pictures().unwrap().get(0);
    assert_eq!(pic.get_license().unwrap().to_str().unwrap(), "CC0-1.0");
    assert!(matches!(
        newer.get_description().which().unwrap(),
        new::package::description::WikiBlock(Ok(b)) if b.to_str().unwrap() == "block-1"
    ));
}

#[test]
fn new_schema_reads_an_old_package_with_defaults() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut pkg = msg.init_root::<store_capnp::package::Builder<'_>>();
        pkg.set_name("old");
        pkg.set_version("0.1.0");
        pkg.set_format(store_capnp::PackageFormat::TarGz);
    }
    let bytes = capnp::serialize::write_message_to_words(&msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let newer = reader.get_root::<new::package::Reader<'_>>().unwrap();
    assert_eq!(newer.get_name().unwrap().to_str().unwrap(), "old");
    assert_eq!(newer.get_size(), 0);
    assert_eq!(newer.get_released().unwrap().get_unix_millis(), 0);
    assert_eq!(newer.get_releases().unwrap().len(), 0);
    assert_eq!(newer.get_provides().unwrap().len(), 0);
    assert!(matches!(newer.get_trust(), Ok(new::Trust::Unknown)));
    assert_eq!(newer.get_icon_sha256().unwrap().to_str().unwrap(), "");
    assert!(matches!(
        newer.get_description().which().unwrap(),
        new::package::description::Text(Ok(t)) if t.to_str().unwrap().is_empty()
    ));
}

#[test]
fn readiness_report_roundtrips_every_state() {
    use fs_interfaces::tasks_capnp::{Readiness, readiness_report};
    for state in [
        Readiness::Ready,
        Readiness::Blocked,
        Readiness::Done,
        Readiness::Unknown,
        Readiness::DependencyCancelled,
        Readiness::Conflict,
    ] {
        let mut msg = capnp::message::Builder::new_default();
        {
            let mut reader = msg.init_root::<readiness_report::Builder<'_>>();
            reader.set_readiness(state);
            let mut by = reader.init_by(2);
            by.set(0, "a");
            by.set(1, "b");
        }
        let reader = msg
            .get_root_as_reader::<readiness_report::Reader<'_>>()
            .unwrap();
        assert_eq!(reader.get_readiness().unwrap(), state);
        assert_eq!(reader.get_by().unwrap().len(), 2);
    }
}

#[test]
fn comment_roundtrips_as_a_note_and_as_a_tombstone() {
    use fs_interfaces::comments_capnp::comment;
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut comm = msg.init_root::<comment::Builder<'_>>();
        let mut env = comm.reborrow().init_envelope();
        env.set_type("Note");
        env.set_context("https://x.example/tasks/1");
        env.set_in_reply_to("https://x.example/comments/0");
        env.set_attributed_to("https://x.example/actors/anna");
        comm.set_body("hello");
    }
    let comm = msg.get_root_as_reader::<comment::Reader<'_>>().unwrap();
    let env = comm.get_envelope().unwrap();
    assert_eq!(env.get_type().unwrap().to_str().unwrap(), "Note");
    assert_eq!(
        env.get_attributed_to().unwrap().to_str().unwrap(),
        "https://x.example/actors/anna"
    );
    assert!(!comm.get_tombstone());

    let mut msg = capnp::message::Builder::new_default();
    {
        let mut comm = msg.init_root::<comment::Builder<'_>>();
        comm.reborrow().init_envelope().set_type("Tombstone");
        comm.set_tombstone(true);
        comm.reborrow().init_deleted().set_unix_millis(5);
    }
    let comm = msg.get_root_as_reader::<comment::Reader<'_>>().unwrap();
    assert!(comm.get_tombstone());
    assert_eq!(comm.get_body().unwrap().to_str().unwrap(), "");
}
