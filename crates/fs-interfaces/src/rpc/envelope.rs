//! The typed object envelope on the wire.
//!
//! Pattern: Adapter — `fs_store::Envelope` <-> `Envelope` in `common.capnp`. Empty text and an absent
//! timestamp mean "none".

use fs_store::Envelope;

use super::ids::{from_millis, read_object_id, to_millis, write_object_id};
use crate::common_capnp::envelope as wire;

fn set_list(mut out: capnp::text_list::Builder<'_>, items: &[String]) {
    for (i, s) in (0u32..).zip(items) {
        out.set(i, s.as_str());
    }
}

fn len(items: &[String]) -> u32 {
    u32::try_from(items.len()).unwrap_or(u32::MAX)
}

fn list(r: capnp::text_list::Reader<'_>) -> capnp::Result<Vec<String>> {
    r.iter().map(|t| Ok(t?.to_string()?)).collect()
}

fn optional(r: capnp::text::Reader<'_>) -> capnp::Result<Option<String>> {
    let s = r.to_string()?;
    Ok((!s.is_empty()).then_some(s))
}

/// Write the envelope of an object.
pub fn write_envelope(mut out: wire::Builder<'_>, e: &Envelope) {
    write_object_id(out.reborrow().init_id(), e.id);
    out.set_iri(e.iri.as_deref().unwrap_or(""));
    out.set_type(e.object_type.as_str());
    out.set_name(e.name.as_str());
    out.set_attributed_to(e.attributed_to.as_deref().unwrap_or(""));
    set_list(out.reborrow().init_to(len(&e.to)), &e.to);
    set_list(out.reborrow().init_cc(len(&e.cc)), &e.cc);
    set_list(out.reborrow().init_bto(len(&e.bto)), &e.bto);
    set_list(out.reborrow().init_bcc(len(&e.bcc)), &e.bcc);
    set_list(out.reborrow().init_audience(len(&e.audience)), &e.audience);
    out.set_context(e.context.as_deref().unwrap_or(""));
    out.set_in_reply_to(e.in_reply_to.as_deref().unwrap_or(""));
    out.reborrow()
        .init_published()
        .set_unix_millis(to_millis(e.published));
    out.reborrow()
        .init_updated()
        .set_unix_millis(to_millis(e.updated));
    set_list(
        out.reborrow().init_sensitive(len(&e.sensitive)),
        &e.sensitive,
    );
    out.set_describes(e.describes.as_deref().unwrap_or(""));
    if let Some(t) = e.start_time {
        out.reborrow()
            .init_start_time()
            .set_unix_millis(to_millis(t));
    }
    if let Some(t) = e.end_time {
        out.reborrow().init_end_time().set_unix_millis(to_millis(t));
    }
}

/// Read an envelope.
///
/// # Errors
/// The message is malformed, or the id or a timestamp is invalid.
pub fn read_envelope(r: wire::Reader<'_>) -> capnp::Result<Envelope> {
    let mut e = Envelope::new(r.get_type()?.to_string()?, r.get_name()?.to_string()?);
    e.id = read_object_id(r.get_id()?)?;
    e.iri = optional(r.get_iri()?)?;
    e.attributed_to = optional(r.get_attributed_to()?)?;
    e.to = list(r.get_to()?)?;
    e.cc = list(r.get_cc()?)?;
    e.bto = list(r.get_bto()?)?;
    e.bcc = list(r.get_bcc()?)?;
    e.audience = list(r.get_audience()?)?;
    e.context = optional(r.get_context()?)?;
    e.in_reply_to = optional(r.get_in_reply_to()?)?;
    e.published = from_millis(r.get_published()?.get_unix_millis())?;
    e.updated = from_millis(r.get_updated()?.get_unix_millis())?;
    e.sensitive = list(r.get_sensitive()?)?;
    e.describes = optional(r.get_describes()?)?;
    e.start_time = if r.has_start_time() {
        Some(from_millis(r.get_start_time()?.get_unix_millis())?)
    } else {
        None
    };
    e.end_time = if r.has_end_time() {
        Some(from_millis(r.get_end_time()?.get_unix_millis())?)
    } else {
        None
    };
    Ok(e)
}
