//! RFC 9457 problems as RPC errors: the declaration of the parts, nothing else.
//!
//! Pattern: none, this file only declares and re-exports; the patterns are named in the parts.

mod answers;
mod ext;
pub mod keys;
mod value;
mod wire;

pub use answers::{
    INVALID_REQUEST, TOO_LARGE, failed, invalid_request, is_unimplemented, too_large, too_large_by,
    unsupported,
};
pub use ext::{MAX_EXTENSIONS, MAX_KEY_BYTES};
pub use value::Problem;
pub use wire::{MAX_DETAIL_BYTES, capped};
