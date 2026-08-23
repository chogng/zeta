//! Cross-platform synchronous Unix domain socket types.
//!
//! Unix targets use the Rust standard library. Windows uses `uds_windows` while the equivalent
//! standard-library API remains unstable.

#[cfg(unix)]
pub use std::os::unix::net::UnixListener;
#[cfg(unix)]
pub use std::os::unix::net::UnixStream;
#[cfg(windows)]
pub use uds_windows::UnixListener;
#[cfg(windows)]
pub use uds_windows::UnixStream;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
