//! `Text` and `MultilineText`: plain text, composed (NFC), without control characters.
//!
//! Pattern: Value object. No standard beyond Unicode (UAX #15 NFC); the limits are ours and provisional:
//! 4096 characters in a line, 65536 in a multi-line text.

use fs_interfaces::types_capnp::BasicType;
use unicode_normalization::UnicodeNormalization;

use crate::value::one_line;
use crate::{BasicValue, TypeProblem};

/// Longest one-line text, in characters.
pub const TEXT_MAX: usize = 4096;
/// Longest multi-line text, in characters.
pub const MULTILINE_MAX: usize = 65_536;

/// One line of text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Text(String);

impl BasicValue for Text {
    const KIND: BasicType = BasicType::Text;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        one_line(typed, TEXT_MAX).map(Self)
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }
}

/// Several lines of text; line ends are `\n`, trailing blanks of a line and blank lines at both ends are dropped.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MultilineText(String);

impl BasicValue for MultilineText {
    const KIND: BasicType = BasicType::MultilineText;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        let unified = typed.replace("\r\n", "\n").replace('\r', "\n");
        if unified
            .chars()
            .any(|c| crate::value::bad_char(c) && c != '\n' && c != '\t')
        {
            return Err(TypeProblem::Character);
        }
        let lines: Vec<&str> = unified.split('\n').map(str::trim_end).collect();
        let text = lines.join("\n");
        let text: String = text.trim_matches('\n').nfc().collect();
        if text.trim().is_empty() {
            return Err(TypeProblem::Empty);
        }
        if text.chars().count() > MULTILINE_MAX {
            return Err(TypeProblem::TooLong);
        }
        Ok(Self(text))
    }

    fn canonical(&self) -> String {
        self.0.clone()
    }
}
