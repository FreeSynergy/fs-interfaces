//! `TypeProblem`: why typed text is not a value of a basic type.
//!
//! Pattern: Value object — a plain code, compared by value. The sentence is the program's (ADR 0057,
//! `coding.md` §2a): [`TypeProblem::code`] is the translation key suffix, never shown as it is.

use fs_interfaces::values::ValueProblem;

/// Why typed text is not (yet) a value. The variants are stable codes for the words of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeProblem {
    /// Nothing typed.
    Empty,
    /// Longer than the type allows.
    TooLong,
    /// A character the type does not hold (control characters; a line break in a one-line text).
    Character,
    /// The e-mail address or phone number breaks a rule; the code of the shared contact value.
    Contact(ValueProblem),
    /// Not a well-formed BCP 47 language tag.
    BadLanguage,
    /// Not a URI or IRI (RFC 3986/3987).
    BadUri,
    /// A scheme that runs code or embeds a document when opened (`javascript:`, `vbscript:`; for a link also `data:`).
    UnsafeScheme,
    /// A URI or IRI must be absolute: it needs a scheme (`https:`).
    NotAbsolute,
    /// Not a date in the form `YYYY-MM-DD`, or the day does not exist.
    BadDate,
    /// Not an RFC 3339 date and time with an offset (`2026-11-10T09:30:00+01:00` or `…Z`).
    BadDateTime,
    /// Not a `geo:` URI (RFC 5870), or a coordinate system other than WGS-84.
    BadPlace,
    /// Latitude outside -90..90 or longitude outside -180..180.
    PlaceRange,
    /// Not an OpenPGP fingerprint (40 or 64 hexadecimal digits).
    BadFingerprint,
    /// Not one of the allowed entries.
    NotInList,
    /// A picture needs a media type `image/…`.
    NotImage,
    /// A keyword with a character it may not hold (comma, control character) or too long.
    BadKeyword,
    /// A name or an address with neither a full text nor a component.
    NoParts,
    /// A country code is two letters A–Z (ISO 3166-1 alpha-2).
    BadCountryCode,
}

impl TypeProblem {
    /// The stable code, as a translation key suffix (`problem.<code>`).
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::TooLong => "too-long",
            Self::Character => "character",
            Self::Contact(p) => contact_code(p),
            Self::BadLanguage => "language.bad-tag",
            Self::BadUri => "uri.bad",
            Self::NotAbsolute => "uri.not-absolute",
            Self::UnsafeScheme => "uri.unsafe-scheme",
            Self::BadDate => "date.bad",
            Self::BadDateTime => "date-time.bad",
            Self::BadPlace => "place.bad",
            Self::PlaceRange => "place.range",
            Self::BadFingerprint => "key.bad-fingerprint",
            Self::NotInList => "choice.not-in-list",
            Self::NotImage => "image.not-image",
            Self::BadKeyword => "keywords.bad",
            Self::NoParts => "no-parts",
            Self::BadCountryCode => "address.bad-country-code",
        }
    }
}

fn contact_code(p: ValueProblem) -> &'static str {
    match p {
        ValueProblem::Empty => "empty",
        ValueProblem::MissingAt => "email.missing-at",
        ValueProblem::ManyAt => "email.many-at",
        ValueProblem::EmptyLocal => "email.empty-local",
        ValueProblem::EmptyDomain => "email.empty-domain",
        ValueProblem::LocalCharacter => "email.local-character",
        ValueProblem::LocalTooLong => "email.local-too-long",
        ValueProblem::BadDomain => "email.bad-domain",
        ValueProblem::DomainWithoutDot => "email.domain-without-dot",
        ValueProblem::AddressTooLong => "email.too-long",
        ValueProblem::NoCountryCode => "phone.no-country-code",
        ValueProblem::PhoneCharacter => "phone.character",
        ValueProblem::PhoneLength => "phone.length",
    }
}

impl std::fmt::Display for TypeProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "problem.{}", self.code())
    }
}

impl std::error::Error for TypeProblem {}

impl From<ValueProblem> for TypeProblem {
    fn from(p: ValueProblem) -> Self {
        if p == ValueProblem::Empty {
            Self::Empty
        } else {
            Self::Contact(p)
        }
    }
}
