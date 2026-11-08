//! The cursor of a page that continues after a place in a sort order.
//!
//! Pattern: none (two pure functions) — shared by every server that pages by a place (`tasks`,
//! `wiki`), so the byte form exists once. A client only returns what it got (`Cursor.opaque`). The
//! first byte is the form: 1 was a position in the whole list (`Page`), 2 is a place in the sort
//! key. Form 1 is not read any more: the position of an item moves when another is added or removed,
//! a place does not. A place is three parts: the sort key, the name and the id.

use fs_objects::ObjectId;

use super::problem::invalid_request;

const FORM: u8 = 2;

/// A place in a sort order: the key the order uses, the name, and the id that breaks ties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceParts {
    /// The sort key.
    pub key: String,
    /// The name.
    pub name: String,
    /// The id.
    pub id: ObjectId,
}

/// The cursor that continues after `place`.
#[must_use]
pub fn encode_place(place: &PlaceParts) -> Vec<u8> {
    let id = place.id.to_string();
    let mut out = vec![FORM];
    for part in [place.key.as_str(), place.name.as_str(), id.as_str()] {
        let len = u32::try_from(part.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(part.as_bytes());
    }
    out
}

/// The place a cursor names; empty = the start.
///
/// # Errors
/// The cursor is not one [`encode_place`] made.
pub fn decode_place(cursor: &[u8]) -> capnp::Result<Option<PlaceParts>> {
    if cursor.is_empty() {
        return Ok(None);
    }
    let invalid = || invalid_request("invalid cursor");
    let (&form, mut rest) = cursor.split_first().ok_or_else(invalid)?;
    if form != FORM {
        return Err(invalid());
    }
    let mut parts: Vec<&str> = Vec::with_capacity(3);
    for _ in 0..3 {
        let (len, tail) = rest.split_first_chunk::<4>().ok_or_else(invalid)?;
        let len = usize::try_from(u32::from_le_bytes(*len)).map_err(|_| invalid())?;
        let (part, tail) = tail.split_at_checked(len).ok_or_else(invalid)?;
        parts.push(std::str::from_utf8(part).map_err(|_| invalid())?);
        rest = tail;
    }
    if !rest.is_empty() {
        return Err(invalid());
    }
    let id: ObjectId = parts[2].parse().map_err(|_| invalid())?;
    Ok(Some(PlaceParts {
        key: parts[0].into(),
        name: parts[1].into(),
        id,
    }))
}
