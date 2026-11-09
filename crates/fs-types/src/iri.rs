//! Link and Reference: absolute URIs and IRIs.
//!
//! Pattern: Adapter over the free crate `iri-string` (RFC 3986 URI, RFC 3987 IRI, syntax-based normalisation). A
//! `Link` is normalised (scheme and host in lower case, dot segments removed) because two spellings of one link
//! should be one value; a `Reference` is an identity (AS2 `id`) and is kept exactly as written.

use fs_interfaces::types_capnp::BasicType;
use iri_string::types::IriStr;

use crate::{BasicValue, TypeProblem};

/// Longest URI we accept, in bytes (the usual practical limit of servers).
pub const URI_MAX: usize = 2048;

/// Check that `typed` is an absolute IRI and return it trimmed.
pub(crate) fn absolute(typed: &str) -> Result<&str, TypeProblem> {
    let t = typed.trim();
    if t.is_empty() {
        return Err(TypeProblem::Empty);
    }
    if t.len() > URI_MAX {
        return Err(TypeProblem::TooLong);
    }
    if IriStr::new(t).is_ok() {
        return Ok(t);
    }
    // Distinguish "no scheme" from "broken": a relative reference is a well-formed but not absolute.
    if iri_string::types::IriReferenceStr::new(t).is_ok() {
        Err(TypeProblem::NotAbsolute)
    } else {
        Err(TypeProblem::BadUri)
    }
}

/// A link: an absolute URI or IRI, normalised.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Link(String);

impl BasicValue for Link {
    const KIND: BasicType = BasicType::Link;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let t = absolute(typed)?;
        let iri = IriStr::new(t).map_err(|_| TypeProblem::BadUri)?;
        Ok(Self(iri.normalize().to_string()))
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }

    /// An `https://` start for typed text that has no scheme and looks like a host name (`example.org/x`).
    fn suggest(typed: &str, _known: &[&str]) -> Vec<String> {
        let t = typed.trim();
        let host = t.split('/').next().unwrap_or("");
        if t.is_empty() || t.contains(':') || !host.contains('.') || host.starts_with('.') {
            return Vec::new();
        }
        let candidate = format!("https://{t}");
        if Self::check(&candidate).is_ok() {
            vec![candidate]
        } else {
            Vec::new()
        }
    }
}

/// A reference to an object: the absolute IRI that is its AS2 `id`, kept as written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reference(String);

impl BasicValue for Reference {
    const KIND: BasicType = BasicType::Reference;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        absolute(typed).map(|t| Self(t.to_owned()))
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }

    /// Known ids that contain what is typed, shortest first.
    fn suggest(typed: &str, known: &[&str]) -> Vec<String> {
        let t = typed.trim();
        if t.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<String> = known
            .iter()
            .filter(|k| k.contains(t) && **k != t)
            .map(|k| (*k).to_owned())
            .collect();
        out.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
        out
    }
}
