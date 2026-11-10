//! Where a program listens and how it is reached: one implementation per system behind [`Endpoint`].
//! Pattern: Facade (declarations).

mod endpoint;
mod socket;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub(super) use endpoint::{Accepted, Endpoint};
pub use socket::{BoundSocket, bind_socket, companion_path, default_socket};
#[cfg(unix)]
pub(super) use unix::Unix as System;
#[cfg(windows)]
pub(super) use windows::Windows as System;
