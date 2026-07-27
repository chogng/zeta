//! Typed construction of Bubblewrap process invocations.
//!
//! This crate deliberately contains no Zeta sandbox policy. It only translates explicit,
//! already-authorized construction choices into an argv vector without invoking a shell.

mod builder;

pub use builder::{BwrapCommand, BwrapCommandBuilder, MountAccess};

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
