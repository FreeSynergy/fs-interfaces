//! `DescribesType`: the trait a data type implements to report itself.
//!
//! Pattern: Registry entry point (`coding.md` §2) — a program lists each data type it owns with
//! `TypeRegistry::register::<T>()`. Trait first (ADR 0043): the generated frontend depends on this and on
//! [`TypeDescription`], not on the program's types.

use super::TypeDescription;

/// A data type that can say what it is made of and what can be done with it.
pub trait DescribesType {
    /// The description: name, owner program, standard, fields and commands. The same on every call.
    fn describe() -> TypeDescription;
}
