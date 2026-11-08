//! The place cursor: a place comes back as it went, and nothing else does.
#![cfg(feature = "rpc")]

use fs_interfaces::rpc::{PlaceParts, decode_place, encode_place};

#[test]
fn a_place_comes_back_as_it_went_and_nothing_else_does() {
    let place = PlaceParts {
        key: "2026-12-01T09:00:00Z".into(),
        name: "Äpfel \u{0}\u{1} mixed Case".into(),
        id: fs_objects::Envelope::new("Task", "x").id,
    };
    let bytes = encode_place(&place);
    assert_eq!(decode_place(&bytes).unwrap(), Some(place));
    assert_eq!(decode_place(&[]).unwrap(), None);
    for bad in [
        &bytes[..bytes.len() - 1],
        &[1, 0, 0, 0, 0, 0, 0, 0, 0],
        b"xx",
    ] {
        assert!(decode_place(bad).is_err(), "{bad:?}");
    }
    let mut longer = bytes;
    longer.push(0);
    assert!(decode_place(&longer).is_err());
}
