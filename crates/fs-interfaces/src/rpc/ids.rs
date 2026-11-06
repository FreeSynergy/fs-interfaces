//! Ids and timestamps on the wire.
//!
//! Pattern: Adapter — UUIDs, object ids and milliseconds between Rust and `common.capnp`. An
//! all-zero `Uuid` means "none".

use chrono::{DateTime, TimeZone, Utc};
use fs_store::ObjectId;
use uuid::Uuid;

use crate::common_capnp::uuid as wire;

/// Write a uuid (`None` = all zero).
pub fn write_uuid(mut out: wire::Builder<'_>, id: Option<Uuid>) {
    let bits = id.map_or(0, |u| u.as_u128());
    out.set_hi(u64::try_from(bits >> 64).unwrap_or(0));
    out.set_lo(u64::try_from(bits & u128::from(u64::MAX)).unwrap_or(0));
}

/// Read a uuid (all zero = `None`).
#[must_use]
pub fn read_uuid(r: wire::Reader<'_>) -> Option<Uuid> {
    let bits = (u128::from(r.get_hi()) << 64) | u128::from(r.get_lo());
    (bits != 0).then(|| Uuid::from_u128(bits))
}

/// Write an object id.
pub fn write_object_id(out: wire::Builder<'_>, id: ObjectId) {
    write_uuid(out, Some(id.as_uuid()));
}

/// Read an object id; none or not a valid id is an error.
///
/// # Errors
/// The id is all zero or not a valid object id.
pub fn read_object_id(r: wire::Reader<'_>) -> capnp::Result<ObjectId> {
    let id = read_uuid(r).ok_or_else(|| capnp::Error::failed("id is missing".into()))?;
    ObjectId::from_uuid(id).map_err(super::failed)
}

/// `DateTime` -> unix milliseconds.
#[must_use]
pub fn to_millis(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

/// Unix milliseconds -> `DateTime`; out of range is an error, not a silent epoch.
///
/// # Errors
/// The value is outside the range of `DateTime`.
pub fn from_millis(v: i64) -> capnp::Result<DateTime<Utc>> {
    Utc.timestamp_millis_opt(v)
        .single()
        .ok_or_else(|| capnp::Error::failed(format!("timestamp {v} out of range")))
}
