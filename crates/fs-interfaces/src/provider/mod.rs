//! The provider rule (ADR 0045): like a parent class, every provider offers **all** methods and
//! attributes of its interface. If something is missing we first look for a way to make it work.
//! If there truly is none, the provider answers explicitly "not possible": [`Unsupported`].
//!
//! In Rust the interface is a trait whose methods all have a default body returning
//! `Err(Unsupported)`; a provider overrides what it can do and reports it as [`Capabilities`].
//!
//! Pattern: Null Object - a default method answering [`Unsupported`] is the explicit "not possible".
//!
//! ```
//! use fs_interfaces::{Capabilities, Unsupported};
//!
//! /// Topic interface "messages". Every provider offers every method.
//! trait Messages {
//!     fn send(&self, text: &str) -> Result<(), Unsupported>;
//!
//!     fn edit(&self, _id: &str, _text: &str) -> Result<(), Unsupported> {
//!         Err(Unsupported::new("message.edit", "a sent e-mail cannot be changed"))
//!     }
//!
//!     fn capabilities(&self) -> Capabilities;
//! }
//!
//! struct Email;
//! impl Messages for Email {
//!     fn send(&self, _text: &str) -> Result<(), Unsupported> { Ok(()) }
//!     fn capabilities(&self) -> Capabilities {
//!         let mut caps = Capabilities::new("email");
//!         caps.supported("message.send");
//!         caps.unsupported("message.edit", "a sent e-mail cannot be changed");
//!         caps
//!     }
//! }
//!
//! let t = Email;
//! assert!(t.send("hi").is_ok());
//! let err = t.edit("1", "x").unwrap_err();
//! assert_eq!(err.capability(), "message.edit");
//! assert_eq!(err.status(), 501);
//! assert!(!t.capabilities().is_supported("message.edit"));
//! ```

mod capabilities;
mod unsupported;

pub use capabilities::{Capabilities, Support};
pub use unsupported::{UNSUPPORTED_PROBLEM_TYPE, Unsupported};
