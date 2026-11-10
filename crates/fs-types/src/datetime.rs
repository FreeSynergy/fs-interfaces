//! `Date` and `DateTime`: RFC 3339.
//!
//! Pattern: Adapter over `chrono` (parsing and calendar arithmetic), Value object around it. A `DateTime` is an
//! instant with its offset; a date-time without offset (JSCalendar `LocalDateTime`) and time zones need the IANA
//! database and are `Unsupported` in this version (DESIGN.md).

use chrono::{DateTime as ChronoDateTime, NaiveDate, SecondsFormat};
use fs_interfaces::types_capnp::BasicType;

use crate::{BasicValue, TypeProblem};

/// A calendar date, `YYYY-MM-DD`, that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date(NaiveDate);

impl Date {
    /// The date as `chrono` knows it, for calculations.
    #[must_use]
    pub fn as_naive(&self) -> NaiveDate {
        self.0
    }
}

impl BasicValue for Date {
    const KIND: BasicType = BasicType::Date;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(TypeProblem::Empty);
        }
        let b = t.as_bytes();
        let shape = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !shape {
            return Err(TypeProblem::BadDate);
        }
        NaiveDate::parse_from_str(t, "%Y-%m-%d")
            .map(Self)
            .map_err(|_| TypeProblem::BadDate)
    }

    fn canonical(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}

/// An instant with the offset it was written in; the canonical text has `T`, and `Z` for UTC.
#[derive(Clone, Copy, Debug)]
pub struct DateTime(ChronoDateTime<chrono::FixedOffset>);

// Equal means equal canonical text: the same instant written in another offset is another value (chrono alone would
// call them equal).
impl PartialEq for DateTime {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.0.offset() == other.0.offset()
    }
}

impl Eq for DateTime {}

impl std::hash::Hash for DateTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.0.offset().local_minus_utc().hash(state);
    }
}

impl DateTime {
    /// The instant as `chrono` knows it, for calculations.
    #[must_use]
    pub fn as_chrono(&self) -> ChronoDateTime<chrono::FixedOffset> {
        self.0
    }
}

impl BasicValue for DateTime {
    const KIND: BasicType = BasicType::DateTime;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(TypeProblem::Empty);
        }
        // RFC 3339: `-00:00` means "offset unknown", which `Z` would turn into "UTC"; not representable, so refused.
        if t.ends_with("-00:00") {
            return Err(TypeProblem::BadDateTime);
        }
        ChronoDateTime::parse_from_rfc3339(t)
            .map(Self)
            .map_err(|_| TypeProblem::BadDateTime)
    }

    fn canonical(&self) -> String {
        self.0.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    }
}
