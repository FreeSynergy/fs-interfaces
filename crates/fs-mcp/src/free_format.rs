//! Which fields carry free-format content (a whole object or text in a specialist or display format).
//!
//! Pattern: Specification — a pure predicate over a name and a type. Trait: none.

use crate::TypeRef;

/// Names of fields that carry a whole object or text in a specialist format (`JSContact`, `JSCalendar`, `GeoJSON`,
/// AS2, Markdown, HTML, Sieve, notes and comments). Matched whole, case-insensitive, on `Text` fields only.
const FREE_FORMAT_NAMES: &[&str] = &[
    "jscontact",
    "jscalendar",
    "geojson",
    "as2",
    "document",
    "details",
    "markdown",
    "notes",
    "comment",
    "comments",
    "bodytext",
    "bodyhtml",
    "reasoning",
    "sieve",
    "html",
    "note",
    "text",
    "preview",
    "summary",
    "detail",
    "result",
    "value",
    "body",
    "content",
    "message",
    "description",
    "memo",
    "template",
    "script",
];

/// Is this a free-format content field? Such a field needs a written `[format]` mark; it is not a secret
/// field, but it can hold whatever a person wrote into it.
#[must_use]
pub fn looks_free_format(name: &str, ty: Option<&TypeRef>) -> bool {
    let text = matches!(ty, Some(TypeRef::Text))
        || matches!(ty, Some(TypeRef::List(inner)) if **inner == TypeRef::Text);
    text && FREE_FORMAT_NAMES.contains(&name.to_lowercase().as_str())
}
