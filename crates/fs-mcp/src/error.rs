//! Errors of reading contracts and mark files.
//!
//! Pattern: none (one plain error enum). Trait: `std::error::Error`.

use std::fmt;

/// Why contracts or marks could not be loaded. Never swallowed: a half-read contract would hide tools
/// or, worse, hide the exclusion of one.
#[derive(Debug)]
pub enum Error {
    /// The schema compiler refused the schemas.
    Compile(String),
    /// The compiler's output could not be read.
    Reflect(String),
    /// A mark file is not valid.
    Marks {
        /// File name.
        file: String,
        /// What is wrong.
        problem: String,
    },
    /// Two tools would get the same name.
    DuplicateName(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(m) => write!(f, "schema compilation failed: {m}"),
            Self::Reflect(m) => write!(f, "cannot read the compiler output: {m}"),
            Self::Marks { file, problem } => write!(f, "mark file {file}: {problem}"),
            Self::DuplicateName(n) => write!(f, "two tools are named {n}"),
        }
    }
}

impl std::error::Error for Error {}
