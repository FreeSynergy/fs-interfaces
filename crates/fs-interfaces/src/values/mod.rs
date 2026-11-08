//! Contact values that are valid when they exist: [`EmailAddress`], [`PhoneNumber`].
//!
//! Pattern: Value object (coding.md §2), in the shared contracts so every program reaches it without fs-people — small, immutable, compared by value, valid on construction.
//! The trait [`ContactValue`] is what a form needs of one: make it from what was typed, or say why
//! not, as a [`ValueProblem`] code. The code is not a sentence: the program's words turn it into
//! one (ADR 0057). A frontend checks as the person types, the core checks again at intake.

mod email;
mod phone;
mod problem;
mod value;

pub use email::EmailAddress;
pub use phone::PhoneNumber;
pub use problem::ValueProblem;
pub use value::ContactValue;
