//! `ValueProblem`: why typed text is not a contact value.
//!
//! Pattern: Value object — a plain code, compared by value; the sentence is the program's.

/// Why typed text is not (yet) a value. The variants are stable codes for the words of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueProblem {
    /// Nothing typed.
    Empty,
    /// An e-mail address needs an `@`.
    MissingAt,
    /// More than one `@`.
    ManyAt,
    /// Nothing before the `@`.
    EmptyLocal,
    /// Nothing after the `@`.
    EmptyDomain,
    /// A character that may not stand before the `@`.
    LocalCharacter,
    /// The part before the `@` is longer than 64 characters.
    LocalTooLong,
    /// The domain is not a valid host name (also after conversion from a Unicode name).
    BadDomain,
    /// The domain has no dot: it names no public host.
    DomainWithoutDot,
    /// A phone number needs `+`, then a country code that does not start with 0, and the number (E.164).
    NoCountryCode,
    /// A character a phone number does not hold.
    PhoneCharacter,
    /// The whole address is longer than 254 characters.
    AddressTooLong,
    /// Fewer than 7 or more than 15 digits.
    PhoneLength,
}
