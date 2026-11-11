//! `Sort.foldersFirst` (0.0.19) is true by default: what an old client sends (no such field) reads as true.
use fs_interfaces::common_capnp;
use fs_interfaces::files_capnp as new;
use fs_interfaces::types_capnp;

#[path = "compat/files_v0_0_18_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod files_capnp;

fn bytes_of_old(descending: bool) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut sort = msg.init_root::<files_capnp::sort::Builder<'_>>();
        sort.set_by(files_capnp::SortBy::Size);
        sort.set_descending(descending);
    }
    capnp::serialize::write_message_to_words(&msg)
}

#[test]
fn an_old_sort_reads_as_folders_first() {
    let bytes = bytes_of_old(true);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let sort = reader.get_root::<new::sort::Reader<'_>>().unwrap();
    assert!(
        sort.get_folders_first(),
        "an old client must keep the folders-first order"
    );
    assert!(sort.get_descending());
    assert!(matches!(sort.get_by(), Ok(new::SortBy::Size)));
}

#[test]
fn a_new_sort_without_the_field_set_is_folders_first_and_false_roundtrips() {
    let mut msg = capnp::message::Builder::new_default();
    msg.init_root::<new::sort::Builder<'_>>();
    assert!(
        msg.get_root_as_reader::<new::sort::Reader<'_>>()
            .unwrap()
            .get_folders_first()
    );
    msg.get_root::<new::sort::Builder<'_>>()
        .unwrap()
        .set_folders_first(false);
    let bytes = capnp::serialize::write_message_to_words(&msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    assert!(
        !reader
            .get_root::<new::sort::Reader<'_>>()
            .unwrap()
            .get_folders_first()
    );
    // The old schema reads the new message as before.
    let old = reader.get_root::<files_capnp::sort::Reader<'_>>().unwrap();
    assert!(!old.get_descending());
}
