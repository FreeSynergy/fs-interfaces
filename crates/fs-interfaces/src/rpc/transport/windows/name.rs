//! The names a pipe and its companion files get on Windows. Pattern: Value object (a name that cannot collide).
//!
//! Pipe names do not tell upper from lower case, and a name may be 256 characters long. So the encoding keeps only `a-z`, `0-9`
//! and `-`, writes every other byte as `_xx`, and cuts a long name with a hash of the whole: two different texts never give the
//! same name, whatever they contain.

use std::fmt::Write as _;
use std::path::Path;

pub(super) const PREFIX: &str = r"\\.\pipe\";
/// The longest an encoded part may be (a pipe name has 256 characters in all).
const MAX_PART: usize = 120;

/// FNV-1a: written out, because the name must be the same in every build of the program, the client's and the server's.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Lower-case letters, digits and `-` stay; every other byte becomes `_` and two hex digits.
#[must_use]
pub(super) fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "_{b:02x}");
        }
    }
    if out.len() > MAX_PART {
        out.truncate(MAX_PART - 17);
        let _ = write!(out, "-{:016x}", fnv(text));
    }
    out
}

/// `\\.\pipe\freesynergy-<sid>-<program>`.
#[must_use]
pub(super) fn default_name(sid: &str, program: &str) -> String {
    format!("{PREFIX}freesynergy-{sid}-{}", encode(program))
}

/// A path that already is a pipe name stays; any other path (a `--socket` given as a file) becomes a pipe name made from it,
/// with `_` where the name of a program has `-`, so the two kinds never meet.
#[must_use]
pub(super) fn pipe_name(sid: &str, path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.starts_with(PREFIX) {
        text.into_owned()
    } else {
        format!("{PREFIX}freesynergy-{sid}_{}", encode(&text))
    }
}

/// The file name (without extension) of a file that belongs to the pipe `name`.
#[must_use]
pub(super) fn companion_stem(name: &str) -> String {
    encode(name.strip_prefix(PREFIX).unwrap_or(name))
}
